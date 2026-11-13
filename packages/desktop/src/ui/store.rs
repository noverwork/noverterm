//! App-wide UI state: saved data, open sessions, and the current page.

use std::collections::{HashMap, HashSet};

use gpui_kit::*;
use serde_json::{json, Value};
use shared::{
    HostGroupRecord, PortForwardRecord, Setting, SnippetRecord, SnippetWriteRequest,
    SshHostAuthMaterial, SshHostRecord, SshKeyRecord,
};
use tokio::sync::mpsc::unbounded_channel;

use crate::app::AppHandle;
use crate::connect::DirectSshConnectInput;
use crate::runtime::local::LocalSessionManager;
use crate::runtime::ssh::{SshConnectResponse, SshHopInput, SshSessionManager};
use crate::runtime::terminal_output::{Channel, TerminalOutput};
use crate::trust::{HostTrustConfirmation, HostTrustMismatch, HostTrustPrompt};
use crate::ui::bridge::backend;
use crate::ui::i18n::{self, t, tf, Locale};
use crate::ui::terminal::{CursorKind, SessionKind, TerminalConfig, TerminalEvent, TerminalView};

const CONFIG_SETTING: &str = "noverterm-config";
const MAX_RECENT_CONNECTIONS: usize = 12;
const MAX_JUMP_HOPS: usize = 5;
/// Session ids are UUIDs; every output frame carries one after its kind byte.
const FRAME_HEADER: usize = 37;

/// Backend events the UI listens for.
const BACKEND_EVENTS: [&str; 10] = [
    "ssh_port_forward",
    "port_forward_status",
    "mcp_changed",
    "mcp_approval_request",
    "mcp_approval_expired",
    "mcp_activity",
    "ai_access_changed",
    "sftp://progress",
    "sftp://complete",
    "sftp://error",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Connections,
    ConnectionForm(Option<String>),
    Keys,
    KeyForm(Option<String>),
    Forwards,
    ForwardForm(Option<String>),
    KnownHosts,
    Snippets,
    /// Editing a snippet, or a new one; `shortcut` starts it pinned.
    SnippetForm {
        id: Option<String>,
        shortcut: bool,
    },
    Sftp,
    Mcp,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Connecting,
    Connected,
    TrustRequired,
    Disconnected,
    Error,
}

impl SessionStatus {
    /// A dropped or failed session keeps its tab until the user deals with it.
    pub fn is_held(self) -> bool {
        matches!(self, Self::Disconnected | Self::Error)
    }

    pub fn label(self) -> SharedString {
        match self {
            Self::Connected => t("live"),
            Self::Connecting => t("connecting"),
            Self::TrustRequired => t("awaiting host trust"),
            Self::Disconnected => t("held · disconnected"),
            Self::Error => t("held · failed"),
        }
    }
}

pub struct Session {
    /// A placeholder until the backend connects, then the backend session id.
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub status: SessionStatus,
    pub kind: SessionKind,
    pub connection_id: Option<String>,
    pub error: Option<String>,
    pub trust_prompt: Option<HostTrustPrompt>,
    pub trust_mismatch: Option<HostTrustMismatch>,
    pub terminal: Option<Entity<TerminalView>>,
    _terminal_events: Option<Subscription>,
}

