//! The terminal's AI assistant (Cmd+K): turns a request into a command, or
//! explains the selected output.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Escape, Input, InputEvent, InputState};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{h_flex, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::ai::{AiApiFormat, AiAskInput, AiProvider, AiTask};
use crate::ui::bridge::backend;
use crate::ui::i18n::{self, t, Locale};
use crate::ui::icons::lucide;
use crate::ui::store::{store, Route, Store, StoreEvent};
use crate::ui::terminal::TerminalView;
use crate::ui::theme;

/// Settings key the assistant's provider and model are saved under.
pub const AI_SETTING: &str = "noverterm-ai";
/// How much recent terminal output the model sees.
const CONTEXT_LINES: usize = 40;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AiConfig {
    pub provider: AiProvider,
    pub model: String,
    /// Empty means the provider's default endpoint.
    pub base_url: String,
    /// OpenCode Go only; `None` picks the format from the model name.
    pub api_format: Option<AiApiFormat>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::Anthropic,
            model: default_model(AiProvider::Anthropic).to_string(),
            base_url: String::new(),
            api_format: None,
        }
    }
}

pub fn default_model(provider: AiProvider) -> &'static str {
    match provider {
        AiProvider::Anthropic => "claude-haiku-5-5",
        AiProvider::OpenaiCompatible => "",
        AiProvider::OpencodeGo => "minimax-m3",
    }
}

/// The saved config; a missing or corrupt value falls back to the defaults.
pub fn ai_config(store: &Store) -> AiConfig {
    store
        .setting(AI_SETTING)
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_default()
}

/// What the panel was opened for.
struct Target {
    session_id: String,
    terminal: Entity<TerminalView>,
    /// Selected terminal text; when present the panel explains it.
    selection: Option<String>,
}

