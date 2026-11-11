//! The SFTP page: two file panes, each this machine or a host, and copies
//! between them.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::ops::Range;
use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::progress::Progress;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Sizable as _, WindowExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use shared::SshHostRecord;

use crate::runtime::sftp::{
    FileEntry, FileType, TransferComplete, TransferDirection, TransferError, TransferProgress,
};
use crate::runtime::ssh::SshConnectResponse;
use crate::sftp as commands;
use crate::trust::{HostTrustConfirmation, HostTrustMismatch, HostTrustPrompt};
use crate::ui::bridge::backend;
use crate::ui::i18n::{t, tf};
use crate::ui::icons::lucide;
use crate::ui::store::{store, Route, SessionStatus, Store, StoreEvent};
use crate::ui::terminal::SessionKind;
use crate::ui::theme;
use crate::ui::views::common;

const LOCAL_MACHINE: &str = "local";
const ROW_HEIGHT: Pixels = px(34.);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

impl Side {
    fn index(self) -> usize {
        self as usize
    }

    fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

#[derive(Clone)]
struct Connection {
    name: String,
    host: String,
    port: u16,
    username: String,
}

impl Connection {
    fn address(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        format!("{}@{host}:{}", self.username, self.port)
    }
}

/// Identifies a machine so both panes never show the same one.
fn machine_key(host: &str, port: u16) -> String {
    format!("{host}:{port}")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Name,
    Size,
    Modified,
}

/// One side of the file browser: this machine, or an SFTP session to a host.
struct Pane {
    local: bool,
    path: String,
    /// The listing, in display order.
    files: Vec<FileEntry>,
    loading: bool,
    /// Names of the selected rows.
    selected: Vec<String>,
    /// Row a Shift+click range starts from.
    anchor: Option<String>,
    sort: (SortKey, bool),
    sftp_session: Option<String>,
    /// The terminal session this pane borrowed its connection from.
    ssh_session: Option<String>,
    /// Connected on its own rather than through a terminal session.
    direct: bool,
    connection: Option<Connection>,
    connection_id: Option<String>,
    connection_error: Option<String>,
    trust_prompt: Option<HostTrustPrompt>,
    trust_mismatch: Option<HostTrustMismatch>,
    connecting: bool,
    closing: bool,
    trust_confirming: bool,
    trust_error: Option<String>,
    /// Bumped whenever the pane changes machine, to drop replies meant for
    /// the one it left.
    generation: u64,
    /// Bumped per listing, so only the latest one lands.
    request: u64,
    path_input: Entity<InputState>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
}

impl Pane {
    fn new(local: bool, window: &mut Window, cx: &mut App) -> Self {
        Self {
            local,
            path: String::new(),
            files: Vec::new(),
            loading: false,
            selected: Vec::new(),
            anchor: None,
            sort: (SortKey::Name, true),
            sftp_session: None,
            ssh_session: None,
            direct: false,
            connection: None,
            connection_id: None,
            connection_error: None,
            trust_prompt: None,
            trust_mismatch: None,
            connecting: false,
            closing: false,
            trust_confirming: false,
            trust_error: None,
            generation: 0,
            request: 0,
            path_input: cx.new(|cx| InputState::new(window, cx)),
            focus: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
        }
    }

    /// Has a listing that files can be copied to or from.
    fn is_ready(&self) -> bool {
        self.local || self.sftp_session.is_some()
    }

    fn label(&self) -> SharedString {
        match (&self.connection, self.local) {
            (_, true) => t("Local"),
            (Some(connection), false) => connection.name.clone().into(),
            (None, false) => t("Remote"),
        }
    }

    /// Machine this pane shows or is connecting to; `None` while picking.
    fn machine(&self) -> Option<String> {
        if self.local {
            return Some(LOCAL_MACHINE.to_string());
        }
        self.connection
            .as_ref()
            .map(|connection| machine_key(&connection.host, connection.port))
    }

    /// Where file commands go: `None` is this machine, else the SFTP session.
    /// A remote pane without a session is an error, never a local path.
    fn target(&self) -> Result<Option<String>, String> {
        if self.local {
            return Ok(None);
        }
        self.sftp_session
            .clone()
            .map(Some)
            .ok_or_else(|| t("SFTP session is not connected.").to_string())
    }

    fn clear_listing(&mut self) {
        self.path.clear();
        self.files.clear();
        self.selected.clear();
        self.anchor = None;
        self.loading = false;
    }

    fn sort_files(&mut self) {
        let (key, ascending) = self.sort;
        self.files.sort_by(|left, right| {
            let folders_first =
                (right.file_type == FileType::Dir).cmp(&(left.file_type == FileType::Dir));
            let by_key = match key {
                SortKey::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
                SortKey::Size => left.size.cmp(&right.size),
                SortKey::Modified => left.modified.unwrap_or(0).cmp(&right.modified.unwrap_or(0)),
            };
            folders_first.then(if ascending { by_key } else { by_key.reverse() })
        });
    }
}

/// A copy waiting for the user to choose between overwriting and renaming.
struct Conflict {
    id: u64,
    source: Side,
    source_path: String,
    target_path: String,
    renamed_target_path: String,
    file_name: String,
    suggested_name: String,
}

/// Rows being dragged from one pane to the other.
#[derive(Clone)]
struct DraggedEntries {
    side: Side,
    entries: Vec<FileEntry>,
}

/// What follows the pointer during a drag.
struct DragChip(SharedString);

impl Render for DragChip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded(theme::RADIUS)
            .border_1()
            .border_color(theme::STENCIL)
            .bg(theme::RAIL)
            .text_color(theme::STENCIL)
            .text_size(px(11.))
            .child(self.0.clone())
    }
}

pub struct SftpView {
    store: Entity<Store>,
    panes: [Pane; 2],
    transfers: Vec<TransferProgress>,
    /// File name per transfer id, for the transfer bar.
    transfer_names: HashMap<String, String>,
    /// Copies not started yet; they start one at a time.
    queue: VecDeque<(Side, FileEntry)>,
    /// A copy is being started or waits on a conflict; the queue resumes after.
    starting: bool,
    conflict: Option<Conflict>,
    next_conflict_id: u64,
    /// The terminal session the right pane last tried to follow.
    attempted_active: Option<String>,
    /// Whether the page was showing at the last store change.
    visible: bool,
    _subscriptions: Vec<Subscription>,
}

