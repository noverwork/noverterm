//! MCP: the settings page, the footer status line, the approval dialog, and
//! the state all three share.

use std::collections::HashSet;
use std::time::Duration;

use chrono::{DateTime, Local};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Sizable as _, WindowExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::json;

use crate::app::AppHandle;
use crate::mcp::{
    McpActivity, McpApprovalRequest, McpDecision, McpSessions, McpSettings, McpStatus,
};
use crate::runtime::port_forward::PortForwardState;
use crate::ui::bridge::backend;
use crate::ui::i18n::{t, tf};
use crate::ui::icons::lucide;
use crate::ui::store::{store, Route, Store, StoreEvent};
use crate::ui::theme;
use crate::ui::views::common;

const ACTIVITY_LIMIT: usize = 20;
/// A finished background command sends no event, so poll while any runs.
const COMMAND_POLL: Duration = Duration::from_secs(5);
/// How long a finished call stays in the status line.
const FLASH: Duration = Duration::from_secs(4);
const MASK: &str = "••••••••";

// --- Shared state --------------------------------------------------------------

pub struct LoggedActivity {
    pub entry: McpActivity,
    pub at: DateTime<Local>,
}

/// What MCP clients are doing, fed by backend events.
pub struct McpEvents {
    app: AppHandle,
    /// Approval requests waiting for the user, oldest first.
    pub approvals: Vec<McpApprovalRequest>,
    /// Requests the backend stopped waiting for; they can only be closed.
    expired: HashSet<String>,
    /// Recent MCP commands, newest first.
    pub activity: Vec<LoggedActivity>,
    /// What MCP clients are doing or hold open right now.
    pub live: McpSessions,
    /// Whether AI may read local terminals without asking.
    pub local_terminal: bool,
    /// The newest activity is still fresh enough for the status line.
    flash: bool,
    _flash: Option<Task<()>>,
    _poll: Option<Task<()>>,
    _subscription: Subscription,
}

struct Shared(Entity<McpEvents>);

impl Global for Shared {}

pub fn events(cx: &App) -> Option<Entity<McpEvents>> {
    cx.try_global::<Shared>().map(|shared| shared.0.clone())
}

impl McpEvents {
    /// Creates the state and makes it reachable through [`events`].
    pub fn install(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let events = cx.new(|cx| Self::new(window, cx));
        cx.set_global(Shared(events.clone()));
        events
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let subscription = cx.subscribe_in(&store, window, Self::on_store_event);
        let mut events = Self {
            app: store.read(cx).app(),
            approvals: Vec::new(),
            expired: HashSet::new(),
            activity: Vec::new(),
            live: McpSessions {
                running: Vec::new(),
                commands: Vec::new(),
                forwards: Vec::new(),
            },
            local_terminal: false,
            flash: false,
            _flash: None,
            _poll: None,
            _subscription: subscription,
        };
        events.refresh_live(cx);
        events.refresh_local_access(cx);
        events
    }

