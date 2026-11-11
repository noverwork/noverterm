//! Port forwards: the saved presets with their running tunnels, the form to
//! add or edit a preset, and the runtime state that outlives both pages.

use std::collections::{HashMap, HashSet};

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::select::Select;
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use shared::{
    PortForwardMappingInput, PortForwardRecord, PortForwardWriteRequest, SshHostAuthMaterial,
    SshHostRecord,
};

use crate::app::AppHandle;
use crate::connect::{DirectSshConnectInput, PortForwardStartInput};
use crate::runtime::port_forward::{PortForwardState, PortForwardStatus};
use crate::ui::bridge::backend;
use crate::ui::i18n::{t, tf};
use crate::ui::icons::lucide;
use crate::ui::store::{store, Route, Store, StoreEvent};
use crate::ui::theme;
use crate::ui::views::common::{self, Choice, ChoiceSelect};

// --- Runtime state -------------------------------------------------------------

/// The tunnels the backend is running, kept current from its status events.
pub struct ForwardRuntime {
    app: AppHandle,
    forwards: HashMap<String, PortForwardStatus>,
    /// A preset starts one tunnel per mapping, and they live or die together:
    /// this maps every runtime id to the others started alongside it, so the
    /// first failure can tear the rest down. Backend forwards carry no group.
    siblings: HashMap<String, Vec<String>>,
    _subscription: Subscription,
}

struct RuntimeGlobal(Entity<ForwardRuntime>);

impl Global for RuntimeGlobal {}

/// The runtime state, created on first use and kept for the life of the app.
fn runtime(cx: &mut App) -> Entity<ForwardRuntime> {
    if let Some(global) = cx.try_global::<RuntimeGlobal>() {
        return global.0.clone();
    }
    let runtime = cx.new(ForwardRuntime::new);
    cx.set_global(RuntimeGlobal(runtime.clone()));
    runtime
}

impl ForwardRuntime {
    fn new(cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let subscription = cx.subscribe(&store, |this, _, event: &StoreEvent, cx| {
            if let StoreEvent::Backend("port_forward_status", payload) = event {
                if let Ok(status) = serde_json::from_str(payload) {
                    this.apply(status, cx);
                }
            }
        });
        Self {
            app: store.read(cx).app(),
            forwards: HashMap::new(),
            siblings: HashMap::new(),
            _subscription: subscription,
        }
    }

    /// Dissolves the group `id` belongs to and returns its other members.
    fn forget_group(&mut self, id: &str) -> Vec<String> {
        let siblings = self.siblings.remove(id).unwrap_or_default();
        for sibling in &siblings {
            self.siblings.remove(sibling);
        }
        siblings
    }

    fn apply(&mut self, status: PortForwardStatus, cx: &mut Context<Self>) {
        match status.state {
            PortForwardState::Error => {
                for sibling in self.forget_group(&status.id) {
                    let app = self.app.clone();
                    // Runs on Tokio whether or not it is awaited. The group is
                    // already failing; a sibling that refuses to stop adds nothing.
                    drop(backend(async move {
                        crate::connect::port_forward_stop(app.clone(), sibling, app.state()).await
                    }));
                }
            }
            // The echo of a stop whose tunnel the user already dismissed.
            PortForwardState::Stopped if !self.forwards.contains_key(&status.id) => return,
            _ => {}
        }
        self.forwards.insert(status.id.clone(), status);
        cx.notify();
    }