impl SftpView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let panes = [Pane::new(true, window, cx), Pane::new(false, window, cx)];
        let mut subscriptions = vec![
            cx.observe_in(&store, window, |this, _, window, cx| {
                this.on_store_changed(window, cx);
                cx.notify();
            }),
            cx.subscribe_in(&store, window, |this, _, event: &StoreEvent, window, cx| {
                if let StoreEvent::Backend(name, payload) = event {
                    this.on_backend_event(name, payload, window, cx);
                }
            }),
        ];
        for side in [Side::Left, Side::Right] {
            let input = &panes[side.index()].path_input;
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                move |this, input, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } => {
                        let path = input.read(cx).value().trim().to_string();
                        if !path.is_empty() {
                            this.navigate(side, path, window, cx);
                        }
                    }
                    // An abandoned edit goes back to the path being shown.
                    InputEvent::Blur => {
                        let path = this.panes[side.index()].path.clone();
                        input.update(cx, |input, cx| input.set_value(path, window, cx));
                    }
                    InputEvent::Change | InputEvent::Focus => {}
                },
            ));
        }
        let mut view = Self {
            store,
            panes,
            transfers: Vec::new(),
            transfer_names: HashMap::new(),
            queue: VecDeque::new(),
            starting: false,
            conflict: None,
            next_conflict_id: 0,
            attempted_active: None,
            visible: true,
            _subscriptions: subscriptions,
        };
        view.navigate(Side::Left, "~".to_string(), window, cx);
        view.follow_active_session(window, cx);
        view
    }

    fn pane(&mut self, side: Side) -> &mut Pane {
        &mut self.panes[side.index()]
    }

    fn on_store_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let visible = self.store.read(cx).route == Route::Sftp;
        // Local listings go stale while the page is away; remote ones are kept.
        if visible && !self.visible {
            for side in [Side::Left, Side::Right] {
                if self.panes[side.index()].local {
                    self.refresh(side, window, cx);
                }
            }
        }
        self.visible = visible;
        if visible {
            self.follow_active_session(window, cx);
        }
    }

    fn on_backend_event(
        &mut self,
        name: &str,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match name {
            "sftp://progress" => {
                let Ok(progress) = serde_json::from_str::<TransferProgress>(payload) else {
                    return;
                };
                match self
                    .transfers
                    .iter_mut()
                    .find(|transfer| transfer.transfer_id == progress.transfer_id)
                {
                    Some(transfer) => *transfer = progress,
                    None => self.transfers.push(progress),
                }
            }
            "sftp://complete" => {
                let Ok(complete) = serde_json::from_str::<TransferComplete>(payload) else {
                    return;
                };
                self.finish_transfer(&complete.transfer_id);
                for side in [Side::Left, Side::Right] {
                    if self.panes[side.index()].is_ready() {
                        self.refresh(side, window, cx);
                    }
                }
            }
            "sftp://error" => {
                let Ok(failed) = serde_json::from_str::<TransferError>(payload) else {
                    return;
                };
                self.finish_transfer(&failed.transfer_id);
                window.push_notification(Notification::error(failed.error), cx);
            }
            _ => return,
        }
        cx.notify();
    }

    fn finish_transfer(&mut self, id: &str) {
        self.transfers.retain(|transfer| transfer.transfer_id != id);
        self.transfer_names.remove(id);
    }

    // --- Listing ------------------------------------------------------------

    fn navigate(&mut self, side: Side, path: String, window: &mut Window, cx: &mut Context<Self>) {
        let app = self.store.read(cx).app();
        let pane = self.pane(side);
        let session = match pane.target() {
            Ok(session) => session,
            Err(error) => return window.push_notification(Notification::warning(error), cx),
        };
        pane.request += 1;
        let (generation, request) = (pane.generation, pane.request);
        if pane.path != path {
            pane.scroll.scroll_to_item(0, ScrollStrategy::Top);
        }
        pane.loading = true;
        pane.path = path.clone();
        pane.selected.clear();
        pane.anchor = None;
        pane.path_input
            .update(cx, |input, cx| input.set_value(path.clone(), window, cx));
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let listed = backend(async move {
                match session {
                    Some(session) => commands::sftp_list_dir(session, path, app.state()).await,
                    None => commands::local_list_dir(path).await,
                }
            })
            .await;
            let _ = this.update_in(cx, |this, window, cx| {
                let pane = this.pane(side);
                if (pane.generation, pane.request) != (generation, request) {
                    return;
                }
                pane.loading = false;
                match listed {
                    Ok(files) => {
                        pane.files = files;
                        pane.sort_files();
                    }
                    Err(error) => window.push_notification(Notification::error(error), cx),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn refresh(&mut self, side: Side, window: &mut Window, cx: &mut Context<Self>) {
        let path = self.pane(side).path.clone();
        self.navigate(side, path, window, cx);
    }

    fn navigate_up(&mut self, side: Side, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(parent) = parent_path(&self.pane(side).path) {
            self.navigate(side, parent, window, cx);
        }
    }

    fn toggle_sort(&mut self, side: Side, key: SortKey, cx: &mut Context<Self>) {
        let pane = self.pane(side);
        pane.sort = if pane.sort.0 == key {
            (key, !pane.sort.1)
        } else {
            (key, true)
        };
        pane.sort_files();
        cx.notify();
    }

    /// Click selects one row, Cmd/Ctrl+click toggles, Shift+click selects a
    /// range, and a double click opens a folder or copies a file across.
    fn click_row(
        &mut self,
        side: Side,
        entry: &FileEntry,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = self.pane(side);
        window.focus(&pane.focus, cx);
        if event.click_count() >= 2 {
            return match entry.file_type {
                FileType::Dir => {
                    let path = join_path(&pane.path, &entry.name);
                    self.navigate(side, path, window, cx)
                }
                _ => self.transfer_many(side, vec![entry.clone()], window, cx),
            };
        }
        let modifiers = event.modifiers();
        let position = |name: &str| pane.files.iter().position(|file| file.name == name);
        let range_start = pane
            .anchor
            .as_deref()
            .filter(|_| modifiers.shift)
            .and_then(position);
        if let (Some(start), Some(end)) = (range_start, position(&entry.name)) {
            pane.selected = pane.files[start.min(end)..=start.max(end)]
                .iter()
                .map(|file| file.name.clone())
                .collect();
        } else {
            pane.anchor = Some(entry.name.clone());
            if !modifiers.secondary(cx) {
                pane.selected = vec![entry.name.clone()];
            } else if let Some(index) = pane.selected.iter().position(|name| *name == entry.name) {
                pane.selected.remove(index);
            } else {
                pane.selected.push(entry.name.clone());
            }
        }
        cx.notify();
    }

    // --- File operations ----------------------------------------------------

    /// Runs a file command, then reloads the pane it changed.
    fn run_file_operation(
        &mut self,
        side: Side,
        operation: impl Future<Output = Result<(), String>> + Send + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |this, cx| {
            let done = backend(operation).await;
            let _ = this.update_in(cx, |this, window, cx| match done {
                Ok(()) => this.refresh(side, window, cx),
                Err(error) => window.push_notification(Notification::error(error), cx),
            });
        })
        .detach();
    }

    /// The pane's file target, or a toast when it has none.
    fn file_target(
        &mut self,
        side: Side,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Option<String>> {
        self.pane(side)
            .target()
            .map_err(|error| window.push_notification(Notification::error(error), cx))
            .ok()
    }

    fn mkdir(&mut self, side: Side, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.file_target(side, window, cx) else {
            return;
        };
        let (app, path) = (
            self.store.read(cx).app(),
            join_path(&self.pane(side).path, &name),
        );
        let operation = async move {
            match session {
                Some(session) => commands::sftp_mkdir(session, path, app.state()).await,
                None => commands::local_mkdir(path).await,
            }
        };
        self.run_file_operation(side, operation, window, cx);
    }

    fn rename(
        &mut self,
        side: Side,
        old_name: &str,
        new_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.file_target(side, window, cx) else {
            return;
        };
        let app = self.store.read(cx).app();
        let folder = &self.pane(side).path;
        let (old_path, new_path) = (join_path(folder, old_name), join_path(folder, &new_name));
        let operation = async move {
            match session {
                Some(session) => {
                    commands::sftp_rename(session, old_path, new_path, app.state()).await
                }
                None => commands::local_rename(old_path, new_path).await,
            }
        };
        self.run_file_operation(side, operation, window, cx);
    }

    fn remove(
        &mut self,
        side: Side,
        names: Vec<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.file_target(side, window, cx) else {
            return;
        };
        let app = self.store.read(cx).app();
        let folder = &self.pane(side).path;
        let paths: Vec<String> = names.iter().map(|name| join_path(folder, name)).collect();
        let operation = async move {
            for path in paths {
                match &session {
                    Some(session) => {
                        commands::sftp_remove(session.clone(), path, app.state()).await?
                    }
                    None => commands::local_remove(path).await?,
                }
            }
            Ok(())
        };
        self.run_file_operation(side, operation, window, cx);
    }

    /// Asks for a new folder's name, or a new name for `renaming`.
    fn prompt_name(
        &mut self,
        side: Side,
        renaming: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = renaming.clone().unwrap_or_default();
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(tf("e.g. {example}", &[("example", "backups")]))
                .default_value(current)
        });
        let error: Rc<RefCell<Option<SharedString>>> = Rc::default();
        let view = cx.entity();
        let field = input.clone();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (view, field, error, renaming) =
                (view.clone(), field.clone(), error.clone(), renaming.clone());
            let (title, label, confirm) = match renaming {
                Some(_) => (t("Rename item"), t("New name"), t("Rename")),
                None => (t("Create folder"), t("Folder name"), t("Create")),
            };
            let description = renaming
                .as_ref()
                .map(|name| tf("Choose a new name for {name}.", &[("name", name)]));
            let message = error.borrow().clone();
            alert
                .confirm()
                .title(title)
                .when_some(description, |alert, description| {
                    alert.description(description)
                })
                .child(
                    v_flex()
                        .gap_2()
                        .child(common::section_title(label))
                        .child(Input::new(&field))
                        .children(message.map(|message| {
                            div().text_xs().text_color(theme::RED_TEXT).child(message)
                        })),
                )
                .ok_text(confirm)
                .cancel_text(t("Cancel"))
                // Enter in the field confirms too: the dialog binds it.
                .on_ok(move |_, window, cx| {
                    match validate_name(&field.read(cx).value(), renaming.as_deref()) {
                        Ok(name) => {
                            view.update(cx, |this, cx| match &renaming {
                                Some(old_name) => this.rename(side, old_name, name, window, cx),
                                None => this.mkdir(side, name, window, cx),
                            });
                            true
                        }
                        Err(message) => {
                            *error.borrow_mut() = Some(t(message));
                            window.refresh();
                            false
                        }
                    }
                })
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
    }

    fn confirm_remove(&mut self, side: Side, window: &mut Window, cx: &mut Context<Self>) {
        let names = self.pane(side).selected.clone();
        let item_name: SharedString = match names.as_slice() {
            [] => return,
            [name] => name.clone().into(),
            _ => tf("{count} items", &[("count", &names.len().to_string())]),
        };
        let view = cx.entity();
        common::confirm_delete(
            t("Delete this item?"),
            t("This action cannot be undone. The item will be permanently removed."),
            Some(item_name),
            t("Delete"),
            window,
            cx,
            move |window, cx| {
                view.update(cx, |this, cx| this.remove(side, names.clone(), window, cx))
            },
        );
    }

    // --- Transfers ----------------------------------------------------------

    /// Copies `entries` into the other pane's current folder, in order.
    fn transfer_many(
        &mut self,
        source: Side,
        entries: Vec<FileEntry>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.queue
            .extend(entries.into_iter().map(|entry| (source, entry)));
        if !self.starting {
            self.start_next_transfer(window, cx);
        }
    }

    /// Starts the next queued copy. A name conflict pauses the queue until the
    /// user answers; a copy that cannot start drops the rest.
    fn start_next_transfer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.starting = false;
        let Some((side, entry)) = self.queue.pop_front() else {
            return;
        };
        let (source, target) = (&self.panes[side.index()], &self.panes[side.other().index()]);
        let refusal = if source.machine().is_some() && source.machine() == target.machine() {
            Some(t(
                "Both sides are the same machine. Pick a different one on one side.",
            ))
        } else if !source.is_ready() || !target.is_ready() {
            Some(t(
                "Connect to a server on both sides before transferring files",
            ))
        } else {
            None
        };
        if let Some(refusal) = refusal {
            self.queue.clear();
            return window.push_notification(Notification::warning(refusal), cx);
        }
        let source_path = join_path(&source.path, &entry.name);
        let target_path = join_path(&target.path, &entry.name);
        let suggested_name = next_available_name(&entry.name, &target.files);
        self.starting = true;
        if suggested_name == entry.name {
            return self.start_transfer(side, source_path, target_path, entry.name, window, cx);
        }
        let conflict = Conflict {
            id: self.next_conflict_id,
            source: side,
            renamed_target_path: join_path(&target.path, &suggested_name),
            source_path,
            target_path,
            file_name: entry.name,
            suggested_name,
        };
        self.next_conflict_id += 1;
        self.open_conflict_dialog(&conflict, entry.file_type == FileType::Dir, window, cx);
        self.conflict = Some(conflict);
    }

    fn start_transfer(
        &mut self,
        side: Side,
        source_path: String,
        target_path: String,
        file_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let app = self.store.read(cx).app();
        let (source, target) = (&self.panes[side.index()], &self.panes[side.other().index()]);
        let (source_target, target_target) = (source.target(), target.target());
        let message = if source.local {
            tf("Uploading {name}...", &[("name", &file_name)])
        } else if target.local {
            tf("Downloading {name}...", &[("name", &file_name)])
        } else {
            tf("Copying {name}...", &[("name", &file_name)])
        };
        cx.spawn_in(window, async move |this, cx| {
            let started = backend(async move {
                match (source_target?, target_target?) {
                    (None, Some(session)) => {
                        commands::sftp_upload(
                            app.clone(),
                            session,
                            source_path,
                            target_path,
                            app.state(),
                            app.state(),
                        )
                        .await
                    }
                    (Some(session), None) => {
                        commands::sftp_download(
                            app.clone(),
                            session,
                            source_path,
                            target_path,
                            app.state(),
                            app.state(),
                        )
                        .await
                    }
                    (Some(source), Some(target)) => {
                        commands::sftp_copy(
                            app.clone(),
                            source,
                            source_path,
                            target,
                            target_path,
                            app.state(),
                            app.state(),
                        )
                        .await
                    }
                    (None, None) => Err(t(
                        "Both sides are the same machine. Pick a different one on one side.",
                    )
                    .to_string()),
                }
            })
            .await;
            let _ = this.update_in(cx, |this, window, cx| match started {
                Ok(id) => {
                    this.transfer_names.insert(id, file_name);
                    window.push_notification(Notification::info(message), cx);
                    this.start_next_transfer(window, cx);
                }
                Err(error) => {
                    this.queue.clear();
                    this.starting = false;
                    window.push_notification(Notification::error(error), cx);
                }
            });
        })
        .detach();
    }

    fn cancel_transfer(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let app = self.store.read(cx).app();
        cx.spawn_in(window, async move |_, cx| {
            let cancelled =
                backend(async move { commands::sftp_cancel_transfer(id, app.state()).await }).await;
            if let Err(error) = cancelled {
                let _ = cx
                    .update(|window, cx| window.push_notification(Notification::error(error), cx));
            }
        })
        .detach();
    }

    fn open_conflict_dialog(
        &mut self,
        conflict: &Conflict,
        is_directory: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = &self.panes[conflict.source.other().index()];
        let destination = target.label();
        // Paths inside the folder that already exist; `None` while scanning.
        let conflicting: Rc<RefCell<Option<Vec<String>>>> = Rc::default();
        if is_directory {
            let app = self.store.read(cx).app();
            let source = &self.panes[conflict.source.index()];
            let (source_target, target_target) = (source.target(), target.target());
            let (source_path, target_path) =
                (conflict.source_path.clone(), conflict.target_path.clone());
            let found = conflicting.clone();
            cx.spawn_in(window, async move |_, cx| {
                let scanned = backend(async move {
                    match (source_target?, target_target?) {
                        (Some(source), Some(target)) => {
                            commands::sftp_copy_conflicts(
                                source,
                                source_path,
                                target,
                                target_path,
                                app.state(),
                            )
                            .await
                        }
                        (None, Some(session)) => {
                            commands::sftp_transfer_conflicts(
                                session,
                                TransferDirection::Upload,
                                source_path,
                                target_path,
                                app.state(),
                            )
                            .await
                        }
                        (Some(session), None) => {
                            commands::sftp_transfer_conflicts(
                                session,
                                TransferDirection::Download,
                                source_path,
                                target_path,
                                app.state(),
                            )
                            .await
                        }
                        (None, None) => Ok(Vec::new()),
                    }
                })
                .await;
                if let Err(error) = &scanned {
                    tracing::warn!(error, "SFTP folder conflict scan failed");
                }
                *found.borrow_mut() = Some(scanned.unwrap_or_default());
                let _ = cx.update(|window, _| window.refresh());
            })
            .detach();
        }

        let (view, id) = (cx.entity(), conflict.id);
        let (file_name, suggested_name) =
            (conflict.file_name.clone(), conflict.suggested_name.clone());
        window.open_alert_dialog(cx, move |alert, _, _| {
            let fact = |label: SharedString, value: String| {
                v_flex()
                    .gap_1()
                    .px_4()
                    .py_3()
                    .rounded(theme::RADIUS)
                    .border_1()
                    .border_color(theme::hairline())
                    .bg(theme::RIB)
                    .child(common::section_title(label))
                    .child(
                        div()
                            .truncate()
                            .text_sm()
                            .text_color(theme::STENCIL)
                            .child(value),
                    )
            };
            let inner = is_directory.then(|| {
                let (title, paths) = match conflicting.borrow().as_deref() {
                    None => (t("Checking folder contents…"), Vec::new()),
                    Some([]) => (t("No files inside would be overwritten"), Vec::new()),
                    Some([path]) => (
                        tf(
                            "Overwrite replaces {count} existing file",
                            &[("count", "1")],
                        ),
                        vec![path.clone()],
                    ),
                    Some(paths) => (
                        tf(
                            "Overwrite replaces {count} existing files",
                            &[("count", &paths.len().to_string())],
                        ),
                        paths.to_vec(),
                    ),
                };
                v_flex()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .rounded(theme::RADIUS)
                    .border_1()
                    .border_color(theme::AMBER.opacity(0.4))
                    .bg(theme::RIB)
                    .child(common::section_title(title).text_color(theme::AMBER_TEXT))
                    .when(!paths.is_empty(), |inner| {
                        inner.child(
                            v_flex()
                                .id("conflicting-files")
                                .max_h(px(160.))
                                .overflow_y_scroll()
                                .gap_1()
                                .text_sm()
                                .text_color(theme::STENCIL)
                                .children(
                                    paths.into_iter().map(|path| div().truncate().child(path)),
                                ),
                        )
                    })
            });
            let answer = |button: Button, choice: Option<bool>| {
                let view = view.clone();
                button.on_click(move |_, window, cx| {
                    window.close_dialog(cx);
                    view.update(cx, |this, cx| this.settle_conflict(id, choice, window, cx));
                })
            };
            let (kind, title_kind) = if is_directory {
                (t("folder"), t("Folder"))
            } else {
                (t("file"), t("File"))
            };
            let dismissed = view.clone();
            alert
                .title(tf("{kind} already exists", &[("kind", &title_kind)]))
                .description(tf(
                    "A {kind} named {name} already exists in {destination}.",
                    &[
                        ("kind", &kind),
                        ("name", &file_name),
                        ("destination", &destination),
                    ],
                ))
                .child(
                    v_flex()
                        .gap_3()
                        .child(fact(t("Original"), file_name.clone()))
                        .children(inner)
                        .child(fact(t("Rename option"), suggested_name.clone())),
                )
                .footer(
                    DialogFooter::new()
                        .child(answer(
                            Button::new("conflict-cancel").outline().label(t("Cancel")),
                            None,
                        ))
                        .child(answer(
                            Button::new("conflict-overwrite")
                                .outline()
                                .icon(lucide("file-exclamation-point"))
                                .label(t("Overwrite")),
                            Some(false),
                        ))
                        .child(answer(
                            Button::new("conflict-rename")
                                .primary()
                                .icon(lucide("copy"))
                                .label(t("Rename to copy")),
                            Some(true),
                        )),
                )
                // Escape counts as Cancel. Deferred so that the next conflict's
                // dialog opens after this one has left the stack.
                .on_close(move |_, window, cx| {
                    let view = dismissed.clone();
                    window.defer(cx, move |window, cx| {
                        view.update(cx, |this, cx| this.settle_conflict(id, None, window, cx));
                    });
                })
        });
    }

    /// The user's answer to conflict `id`: `Some(true)` copies under the
    /// suggested name, `Some(false)` overwrites, `None` skips the file.
    fn settle_conflict(
        &mut self,
        id: u64,
        rename: Option<bool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(conflict) = self.conflict.take_if(|conflict| conflict.id == id) else {
            return;
        };
        match rename {
            None => self.start_next_transfer(window, cx),
            Some(true) => self.start_transfer(
                conflict.source,
                conflict.source_path,
                conflict.renamed_target_path,
                conflict.suggested_name,
                window,
                cx,
            ),
            Some(false) => self.start_transfer(
                conflict.source,
                conflict.source_path,
                conflict.target_path,
                conflict.file_name,
                window,
                cx,
            ),
        }
    }

    /// A pane changing machine makes a pending conflict meaningless.
    fn drop_conflict(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.conflict.take().is_some() {
            self.queue.clear();
            self.starting = false;
            window.close_dialog(cx);
        }
    }

    // --- Connections --------------------------------------------------------

    /// A choice made on a pane drops its pending connect and trust attempts;
    /// on the right pane it also stops following the active terminal.
    fn claim(&mut self, side: Side, cx: &mut Context<Self>) {
        let active = self.store.read(cx).active_session.clone();
        let pane = self.pane(side);
        pane.generation += 1;
        pane.trust_confirming = false;
        pane.trust_error = None;
        if side == Side::Right {
            self.attempted_active = active;
        }
    }

    /// The right pane opens the active terminal's host until the user picks
    /// something else for it.
    fn follow_active_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let Some(session) = store.active().filter(|session| {
            session.kind == SessionKind::Ssh && session.status == SessionStatus::Connected
        }) else {
            self.attempted_active = None;
            return;
        };
        let right = &self.panes[Side::Right.index()];
        let taken = right.local
            || right.connection_error.is_some()
            || right.direct
            || right.closing
            || right.connecting;
        let showing =
            right.sftp_session.is_some() && right.ssh_session.as_deref() == Some(&session.id);
        let on_left = self.panes[Side::Left.index()].machine()
            == Some(machine_key(&session.host, session.port));
        if taken || showing || on_left || self.attempted_active.as_deref() == Some(&session.id) {
            return;
        }
        let (id, connection) = (session.id.clone(), session_connection(session));
        self.attempted_active = Some(id.clone());
        self.open_session(Side::Right, id, connection, window, cx);
    }

    /// Closes the pane's SFTP session and returns it to the machine picker.
    fn close_sftp(&mut self, side: Side, window: &mut Window, cx: &mut Context<Self>) {
        self.drop_conflict(window, cx);
        let app = self.store.read(cx).app();
        let pane = self.pane(side);
        pane.generation += 1;
        pane.connecting = false;
        pane.ssh_session = None;
        pane.direct = false;
        pane.connection = None;
        pane.connection_id = None;
        pane.connection_error = None;
        pane.trust_prompt = None;
        pane.trust_mismatch = None;
        pane.clear_listing();
        cx.notify();
        let Some(session) = pane.sftp_session.take() else {
            return;
        };
        pane.closing = true;
        cx.spawn_in(window, async move |this, cx| {
            let closed =
                backend(async move { commands::sftp_close(session, app.state()).await }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.pane(side).closing = false;
                if let Err(error) = closed {
                    let message = tf(
                        "Disconnected. Remote cleanup failed: {error}",
                        &[("error", &error)],
                    );
                    window.push_notification(Notification::warning(message), cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Points the pane at a host and shows it connecting.
    fn begin_connecting(
        &mut self,
        side: Side,
        connection: Connection,
        connection_id: Option<String>,
        ssh_session: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> u64 {
        if self.pane(side).sftp_session.is_some() {
            self.close_sftp(side, window, cx);
        }
        self.drop_conflict(window, cx);
        let pane = self.pane(side);
        pane.generation += 1;
        pane.local = false;
        pane.connecting = true;
        pane.direct = ssh_session.is_none();
        pane.ssh_session = ssh_session;
        pane.connection = Some(connection);
        pane.connection_id = connection_id;
        pane.connection_error = None;
        pane.trust_prompt = None;
        pane.trust_mismatch = None;
        pane.clear_listing();
        cx.notify();
        pane.generation
    }

    /// Opens a saved host in the pane over its own SSH connection.
    fn connect(
        &mut self,
        side: Side,
        host: SshHostRecord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let connection = Connection {
            name: host.name.clone(),
            host: host.host.clone(),
            port: host.port as u16,
            username: host.username.clone(),
        };
        let generation =
            self.begin_connecting(side, connection, Some(host.id.clone()), None, window, cx);
        let store = self.store.read(cx);
        let input = match store.connect_input(&host) {
            Ok(input) => input,
            Err(error) => return self.finish_connecting(side, generation, Err(error), window, cx),
        };
        let app = store.app();
        cx.spawn_in(window, async move |this, cx| {
            let connected = backend(async move {
                let response = commands::sftp_connect_direct(
                    app.clone(),
                    input.host,
                    input.port,
                    input.username,
                    input.password,
                    input.private_key,
                    input.passphrase,
                    input.jumps,
                    app.state(),
                    app.state(),
                )
                .await?;
                let home = match &response {
                    SshConnectResponse::Connected { session_id } => {
                        home_dir(&app, session_id).await
                    }
                    _ => String::new(),
                };
                Ok((response, home))
            })
            .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.finish_connecting(side, generation, connected, window, cx);
            });
        })
        .detach();
    }

    /// Opens an SFTP channel on a terminal session's connection.
    fn open_session(
        &mut self,
        side: Side,
        ssh_session: String,
        connection: Connection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let generation = self.begin_connecting(
            side,
            connection,
            None,
            Some(ssh_session.clone()),
            window,
            cx,
        );
        let app = self.store.read(cx).app();
        cx.spawn_in(window, async move |this, cx| {
            let connected = backend(async move {
                let session_id = commands::sftp_open(ssh_session, app.state()).await?;
                let home = home_dir(&app, &session_id).await;
                Ok((SshConnectResponse::Connected { session_id }, home))
            })
            .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.finish_connecting(side, generation, connected, window, cx);
            });
        })
        .detach();
    }

    fn finish_connecting(
        &mut self,
        side: Side,
        generation: u64,
        connected: Result<(SshConnectResponse, String), String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let app = self.store.read(cx).app();
        let pane = self.pane(side);
        if pane.generation != generation {
            // The user moved on; do not leave the session open behind them.
            if let Ok((SshConnectResponse::Connected { session_id }, _)) = connected {
                cx.spawn(async move |_, _| {
                    let _ =
                        backend(async move { commands::sftp_close(session_id, app.state()).await })
                            .await;
                })
                .detach();
            }
            return;
        }
        pane.connecting = false;
        match connected {
            Ok((SshConnectResponse::Connected { session_id }, home)) => {
                pane.sftp_session = Some(session_id);
                self.navigate(side, home, window, cx);
            }
            Ok((SshConnectResponse::TrustRequired { prompt }, _)) => {
                pane.trust_prompt = Some(prompt)
            }
            Ok((SshConnectResponse::TrustMismatch { mismatch }, _)) => {
                let error = tf(
                    "Host trust mismatch for {host}: expected {expected}, got {presented}",
                    &[
                        ("host", &mismatch.host),
                        ("expected", &mismatch.expected_fingerprint),
                        ("presented", &mismatch.presented_fingerprint),
                    ],
                );
                pane.connection_error = Some(error.to_string());
                pane.trust_mismatch = Some(mismatch);
            }
            Err(error) => pane.connection_error = Some(error),
        }
        cx.notify();
    }

    fn use_local(&mut self, side: Side, window: &mut Window, cx: &mut Context<Self>) {
        self.claim(side, cx);
        self.close_sftp(side, window, cx);
        self.pane(side).local = true;
        self.navigate(side, "~".to_string(), window, cx);
    }

    /// Leaves local mode and shows the machine picker.
    fn choose_machine(&mut self, side: Side, cx: &mut Context<Self>) {
        let pane = self.pane(side);
        pane.generation += 1;
        pane.local = false;
        pane.clear_listing();
        cx.notify();
    }

    fn disconnect(&mut self, side: Side, window: &mut Window, cx: &mut Context<Self>) {
        self.claim(side, cx);
        self.close_sftp(side, window, cx);
    }

    fn retry(&mut self, side: Side, window: &mut Window, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let active = store.active_session.clone();
        let pane = &self.panes[side.index()];
        if pane.trust_confirming || pane.connecting {
            return;
        }
        if let (Some(ssh_session), Some(connection)) =
            (pane.ssh_session.clone(), pane.connection.clone())
        {
            if side == Side::Right {
                self.attempted_active = active;
            }
            return self.open_session(side, ssh_session, connection, window, cx);
        }
        let host = pane
            .connection_id
            .as_deref()
            .and_then(|id| store.host(id))
            .cloned();
        match host {
            Some(host) => {
                self.claim(side, cx);
                self.connect(side, host, window, cx);
            }
            None => {
                self.pane(side).connection_error = Some(
                    t("Saved connection not found. Open Connections and try again.").to_string(),
                );
                cx.notify();
            }
        }
    }

    /// Saves the host key the pane is asking about (or the new one, after a
    /// mismatch), then reconnects.
    fn trust_and_retry(&mut self, side: Side, window: &mut Window, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let app = store.app();
        let pane = &self.panes[side.index()];
        let confirmation = match (&pane.trust_prompt, &pane.trust_mismatch) {
            (Some(prompt), _) => HostTrustConfirmation {
                host: prompt.host.clone(),
                port: prompt.port,
                algorithm: prompt.algorithm.clone(),
                fingerprint: prompt.fingerprint.clone(),
            },
            (None, Some(mismatch)) => HostTrustConfirmation {
                host: mismatch.host.clone(),
                port: mismatch.port,
                algorithm: mismatch.presented_algorithm.clone(),
                fingerprint: mismatch.presented_fingerprint.clone(),
            },
            (None, None) => return,
        };
        let Some(connection_id) = pane
            .connection_id
            .as_deref()
            .filter(|_| !pane.trust_confirming)
        else {
            return;
        };
        let saved = store.host(connection_id).is_some();
        let pane = self.pane(side);
        cx.notify();
        if !saved {
            pane.trust_error =
                Some(t("Saved connection not found. Open Connections and try again.").to_string());
            return;
        }
        pane.trust_confirming = true;
        pane.trust_error = None;
        let generation = pane.generation;
        cx.spawn_in(window, async move |this, cx| {
            let confirmed = backend(async move {
                crate::connect::ssh_confirm_host_trust(confirmation, app.state()).await
            })
            .await;
            let _ = this.update_in(cx, |this, window, cx| {
                let pane = this.pane(side);
                if pane.generation != generation {
                    return;
                }
                pane.trust_confirming = false;
                match confirmed {
                    Ok(()) => this.retry(side, window, cx),
                    Err(error) => pane.trust_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    // --- Rendering ----------------------------------------------------------

    fn render_pane(&self, side: Side, cx: &mut Context<Self>) -> Div {
        let pane = &self.panes[side.index()];
        v_flex()
            .relative()
            .w_1_2()
            .h_full()
            .min_h_0()
            .when(side == Side::Left, |pane| {
                pane.border_r_1().border_color(theme::hairline())
            })
            .child(if pane.is_ready() {
                self.render_browser(side, cx).into_any_element()
            } else {
                self.render_picker(side, cx).into_any_element()
            })
            .children(self.render_status(side, cx))
    }

    fn render_browser(&self, side: Side, cx: &mut Context<Self>) -> Div {
        let pane = &self.panes[side.index()];
        let index = side.index();
        let local = pane.local;
        let identity_hint = if local {
            t("Switch machine")
        } else {
            t("Disconnect and switch machine")
        };
        let address: SharedString = match (&pane.connection, local) {
            (_, true) => t("This machine"),
            (Some(connection), false) => connection.address().into(),
            (None, false) => "".into(),
        };
        let tool = |id: &'static str, icon: &'static str, label: SharedString| {
            Button::new((id, index))
                .ghost()
                .small()
                .icon(lucide(icon))
                .tooltip(label)
        };
        let sort_header = |id: &'static str, label: SharedString, key: SortKey| {
            let icon = match pane.sort {
                (current, true) if current == key => "arrow-up",
                (current, false) if current == key => "arrow-down",
                _ => "arrow-up-down",
            };
            h_flex()
                .id((id, index))
                .gap_1p5()
                .cursor_pointer()
                .hover(|style| style.text_color(theme::STENCIL))
                .on_click(cx.listener(move |this, _, _, cx| this.toggle_sort(side, key, cx)))
                .child(common::section_title(label))
                .child(lucide(icon).size_3())
        };
        let rows = pane.files.len() + 1;
        let list = if pane.loading && pane.files.is_empty() {
            h_flex()
                .flex_1()
                .justify_center()
                .gap_2()
                .text_sm()
                .text_color(theme::STENCIL_MUTE)
                .child(Spinner::new().small())
                .child(t("Loading…"))
                .into_any_element()
        } else {
            uniform_list(
                ("files", index),
                rows,
                cx.processor(move |this, range: Range<usize>, _, cx| {
                    range
                        .map(|row| this.render_row(side, row, cx))
                        .collect::<Vec<_>>()
                }),
            )
            .track_scroll(&pane.scroll)
            .flex_1()
            .py_1()
            .into_any_element()
        };

        v_flex()
            .flex_1()
            .min_h_0()
            .child(
                h_flex()
                    .id(("identity", index))
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(theme::hairline())
                    .bg(theme::RIB)
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x1f252c)))
                    .tooltip(move |window, cx| {
                        Tooltip::new(identity_hint.clone()).build(window, cx)
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if local {
                            this.choose_machine(side, cx);
                        } else {
                            this.disconnect(side, window, cx);
                        }
                    }))
                    .child(
                        lucide(if local { "laptop" } else { "server" })
                            .small()
                            .text_color(theme::STENCIL_DIM),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .truncate()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(pane.label()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(address),
                            ),
                    )
                    .child(
                        lucide("chevrons-up-down")
                            .small()
                            .text_color(theme::STENCIL_FAINT),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(theme::hairline())
                    .child(
                        div()
                            .id(("path", index))
                            .flex_1()
                            .min_w_0()
                            .tooltip(|window, cx| {
                                Tooltip::new(t("Type a path and press Enter")).build(window, cx)
                            })
                            .child(Input::new(&pane.path_input).small()),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .gap_1()
                            .child(tool("refresh", "refresh-cw", t("Refresh")).on_click(
                                cx.listener(move |this, _, window, cx| {
                                    this.refresh(side, window, cx)
                                }),
                            ))
                            .child(tool("new-folder", "folder-plus", t("New folder")).on_click(
                                cx.listener(move |this, _, window, cx| {
                                    this.prompt_name(side, None, window, cx)
                                }),
                            ))
                            .child(
                                tool("rename", "pencil", t("Rename"))
                                    .disabled(pane.selected.len() != 1)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        let name = this.pane(side).selected.first().cloned();
                                        if name.is_some() {
                                            this.prompt_name(side, name, window, cx);
                                        }
                                    })),
                            )
                            .child(
                                tool("delete", "trash", t("Delete"))
                                    .disabled(pane.selected.is_empty())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.confirm_remove(side, window, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .id(("drop-zone", index))
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .m_2()
                    .p_3()
                    .border_1()
                    .border_color(gpui_kit::transparent_black())
                    .track_focus(&pane.focus)
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if event.keystroke.key == "a" && event.keystroke.modifiers.secondary(cx) {
                            let pane = this.pane(side);
                            pane.selected =
                                pane.files.iter().map(|file| file.name.clone()).collect();
                            cx.stop_propagation();
                            cx.notify();
                        }
                    }))
                    .drag_over::<DraggedEntries>(move |style, dragged, _, _| {
                        if dragged.side == side {
                            style
                        } else {
                            style.bg(theme::wash()).border_color(theme::STENCIL)
                        }
                    })
                    .on_drop(
                        cx.listener(move |this, dragged: &DraggedEntries, window, cx| {
                            if dragged.side != side {
                                this.transfer_many(
                                    dragged.side,
                                    dragged.entries.clone(),
                                    window,
                                    cx,
                                );
                            }
                        }),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .px_3()
                            .py_2()
                            .border_b_1()
                            .border_color(theme::hairline())
                            .text_color(theme::STENCIL_MUTE)
                            .child(div().flex_1().child(sort_header(
                                "sort-name",
                                t("Name"),
                                SortKey::Name,
                            )))
                            .child(h_flex().w(px(96.)).justify_end().child(sort_header(
                                "sort-size",
                                t("Size"),
                                SortKey::Size,
                            )))
                            .child(h_flex().w(px(160.)).justify_end().child(sort_header(
                                "sort-modified",
                                t("Modified"),
                                SortKey::Modified,
                            ))),
                    )
                    .child(list)
                    .when(pane.loading && !pane.files.is_empty(), |zone| {
                        zone.child(
                            h_flex()
                                .absolute()
                                .bottom_2()
                                .right_2()
                                .gap_2()
                                .px_3()
                                .py_1p5()
                                .rounded(theme::RADIUS)
                                .border_1()
                                .border_color(theme::hairline())
                                .bg(theme::RAIL)
                                .text_xs()
                                .child(Spinner::new().xsmall())
                                .child(t("Refreshing…")),
                        )
                    }),
            )
    }

    /// Row 0 is the parent directory; the rest are the listing.
    fn render_row(&self, side: Side, row: usize, cx: &mut Context<Self>) -> Stateful<Div> {
        let pane = &self.panes[side.index()];
        let base = h_flex()
            .id(("file-row", row))
            .w_full()
            .h(ROW_HEIGHT)
            .gap_3()
            .px_3()
            .rounded(theme::RADIUS)
            .border_1()
            .border_color(gpui_kit::transparent_black())
            .text_sm()
            .cursor_pointer();
        let cells = |icon: &'static str, name: Div, size: SharedString, modified: SharedString| {
            [
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_2()
                    .child(
                        lucide(icon)
                            .small()
                            .flex_shrink_0()
                            .text_color(theme::STENCIL_DIM),
                    )
                    .child(name.min_w_0().truncate()),
                div()
                    .w(px(96.))
                    .flex_shrink_0()
                    .text_right()
                    .text_xs()
                    .text_color(theme::STENCIL_MUTE)
                    .child(size),
                div()
                    .w(px(160.))
                    .flex_shrink_0()
                    .text_right()
                    .text_xs()
                    .text_color(theme::STENCIL_MUTE)
                    .child(modified),
            ]
        };
        let Some(entry) = row.checked_sub(1).and_then(|index| pane.files.get(index)) else {
            return base
                .hover(|style| style.bg(theme::wash()))
                .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                    if event.click_count() >= 2 {
                        this.navigate_up(side, window, cx);
                    }
                }))
                .children(cells(
                    "folder",
                    div().font_weight(FontWeight::MEDIUM).child(".."),
                    "—".into(),
                    "—".into(),
                ));
        };
        let selected = pane.selected.contains(&entry.name);
        // Dragging a selected row carries the whole selection.
        let dragged: Vec<FileEntry> = if selected {
            pane.files
                .iter()
                .filter(|file| pane.selected.contains(&file.name))
                .cloned()
                .collect()
        } else {
            vec![entry.clone()]
        };
        let chip: SharedString = match dragged.as_slice() {
            [single] => single.name.clone().into(),
            many => tf("{count} items", &[("count", &many.len().to_string())]),
        };
        let (icon, size) = match entry.file_type {
            FileType::Dir => ("folder", "—".to_string()),
            FileType::File => ("file-text", format_size(entry.size)),
            FileType::Symlink => ("link-2", format_size(entry.size)),
            FileType::Other => ("file-question-mark", format_size(entry.size)),
        };
        let name = div()
            .when(entry.file_type == FileType::Dir, |name| {
                name.font_weight(FontWeight::MEDIUM)
            })
            .child(entry.name.clone());
        let clicked = entry.clone();
        base.map(|row| {
            if selected {
                row.border_color(theme::STENCIL)
                    .bg(theme::STENCIL.opacity(0.06))
            } else {
                row.hover(|style| style.bg(theme::wash()))
            }
        })
        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            this.click_row(side, &clicked, event, window, cx);
        }))
        .on_drag(
            DraggedEntries {
                side,
                entries: dragged,
            },
            move |_, _, _, cx| cx.new(|_| DragChip(chip.clone())),
        )
        .children(cells(
            icon,
            name,
            size.into(),
            format_modified(entry.modified).into(),
        ))
    }

    fn render_picker(&self, side: Side, cx: &mut Context<Self>) -> Stateful<Div> {
        let pane = &self.panes[side.index()];
        let store = self.store.read(cx);
        let busy = pane.connecting
            || pane.closing
            || pane.trust_prompt.is_some()
            || pane.trust_mismatch.is_some()
            || pane.connection_error.is_some();
        let other_machine = self.panes[side.other().index()].machine();
        let choice = |id: ElementId,
                      icon: &'static str,
                      live: bool,
                      title: SharedString,
                      address: Option<String>,
                      machine: String| {
            let taken = other_machine.as_deref() == Some(&machine);
            h_flex()
                .id(id)
                .gap_3()
                .p_3()
                .rounded(theme::RADIUS)
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::RIB)
                .map(|choice| {
                    if busy || taken {
                        choice.opacity(0.5)
                    } else {
                        choice.cursor_pointer().hover(|style| {
                            style
                                .border_color(theme::STENCIL.opacity(0.2))
                                .bg(rgb(0x1f252c))
                        })
                    }
                })
                .when(taken, |choice| {
                    choice.tooltip(|window, cx| {
                        Tooltip::new(t("Already open on the other side")).build(window, cx)
                    })
                })
                .child(
                    div()
                        .size_8()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .rounded(theme::RADIUS)
                        .border_1()
                        .map(|badge| {
                            if live {
                                badge
                                    .border_color(theme::GREEN_TEXT.opacity(0.3))
                                    .text_color(theme::GREEN_TEXT)
                            } else {
                                badge
                                    .border_color(theme::hairline())
                                    .text_color(theme::STENCIL_DIM)
                            }
                        })
                        .child(lucide(icon).small()),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .child(title),
                        )
                        .children(address.map(|address| {
                            div()
                                .truncate()
                                .text_xs()
                                .text_color(theme::STENCIL_MUTE)
                                .child(address)
                        })),
                )
                // A disabled choice keeps its handler off rather than ignoring clicks.
                .map(|choice| (choice, !(busy || taken)))
        };
        let sessions: Vec<_> = store
            .sessions
            .iter()
            .filter(|session| {
                session.kind == SessionKind::Ssh && session.status == SessionStatus::Connected
            })
            .collect();

        let (local, enabled) = choice(
            "choose-local".into(),
            "laptop",
            false,
            t("Local"),
            None,
            LOCAL_MACHINE.to_string(),
        );
        v_flex()
            .id(("picker", side.index()))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_4()
            .gap_2()
            .child(
                div()
                    .pb_1()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(t("Select a connection")),
            )
            .child(local.when(enabled, |choice| {
                choice.on_click(
                    cx.listener(move |this, _, window, cx| this.use_local(side, window, cx)),
                )
            }))
            .when(!sessions.is_empty(), |picker| {
                picker
                    .child(common::section_title(t("Open sessions")).mt_2())
                    .children(sessions.into_iter().map(|session| {
                        let connection = session_connection(session);
                        let (choice, enabled) = choice(
                            SharedString::from(format!("session-{}", session.id)).into(),
                            "square-terminal",
                            true,
                            session.name.clone().into(),
                            Some(connection.address()),
                            machine_key(&session.host, session.port),
                        );
                        let id = session.id.clone();
                        choice.when(enabled, |choice| {
                            choice.on_click(cx.listener(move |this, _, window, cx| {
                                this.claim(side, cx);
                                this.open_session(side, id.clone(), connection.clone(), window, cx);
                            }))
                        })
                    }))
            })
            .child(common::section_title(t("Saved connections")).mt_2())
            .children(store.hosts.iter().map(|host| {
                let (choice, enabled) = choice(
                    SharedString::from(format!("host-{}", host.id)).into(),
                    "server",
                    false,
                    host.name.clone().into(),
                    Some(session_address(
                        &host.username,
                        &host.host,
                        host.port as u16,
                    )),
                    machine_key(&host.host, host.port as u16),
                );
                let host = host.clone();
                choice.when(enabled, |choice| {
                    choice.on_click(cx.listener(move |this, _, window, cx| {
                        this.claim(side, cx);
                        this.connect(side, host.clone(), window, cx);
                    }))
                })
            }))
            .when(store.hosts.is_empty(), |picker| {
                picker.child(
                    div()
                        .py_4()
                        .text_center()
                        .text_sm()
                        .text_color(theme::STENCIL_MUTE)
                        .child(t("No saved connections. Add one in Connections.")),
                )
            })
            .when(pane.closing, |picker| {
                picker.child(
                    h_flex()
                        .mt_1()
                        .gap_2()
                        .text_sm()
                        .text_color(theme::STENCIL_MUTE)
                        .child(Spinner::new().small())
                        .child(t("Disconnecting...")),
                )
            })
    }

    /// What covers a pane while it connects, asks about a host key, or failed.
    fn render_status(&self, side: Side, cx: &mut Context<Self>) -> Option<Div> {
        let pane = &self.panes[side.index()];
        let index = side.index();
        let confirming = pane.trust_confirming;
        let can_trust = pane.connection_id.is_some();
        let cancel = || {
            Button::new(("status-cancel", index))
                .outline()
                .label(t("Cancel"))
                .on_click(cx.listener(move |this, _, window, cx| this.disconnect(side, window, cx)))
        };
        let trust = |label: SharedString| {
            Button::new(("status-trust", index))
                .label(label)
                .loading(confirming)
                .disabled(confirming)
                .on_click(
                    cx.listener(move |this, _, window, cx| this.trust_and_retry(side, window, cx)),
                )
        };
        let retry = || {
            Button::new(("status-retry", index))
                .label(t("Retry session"))
                .disabled(confirming)
                .on_click(cx.listener(move |this, _, window, cx| this.retry(side, window, cx)))
        };
        let cover = || {
            v_flex()
                .absolute()
                .inset_0()
                .occlude()
                .items_center()
                .justify_center()
                .p_4()
                .bg(theme::PORT.opacity(0.9))
        };
        let card = |id: &'static str, accent: Rgba| {
            v_flex()
                .id((id, index))
                .w_full()
                .max_w(px(512.))
                .max_h_full()
                .overflow_y_scroll()
                .rounded(theme::RADIUS)
                .border_1()
                .border_color(accent.opacity(0.6))
                .bg(theme::RAIL)
        };
        let fact = |label: SharedString, value: String| {
            v_flex()
                .gap_1()
                .px_4()
                .py_3()
                .rounded(theme::RADIUS)
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::RIB)
                .text_sm()
                .child(div().text_color(theme::STENCIL_MUTE).child(label))
                .child(value)
        };
        let note = |error: String| {
            div()
                .px_4()
                .py_3()
                .rounded(theme::RADIUS)
                .border_1()
                .border_color(theme::SEAL.opacity(0.6))
                .text_sm()
                .text_color(theme::RED_TEXT)
                .child(error)
        };
        let muted =
            |text: SharedString| div().text_xs().text_color(theme::STENCIL_MUTE).child(text);

        if pane.connecting {
            let name = pane
                .connection
                .as_ref()
                .map(|connection| connection.name.as_str());
            return Some(
                cover()
                    .gap_4()
                    .child(Spinner::new().large().color(theme::STENCIL.into()))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(match name {
                                Some(name) => tf("Connecting to {name}", &[("name", name)]),
                                None => t("Connecting…"),
                            }),
                    )
                    .child(cancel()),
            );
        }
        if let Some(prompt) = &pane.trust_prompt {
            return Some(
                cover().child(
                    card("trust", theme::AMBER)
                        .child(
                            h_flex()
                                .gap_3()
                                .px_6()
                                .py_4()
                                .border_b_1()
                                .border_color(theme::hairline())
                                .child(lucide("circle-alert").size_5().text_color(theme::AMBER_TEXT))
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(t("Verify SSH host identity")),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_3()
                                .px_6()
                                .py_4()
                                .child(fact(t("Host"), format!("{}:{}", prompt.host, prompt.port)))
                                .child(fact(t("Algorithm"), prompt.algorithm.clone()))
                                .child(fact(t("Fingerprint"), prompt.fingerprint.clone()))
                                .child(muted(t(
                                    "Only continue if this fingerprint matches the server you expect. It will be saved locally in Known Hosts.",
                                )))
                                .children(pane.trust_error.clone().map(note)),
                        )
                        .child(
                            h_flex()
                                .flex_wrap()
                                .justify_end()
                                .gap_3()
                                .px_6()
                                .py_4()
                                .border_t_1()
                                .border_color(theme::hairline())
                                .bg(theme::PORT)
                                .child(if can_trust {
                                    trust(t("Trust host and retry")).warning().into_any_element()
                                } else {
                                    div()
                                        .text_sm()
                                        .text_color(theme::STENCIL_MUTE)
                                        .child(t("Save this connection first to trust the host and retry automatically."))
                                        .into_any_element()
                                })
                                .child(cancel()),
                        ),
                ),
            );
        }
        if pane.trust_mismatch.is_none() && pane.connection_error.is_none() {
            return None;
        }
        let error = pane
            .connection_error
            .clone()
            .map_or_else(|| t("Unknown error"), SharedString::from);
        let mismatch = pane.trust_mismatch.as_ref();
        Some(
            cover().child(
                card("failed", theme::SEAL)
                    .p_6()
                    .gap_4()
                    .child(
                        h_flex().justify_center().text_size(px(11.)).child(
                            div()
                                .px_1()
                                .bg(theme::STENCIL)
                                .text_color(theme::SEAL)
                                .line_height(px(16.))
                                .child("HELD"),
                        ),
                    )
                    .child(
                        div()
                            .text_center()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t("Connection failed")),
                    )
                    .child(div().text_center().text_sm().text_color(theme::STENCIL_MUTE).child(error))
                    .when_some(mismatch, |card, mismatch| {
                        card.child(
                            v_flex()
                                .gap_3()
                                .p_4()
                                .rounded(theme::RADIUS)
                                .border_1()
                                .border_color(theme::SEAL.opacity(0.6))
                                .bg(theme::RIB)
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(theme::RED_TEXT)
                                        .child(t("Saved fingerprint does not match.")),
                                )
                                .child(fact(t("Expected"), mismatch.expected_fingerprint.clone()).bg(theme::RAIL))
                                .child(fact(t("Presented"), mismatch.presented_fingerprint.clone()).bg(theme::RAIL))
                                .child(muted(t(
                                    "This may indicate the server changed keys or a man-in-the-middle risk. Not updating trust automatically.",
                                ))),
                        )
                        .children(pane.trust_error.clone().map(note))
                    })
                    .child(
                        h_flex()
                            .flex_wrap()
                            .justify_center()
                            .gap_3()
                            .when(mismatch.is_some() && can_trust, |row| {
                                row.child(trust(t("Delete & trust new key")).danger())
                            })
                            .child(if mismatch.is_some() { retry().outline() } else { retry().danger() })
                            .child(cancel()),
                    ),
            ),
        )
    }

    fn render_transfers(&self, cx: &mut Context<Self>) -> Option<Div> {
        if self.transfers.is_empty() {
            return None;
        }
        let rows = self.transfers.iter().map(|transfer| {
            let id = transfer.transfer_id.clone();
            let label = self
                .transfer_names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| id.chars().take(8).collect());
            let (icon, title) = match transfer.direction {
                TransferDirection::Upload => ("arrow-up", tf("Uploading {id}", &[("id", &label)])),
                TransferDirection::Copy => {
                    ("arrow-left-right", tf("Copying {id}", &[("id", &label)]))
                }
                TransferDirection::Download => {
                    ("arrow-down", tf("Downloading {id}", &[("id", &label)]))
                }
            };
            let percent = if transfer.total_bytes == 0 {
                0.
            } else {
                (transfer.bytes_transferred as f32 / transfer.total_bytes as f32 * 100.).min(100.)
            };
            h_flex()
                .gap_3()
                .px_2()
                .py_2()
                .child(lucide(icon).small().text_color(theme::STENCIL_DIM))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .child(
                            h_flex()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .truncate()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .text_xs()
                                        .text_color(theme::STENCIL_DIM)
                                        .child(format!("{percent:.0}%")),
                                ),
                        )
                        .child(
                            Progress::new(SharedString::from(format!("progress-{id}")))
                                .value(percent),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::STENCIL_MUTE)
                                .child(format!(
                                    "{} / {}",
                                    format_size(transfer.bytes_transferred),
                                    format_size(transfer.total_bytes)
                                )),
                        ),
                )
                .child(
                    div()
                        .w(px(88.))
                        .flex_shrink_0()
                        .text_right()
                        .text_xs()
                        .text_color(theme::STENCIL_DIM)
                        .child(format!("{}/s", format_size(transfer.speed_bps))),
                )
                .child(
                    Button::new(SharedString::from(format!("cancel-{id}")))
                        .ghost()
                        .xsmall()
                        .icon(lucide("x"))
                        .tooltip(tf("Cancel transfer {id}", &[("id", &label)]))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.cancel_transfer(id.clone(), window, cx);
                        })),
                )
        });
        let count = self.transfers.len().to_string();
        Some(
            v_flex()
                .flex_shrink_0()
                .border_t_1()
                .border_color(theme::hairline())
                .bg(theme::RAIL)
                .child(
                    h_flex()
                        .justify_between()
                        .px_4()
                        .py_2()
                        .border_b_1()
                        .border_color(theme::hairline())
                        .child(
                            h_flex()
                                .gap_2()
                                .child(div().size_2().rounded_full().bg(theme::AMBER_TEXT))
                                .child(common::section_title(t("Transfers"))),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::STENCIL_MUTE)
                                .child(tf("{count} active", &[("count", &count)])),
                        ),
                )
                .child(
                    v_flex()
                        .id("transfers")
                        .max_h(px(176.))
                        .overflow_y_scroll()
                        .px_3()
                        .py_1()
                        .children(rows),
                ),
        )
    }
}