    fn on_store_event(
        &mut self,
        _: &Entity<Store>,
        event: &StoreEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let StoreEvent::Backend(name, payload) = event else {
            return;
        };
        match *name {
            "mcp_approval_request" => {
                let Ok(request) = serde_json::from_str::<McpApprovalRequest>(payload) else {
                    return;
                };
                self.approvals.push(request);
                // The agent is blocked until the user answers.
                cx.activate(true);
                window.activate_window();
                if self.approvals.len() == 1 {
                    open_approval_dialog(cx.entity(), window, cx);
                }
                cx.notify();
            }
            "mcp_approval_expired" => {
                if let Ok(id) = serde_json::from_str::<String>(payload) {
                    self.expired.insert(id);
                    cx.notify();
                }
            }
            "mcp_activity" => {
                let Ok(entry) = serde_json::from_str::<McpActivity>(payload) else {
                    return;
                };
                self.activity.insert(
                    0,
                    LoggedActivity {
                        entry,
                        at: Local::now(),
                    },
                );
                self.activity.truncate(ACTIVITY_LIMIT);
                self.flash = true;
                self._flash = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(FLASH).await;
                    let _ = this.update(cx, |this, cx| {
                        this.flash = false;
                        cx.notify();
                    });
                }));
                cx.notify();
            }
            // MCP forwards can fail or be stopped from the Forwards page.
            "mcp_changed" | "port_forward_status" => self.refresh_live(cx),
            "ai_access_changed" => self.refresh_local_access(cx),
            _ => {}
        }
    }

    fn refresh_local_access(&mut self, cx: &mut Context<Self>) {
        let app = self.app.clone();
        cx.spawn(async move |this, cx| {
            if let Ok(status) = backend(crate::mcp::mcp_status(app)).await {
                let _ = this.update(cx, |this, cx| {
                    this.local_terminal = status.settings.allow_local_terminal;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn refresh_live(&mut self, cx: &mut Context<Self>) {
        let app = self.app.clone();
        cx.spawn(async move |this, cx| {
            let Ok(live) = backend(crate::mcp::mcp_sessions(app)).await else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                let running = live.commands.iter().any(|command| !command.finished);
                this.live = live;
                this._poll = running.then(|| {
                    cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(COMMAND_POLL).await;
                        let _ = this.update(cx, |this, cx| this.refresh_live(cx));
                    })
                });
                cx.notify();
            });
        })
        .detach();
    }

    /// Answers the oldest request; `true` when none is left waiting.
    fn respond(&mut self, decision: McpDecision, cx: &mut Context<Self>) -> bool {
        if !self.approvals.is_empty() {
            let request = self.approvals.remove(0);
            if self.expired.remove(&request.id) {
                cx.notify();
                return self.approvals.is_empty();
            }
            let app = self.app.clone();
            cx.spawn(async move |_, _| {
                let _ = backend(async move {
                    crate::mcp::mcp_respond(app.state(), request.id, decision).await
                })
                .await;
            })
            .detach();
            cx.notify();
        }
        self.approvals.is_empty()
    }
}

fn activity_label(entry: &McpActivity) -> SharedString {
    match (entry.status.as_str(), entry.exit_status) {
        ("finished", None) => t("Done"),
        ("finished", Some(code)) => tf("exit {code}", &[("code", &code.to_string())]),
        ("started", _) => t("Started"),
        ("denied", _) => t("Denied"),
        ("stopped", _) => t("Stopped"),
        _ => t("Failed"),
    }
}

fn activity_tone(entry: &McpActivity) -> Rgba {
    match (entry.status.as_str(), entry.exit_status.unwrap_or(0)) {
        ("started", _) | ("finished", 0) => theme::GREEN_TEXT,
        ("denied" | "stopped", _) => theme::STENCIL_MUTE,
        _ => theme::RED_TEXT,
    }
}

fn clock(time: DateTime<Local>) -> String {
    time.format("%H:%M").to_string()
}

fn dot(color: Rgba) -> Div {
    div().size_1p5().flex_shrink_0().rounded_full().bg(color)
}

// --- Approval dialog -----------------------------------------------------------

/// Shows the oldest waiting request; it stays up until the queue is empty.
fn open_approval_dialog(events: Entity<McpEvents>, window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, move |dialog, _, cx| {
        let state = events.read(cx);
        let Some(request) = state.approvals.first() else {
            return dialog;
        };
        let waiting = state.approvals.len() - 1;
        let expired = state.expired.contains(&request.id);
        let decide = |decision: McpDecision| {
            let events = events.clone();
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                if events.update(cx, |events, cx| events.respond(decision, cx)) {
                    window.close_dialog(cx);
                }
            }
        };
        let deny = events.clone();
        dialog
            .w(px(512.))
            .border_color(theme::AMBER.opacity(0.6))
            .close_button(false)
            .overlay_closable(false)
            // Escape denies; the dialog only closes once nothing is waiting.
            .on_cancel(move |_, _, cx| {
                deny.update(cx, |events, cx| events.respond(McpDecision::Deny, cx))
            })
            .title(
                h_flex()
                    .gap_3()
                    .child(
                        lucide("shield-alert")
                            .size_5()
                            .flex_shrink_0()
                            .text_color(theme::AMBER_TEXT),
                    )
                    .child(tf(
                        "AI wants to run a command on {host}",
                        &[("host", &request.host_name)],
                    )),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme::STENCIL_MUTE)
                            .child(request.target.clone()),
                    )
                    .child(
                        div()
                            .id("mcp-approval-command")
                            .max_h(px(256.))
                            .overflow_y_scroll()
                            .px_4()
                            .py_3()
                            .rounded(theme::RADIUS)
                            .border_1()
                            .border_color(theme::hairline())
                            .bg(theme::PORT)
                            .text_sm()
                            .child(request.command.clone()),
                    )
                    .when(expired, |body| {
                        body.child(
                            div()
                                .text_sm()
                                .text_color(theme::RED_TEXT)
                                .child(t("Timed out: the AI stopped waiting for an answer.")),
                        )
                    })
                    .when(waiting > 0, |body| {
                        body.child(div().text_xs().text_color(theme::STENCIL_MUTE).child(tf(
                            "{count} more waiting",
                            &[("count", &waiting.to_string())],
                        )))
                    }),
            )
            .footer(
                h_flex()
                    .flex_wrap()
                    .justify_end()
                    .gap_3()
                    .when(expired, |footer| {
                        // Any answer just closes it; Deny says nothing new to the backend.
                        footer.child(
                            Button::new("mcp-close")
                                .outline()
                                .label(t("Close"))
                                .on_click(decide(McpDecision::Deny)),
                        )
                    })
                    .child(
                        Button::new("mcp-deny")
                            .outline()
                            .label(t("Deny"))
                            .disabled(expired)
                            .on_click(decide(McpDecision::Deny)),
                    )
                    .child(
                        Button::new("mcp-allow-host")
                            .outline()
                            .label(t("Always allow on this host"))
                            .disabled(expired)
                            .on_click(decide(McpDecision::AllowHost)),
                    )
                    .child(
                        Button::new("mcp-allow")
                            .warning()
                            .label(t("Allow"))
                            .disabled(expired)
                            .on_click(decide(McpDecision::Allow)),
                    ),
            )
    });
}