    /// Picks up tunnels started elsewhere, e.g. by an AI agent.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let app = self.app.clone();
        cx.spawn(async move |this, cx| {
            let listed =
                backend(async move { crate::connect::port_forward_list(app.state()).await }).await;
            let _ = this.update(cx, |this, cx| {
                for status in listed.unwrap_or_default() {
                    this.apply(status, cx);
                }
            });
        })
        .detach();
    }

    /// Starts every mapping in the preset as one group. A mapping that fails
    /// outright takes the whole group down; one that fails later (a bind clash
    /// only surfaces on the status event) is rolled back by `apply`.
    fn start_saved(
        &mut self,
        preset: &PortForwardRecord,
        connection: DirectSshConnectInput,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), String>> {
        let inputs: Vec<PortForwardStartInput> = preset
            .mappings
            .iter()
            .map(|mapping| PortForwardStartInput {
                name: preset.name.clone(),
                host: connection.host.clone(),
                port: connection.port,
                username: connection.username.clone(),
                password: connection.password.clone(),
                private_key: connection.private_key.clone(),
                passphrase: connection.passphrase.clone(),
                jumps: connection.jumps.clone(),
                bind_host: mapping.bind_host.clone(),
                bind_port: mapping.bind_port as u16,
                target_host: mapping.target_host.clone(),
                target_port: mapping.target_port as u16,
            })
            .collect();
        let app = self.app.clone();
        cx.spawn(async move |this, cx| {
            let mut started: Vec<String> = Vec::new();
            for input in inputs {
                let handle = app.clone();
                let result = backend(async move {
                    crate::connect::port_forward_start(
                        handle.clone(),
                        input,
                        handle.state(),
                        handle.state(),
                    )
                    .await
                })
                .await;
                match result {
                    Ok(status) => {
                        started.push(status.id.clone());
                        let _ = this.update(cx, |this, cx| {
                            // A status event may have beaten the reply here, and it is newer.
                            if !this.forwards.contains_key(&status.id) {
                                this.apply(status, cx);
                            }
                        });
                    }
                    Err(error) => {
                        for id in started {
                            let handle = app.clone();
                            let _ = backend(async move {
                                crate::connect::port_forward_stop(
                                    handle.clone(),
                                    id,
                                    handle.state(),
                                )
                                .await
                            })
                            .await;
                        }
                        return Err(error);
                    }
                }
            }
            if started.len() > 1 {
                let _ = this.update(cx, |this, _| {
                    for id in &started {
                        let others = started
                            .iter()
                            .filter(|other| *other != id)
                            .cloned()
                            .collect();
                        this.siblings.insert(id.clone(), others);
                    }
                });
            }
            Ok(())
        })
    }

    /// Stops the tunnels that are still up, then forgets all of `ids`.
    fn stop_all(&mut self, ids: Vec<String>, cx: &mut Context<Self>) -> Task<Result<(), String>> {
        let app = self.app.clone();
        cx.spawn(async move |this, cx| {
            for id in ids {
                let running = this
                    .update(cx, |this, _| {
                        this.forget_group(&id);
                        this.forwards.get(&id).is_some_and(|forward| {
                            matches!(
                                forward.state,
                                PortForwardState::Connecting | PortForwardState::Listening
                            )
                        })
                    })
                    .unwrap_or(false);
                if running {
                    let (handle, id) = (app.clone(), id.clone());
                    backend(async move {
                        crate::connect::port_forward_stop(handle.clone(), id, handle.state()).await
                    })
                    .await?;
                }
                let _ = this.update(cx, |this, cx| {
                    this.forwards.remove(&id);
                    cx.notify();
                });
            }
            Ok(())
        })
    }
}

/// Runtime forwards carry no preset id, so each mapping is matched back to its
/// running tunnel by the fields the backend echoes. The result is aligned with
/// `preset.mappings`; `None` means that mapping is not running.
fn runtimes_for<'a>(
    preset: &PortForwardRecord,
    host: Option<&SshHostRecord>,
    forwards: &'a HashMap<String, PortForwardStatus>,
) -> Vec<Option<&'a PortForwardStatus>> {
    preset
        .mappings
        .iter()
        .map(|mapping| {
            let host = host?;
            forwards.values().find(|forward| {
                forward.name == preset.name
                    && forward.host == host.host
                    && i32::from(forward.port) == host.port
                    && forward.username == host.username
                    && forward.bind_host == mapping.bind_host
                    && i32::from(forward.bind_port) == mapping.bind_port
                    && forward.target_host == mapping.target_host
                    && i32::from(forward.target_port) == mapping.target_port
            })
        })
        .collect()
}

/// The group is only as healthy as its worst mapping. `None` when nothing in
/// the group is running at all.
fn group_state(runtimes: &[Option<&PortForwardStatus>]) -> Option<PortForwardState> {
    let running: Vec<&PortForwardState> = runtimes
        .iter()
        .flatten()
        .map(|forward| &forward.state)
        .collect();
    if running.is_empty() {
        return None;
    }
    Some(
        if running
            .iter()
            .any(|state| matches!(state, PortForwardState::Error))
        {
            PortForwardState::Error
        } else if running
            .iter()
            .any(|state| matches!(state, PortForwardState::Connecting))
        {
            PortForwardState::Connecting
        } else if running.len() == runtimes.len()
            && running
                .iter()
                .all(|state| matches!(state, PortForwardState::Listening))
        {
            PortForwardState::Listening
        } else {
            PortForwardState::Stopped
        },
    )
}

/// The colour and label a tunnel state is shown with.
fn state_badge(state: &PortForwardState) -> (Rgba, SharedString) {
    match state {
        PortForwardState::Listening => (theme::GREEN_TEXT, t("Listening")),
        PortForwardState::Connecting => (theme::AMBER_TEXT, t("Connecting")),
        PortForwardState::Error => (theme::RED_TEXT, t("Error")),
        PortForwardState::Stopped => (theme::STENCIL_MUTE, t("Stopped")),
    }
}