pub enum StoreEvent {
    /// A backend event, as `(name, JSON payload)`.
    Backend(&'static str, String),
    /// The terminal of this session asked for the AI assistant.
    Assistant(String),
}

pub struct Store {
    app: AppHandle,
    pub route: Route,
    pub loading: bool,
    pub load_error: Option<String>,
    pub hosts: Vec<SshHostRecord>,
    pub groups: Vec<HostGroupRecord>,
    pub keys: Vec<SshKeyRecord>,
    pub forwards: Vec<PortForwardRecord>,
    pub snippets: Vec<SnippetRecord>,
    pub settings: Vec<Setting>,
    /// Open tabs, in tab order.
    pub sessions: Vec<Session>,
    pub active_session: Option<String>,
    pub trust_confirming: bool,
    pub trust_error: Option<String>,
    pub show_settings: bool,
    /// Grid size of the terminal on screen; new sessions start at it.
    grid: (usize, usize),
    /// Output that arrived before its session finished connecting.
    early_output: HashMap<String, Vec<u8>>,
    /// Sessions that closed before they finished connecting, with the error.
    early_close: HashMap<String, Option<String>>,
    /// Closed tabs whose backend session has not confirmed the close yet.
    removed: HashSet<String>,
    _tasks: Vec<Task<()>>,
}

impl EventEmitter<StoreEvent> for Store {}

/// The store, reachable from any view.
pub struct Globals {
    pub store: Entity<Store>,
}

impl Global for Globals {}

pub fn store(cx: &App) -> Entity<Store> {
    cx.global::<Globals>().store.clone()
}

impl Store {
    pub fn new(app: AppHandle, cx: &mut Context<Self>) -> Self {
        let (frames, mut incoming) = unbounded_channel::<Vec<u8>>();
        let subscribed = app
            .state::<TerminalOutput>()
            .subscribe(Channel::new(move |frame| {
                frames.send(frame).map_err(|error| error.to_string())
            }));
        let output = cx.spawn(async move |this, cx| {
            while let Some(frame) = incoming.recv().await {
                if this
                    .update(cx, |this, cx| this.on_output_frame(frame, cx))
                    .is_err()
                {
                    break;
                }
            }
        });

        let (events, mut backend_events) = unbounded_channel::<(&'static str, String)>();
        for name in BACKEND_EVENTS {
            let events = events.clone();
            app.listen(name, move |event| {
                let _ = events.send((name, event.payload().to_string()));
            });
        }
        let listener = cx.spawn(async move |this, cx| {
            while let Some((name, payload)) = backend_events.recv().await {
                let alive = this.update(cx, |this, cx| {
                    if name == "ai_access_changed" {
                        this.refresh(cx);
                    }
                    cx.emit(StoreEvent::Backend(name, payload));
                });
                if alive.is_err() {
                    break;
                }
            }
        });

        let mut store = Self {
            app,
            route: Route::Connections,
            loading: true,
            load_error: subscribed.err(),
            hosts: Vec::new(),
            groups: Vec::new(),
            keys: Vec::new(),
            forwards: Vec::new(),
            snippets: Vec::new(),
            settings: Vec::new(),
            sessions: Vec::new(),
            active_session: None,
            trust_confirming: false,
            trust_error: None,
            show_settings: false,
            grid: (80, 24),
            early_output: HashMap::new(),
            early_close: HashMap::new(),
            removed: HashSet::new(),
            _tasks: vec![output, listener],
        };
        store.refresh(cx);
        store
    }

    pub fn app(&self) -> AppHandle {
        self.app.clone()
    }

    pub fn navigate(&mut self, route: Route, cx: &mut Context<Self>) {
        self.route = route;
        cx.notify();
    }

    // --- Saved data ---------------------------------------------------------

    /// Reloads everything saved in the database.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let app = self.app.clone();
        cx.spawn(async move |this, cx| {
            let loaded = backend(async move {
                Ok::<_, String>((
                    crate::store::settings::get_all_settings(app.state()).await?,
                    crate::store::groups::host_group_list(app.state()).await?,
                    crate::store::hosts::host_list(app.state()).await?,
                    crate::store::keys::key_list(app.state()).await?,
                    crate::store::port_forwards::port_forward_preset_list(app.state()).await?,
                    crate::store::snippets::snippet_list(app.state()).await?,
                ))
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                match loaded {
                    Ok((settings, groups, hosts, keys, forwards, snippets)) => {
                        let first_load = this.loading;
                        this.settings = settings;
                        this.groups = groups;
                        this.hosts = hosts;
                        this.keys = keys;
                        this.forwards = forwards;
                        this.snippets = snippets;
                        this.load_error = None;
                        if first_load {
                            let saved = this.setting(i18n::LOCALE_SETTING);
                            i18n::set_locale(
                                saved
                                    .and_then(Locale::from_code)
                                    .unwrap_or_else(Locale::system),
                            );
                        }
                        this.apply_terminal_config(cx);
                    }
                    Err(error) => this.load_error = Some(error),
                }
                this.loading = false;
                cx.notify();
            });
        })
        .detach();
    }

    pub fn setting(&self, key: &str) -> Option<&str> {
        self.settings
            .iter()
            .find(|setting| setting.key == key)
            .map(|setting| setting.value.as_str())
    }