pub struct Assistant {
    store: Entity<Store>,
    input: Entity<InputState>,
    target: Option<Target>,
    reply: Option<String>,
    error: Option<SharedString>,
    /// The request in flight; dropping it abandons the answer.
    asking: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl Assistant {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let input = cx.new(|cx| InputState::new(window, cx));
        let subscriptions = vec![
            cx.observe(&store, |_, _, cx| cx.notify()),
            cx.subscribe_in(&store, window, |this, _, event: &StoreEvent, window, cx| {
                if let StoreEvent::Assistant(session_id) = event {
                    this.open(session_id.clone(), window, cx);
                }
            }),
            cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    if this.insertable() && this.input.read(cx).value().trim().is_empty() {
                        this.insert(window, cx);
                    } else {
                        this.ask(cx);
                    }
                }
            }),
        ];
        Self {
            store,
            input,
            target: None,
            reply: None,
            error: None,
            asking: None,
            _subscriptions: subscriptions,
        }
    }

    fn open(&mut self, session_id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(terminal) = self
            .store
            .read(cx)
            .session(&session_id)
            .and_then(|session| session.terminal.clone())
        else {
            return;
        };
        let selection = terminal.read(cx).selection_text();
        let explaining = selection.is_some();
        self.target = Some(Target {
            session_id,
            terminal,
            selection,
        });
        self.reply = None;
        self.error = None;
        self.asking = None;
        let placeholder = if explaining {
            t("Ask a follow-up about the selection")
        } else {
            t("Describe what you want to do")
        };
        self.input.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.set_placeholder(placeholder, window, cx);
            input.focus(window, cx);
        });
        if explaining {
            self.ask(cx);
        }
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = None;
        if let Some(target) = self.target.take() {
            target.terminal.focus_handle(cx).focus(window, cx);
        }
        cx.notify();
    }

    /// A newline would run the command, so only a single line goes straight
    /// into the prompt.
    fn insertable(&self) -> bool {
        let generating = self
            .target
            .as_ref()
            .is_some_and(|target| target.selection.is_none());
        generating
            && self
                .reply
                .as_ref()
                .is_some_and(|reply| !reply.contains('\n'))
    }

    fn insert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let (Some(target), Some(reply)) = (&self.target, &self.reply) {
            target
                .terminal
                .update(cx, |terminal, cx| terminal.paste(reply, cx));
        }
        self.close(window, cx);
    }

    fn ask(&mut self, cx: &mut Context<Self>) {
        let Some(target) = &self.target else {
            return;
        };
        let question = self.input.read(cx).value().to_string();
        if target.selection.is_none() && question.trim().is_empty() {
            return;
        }
        let config = ai_config(self.store.read(cx));
        let app = self.store.read(cx).app();
        let (task, input, question) = match &target.selection {
            Some(selection) => (AiTask::Explain, selection.clone(), Some(question)),
            None => (AiTask::Command, question, None),
        };
        let request = AiAskInput {
            provider: config.provider,
            model: config.model,
            base_url: Some(config.base_url).filter(|url| !url.is_empty()),
            api_format: config.api_format,
            task,
            input,
            context: Some(target.terminal.read(cx).recent_text(CONTEXT_LINES)),
            question,
            language: match i18n::locale() {
                Locale::ZhTw => "Traditional Chinese (Taiwan)",
                Locale::En => "English",
            }
            .to_string(),
        };
        self.error = None;
        self.reply = None;
        self.asking = Some(cx.spawn(async move |this, cx| {
            let answer = backend(crate::ai::ai_ask(app, request)).await;
            let _ = this.update(cx, |this, cx| {
                this.asking = None;
                match answer {
                    Ok(reply) => this.reply = Some(reply),
                    Err(error) => this.error = Some(error.into()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
}

impl Render for Assistant {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        // The panel belongs to one terminal and hides with it.
        let Some(target) = self.target.as_ref().filter(|target| {
            store.route == Route::Terminal
                && store.active_session.as_deref() == Some(target.session_id.as_str())
        }) else {
            return div().into_any_element();
        };
        let explaining = target.selection.is_some();
        let asking = self.asking.is_some();
        let answer = match (&self.error, &self.reply) {
            (Some(error), _) => Some(
                div()
                    .text_xs()
                    .text_color(theme::RED_TEXT)
                    .child(error.clone())
                    .into_any_element(),
            ),
            (None, Some(reply)) if explaining => Some(
                div()
                    .id("assistant-reply")
                    .max_h(px(256.))
                    .overflow_y_scroll()
                    .text_sm()
                    .child(reply.clone())
                    .into_any_element(),
            ),
            (None, Some(reply)) => {
                let copy = reply.clone();
                Some(
                    h_flex()
                        .items_start()
                        .gap_2()
                        .child(
                            div()
                                .id("assistant-reply")
                                .flex_1()
                                .min_w_0()
                                .max_h(px(160.))
                                .overflow_y_scroll()
                                .text_sm()
                                .child(reply.clone()),
                        )
                        .when(self.insertable(), |row| {
                            row.child(
                                Button::new("assistant-insert")
                                    .primary()
                                    .small()
                                    .icon(lucide("corner-down-left"))
                                    .label(t("Insert"))
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.insert(window, cx)),
                                    ),
                            )
                        })
                        .child(
                            Button::new("assistant-copy")
                                .ghost()
                                .small()
                                .icon(lucide("copy"))
                                .label(t("Copy"))
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()));
                                }),
                        )
                        .into_any_element(),
                )
            }
            (None, None) => None,
        };

        // Sits over the terminal area: clear of the shell's 56px sidebar and
        // 24px footer, 12px in from each edge.
        h_flex()
            .absolute()
            .left(px(68.))
            .right(px(12.))
            .bottom(px(36.))
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(672.))
                    .occlude()
                    .rounded(theme::RADIUS)
                    .border_1()
                    .border_color(theme::STENCIL.opacity(0.15))
                    .bg(theme::RAIL)
                    .text_color(theme::STENCIL)
                    .shadow_lg()
                    .on_action(cx.listener(|this, _: &Escape, window, cx| this.close(window, cx)))
                    .child(
                        h_flex()
                            .gap_2()
                            .px_3()
                            .py_2()
                            .child(
                                lucide("sparkles")
                                    .size_4()
                                    .flex_shrink_0()
                                    .text_color(theme::STENCIL_DIM),
                            )
                            .child(
                                div().flex_1().min_w_0().child(
                                    Input::new(&self.input)
                                        .small()
                                        .appearance(false)
                                        .disabled(asking),
                                ),
                            )
                            .when(asking, |row| row.child(Spinner::new().small()))
                            .child(
                                Button::new("assistant-close")
                                    .ghost()
                                    .xsmall()
                                    .icon(lucide("x"))
                                    .tooltip(t("Close AI assistant"))
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.close(window, cx)),
                                    ),
                            ),
                    )
                    .children(answer.map(|answer| {
                        div()
                            .px_3()
                            .py_2()
                            .border_t_1()
                            .border_color(theme::hairline())
                            .child(answer)
                    })),
            )
            .into_any_element()
    }
}