// --- Footer status line --------------------------------------------------------

pub struct StatusLine {
    store: Entity<Store>,
    events: Entity<McpEvents>,
    open: bool,
    error: Option<SharedString>,
    _subscription: Subscription,
}

impl StatusLine {
    pub fn new(events: Entity<McpEvents>, cx: &mut Context<Self>) -> Self {
        Self {
            store: store(cx),
            _subscription: cx.observe(&events, |_, _, cx| cx.notify()),
            events,
            open: false,
            error: None,
        }
    }

    fn stop(&mut self, id: String, forward: bool, cx: &mut Context<Self>) {
        self.error = None;
        cx.notify();
        let app = self.store.read(cx).app();
        cx.spawn(async move |this, cx| {
            let stopped = backend(async move {
                if forward {
                    crate::connect::port_forward_stop(app.clone(), id, app.state())
                        .await
                        .map(|_| ())
                } else {
                    crate::mcp::mcp_stop_command(app, id)
                }
            })
            .await;
            if let Err(error) = stopped {
                let _ = this.update(cx, |this, cx| {
                    this.error = Some(error.into());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn go(&mut self, route: Route, cx: &mut Context<Self>) {
        self.open = false;
        self.store.update(cx, |store, cx| store.navigate(route, cx));
        cx.notify();
    }

    fn render_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let events = self.events.read(cx);
        let section = |title: SharedString| {
            v_flex()
                .gap_1()
                .child(common::section_title(title).mb_0p5())
        };
        let row = || h_flex().gap_2();
        let mono = |text: &str| div().min_w_0().truncate().child(text.to_string());
        let mute = |text: &str| {
            div()
                .flex_shrink_0()
                .text_color(theme::STENCIL_MUTE)
                .child(text.to_string())
        };
        let time = |text: String| {
            div()
                .ml_auto()
                .flex_shrink_0()
                .text_color(theme::STENCIL_FAINT)
                .child(text)
        };
        let stop_button = |id: &str, forward: bool| {
            let stop_id = id.to_string();
            Button::new(SharedString::from(format!("stop-{id}")))
                .ghost()
                .xsmall()
                .icon(lucide("square"))
                .tooltip(t("Stop"))
                .on_click(
                    cx.listener(move |this, _, _, cx| this.stop(stop_id.clone(), forward, cx)),
                )
        };

        let mut body = v_flex().gap_4().p_3();
        if let Some(error) = &self.error {
            body = body.child(div().text_color(theme::RED_TEXT).child(error.clone()));
        }
        if !events.live.running.is_empty() {
            body = body.child(
                section(t("Working now")).children(events.live.running.iter().map(|call| {
                    row()
                        .child(dot(theme::AMBER))
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_color(theme::STENCIL_DIM)
                                .child(call.tool.clone()),
                        )
                        .children(call.host.as_deref().map(mute))
                        .children(call.detail.as_deref().map(mono))
                })),
            );
        }
        if !events.live.commands.is_empty() {
            body = body.child(section(t("Background commands")).children(
                events.live.commands.iter().map(|command| {
                    let started = DateTime::parse_from_rfc3339(&command.started_at)
                        .map(|time| clock(time.with_timezone(&Local)))
                        .unwrap_or_default();
                    row()
                        .child(dot(if command.finished {
                            theme::STENCIL_FAINT
                        } else {
                            theme::GREEN_TEXT
                        }))
                        .child(mute(&command.host_name))
                        .child(mono(&command.command))
                        .child(time(started))
                        .child(stop_button(&command.id, false))
                }),
            ));
        }
        if !events.live.forwards.is_empty() {
            body = body.child(section(t("Port forwards")).children(
                events.live.forwards.iter().map(|forward| {
                    let listening = matches!(forward.state, PortForwardState::Listening);
                    row()
                        .child(dot(if listening {
                            theme::GREEN_TEXT
                        } else {
                            theme::AMBER
                        }))
                        .child(mute(forward.name.trim_start_matches("MCP: ")))
                        .child(
                            div()
                                .flex_shrink_0()
                                .child(format!("{}:{}", forward.bind_host, forward.bind_port)),
                        )
                        .child(
                            lucide("arrow-right")
                                .size_3()
                                .flex_shrink_0()
                                .text_color(theme::STENCIL_FAINT),
                        )
                        .child(
                            mono(&format!("{}:{}", forward.target_host, forward.target_port))
                                .text_color(theme::STENCIL_DIM),
                        )
                        .child(div().ml_auto().child(stop_button(&forward.id, true)))
                }),
            ));
        }
        let recent = section(t("Recent MCP commands"));
        body = body.child(if events.activity.is_empty() {
            recent.child(
                div()
                    .text_color(theme::STENCIL_MUTE)
                    .child(t("No agent has used noverterm yet.")),
            )
        } else {
            recent.children(events.activity.iter().take(8).map(|logged| {
                row()
                    .child(
                        div()
                            .w_16()
                            .flex_shrink_0()
                            .text_color(activity_tone(&logged.entry))
                            .child(activity_label(&logged.entry)),
                    )
                    .child(mute(&logged.entry.host_name))
                    .child(mono(&logged.entry.command))
                    .child(time(clock(logged.at)))
            }))
        });

        v_flex()
            .w(px(448.))
            .text_xs()
            .text_color(theme::STENCIL)
            .child(body)
            .child(
                h_flex()
                    .justify_between()
                    .px_3()
                    .py_1p5()
                    .border_t_1()
                    .border_color(theme::hairline())
                    .bg(theme::PORT)
                    .child(
                        Button::new("mcp-host-access")
                            .ghost()
                            .xsmall()
                            .label(t("Host MCP access"))
                            .tooltip(t("Set a host's MCP access in its edit page"))
                            .on_click(
                                cx.listener(|this, _, _, cx| this.go(Route::Connections, cx)),
                            ),
                    )
                    .child(
                        Button::new("mcp-settings")
                            .ghost()
                            .xsmall()
                            .label(t("MCP settings"))
                            .on_click(cx.listener(|this, _, _, cx| this.go(Route::Mcp, cx))),
                    ),
            )
            .into_any_element()
    }
}

impl Render for StatusLine {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let events = self.events.read(cx);
        let current = events.live.running.last();
        let latest = events.activity.first().filter(|_| events.flash);
        let held = events
            .live
            .commands
            .iter()
            .filter(|command| !command.finished)
            .count()
            + events.live.forwards.len();
        let detail = |text: &str| {
            div()
                .min_w_0()
                .truncate()
                .text_color(theme::STENCIL_DIM)
                .child(text.to_string())
        };
        let label = h_flex().h_full().min_w_0().gap_2().px_2();
        let label = if !events.approvals.is_empty() {
            label
                .bg(theme::AMBER.opacity(0.2))
                .text_color(theme::STENCIL)
                .child(dot(theme::AMBER))
                .child("MCP")
                .child(
                    div()
                        .truncate()
                        .text_color(theme::AMBER_TEXT)
                        .child(SharedString::from(t("Waiting for approval").to_uppercase())),
                )
        } else if let Some(call) = current {
            let host = call
                .host
                .as_deref()
                .map(|host| format!("·{host}"))
                .unwrap_or_default();
            label
                .bg(theme::AMBER.opacity(0.2))
                .text_color(theme::STENCIL)
                .child(dot(theme::AMBER))
                .child(
                    div()
                        .flex_shrink_0()
                        .child(format!("MCP{host}").to_uppercase()),
                )
                .child(detail(call.detail.as_deref().unwrap_or(&call.tool)))
        } else if let Some(logged) = latest {
            let tone = activity_tone(&logged.entry);
            label
                .child(dot(tone))
                .child(
                    div()
                        .flex_shrink_0()
                        .child(format!("MCP·{}", logged.entry.host_name).to_uppercase()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_color(tone)
                        .child(SharedString::from(
                            activity_label(&logged.entry).to_uppercase(),
                        )),
                )
                .child(detail(&logged.entry.command))
        } else if held > 0 {
            label
                .bg(theme::GREEN_TEXT.opacity(0.15))
                .text_color(theme::STENCIL)
                .child(dot(theme::GREEN_TEXT))
                .child("MCP")
                .child(SharedString::from(
                    tf("{count} active", &[("count", &held.to_string())]).to_uppercase(),
                ))
        } else {
            label
                .child(
                    div()
                        .size_2()
                        .flex_shrink_0()
                        .rounded_full()
                        .border_2()
                        .border_color(theme::STENCIL_FAINT),
                )
                .child("MCP")
        };
        let view = cx.entity();
        Popover::new("mcp-status")
            .anchor(Anchor::BottomRight)
            .p_0()
            .open(self.open)
            .on_open_change(cx.listener(|this, open: &bool, _, cx| {
                this.open = *open;
                cx.notify();
            }))
            .trigger(
                Button::new("mcp-status-trigger")
                    .ghost()
                    .xsmall()
                    .tooltip(t("MCP activity"))
                    .child(label),
            )
            .content(move |_, _, cx| view.update(cx, |this, cx| this.render_panel(cx)))
    }
}

// --- Page ----------------------------------------------------------------------

/// `(label, config file)`; no file means the snippet is a shell command.
const CLIENTS: [(&str, Option<&str>); 4] = [
    ("Claude Code", None),
    ("Cursor", Some("~/.cursor/mcp.json")),
    ("VS Code", Some(".vscode/mcp.json")),
    ("Claude Desktop", Some("claude_desktop_config.json")),
];

const TOOLS: [(&str, &str); 23] = [
    ("list_hosts", "Lists saved hosts."),
    ("run_command", "Runs a command on a host."),
    (
        "run_command_multi",
        "Runs a command on several hosts at once.",
    ),
    ("upload_file", "Uploads a file to a host."),
    ("download_file", "Downloads a file from a host."),
    (
        "transfer_status",
        "Waits for a long file transfer to finish.",
    ),
    ("list_dir", "Lists a directory on a host."),
    ("read_file", "Reads a text file on a host."),
    ("write_file", "Writes a text file on a host."),
    ("list_snippets", "Lists saved snippets."),
    ("run_snippet", "Runs a saved snippet on a host."),
    ("list_port_forwards", "Lists running port forwards."),
    ("start_port_forward", "Opens a port forward through a host."),
    ("stop_port_forward", "Stops a port forward the AI opened."),
    ("list_links", "Lists saved Links and their public URLs."),
    (
        "start_link",
        "Publishes a local web service at a public URL.",
    ),
    ("stop_link", "Takes a Link offline."),
    (
        "start_command",
        "Starts a long-running or interactive command.",
    ),
    ("read_command", "Reads a started command's new output."),
    ("write_command", "Sends input to a started command."),
    ("stop_command", "Stops a started command."),
    ("list_terminals", "Lists open terminal tabs."),
    ("read_terminal", "Reads a terminal tab's recent output."),
];

/// What to paste into the client at `CLIENTS[client]` to connect it.
fn client_snippet(client: usize, url: &str, token: &str) -> String {
    let auth = format!("Bearer {token}");
    let config = match client {
        0 => {
            return format!(
                "claude mcp add --transport http noverterm {url} --header \"Authorization: {auth}\""
            )
        }
        1 => {
            json!({ "mcpServers": { "noverterm": { "url": url, "headers": { "Authorization": auth } } } })
        }
        2 => json!({
            "servers": { "noverterm": { "type": "http", "url": url, "headers": { "Authorization": auth } } }
        }),
        _ => json!({
            "mcpServers": {
                "noverterm": {
                    "command": "npx",
                    "args": ["-y", "mcp-remote", url, "--header", "Authorization:${AUTH_HEADER}"],
                    "env": { "AUTH_HEADER": auth },
                }
            }
        }),
    };
    serde_json::to_string_pretty(&config).unwrap_or_default()
}

pub struct McpView {
    store: Entity<Store>,
    events: Option<Entity<McpEvents>>,
    status: Option<McpStatus>,
    port: Entity<InputState>,
    saving: bool,
    error: Option<SharedString>,
    /// Index into `CLIENTS`.
    client: usize,
    reveal: bool,
    /// Which copy button was just used.
    copied: Option<&'static str>,
    confirming_regenerate: bool,
    _copied: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl McpView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let events = events(cx);
        let port = cx.new(|cx| InputState::new(window, cx));
        let mut subscriptions = vec![cx.subscribe(&port, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        })];
        subscriptions.extend(
            events
                .as_ref()
                .map(|events| cx.observe(events, |_, _, cx| cx.notify())),
        );
        let app = store.read(cx).app();
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(status) = backend(crate::mcp::mcp_status(app)).await {
                let _ = this.update_in(cx, |this, window, cx| this.set_status(status, window, cx));
            }
        })
        .detach();
        Self {
            store,
            events,
            status: None,
            port,
            saving: false,
            error: None,
            client: 0,
            reveal: false,
            copied: None,
            confirming_regenerate: false,
            _copied: None,
            _subscriptions: subscriptions,
        }
    }