fn dot(color: Rgba) -> Div {
    div().size_1p5().flex_shrink_0().rounded_full().bg(color)
}

// --- List --------------------------------------------------------------------

pub struct ForwardsView {
    store: Entity<Store>,
    runtime: Entity<ForwardRuntime>,
    search: Entity<InputState>,
    error: Option<SharedString>,
    /// Presets with a start, stop or delete under way.
    busy: HashSet<String>,
    _subscriptions: Vec<Subscription>,
}

impl ForwardsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let runtime = runtime(cx);
        runtime.update(cx, |runtime, cx| runtime.refresh(cx));
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(t("Search forwards")));
        let subscriptions = vec![
            cx.observe(&store, |_, _, cx| cx.notify()),
            cx.observe(&runtime, |_, _, cx| cx.notify()),
            cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
        ];
        Self {
            store,
            runtime,
            search,
            error: None,
            busy: HashSet::new(),
            _subscriptions: subscriptions,
        }
    }

    /// Marks `id` busy until `task` ends, then shows its error if it had one.
    fn track(&mut self, id: String, task: Task<Result<(), String>>, cx: &mut Context<Self>) {
        self.error = None;
        self.busy.insert(id.clone());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.busy.remove(&id);
                this.error = result.err().map(SharedString::from);
                cx.notify();
            });
        })
        .detach();
    }

    fn forward(&mut self, preset: &PortForwardRecord, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let connection = match store.host(&preset.host_id) {
            Some(host) => store.connect_input(host),
            None => Err(t(
                "Saved connection not found. Open Connections and verify this host still exists.",
            )
            .to_string()),
        };
        let task = match connection {
            Ok(connection) => self.runtime.update(cx, |runtime, cx| {
                runtime.start_saved(preset, connection, cx)
            }),
            Err(error) => Task::ready(Err(error)),
        };
        self.track(preset.id.clone(), task, cx);
    }

    fn stop(&mut self, preset_id: String, runtime_ids: Vec<String>, cx: &mut Context<Self>) {
        let task = self
            .runtime
            .update(cx, |runtime, cx| runtime.stop_all(runtime_ids, cx));
        self.track(preset_id, task, cx);
    }

    fn edit(&mut self, id: &str, cx: &mut Context<Self>) {
        let route = Route::ForwardForm(Some(id.to_string()));
        self.store.update(cx, |store, cx| store.navigate(route, cx));
    }

    fn request_delete(
        &mut self,
        preset: &PortForwardRecord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.error = None;
        cx.notify();
        let (view, id) = (cx.entity(), preset.id.clone());
        common::confirm_delete(
            t("Delete saved forward?"),
            t("This removes the saved forwarding preset. Existing runtime forwards are not stopped automatically."),
            Some(preset.name.clone().into()),
            t("Delete forward"),
            window,
            cx,
            move |_, cx| view.update(cx, |this, cx| this.delete(id.clone(), cx)),
        );
    }

    fn delete(&mut self, id: String, cx: &mut Context<Self>) {
        let (app, store, preset_id) = (self.store.read(cx).app(), self.store.clone(), id.clone());
        let task = cx.spawn(async move |_, cx| {
            let deleted = backend(async move {
                crate::store::port_forwards::port_forward_preset_delete(id, app.state()).await
            })
            .await;
            store.update(cx, |store, cx| store.refresh(cx));
            deleted
        });
        self.track(preset_id, task, cx);
    }

    fn render_card(&self, preset: &PortForwardRecord, cx: &Context<Self>) -> impl IntoElement {
        let host = self.store.read(cx).host(&preset.host_id);
        let runtimes = runtimes_for(preset, host, &self.runtime.read(cx).forwards);
        let state = group_state(&runtimes);
        let badge = state.as_ref().map(state_badge);
        let (badge_color, badge_label) = badge.clone().unwrap_or((theme::STENCIL_MUTE, t("Idle")));
        let running = state.is_some();
        let busy = self.busy.contains(&preset.id);
        let can_start = host.is_some();
        let runtime_ids: Vec<String> = runtimes
            .iter()
            .flatten()
            .map(|forward| forward.id.clone())
            .collect();
        let group: SharedString = format!("forward-{}", preset.id).into();
        let view = cx.entity();
        let icon_button = |id: &'static str, icon: &'static str, label: SharedString| {
            Button::new(SharedString::from(format!("{id}-{}", preset.id)))
                .ghost()
                .small()
                .icon(lucide(icon))
                .tooltip(label)
        };
        let (activate, activate_ids) = (preset.clone(), runtime_ids.clone());
        let (edit, delete) = (preset.id.clone(), preset.clone());
        let (menu_preset, menu_ids) = (preset.clone(), runtime_ids);

        let mappings =
            preset
                .mappings
                .iter()
                .zip(&runtimes)
                .enumerate()
                .map(|(index, (mapping, runtime))| {
                    let color = runtime.map_or(theme::STENCIL_FAINT, |forward| {
                        state_badge(&forward.state).0
                    });
                    v_flex()
                        .gap_1()
                        .px_3()
                        .py_2()
                        .when(index > 0, |row| {
                            row.border_t_1().border_color(theme::hairline())
                        })
                        .child(
                            h_flex()
                                .gap_2()
                                .child(dot(color))
                                .child(
                                    div().truncate().child(format!(
                                        "{}:{}",
                                        mapping.bind_host, mapping.bind_port
                                    )),
                                )
                                .child(
                                    lucide("arrow-right")
                                        .xsmall()
                                        .flex_shrink_0()
                                        .text_color(theme::STENCIL_FAINT),
                                )
                                .child(div().truncate().text_color(theme::STENCIL_DIM).child(
                                    format!("{}:{}", mapping.target_host, mapping.target_port),
                                )),
                        )
                        .children(
                            runtime
                                .and_then(|forward| forward.error.clone())
                                .map(|error| {
                                    h_flex()
                                        .items_start()
                                        .gap_1p5()
                                        .pl_3p5()
                                        .text_color(theme::RED_TEXT)
                                        .child(lucide("circle-alert").xsmall().flex_shrink_0())
                                        .child(div().min_w_0().child(error))
                                }),
                        )
                });

        common::card(group.clone())
            .group(group.clone())
            .cursor_pointer()
            .when_some(badge, |card, (color, _)| {
                card.border_color(color.opacity(0.3))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                if this.busy.contains(&activate.id) {
                    return;
                }
                if running {
                    this.stop(activate.id.clone(), activate_ids.clone(), cx);
                } else if can_start {
                    this.forward(&activate, cx);
                }
            }))
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(common::icon_box(if busy {
                        "loader-circle"
                    } else {
                        "network"
                    }))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .truncate()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(preset.name.clone()),
                                    )
                                    .child(
                                        h_flex()
                                            .flex_shrink_0()
                                            .gap_1p5()
                                            .px_1p5()
                                            .rounded(theme::RADIUS)
                                            .border_1()
                                            .border_color(theme::hairline())
                                            .text_size(px(11.))
                                            .text_color(badge_color)
                                            .child(dot(badge_color))
                                            .child(SharedString::from(badge_label.to_uppercase())),
                                    ),
                            )
                            .child(match host {
                                Some(host) => div()
                                    .truncate()
                                    .text_xs()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(format!(
                                        "{}@{}:{}",
                                        host.username, host.host, host.port
                                    )),
                                None => h_flex()
                                    .gap_1p5()
                                    .text_xs()
                                    .text_color(theme::AMBER_TEXT)
                                    .child(lucide("circle-alert").xsmall().flex_shrink_0())
                                    .child(t("Connection was deleted")),
                            }),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .gap_1()
                            .invisible()
                            .group_hover(group, |style| style.visible())
                            .child(
                                icon_button(
                                    "edit",
                                    "pencil",
                                    if running {
                                        t("Stop the forward to edit it")
                                    } else {
                                        t("Edit")
                                    },
                                )
                                .disabled(running)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.edit(&edit, cx);
                                    },
                                )),
                            )
                            .child(
                                icon_button(
                                    "delete",
                                    "trash",
                                    if running {
                                        t("Stop the forward to delete it")
                                    } else {
                                        t("Delete")
                                    },
                                )
                                .disabled(running || busy)
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.request_delete(&delete, window, cx);
                                    },
                                )),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .mt_3()
                    .rounded(theme::RADIUS)
                    .border_1()
                    .border_color(theme::hairline())
                    .bg(theme::RAIL)
                    .text_xs()
                    .children(mappings),
            )
            .context_menu(move |menu, _, _| {
                let item = |label: SharedString, icon: &'static str| {
                    PopupMenuItem::new(label).icon(lucide(icon))
                };
                let (toggle_view, edit_view, delete_view) =
                    (view.clone(), view.clone(), view.clone());
                let (toggle, ids) = (menu_preset.clone(), menu_ids.clone());
                let (edit, delete) = (menu_preset.id.clone(), menu_preset.clone());
                let toggle_item = if running {
                    item(t("Stop"), "square")
                        .disabled(busy)
                        .on_click(move |_, _, cx| {
                            toggle_view.update(cx, |this, cx| {
                                this.stop(toggle.id.clone(), ids.clone(), cx)
                            });
                        })
                } else {
                    item(t("Forward"), "play")
                        .disabled(!can_start || busy)
                        .on_click(move |_, _, cx| {
                            toggle_view.update(cx, |this, cx| this.forward(&toggle, cx));
                        })
                };
                menu.label(menu_preset.name.clone())
                    .separator()
                    .item(toggle_item)
                    .item(
                        item(t("Edit"), "pencil")
                            .disabled(running)
                            .on_click(move |_, _, cx| {
                                edit_view.update(cx, |this, cx| this.edit(&edit, cx));
                            }),
                    )
                    .separator()
                    .item(
                        item(t("Delete"), "trash")
                            .disabled(running || busy)
                            .on_click(move |_, window, cx| {
                                delete_view.update(cx, |this, cx| {
                                    this.request_delete(&delete, window, cx)
                                });
                            }),
                    )
            })
    }
}