    /// Saves one setting, then reloads.
    pub fn set_setting(&mut self, key: &str, value: String, cx: &mut Context<Self>) {
        let app = self.app.clone();
        let setting = Setting {
            key: key.to_string(),
            value,
        };
        // Reflect it right away; the reload confirms it.
        match self
            .settings
            .iter_mut()
            .find(|saved| saved.key == setting.key)
        {
            Some(saved) => saved.value = setting.value.clone(),
            None => self.settings.push(setting.clone()),
        }
        cx.spawn(async move |this, cx| {
            let saved =
                backend(
                    async move { crate::store::settings::set_setting(setting, app.state()).await },
                )
                .await;
            if let Err(error) = saved {
                tracing::warn!(error, "Failed to save setting");
            }
            let _ = this.update(cx, |this, cx| this.refresh(cx));
        })
        .detach();
        cx.notify();
    }

    fn config(&self) -> Value {
        self.setting(CONFIG_SETTING)
            .and_then(|value| serde_json::from_str::<Value>(value).ok())
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({}))
    }

    pub fn terminal_config(&self) -> TerminalConfig {
        let config = self.config();
        let terminal = &config["terminal"];
        let defaults = TerminalConfig::default();
        TerminalConfig {
            font_size: terminal["fontSize"]
                .as_f64()
                .map_or(defaults.font_size, |size| size as f32),
            cursor_style: serde_json::from_value::<CursorKind>(terminal["cursorStyle"].clone())
                .unwrap_or(defaults.cursor_style),
            cursor_blink: terminal["cursorBlink"]
                .as_bool()
                .unwrap_or(defaults.cursor_blink),
            scrollback: terminal["scrollback"]
                .as_u64()
                .map_or(defaults.scrollback, |lines| lines as usize),
            option_as_meta: terminal["optionAsMeta"]
                .as_bool()
                .unwrap_or(defaults.option_as_meta),
        }
    }

    /// Saves the terminal settings and applies them to every open tab.
    pub fn set_terminal_config(&mut self, terminal: TerminalConfig, cx: &mut Context<Self>) {
        let mut config = self.config();
        config["terminal"] = json!({
            "fontSize": terminal.font_size,
            "cursorStyle": terminal.cursor_style,
            "cursorBlink": terminal.cursor_blink,
            "scrollback": terminal.scrollback,
            "optionAsMeta": terminal.option_as_meta,
        });
        self.set_setting(CONFIG_SETTING, config.to_string(), cx);
        self.apply_terminal_config(cx);
    }

    fn apply_terminal_config(&self, cx: &mut Context<Self>) {
        let config = self.terminal_config();
        for terminal in self
            .sessions
            .iter()
            .filter_map(|session| session.terminal.as_ref())
        {
            terminal.update(cx, |terminal, cx| terminal.set_config(config.clone(), cx));
        }
    }

    /// Recently opened connections that still exist, newest first.
    pub fn recent_connection_ids(&self) -> Vec<String> {
        self.config()["recentConnectionIds"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|id| id.as_str())
            .filter(|id| self.host(id).is_some())
            .map(str::to_string)
            .collect()
    }

    fn record_recent_connection(&mut self, host_id: &str, cx: &mut Context<Self>) {
        let mut recent = self.recent_connection_ids();
        recent.retain(|id| id != host_id);
        recent.insert(0, host_id.to_string());
        recent.truncate(MAX_RECENT_CONNECTIONS);
        let mut config = self.config();
        config["recentConnectionIds"] = json!(recent);
        self.set_setting(CONFIG_SETTING, config.to_string(), cx);
    }

    /// Drops a deleted host from the recent list.
    pub fn forget_recent_connection(&mut self, host_id: &str, cx: &mut Context<Self>) {
        let mut recent = self.recent_connection_ids();
        recent.retain(|id| id != host_id);
        let mut config = self.config();
        config["recentConnectionIds"] = json!(recent);
        self.set_setting(CONFIG_SETTING, config.to_string(), cx);
    }

    pub fn host(&self, id: &str) -> Option<&SshHostRecord> {
        self.hosts.iter().find(|host| host.id == id)
    }

    /// Pinned snippets in sidebar order.
    pub fn pinned_snippets(&self) -> Vec<SnippetRecord> {
        let mut pinned: Vec<SnippetRecord> = self
            .snippets
            .iter()
            .filter(|snippet| snippet.pin_icon.is_some())
            .cloned()
            .collect();
        pinned.sort_by_key(|snippet| snippet.pin_order.unwrap_or(0));
        pinned
    }