impl Render for SftpView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(theme::PORT)
            .text_color(theme::STENCIL)
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_3()
                    .px_6()
                    .py_4()
                    .border_b_1()
                    .border_color(theme::hairline())
                    .child(common::icon_box("folder-open"))
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t("SFTP File Browser")),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_pane(Side::Left, cx))
                    .child(self.render_pane(Side::Right, cx)),
            )
            .children(self.render_transfers(cx))
    }
}

/// The remote home directory, or the server's working directory when it does
/// not expose one.
async fn home_dir(app: &crate::app::AppHandle, session_id: &str) -> String {
    commands::sftp_home_dir(session_id.to_string(), app.state())
        .await
        .unwrap_or_else(|_| ".".to_string())
}

fn session_connection(session: &crate::ui::store::Session) -> Connection {
    Connection {
        name: session.name.clone(),
        host: session.host.clone(),
        port: session.port,
        username: session.username.clone(),
    }
}

fn session_address(username: &str, host: &str, port: u16) -> String {
    Connection {
        name: String::new(),
        host: host.to_string(),
        port,
        username: username.to_string(),
    }
    .address()
}

fn join_path(base: &str, name: &str) -> String {
    if base.is_empty() || base == "/" {
        format!("{base}{name}")
    } else {
        format!("{}/{name}", base.trim_end_matches('/'))
    }
}