impl Render for ForwardsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        let query = self.search.read(cx).value().trim().to_lowercase();
        let mut visible: Vec<&PortForwardRecord> = store
            .forwards
            .iter()
            .filter(|forward| {
                let matches = |text: String| text.to_lowercase().contains(&query);
                query.is_empty()
                    || matches(forward.name.clone())
                    || store
                        .host(&forward.host_id)
                        .is_some_and(|host| matches(format!("{}@{}", host.username, host.host)))
                    || forward.mappings.iter().any(|mapping| {
                        matches(format!("{}:{}", mapping.bind_host, mapping.bind_port))
                            || matches(format!("{}:{}", mapping.target_host, mapping.target_port))
                    })
            })
            .collect();
        visible.sort_by_key(|forward| forward.name.to_lowercase());
        let body = if store.forwards.is_empty() {
            common::empty_state(t("No saved forwards yet")).into_any_element()
        } else if visible.is_empty() {
            common::empty_state(t("No forwards match your search.")).into_any_element()
        } else {
            div()
                .grid()
                .grid_cols(3)
                .gap_3()
                .children(
                    visible
                        .into_iter()
                        .map(|forward| self.render_card(forward, cx)),
                )
                .into_any_element()
        };

        common::page(
            t("Port Forwards"),
            h_flex()
                .gap_2()
                .child(
                    div().w(px(192.)).child(
                        Input::new(&self.search)
                            .small()
                            .prefix(lucide("search").xsmall().text_color(theme::STENCIL_FAINT)),
                    ),
                )
                .child(
                    Button::new("add-forward")
                        .primary()
                        .small()
                        .icon(lucide("plus"))
                        .label(t("New Forward"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.store.update(cx, |store, cx| {
                                store.navigate(Route::ForwardForm(None), cx)
                            });
                        })),
                ),
            common::page_body("forwards")
                .children(
                    self.error
                        .clone()
                        .map(|error| common::error_banner(error).mb_4()),
                )
                .child(body),
        )
    }
}