    fn set_status(&mut self, status: McpStatus, window: &mut Window, cx: &mut Context<Self>) {
        let port = status.settings.port.to_string();
        self.port
            .update(cx, |input, cx| input.set_value(port, window, cx));
        self.error = status.error.clone().map(SharedString::from);
        self.status = Some(status);
        cx.notify();
    }

    /// Runs a backend call that returns the new status.
    fn apply<F>(&mut self, call: F, window: &mut Window, cx: &mut Context<Self>)
    where
        F: std::future::Future<Output = Result<McpStatus, String>> + Send + 'static,
    {
        self.saving = true;
        self.error = None;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = backend(call).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.saving = false;
                match result {
                    Ok(status) => {
                        if let Some(events) = &this.events {
                            events.update(cx, |events, cx| {
                                events.local_terminal = status.settings.allow_local_terminal;
                                cx.notify();
                            });
                        }
                        this.set_status(status, window, cx);
                    }
                    Err(error) => this.error = Some(error.into()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn configure(
        &mut self,
        change: impl FnOnce(&mut McpSettings),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(status) = &self.status else {
            return;
        };
        let mut settings = status.settings;
        change(&mut settings);
        let app = self.store.read(cx).app();
        self.apply(crate::mcp::mcp_configure(app, settings), window, cx);
    }

    fn apply_port(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.port.read(cx).value().trim().parse::<u16>() {
            Ok(port) if port > 0 => self.configure(|settings| settings.port = port, window, cx),
            _ => {
                self.error = Some(t("Port must be between 1 and 65535"));
                cx.notify();
            }
        }
    }

    fn copy(&mut self, key: &'static str, text: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.copied = Some(key);
        self._copied = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let _ = this.update(cx, |this, cx| {
                this.copied = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn copy_button(&self, key: &'static str, text: String, cx: &Context<Self>) -> Button {
        let copied = self.copied == Some(key);
        Button::new(key)
            .ghost()
            .xsmall()
            .icon(if copied {
                lucide("check").text_color(theme::GREEN_TEXT)
            } else {
                lucide("copy")
            })
            .label(t("Copy"))
            .on_click(cx.listener(move |this, _, _, cx| this.copy(key, text.clone(), cx)))
    }
}

impl Render for McpView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let saving = self.saving;
        let settings = self.status.as_ref().map(|status| status.settings);
        let allow_local = settings.is_some_and(|settings| settings.allow_local_terminal);
        let port_changed = settings
            .is_some_and(|settings| self.port.read(cx).value().trim() != settings.port.to_string());
        let running = self.status.as_ref().and_then(|status| {
            let token = status
                .token
                .clone()
                .filter(|_| status.running && !status.url.is_empty())?;
            Some((status.url.clone(), token))
        });
        let row = |label: SharedString, control: AnyElement| {
            h_flex()
                .gap_3()
                .text_sm()
                .child(
                    div()
                        .w(px(128.))
                        .flex_shrink_0()
                        .text_color(theme::STENCIL_MUTE)
                        .child(label),
                )
                .child(control)
        };

        let mut rows =
            v_flex()
                .gap_3()
                .child(row(
                    t("Port"),
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .w(px(112.))
                                .child(Input::new(&self.port).small().disabled(saving)),
                        )
                        .when(port_changed, |port| {
                            port.child(
                                Button::new("apply-port")
                                    .outline()
                                    .small()
                                    .label(t("Apply"))
                                    .disabled(saving)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.apply_port(window, cx)
                                    })),
                            )
                        })
                        .into_any_element(),
                ))
                .child(row(
                    t("Local terminal"),
                    h_flex()
                        .gap_3()
                        .child(
                            Switch::new("allow-local-terminal")
                                .checked(allow_local)
                                .color(theme::AMBER)
                                .disabled(saving || settings.is_none())
                                .tooltip(t("Let AI read local terminals without asking"))
                                .on_click(cx.listener(|this, checked: &bool, window, cx| {
                                    let checked = *checked;
                                    this.configure(
                                        |settings| settings.allow_local_terminal = checked,
                                        window,
                                        cx,
                                    );
                                })),
                        )
                        .child(div().text_xs().text_color(theme::STENCIL_MUTE).child(
                            if allow_local {
                                t("AI reads local terminals without asking")
                            } else {
                                t("Ask before AI reads a local terminal")
                            },
                        ))
                        .into_any_element(),
                ));

        if let Some((url, token)) = &running {
            let token_row = h_flex()
                .gap_2()
                .min_w_0()
                .child(div().truncate().text_xs().child(if self.reveal {
                    SharedString::from(token.clone())
                } else {
                    MASK.into()
                }))
                .child(
                    Button::new("reveal-token")
                        .ghost()
                        .xsmall()
                        .icon(lucide(if self.reveal { "eye-off" } else { "eye" }))
                        .tooltip(if self.reveal { t("Hide") } else { t("Show") })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.reveal = !this.reveal;
                            cx.notify();
                        })),
                )
                .child(self.copy_button("copy-token", token.clone(), cx));
            let token_row = if self.confirming_regenerate {
                token_row
                    .child(div().text_xs().text_color(theme::STENCIL_MUTE).child(t(
                        "Clients using the current token will stop working. Regenerate it?",
                    )))
                    .child(
                        Button::new("cancel-regenerate")
                            .ghost()
                            .xsmall()
                            .label(t("Cancel"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.confirming_regenerate = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("confirm-regenerate")
                            .ghost()
                            .xsmall()
                            .text_color(theme::RED_TEXT)
                            .label(t("Regenerate"))
                            .disabled(saving)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.confirming_regenerate = false;
                                let app = this.store.read(cx).app();
                                this.apply(crate::mcp::mcp_regenerate_token(app), window, cx);
                            })),
                    )
            } else {
                token_row.child(
                    Button::new("regenerate")
                        .ghost()
                        .xsmall()
                        .icon(lucide("refresh-cw"))
                        .label(t("Regenerate"))
                        .disabled(saving)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.confirming_regenerate = true;
                            cx.notify();
                        })),
                )
            };
            rows = rows
                .child(row(
                    "URL".into(),
                    h_flex()
                        .gap_2()
                        .child(div().text_xs().child(url.clone()))
                        .child(self.copy_button("copy-url", url.clone(), cx))
                        .into_any_element(),
                ))
                .child(row("Token".into(), token_row.into_any_element()));
        }

        let mut body = v_flex().gap_6().child(rows).children(
            self.error
                .clone()
                .map(|error| div().text_xs().text_color(theme::RED_TEXT).child(error)),
        );

        if let Some((url, token)) = &running {
            let tabs = CLIENTS.iter().enumerate().map(|(index, (label, _))| {
                let selected = self.client == index;
                div()
                    .id(*label)
                    .h_7()
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded(theme::RADIUS)
                    .border_1()
                    .text_xs()
                    .cursor_pointer()
                    .map(|tab| {
                        if selected {
                            tab.border_color(theme::STENCIL).text_color(theme::STENCIL)
                        } else {
                            tab.border_color(theme::hairline())
                                .text_color(theme::STENCIL_MUTE)
                                .hover(|style| style.text_color(theme::STENCIL))
                        }
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.client = index;
                        cx.notify();
                    }))
                    .child(*label)
            });
            let file: SharedString = match CLIENTS[self.client].1 {
                Some(file) => file.into(),
                None => t("Terminal"),
            };
            let shown = client_snippet(self.client, url, if self.reveal { token } else { MASK });
            body = body
                .child(
                    v_flex()
                        .gap_3()
                        .child(common::section_title(t("Connect a client")))
                        .child(h_flex().flex_wrap().gap_1().children(tabs))
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    h_flex()
                                        .justify_between()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme::STENCIL_MUTE)
                                                .child(file),
                                        )
                                        .child(self.copy_button(
                                            "copy-snippet",
                                            client_snippet(self.client, url, token),
                                            cx,
                                        )),
                                )
                                .child(
                                    div()
                                        .px_3()
                                        .py_2()
                                        .rounded(theme::RADIUS)
                                        .border_1()
                                        .border_color(theme::hairline())
                                        .bg(theme::PORT)
                                        .text_size(px(11.))
                                        .text_color(theme::STENCIL_DIM)
                                        .child(shown),
                                ),
                        ),
                )
                .child(
                    v_flex()
                        .gap_3()
                        .child(common::section_title(t("Tools")))
                        .child(v_flex().gap_2().children(TOOLS.map(|(name, description)| {
                            h_flex()
                                .gap_3()
                                .child(div().w(px(160.)).flex_shrink_0().text_xs().child(name))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme::STENCIL_DIM)
                                        .child(t(description)),
                                )
                        }))),
                );
        }

        let activity = self.events.as_ref().map(|events| &events.read(cx).activity);
        if let Some(activity) = activity.filter(|activity| !activity.is_empty()) {
            body = body.child(
                v_flex()
                    .gap_3()
                    .child(common::section_title(t("Recent MCP commands")))
                    .child(v_flex().gap_1().children(activity.iter().map(|logged| {
                        h_flex()
                            .gap_2()
                            .text_xs()
                            .child(
                                div()
                                    .text_color(activity_tone(&logged.entry))
                                    .child(activity_label(&logged.entry)),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(logged.entry.host_name.clone()),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .child(logged.entry.command.clone()),
                            )
                    }))),
            );
        }

        common::page("MCP".into(), div(), common::page_body("mcp").child(body))
    }
}