fn parent_path(path: &str) -> Option<String> {
    match path {
        "" | "/" => None,
        "~" => Some("/".to_string()),
        _ => Some(match path.rfind('/') {
            None | Some(0) => "/".to_string(),
            Some(slash) => path[..slash].to_string(),
        }),
    }
}

/// `name`, or the first `name (n).ext` that `entries` does not already hold.
fn next_available_name(name: &str, entries: &[FileEntry]) -> String {
    let taken = |candidate: &str| entries.iter().any(|entry| entry.name == candidate);
    if !taken(name) {
        return name.to_string();
    }
    let (base, extension) = match name.rfind('.') {
        Some(dot) if dot > 0 => name.split_at(dot),
        _ => (name, ""),
    };
    // "report (2)" continues from 3 rather than becoming "report (2) (1)".
    let numbered = base.strip_suffix(')').and_then(|rest| {
        let (root, number) = rest.rsplit_once(" (")?;
        Some((root, number.parse::<u64>().ok()? + 1))
    });
    let (root, first) = numbered.unwrap_or((base, 1));
    (first..)
        .map(|index| format!("{root} ({index}){extension}"))
        .find(|candidate| !taken(candidate))
        .expect("an unbounded range always yields a free name")
}

/// The trimmed name, or the i18n key of what is wrong with it. `renaming` is
/// the current name when renaming, `None` when naming a new folder.
fn validate_name(value: &str, renaming: Option<&str>) -> Result<String, &'static str> {
    let name = value.trim();
    let folder = renaming.is_none();
    if name.is_empty() {
        return Err(if folder {
            "Folder name is required"
        } else {
            "Name is required"
        });
    }
    if renaming == Some(name) {
        return Err("Name is unchanged");
    }
    if name == "." || name == ".." {
        return Err(if folder {
            "Folder name cannot be '.' or '..'"
        } else {
            "Name cannot be '.' or '..'"
        });
    }
    if name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err(if folder {
            "Folder name contains invalid characters"
        } else {
            "Name contains invalid characters"
        });
    }
    Ok(name.to_string())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.;
    let mut unit = "KB";
    for next in ["MB", "GB", "TB"] {
        if value < 1024. {
            break;
        }
        value /= 1024.;
        unit = next;
    }
    if value >= 10. {
        format!("{value:.0} {unit}")
    } else {
        format!("{value:.1} {unit}")
    }
}