    /// The jump chain for `host`, outermost bastion first.
    pub fn jumps(&self, host: &SshHostRecord) -> Vec<SshHopInput> {
        let mut jumps = Vec::new();
        let mut next = host.jump_host_id.as_deref().and_then(|id| self.host(id));
        // The store rejects loops and chains deeper than MAX_JUMP_HOPS; the cap
        // only keeps stale data from spinning forever.
        while let Some(hop) = next {
            if jumps.len() >= MAX_JUMP_HOPS {
                break;
            }
            let (password, private_key, passphrase) = auth_parts(hop.auth.as_ref());
            jumps.insert(
                0,
                SshHopInput {
                    host: hop.host.clone(),
                    port: hop.port as u16,
                    username: hop.username.clone(),
                    password,
                    private_key,
                    passphrase,
                },
            );
            next = hop.jump_host_id.as_deref().and_then(|id| self.host(id));
        }
        jumps
    }

    /// What the backend needs to open an SSH connection to a saved host.
    pub fn connect_input(&self, host: &SshHostRecord) -> Result<DirectSshConnectInput, String> {
        if host.auth.is_none() {
            return Err(tf(
                "{name} has no saved password or SSH key",
                &[("name", &host.name)],
            )
            .to_string());
        }
        let (password, private_key, passphrase) = auth_parts(host.auth.as_ref());
        Ok(DirectSshConnectInput {
            host: host.host.clone(),
            port: host.port as u16,
            username: host.username.clone(),
            password,
            private_key,
            passphrase,
            jumps: self.jumps(host),
        })
    }

    // --- Sessions -----------------------------------------------------------

    pub fn session(&self, id: &str) -> Option<&Session> {
        self.sessions.iter().find(|session| session.id == id)
    }