// --- Form --------------------------------------------------------------------

struct MappingRow {
    bind_host: Entity<InputState>,
    bind_port: Entity<InputState>,
    target_host: Entity<InputState>,
    target_port: Entity<InputState>,
    /// Copies the bind port into the target port as it is typed.
    _mirror: Subscription,
}

pub struct ForwardForm {
    store: Entity<Store>,
    /// The preset being edited; `None` adds a new one.
    id: Option<String>,
    /// Editing a preset that no longer exists.
    missing: bool,
    /// Empty value means no connection chosen yet.
    connection: Entity<ChoiceSelect>,
    name: Entity<InputState>,
    rows: Vec<MappingRow>,
    error: Option<SharedString>,
    saving: bool,
    _subscription: Subscription,
}

impl ForwardForm {
    pub fn new(id: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let (forward, connections) = {
            let store = store.read(cx);
            let forward = id
                .as_deref()
                .and_then(|id| store.forwards.iter().find(|forward| forward.id == id))
                .cloned();
            let mut hosts: Vec<&SshHostRecord> = store.hosts.iter().collect();
            hosts.sort_by_key(|host| host.name.to_lowercase());
            let mut connections = vec![Choice::new("", t("— Select a saved connection —"))];
            connections.extend(hosts.into_iter().map(|host| {
                Choice::new(
                    host.id.clone(),
                    format!(
                        "{} ({}@{}:{})",
                        host.name, host.username, host.host, host.port
                    ),
                )
            }));
            (forward, connections)
        };
        let host_value = forward
            .as_ref()
            .map(|forward| forward.host_id.clone())
            .unwrap_or_default();
        let name_value = forward
            .as_ref()
            .map(|forward| forward.name.clone())
            .unwrap_or_default();
        let rows = match &forward {
            Some(forward) => forward
                .mappings
                .iter()
                .map(|mapping| Self::new_row(Some(mapping), window, cx))
                .collect(),
            None => vec![Self::new_row(None, window, cx)],
        };
        let connection = common::new_select(connections, &host_value, window, cx);
        Self {
            missing: id.is_some() && forward.is_none(),
            // The summary under the dropdown follows the selection.
            _subscription: cx.observe(&connection, |_, _, cx| cx.notify()),
            connection,
            name: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(tf("e.g. {example}", &[("example", "Staging Postgres")]))
                    .default_value(name_value)
            }),
            rows,
            store,
            id,
            error: None,
            saving: false,
        }
    }

    fn new_row(
        mapping: Option<&PortForwardMappingInput>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> MappingRow {
        let mut host = |value: Option<&String>| {
            let value = value.cloned().unwrap_or_else(|| "127.0.0.1".to_string());
            cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(tf("e.g. {example}", &[("example", "127.0.0.1")]))
                    .default_value(value)
            })
        };
        let bind_host = host(mapping.map(|mapping| &mapping.bind_host));
        let target_host = host(mapping.map(|mapping| &mapping.target_host));
        let mut port = |value: Option<i32>| {
            let value = value.map(|port| port.to_string()).unwrap_or_default();
            cx.new(|cx| InputState::new(window, cx).default_value(value))
        };
        let bind_port = port(mapping.map(|mapping| mapping.bind_port));
        let target_port = port(mapping.map(|mapping| mapping.target_port));
        let target = target_port.clone();
        let mirror = cx.subscribe_in(
            &bind_port,
            window,
            move |_, bind_port, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = bind_port.read(cx).value();
                    target.update(cx, |target, cx| target.set_value(value, window, cx));
                }
            },
        );
        MappingRow {
            bind_host,
            bind_port,
            target_host,
            target_port,
            _mirror: mirror,
        }
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        self.store
            .update(cx, |store, cx| store.navigate(Route::Forwards, cx));
    }

    /// The preset the fields describe, or what is wrong with them.
    fn request(&self, cx: &App) -> Result<PortForwardWriteRequest, SharedString> {
        let host_id = common::selected(&self.connection, cx)
            .filter(|id| self.store.read(cx).host(id).is_some())
            .ok_or_else(|| t("Select a saved connection to continue"))?;
        let name = self.name.read(cx).value().trim().to_string();
        if name.is_empty() {
            return Err(t("Name is required"));
        }
        if self.rows.is_empty() {
            return Err(t("Add at least one port mapping"));
        }
        let mut mappings: Vec<PortForwardMappingInput> = Vec::new();
        for (index, row) in self.rows.iter().enumerate() {
            let label = tf("Row {number}", &[("number", &(index + 1).to_string())]);
            let port = |input: &Entity<InputState>, field: SharedString| {
                parse_port(&input.read(cx).value()).ok_or_else(|| {
                    tf(
                        "{label} must be a number from 1 to 65535",
                        &[("label", &field)],
                    )
                })
            };
            let bind_host = row.bind_host.read(cx).value().trim().to_string();
            let bind_port = port(
                &row.bind_port,
                tf("{label}: Bind port", &[("label", &label)]),
            )?;
            if mappings
                .iter()
                .any(|mapping| mapping.bind_host == bind_host && mapping.bind_port == bind_port)
            {
                return Err(tf(
                    "{label}: {address} is already used above",
                    &[
                        ("label", &label),
                        ("address", &format!("{bind_host}:{bind_port}")),
                    ],
                ));
            }
            mappings.push(PortForwardMappingInput {
                bind_host,
                bind_port,
                target_host: row.target_host.read(cx).value().trim().to_string(),
                target_port: port(
                    &row.target_port,
                    tf("{label}: Target port", &[("label", &label)]),
                )?,
            });
        }
        Ok(PortForwardWriteRequest {
            name,
            host_id,
            mappings,
        })
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let request = match self.request(cx) {
            Ok(request) => request,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        self.error = None;
        self.saving = true;
        cx.notify();
        let (app, id) = (self.store.read(cx).app(), self.id.clone());
        cx.spawn(async move |this, cx| {
            let saved = backend(async move {
                match id {
                    Some(id) => {
                        crate::store::port_forwards::port_forward_preset_update(
                            id,
                            request,
                            app.state(),
                        )
                        .await
                    }
                    None => {
                        crate::store::port_forwards::port_forward_preset_create(
                            request,
                            app.state(),
                        )
                        .await
                    }
                }
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                this.saving = false;
                match saved {
                    Ok(_) => {
                        this.store.update(cx, |store, cx| store.refresh(cx));
                        this.back(cx);
                    }
                    Err(error) => this.error = Some(error.into()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn render_row(&self, index: usize, row: &MappingRow, cx: &Context<Self>) -> impl IntoElement {
        let number = (index + 1).to_string();
        let saving = self.saving;
        let pair = |host_label: SharedString,
                    host: &Entity<InputState>,
                    port_label: SharedString,
                    port: &Entity<InputState>| {
            h_flex()
                .items_start()
                .gap_3()
                .child(common::field(host_label, Input::new(host).disabled(saving), None).flex_1())
                .child(
                    common::field(port_label, Input::new(port).disabled(saving), None).w(px(112.)),
                )
        };
        v_flex()
            .gap_2()
            .p_3()
            .rounded(theme::RADIUS)
            .border_1()
            .border_color(theme::hairline())
            .bg(theme::RAIL)
            .child(
                h_flex()
                    .justify_between()
                    .child(common::section_title(tf(
                        "Port {number}",
                        &[("number", &number)],
                    )))
                    .child(
                        Button::new(("remove-port", index))
                            .ghost()
                            .xsmall()
                            .icon(lucide("trash"))
                            .tooltip(tf("Remove port {number}", &[("number", &number)]))
                            .disabled(saving || self.rows.len() == 1)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.rows.remove(index);
                                cx.notify();
                            })),
                    ),
            )
            .child(pair(
                t("Bind host"),
                &row.bind_host,
                t("Bind port"),
                &row.bind_port,
            ))
            .child(
                h_flex().justify_center().py_1().child(
                    lucide("arrow-down")
                        .small()
                        .text_color(theme::STENCIL_FAINT),
                ),
            )
            .child(pair(
                t("Target host"),
                &row.target_host,
                t("Target port"),
                &row.target_port,
            ))
    }
}

/// A TCP port typed by the user: digits only, 1 to 65535.
fn parse_port(value: &str) -> Option<i32> {
    let value = value.trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok().filter(|port| (1..=65535).contains(port))
}

impl Render for ForwardForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.missing {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_5()
                .child(lucide("circle-alert").size_7().text_color(theme::RED_TEXT))
                .child(
                    div()
                        .text_xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(t("Port forward not found")),
                )
                .child(
                    Button::new("back")
                        .primary()
                        .label(t("Back to port forwards"))
                        .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
                )
                .into_any_element();
        }
        let editing = self.id.is_some();
        let submit_label = match (self.saving, editing) {
            (true, _) => t("Saving…"),
            (false, true) => t("Save changes"),
            (false, false) => t("Save forward"),
        };
        let saving = self.saving;
        let store = self.store.read(cx);
        let selected = common::selected(&self.connection, cx);
        let summary = selected
            .as_deref()
            .and_then(|id| store.host(id))
            .map(|host| {
                let auth = match host.auth {
                    Some(SshHostAuthMaterial::PublicKeyAndPassword { .. }) => t("Key + Password"),
                    Some(SshHostAuthMaterial::PublicKey { .. }) => t("SSH Key"),
                    _ => t("Password"),
                };
                v_flex()
                    .gap_0p5()
                    .px_3()
                    .py_2p5()
                    .rounded(theme::RADIUS)
                    .border_1()
                    .border_color(theme::hairline())
                    .bg(theme::RAIL)
                    .child(
                        div()
                            .truncate()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(host.name.clone()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(11.))
                            .text_color(theme::STENCIL_MUTE)
                            .child(format!("{}@{}:{}", host.username, host.host, host.port)),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::STENCIL_MUTE)
                            .child(auth),
                    )
            });
        let connection_section = common::form_section(
            "server",
            t("Saved connection"),
            Some(t(
                "Target host is resolved from this SSH server, not your local machine.",
            )),
        );
        let connection_section = if store.hosts.is_empty() {
            connection_section.child(
                v_flex()
                    .items_center()
                    .gap_1()
                    .px_4()
                    .py_6()
                    .rounded(theme::RADIUS)
                    .border_1()
                    .border_dashed()
                    .border_color(theme::hairline())
                    .child(
                        lucide("server")
                            .size_8()
                            .mb_2()
                            .text_color(theme::STENCIL_FAINT),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme::STENCIL_MUTE)
                            .child(t("No saved connections yet.")),
                    )
                    .child(div().text_xs().text_color(theme::STENCIL_MUTE).child(tf(
                        "Create a Host in {page} first, then return here to save a tunnel.",
                        &[("page", &t("Connections"))],
                    ))),
            )
        } else {
            connection_section
                .child(common::field(
                    t("Connection"),
                    Select::new(&self.connection).disabled(saving),
                    None,
                ))
                .children(summary)
                .child(common::field(
                    t("Name"),
                    Input::new(&self.name).disabled(saving),
                    None,
                ))
        };

        common::form_page(
            if editing {
                t("Edit port forward")
            } else {
                t("New port forward")
            },
            self.error.clone(),
            v_flex().gap_4().child(connection_section).child(
                common::form_section("network", t("Forward route"), None)
                    .children(
                        self.rows
                            .iter()
                            .enumerate()
                            .map(|(index, row)| self.render_row(index, row, cx)),
                    )
                    .child(
                        div().flex().child(
                            Button::new("add-port")
                                .outline()
                                .small()
                                .icon(lucide("plus"))
                                .label(t("Add port"))
                                .disabled(saving)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    let row = Self::new_row(None, window, cx);
                                    this.rows.push(row);
                                    cx.notify();
                                })),
                        ),
                    ),
            ),
            common::form_buttons(
                submit_label,
                saving,
                cx.listener(|this, _, _, cx| this.back(cx)),
                cx.listener(|this, _, _, cx| this.save(cx)),
            ),
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use shared::{AiAccess, PortForwardMappingInput, PortForwardRecord, SshHostRecord};

    use super::{group_state, parse_port, runtimes_for};
    use crate::runtime::port_forward::{PortForwardState, PortForwardStatus};

    #[test]
    fn ports_are_digits_from_1_to_65535() {
        assert_eq!(parse_port(" 5432 "), Some(5432));
        assert_eq!(parse_port("65535"), Some(65535));
        for bad in ["", "0", "65536", "-1", "+80", "80.0", "http", "99999999999"] {
            assert_eq!(parse_port(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_group_is_as_healthy_as_its_worst_mapping() {
        let mapping = |port: i32| PortForwardMappingInput {
            bind_host: "127.0.0.1".to_string(),
            bind_port: port,
            target_host: "db".to_string(),
            target_port: port,
        };
        let preset = PortForwardRecord {
            id: "preset".to_string(),
            name: "Postgres".to_string(),
            host_id: "host".to_string(),
            host_name: "api".to_string(),
            mappings: vec![mapping(5432), mapping(6379)],
        };
        let host = SshHostRecord {
            id: "host".to_string(),
            name: "api".to_string(),
            host: "10.0.0.11".to_string(),
            port: 22,
            username: "deploy".to_string(),
            ssh_key_id: None,
            group_id: None,
            jump_host_id: None,
            auth: None,
            ai_access: AiAccess::Ask,
        };
        let status = |id: &str, name: &str, port: u16, state: PortForwardState| PortForwardStatus {
            id: id.to_string(),
            name: name.to_string(),
            host: "10.0.0.11".to_string(),
            port: 22,
            username: "deploy".to_string(),
            bind_host: "127.0.0.1".to_string(),
            bind_port: port,
            target_host: "db".to_string(),
            target_port: port,
            state,
            error: None,
        };
        let state_of = |statuses: Vec<PortForwardStatus>, host: Option<&SshHostRecord>| {
            let forwards: HashMap<String, PortForwardStatus> = statuses
                .into_iter()
                .map(|status| (status.id.clone(), status))
                .collect();
            let runtimes = runtimes_for(&preset, host, &forwards);
            (
                runtimes
                    .iter()
                    .map(|runtime| runtime.map(|forward| forward.id.clone()))
                    .collect::<Vec<_>>(),
                group_state(&runtimes),
            )
        };

        let (ids, state) = state_of(Vec::new(), Some(&host));
        assert_eq!(ids, [None, None]);
        assert!(state.is_none());

        // Another preset's tunnel on the same ports is not this preset's.
        let other = status("x", "Other", 5432, PortForwardState::Listening);
        assert!(state_of(vec![other], Some(&host)).1.is_none());

        let one = status("a", "Postgres", 5432, PortForwardState::Listening);
        let (ids, state) = state_of(vec![one.clone()], Some(&host));
        assert_eq!(ids, [Some("a".to_string()), None]);
        assert!(matches!(state, Some(PortForwardState::Stopped)));
        assert!(state_of(vec![one.clone()], None).1.is_none());

        let two = |state| status("b", "Postgres", 6379, state);
        let both = |state| state_of(vec![one.clone(), two(state)], Some(&host)).1;
        assert!(matches!(
            both(PortForwardState::Listening),
            Some(PortForwardState::Listening)
        ));
        assert!(matches!(
            both(PortForwardState::Connecting),
            Some(PortForwardState::Connecting)
        ));
        assert!(matches!(
            both(PortForwardState::Error),
            Some(PortForwardState::Error)
        ));
    }
}