fn format_modified(timestamp: Option<u64>) -> String {
    timestamp
        .and_then(|seconds| chrono::DateTime::from_timestamp(i64::try_from(seconds).ok()?, 0))
        .map_or_else(
            || "—".to_string(),
            |time| {
                time.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            },
        )
}

#[cfg(test)]
mod tests {
    use super::{
        format_size, join_path, next_available_name, parent_path, validate_name, FileEntry,
        FileType,
    };

    fn entries(names: &[&str]) -> Vec<FileEntry> {
        names
            .iter()
            .map(|name| FileEntry {
                name: name.to_string(),
                size: 0,
                modified: None,
                file_type: FileType::File,
            })
            .collect()
    }

    #[test]
    fn test_next_available_name_counts_past_existing_copies() {
        assert_eq!(next_available_name("a.txt", &entries(&["b.txt"])), "a.txt");
        assert_eq!(
            next_available_name("a.txt", &entries(&["a.txt"])),
            "a (1).txt"
        );
        assert_eq!(
            next_available_name("a.txt", &entries(&["a.txt", "a (1).txt"])),
            "a (2).txt"
        );
        assert_eq!(
            next_available_name("a (2).txt", &entries(&["a (2).txt"])),
            "a (3).txt"
        );
        assert_eq!(next_available_name(".env", &entries(&[".env"])), ".env (1)");
        assert_eq!(next_available_name("logs", &entries(&["logs"])), "logs (1)");
    }