    fn session_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.iter_mut().find(|session| session.id == id)
    }

    pub fn active(&self) -> Option<&Session> {
        self.active_session
            .as_deref()
            .and_then(|id| self.session(id))
    }

    pub fn set_active(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        self.active_session = id;
        cx.notify();
    }

    /// Switches to a tab. Coming back to an SSH tab whose connection dropped
    /// reconnects it once; trust prompts still wait for the user.
    pub fn activate_session(&mut self, id: &str, cx: &mut Context<Self>) {
        self.active_session = Some(id.to_string());
        self.route = Route::Terminal;
        let dropped = self
            .session(id)
            .is_some_and(|session| session.kind == SessionKind::Ssh && session.status.is_held());
        if dropped {
            self.retry_active(cx);
        }
        cx.notify();
    }

    fn add_pending(
        &mut self,
        name: String,
        kind: SessionKind,
        host: Option<&SshHostRecord>,
        cx: &mut Context<Self>,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.sessions.push(Session {
            id: id.clone(),
            name,
            host: host.map_or_else(|| "localhost".to_string(), |host| host.host.clone()),
            port: host.map_or(0, |host| host.port as u16),
            username: host.map_or_else(String::new, |host| host.username.clone()),
            status: SessionStatus::Connecting,
            kind,
            connection_id: host.map(|host| host.id.clone()),
            error: None,
            trust_prompt: None,
            trust_mismatch: None,
            terminal: None,
            _terminal_events: None,
        });
        self.active_session = Some(id.clone());
        self.route = Route::Terminal;
        cx.notify();
        id
    }

    fn fail(&mut self, id: &str, error: String, cx: &mut Context<Self>) {
        if let Some(session) = self.session_mut(id) {
            session.status = SessionStatus::Error;
            session.error = Some(error);
        }
        cx.notify();
    }

    /// Swaps the placeholder session for the connected one and gives it a terminal.
    fn finish_connection(&mut self, pending_id: &str, session_id: String, cx: &mut Context<Self>) {
        let config = self.terminal_config();
        let (app, grid) = (self.app.clone(), self.grid);
        let was_active = self.active_session.as_deref() == Some(pending_id);
        let early_output = self.early_output.remove(&session_id);
        let early_close = self.early_close.remove(&session_id);
        let Some(session) = self.session_mut(pending_id) else {
            // The tab was closed while it connected.
            self.removed.insert(session_id.clone());
            self.disconnect_backend(session_id, SessionKind::Ssh, cx);
            return;
        };
        let kind = session.kind;
        let terminal = cx.new(|cx| {
            let mut terminal = TerminalView::new(session_id.clone(), kind, config, app, cx);
            terminal.set_grid(grid.0, grid.1, cx);
            if let Some(output) = &early_output {
                terminal.feed(output, cx);
            }
            terminal
        });
        let events = cx.subscribe(&terminal, Self::on_terminal_event);
        session.id = session_id.clone();
        session.terminal = Some(terminal);
        session._terminal_events = Some(events);
        session.trust_prompt = None;
        session.trust_mismatch = None;
        (session.status, session.error) = match early_close {
            Some(Some(error)) => (SessionStatus::Error, Some(error)),
            Some(None) => (SessionStatus::Disconnected, None),
            None => (SessionStatus::Connected, None),
        };
        if was_active {
            self.active_session = Some(session_id);
        }
        cx.notify();
    }

    fn on_terminal_event(
        &mut self,
        terminal: Entity<TerminalView>,
        event: &TerminalEvent,
        cx: &mut Context<Self>,
    ) {
        let id = terminal.read(cx).session_id().to_string();
        match event {
            TerminalEvent::Resized { cols, rows } => {
                self.grid = (*cols, *rows);
                for session in &self.sessions {
                    if let Some(other) = session
                        .terminal
                        .as_ref()
                        .filter(|other| **other != terminal)
                    {
                        other.update(cx, |other, cx| other.set_grid(*cols, *rows, cx));
                    }
                }
            }
            TerminalEvent::Error(error) => self.fail(&id, error.clone(), cx),
            TerminalEvent::RequestClose => {
                // A hidden terminal can keep focus while another tab is shown.
                if let Some(active) = self.active_session.clone() {
                    self.close_sessions(&[active], cx);
                }
            }
            TerminalEvent::RequestAssistant => cx.emit(StoreEvent::Assistant(id)),
        }
    }

    fn on_output_frame(&mut self, frame: Vec<u8>, cx: &mut Context<Self>) {
        if frame.len() < FRAME_HEADER || frame[0] > 2 {
            tracing::error!("Invalid terminal output frame");
            return;
        }
        let Ok(id) = std::str::from_utf8(&frame[1..FRAME_HEADER]) else {
            return;
        };
        let (kind, data) = (frame[0], &frame[FRAME_HEADER..]);
        if kind == 0 {
            match self
                .session(id)
                .and_then(|session| session.terminal.clone())
            {
                Some(terminal) => terminal.update(cx, |terminal, cx| terminal.feed(data, cx)),
                None if self.removed.contains(id) => {}
                None => self
                    .early_output
                    .entry(id.to_string())
                    .or_default()
                    .extend_from_slice(data),
            }
            // The backend holds further output until this much is acknowledged.
            if let Err(error) = self
                .app
                .state::<TerminalOutput>()
                .ack(id, data.len() as u32)
            {
                let id = id.to_string();
                self.fail(&id, error, cx);
            }
            return;
        }

        let error = (kind == 2).then(|| String::from_utf8_lossy(data).into_owned());
        if self.removed.remove(id) {
            return;
        }
        let id = id.to_string();
        match self.session_mut(&id) {
            Some(session) => {
                if let Some(error) = error {
                    session.status = SessionStatus::Error;
                    session.error = Some(error);
                } else if session.status != SessionStatus::Error {
                    session.status = SessionStatus::Disconnected;
                }
                cx.notify();
            }
            None => {
                self.early_close.insert(id, error);
            }
        }
    }

    /// A saved connection's tab name; repeats get a `#2`, `#3` suffix.
    fn session_name(&self, host: &SshHostRecord) -> String {
        let open = self
            .sessions
            .iter()
            .filter(|session| {
                session.connection_id.as_deref() == Some(host.id.as_str())
                    && session.status != SessionStatus::Disconnected
            })
            .count();
        if open == 0 {
            host.name.clone()
        } else {
            format!("{} #{}", host.name, open + 1)
        }
    }

    /// Opens a new tab connected to a saved host. `then_run` is typed into the
    /// session once it connects.
    pub fn connect_host(
        &mut self,
        host_id: &str,
        then_run: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(host) = self.host(host_id).cloned() else {
            return;
        };
        let name = self.session_name(&host);
        let pending = self.add_pending(name, SessionKind::Ssh, Some(&host), cx);
        self.connect_in_session(pending, host, then_run, cx);
    }

    fn connect_in_session(
        &mut self,
        pending: String,
        host: SshHostRecord,
        then_run: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let input = match self.connect_input(&host) {
            Ok(input) => input,
            // Fail the pending tab; otherwise it spins on "Connecting" forever.
            Err(error) => return self.fail(&pending, error, cx),
        };
        let (app, (cols, rows)) = (self.app.clone(), self.grid);
        cx.spawn(async move |this, cx| {
            let connected = backend(async move {
                crate::connect::ssh_connect_direct(
                    app.clone(),
                    input,
                    app.state(),
                    app.state(),
                    cols as u32,
                    rows as u32,
                )
                .await
            })
            .await;
            let _ = this.update(cx, |this, cx| match connected {
                Ok(SshConnectResponse::Connected { session_id }) => {
                    this.finish_connection(&pending, session_id.clone(), cx);
                    this.record_recent_connection(&host.id, cx);
                    if let Some(command) = then_run {
                        this.write_session(&session_id, format!("{command}\n"), cx);
                    }
                }
                Ok(SshConnectResponse::TrustRequired { prompt }) => {
                    if let Some(session) = this.session_mut(&pending) {
                        session.status = SessionStatus::TrustRequired;
                        session.error = None;
                        session.trust_prompt = Some(prompt);
                        session.trust_mismatch = None;
                    }
                    cx.notify();
                }
                Ok(SshConnectResponse::TrustMismatch { mismatch }) => {
                    let error = tf(
                        "Host trust mismatch for {host}: expected {expected}, got {presented}",
                        &[
                            ("host", &mismatch.host),
                            ("expected", &mismatch.expected_fingerprint),
                            ("presented", &mismatch.presented_fingerprint),
                        ],
                    );
                    if let Some(session) = this.session_mut(&pending) {
                        session.status = SessionStatus::Error;
                        session.error = Some(error.to_string());
                        session.trust_prompt = None;
                        session.trust_mismatch = Some(mismatch);
                    }
                    cx.notify();
                }
                Err(error) => this.fail(&pending, error, cx),
            });
        })
        .detach();
    }

    /// Opens a local shell tab. `then_run` is typed into it once it starts.
    pub fn connect_local(
        &mut self,
        name: Option<String>,
        then_run: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let name = name.unwrap_or_else(|| t("Local Terminal").to_string());
        let pending = self.add_pending(name, SessionKind::Local, None, cx);
        let (app, (cols, rows)) = (self.app.clone(), self.grid);
        cx.spawn(async move |this, cx| {
            let connected = backend(async move {
                app.state::<LocalSessionManager>()
                    .connect(&app.state::<TerminalOutput>(), cols as u32, rows as u32)
                    .await
            })
            .await;
            let _ = this.update(cx, |this, cx| match connected {
                Ok(session_id) => {
                    this.finish_connection(&pending, session_id.clone(), cx);
                    if let Some(command) = then_run {
                        this.write_session(&session_id, format!("{command}\n"), cx);
                    }
                }
                Err(error) => this.fail(&pending, error, cx),
            });
        })
        .detach();
    }

    /// Reconnects the active tab in place.
    pub fn retry_active(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.active() else {
            return;
        };
        let id = session.id.clone();
        let Some(host) = session
            .connection_id
            .as_deref()
            .and_then(|host_id| self.host(host_id))
            .cloned()
        else {
            return;
        };
        if let Some(session) = self.session_mut(&id) {
            session.status = SessionStatus::Connecting;
            session.error = None;
            session.trust_prompt = None;
            session.trust_mismatch = None;
            session.terminal = None;
            session._terminal_events = None;
            session.host = host.host.clone();
            session.port = host.port as u16;
            session.username = host.username.clone();
        }
        self.route = Route::Terminal;
        cx.notify();
        self.connect_in_session(id, host, None, cx);
    }

    /// Trusts the host key the active tab is asking about, then reconnects.
    pub fn trust_active_host(&mut self, cx: &mut Context<Self>) {
        let confirmation = self.active().and_then(|session| {
            session
                .trust_prompt
                .as_ref()
                .map(|prompt| HostTrustConfirmation {
                    host: prompt.host.clone(),
                    port: prompt.port,
                    algorithm: prompt.algorithm.clone(),
                    fingerprint: prompt.fingerprint.clone(),
                })
        });
        self.confirm_trust_and_retry(confirmation, cx);
    }

    /// Replaces the saved host key with the one the host presented.
    pub fn trust_new_host_key(&mut self, cx: &mut Context<Self>) {
        let confirmation = self.active().and_then(|session| {
            session
                .trust_mismatch
                .as_ref()
                .map(|mismatch| HostTrustConfirmation {
                    host: mismatch.host.clone(),
                    port: mismatch.port,
                    algorithm: mismatch.presented_algorithm.clone(),
                    fingerprint: mismatch.presented_fingerprint.clone(),
                })
        });
        self.confirm_trust_and_retry(confirmation, cx);
    }

    fn confirm_trust_and_retry(
        &mut self,
        confirmation: Option<HostTrustConfirmation>,
        cx: &mut Context<Self>,
    ) {
        let Some(confirmation) = confirmation else {
            return;
        };
        let saved = self
            .active()
            .and_then(|session| session.connection_id.as_deref())
            .is_some_and(|host_id| self.host(host_id).is_some());
        if !saved {
            self.trust_error =
                Some(t("Saved connection not found. Open Connections and try again.").to_string());
            cx.notify();
            return;
        }
        self.trust_confirming = true;
        self.trust_error = None;
        cx.notify();
        let app = self.app.clone();
        cx.spawn(async move |this, cx| {
            let confirmed = backend(async move {
                crate::connect::ssh_confirm_host_trust(confirmation, app.state()).await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                this.trust_confirming = false;
                match confirmed {
                    Ok(()) => this.retry_active(cx),
                    Err(error) => this.trust_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn write_session(&mut self, id: &str, text: String, cx: &mut Context<Self>) {
        if let Some(terminal) = self
            .session(id)
            .and_then(|session| session.terminal.clone())
        {
            terminal.update(cx, |terminal, _| terminal.send_text(text));
        }
    }

    fn disconnect_backend(&mut self, id: String, kind: SessionKind, cx: &mut Context<Self>) {
        let app = self.app.clone();
        cx.spawn(async move |_, _| {
            // Teardown failures are ignored: the tab is gone either way.
            let _ = backend(async move {
                match kind {
                    SessionKind::Local => app.state::<LocalSessionManager>().disconnect(&id).await,
                    SessionKind::Ssh => {
                        app.state::<SshSessionManager>()
                            .disconnect(app.clone(), &id)
                            .await
                    }
                }
            })
            .await;
        })
        .detach();
    }

    /// Closes tabs. Closing the active one focuses its left neighbour, or the
    /// new first tab; closing the last tab leaves the terminal page.
    pub fn close_sessions(&mut self, ids: &[String], cx: &mut Context<Self>) {
        for id in ids {
            let Some(index) = self.sessions.iter().position(|session| session.id == *id) else {
                continue;
            };
            if self.active_session.as_deref() == Some(id.as_str()) {
                let neighbour = if index > 0 { index - 1 } else { index + 1 };
                self.active_session = self
                    .sessions
                    .get(neighbour)
                    .map(|session| session.id.clone());
            }
            let session = self.sessions.remove(index);
            // Only a connected session has a backend counterpart to tear down.
            if session.terminal.is_some() {
                if session.status == SessionStatus::Connected {
                    self.removed.insert(session.id.clone());
                }
                self.disconnect_backend(session.id, session.kind, cx);
            }
        }
        if self.sessions.is_empty() && self.route == Route::Terminal {
            self.route = Route::Connections;
        }
        cx.notify();
    }

    /// Closes every tab of a deleted connection.
    pub fn close_connection_sessions(&mut self, host_id: &str, cx: &mut Context<Self>) {
        let ids: Vec<String> = self
            .sessions
            .iter()
            .filter(|session| session.connection_id.as_deref() == Some(host_id))
            .map(|session| session.id.clone())
            .collect();
        self.close_sessions(&ids, cx);
    }

    pub fn duplicate_session(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(session) = self.session(id) else {
            return;
        };
        match (session.kind, session.connection_id.clone()) {
            (SessionKind::Local, _) => self.connect_local(Some(session.name.clone()), None, cx),
            (SessionKind::Ssh, Some(host_id)) => self.connect_host(&host_id, None, cx),
            (SessionKind::Ssh, None) => {}
        }
    }

    /// Moves the tab `from` to where the tab `to` is.
    pub fn reorder_sessions(&mut self, from: &str, to: &str, cx: &mut Context<Self>) {
        let position = |id: &str| self.sessions.iter().position(|session| session.id == id);
        let (Some(from), Some(to)) = (position(from), position(to)) else {
            return;
        };
        if from != to {
            let session = self.sessions.remove(from);
            self.sessions.insert(to, session);
            cx.notify();
        }
    }

    // --- Snippets -----------------------------------------------------------

    /// Runs a command in an open session of `host_id` (or the local shell when
    /// `None`), opening one if there is none.
    pub fn run_snippet(&mut self, host_id: Option<&str>, command: String, cx: &mut Context<Self>) {
        let existing = self
            .sessions
            .iter()
            .find(|session| {
                session.status != SessionStatus::Disconnected
                    && match host_id {
                        Some(host_id) => session.connection_id.as_deref() == Some(host_id),
                        None => session.kind == SessionKind::Local,
                    }
            })
            .filter(|session| session.terminal.is_some())
            .map(|session| session.id.clone());
        match (existing, host_id) {
            (Some(id), _) => {
                self.active_session = Some(id.clone());
                self.route = Route::Terminal;
                self.write_session(&id, format!("{command}\n"), cx);
                cx.notify();
            }
            (None, Some(host_id)) => self.connect_host(host_id, Some(command), cx),
            (None, None) => self.connect_local(None, Some(command), cx),
        }
    }

    /// Pinned snippets behave like app launchers: every click opens a fresh tab
    /// named after the snippet and runs its command there.
    pub fn launch_pinned_snippet(&mut self, snippet: &SnippetRecord, cx: &mut Context<Self>) {
        match &snippet.host_id {
            None => self.connect_local(Some(snippet.title.clone()), Some(snippet.body.clone()), cx),
            Some(host_id) => self.connect_host(host_id, Some(snippet.body.clone()), cx),
        }
    }

    /// Applies the new sidebar order right away and persists it in the background.
    pub fn reorder_pinned_snippets(&mut self, ids: Vec<String>, cx: &mut Context<Self>) {
        for snippet in &mut self.snippets {
            if let Some(index) = ids.iter().position(|id| *id == snippet.id) {
                snippet.pin_order = Some(index as i32);
            }
        }
        cx.notify();
        let app = self.app.clone();
        cx.spawn(async move |this, cx| {
            let _ = backend(async move {
                crate::store::snippets::snippet_reorder_pins(ids, app.state()).await
            })
            .await;
            let _ = this.update(cx, |this, cx| this.refresh(cx));
        })
        .detach();
    }

    pub fn unpin_snippet(&mut self, snippet: &SnippetRecord, cx: &mut Context<Self>) {
        let app = self.app.clone();
        let (id, request) = (
            snippet.id.clone(),
            SnippetWriteRequest {
                host_id: snippet.host_id.clone(),
                title: snippet.title.clone(),
                body: snippet.body.clone(),
                pin_icon: None,
                pin_color: None,
            },
        );
        cx.spawn(async move |this, cx| {
            let _ = backend(async move {
                crate::store::snippets::snippet_update(id, request, app.state()).await
            })
            .await;
            let _ = this.update(cx, |this, cx| this.refresh(cx));
        })
        .detach();
    }
}

/// Splits saved auth into `(password, private key, passphrase)`.
fn auth_parts(
    auth: Option<&SshHostAuthMaterial>,
) -> (Option<String>, Option<String>, Option<String>) {
    match auth.cloned() {
        Some(SshHostAuthMaterial::Password { password }) => (Some(password), None, None),
        Some(SshHostAuthMaterial::PublicKey {
            private_key,
            passphrase,
        }) => (None, Some(private_key), passphrase),
        Some(SshHostAuthMaterial::PublicKeyAndPassword {
            private_key,
            passphrase,
            password,
        }) => (Some(password), Some(private_key), passphrase),
        None => (None, None, None),
    }
}