    #[test]
    fn test_validate_name_rejects_bad_names() {
        assert_eq!(validate_name("  backups ", None), Ok("backups".to_string()));
        assert_eq!(validate_name("  ", None), Err("Folder name is required"));
        assert_eq!(
            validate_name("..", None),
            Err("Folder name cannot be '.' or '..'")
        );
        assert_eq!(
            validate_name("a/b", None),
            Err("Folder name contains invalid characters")
        );
        assert_eq!(validate_name("old", Some("old")), Err("Name is unchanged"));
        assert_eq!(
            validate_name("a:b", Some("old")),
            Err("Name contains invalid characters")
        );
        assert_eq!(validate_name("new", Some("old")), Ok("new".to_string()));
    }

    #[test]
    fn test_paths_join_and_step_up() {
        assert_eq!(join_path("/", "etc"), "/etc");
        assert_eq!(join_path("", "etc"), "etc");
        assert_eq!(join_path("~", "work"), "~/work");
        assert_eq!(join_path("/var/", "log"), "/var/log");
        assert_eq!(parent_path("/var/log"), Some("/var".to_string()));
        assert_eq!(parent_path("/var"), Some("/".to_string()));
        assert_eq!(parent_path("~"), Some("/".to_string()));
        assert_eq!(parent_path("/"), None);
    }

    #[test]
    fn test_format_size_picks_a_unit() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(20 * 1024 * 1024), "20 MB");
    }
}
