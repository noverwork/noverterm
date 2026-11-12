//! The settings dialog: language, terminal, AI assistant, updates, and backup.

use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{
    h_flex, v_flex, Disableable as _, IndexPath, Sizable as _, WindowExt as _,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::ai::{AiApiFormat, AiProvider};
use crate::ui::bridge::backend;
use crate::ui::i18n::{self, t, tf, Locale};
use crate::ui::icons::lucide;
use crate::ui::store::{store, Store};
use crate::ui::terminal::{CursorKind, TerminalConfig};
use crate::ui::theme;
use crate::ui::views::assistant::{ai_config, default_model, AiConfig, AI_SETTING};
use crate::ui::views::common::{self, Choice, ChoiceSelect};
use crate::ui::views::updater;

/// Opens the dialog; closing it clears `Store::show_settings`.
pub fn open(window: &mut Window, cx: &mut App) {
    let view = cx.new(|cx| SettingsView::new(window, cx));
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .w(px(512.))
            .title(t("Settings"))
            .child(view.clone())
            .on_close(|_, _, cx| {
                store(cx).update(cx, |store, cx| {
                    store.show_settings = false;
                    cx.notify();
                });
            })
    });
}

/// The serde name of a provider or API format, used as its select value.
fn code<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn parse<T: serde::de::DeserializeOwned>(code: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(code.to_string())).ok()
}

#[derive(PartialEq)]
enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Error(String),
}

struct SettingsView {
    store: Entity<Store>,
    language: Entity<ChoiceSelect>,
    font_size: Entity<InputState>,
    scrollback: Entity<InputState>,
    cursor_style: Entity<ChoiceSelect>,
    provider: Entity<ChoiceSelect>,
    model: Entity<InputState>,
    /// Empty value picks the format from the model name.
    api_format: Entity<ChoiceSelect>,
    base_url: Entity<InputState>,
    api_key: Entity<InputState>,
    has_key: bool,
    saving: bool,
    ai_notice: Option<SharedString>,
    ai_error: Option<SharedString>,
    update: UpdateStatus,
    _update: Option<Task<()>>,
    backing_up: bool,
    backup_notice: Option<SharedString>,
    backup_error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl SettingsView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let config = ai_config(store.read(cx));
        let languages = Locale::ALL
            .map(|locale| Choice::new(locale.code(), locale.label()))
            .to_vec();
        let providers = vec![
            Choice::new(code(AiProvider::Anthropic), "Anthropic"),
            Choice::new(code(AiProvider::OpencodeGo), "OpenCode Go"),
            Choice::new(
                code(AiProvider::OpenaiCompatible),
                t("OpenAI-compatible (OpenAI, Ollama…)"),
            ),
        ];
        let formats = vec![
            Choice::new("", t("Auto (by model)")),
            Choice::new(code(AiApiFormat::Anthropic), "Anthropic · /messages"),
            Choice::new(code(AiApiFormat::Openai), "OpenAI · /chat/completions"),
            Choice::new(code(AiApiFormat::Responses), "Responses · /responses"),
            Choice::new(code(AiApiFormat::Gemini), "Gemini · generateContent"),
        ];
        let language = common::new_select(languages, i18n::locale().code(), window, cx);
        let terminal = store.read(cx).terminal_config();
        let cursor_styles = vec![
            Choice::new(code(CursorKind::Block), t("Block")),
            Choice::new(code(CursorKind::Underline), t("Underline")),
            Choice::new(code(CursorKind::Bar), t("Bar")),
        ];
        let cursor_style =
            common::new_select(cursor_styles, &code(terminal.cursor_style), window, cx);
        let font_size =
            cx.new(|cx| InputState::new(window, cx).default_value(terminal.font_size.to_string()));
        let scrollback =
            cx.new(|cx| InputState::new(window, cx).default_value(terminal.scrollback.to_string()));
        let provider = common::new_select(providers, &code(config.provider), window, cx);
        let api_format = common::new_select(
            formats,
            &config.api_format.map(code).unwrap_or_default(),
            window,
            cx,
        );
        let subscriptions = vec![
            cx.subscribe(
                &language,
                |this, _, event: &SelectEvent<Vec<Choice>>, cx| {
                    let SelectEvent::Confirm(value) = event;
                    if let Some(locale) = value.as_deref().and_then(Locale::from_code) {
                        i18n::set_locale(locale);
                        this.store.update(cx, |store, cx| {
                            store.set_setting(i18n::LOCALE_SETTING, locale.code().to_string(), cx)
                        });
                        cx.refresh_windows();
                    }
                },
            ),
            cx.subscribe(
                &cursor_style,
                |this, select, _: &SelectEvent<Vec<Choice>>, cx| {
                    if let Some(style) = common::selected(&select, cx).and_then(|code| parse(&code))
                    {
                        this.set_terminal(|config| config.cursor_style = style, cx);
                    }
                },
            ),
            cx.subscribe(&font_size, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.save_numbers(cx);
                }
            }),
            cx.subscribe(&scrollback, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.save_numbers(cx);
                }
            }),
            cx.subscribe_in(
                &provider,
                window,
                |this, _, _: &SelectEvent<Vec<Choice>>, window, cx| {
                    this.provider_changed(window, cx)
                },
            ),
        ];
        let view = Self {
            model: cx.new(|cx| InputState::new(window, cx).default_value(config.model)),
            base_url: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(tf(
                        "e.g. {example}",
                        &[("example", "http://localhost:11434/v1")],
                    ))
                    .default_value(config.base_url)
            }),
            api_key: cx.new(|cx| InputState::new(window, cx).masked(true)),
            store,
            language,
            font_size,
            scrollback,
            cursor_style,
            provider,
            api_format,
            has_key: false,
            saving: false,
            ai_notice: None,
            ai_error: None,
            update: UpdateStatus::Idle,
            _update: None,
            backing_up: false,
            backup_notice: None,
            backup_error: None,
            _subscriptions: subscriptions,
        };
        view.refresh_key_state(cx);
        view
    }

    fn set_terminal(&self, change: impl FnOnce(&mut TerminalConfig), cx: &mut Context<Self>) {
        self.store.update(cx, |store, cx| {
            let mut config = store.terminal_config();
            change(&mut config);
            store.set_terminal_config(config, cx);
        });
        cx.notify();
    }

    /// Saves the typed font size and scrollback, clamped to sane values; an
    /// unparsable entry keeps the saved value.
    fn save_numbers(&mut self, cx: &mut Context<Self>) {
        let font_size = self.font_size.read(cx).value().trim().parse::<f32>().ok();
        let scrollback = self
            .scrollback
            .read(cx)
            .value()
            .trim()
            .parse::<usize>()
            .ok();
        self.set_terminal(
            |config| {
                if let Some(size) = font_size.filter(|size| size.is_finite()) {
                    config.font_size = size.clamp(8., 32.);
                }
                if let Some(lines) = scrollback {
                    config.scrollback = lines.min(100_000);
                }
            },
            cx,
        );
    }

    fn provider(&self, cx: &App) -> AiProvider {
        common::selected(&self.provider, cx)
            .and_then(|code| parse(&code))
            .unwrap_or(AiProvider::Anthropic)
    }

    fn refresh_key_state(&self, cx: &mut Context<Self>) {
        let provider = self.provider(cx);
        let app = self.store.read(cx).app();
        cx.spawn(async move |this, cx| {
            let has_key = backend(crate::ai::ai_has_api_key(app, provider))
                .await
                .unwrap_or(false);
            let _ = this.update(cx, |this, cx| {
                this.has_key = has_key;
                cx.notify();
            });
        })
        .detach();
    }

    /// Each provider has its own model, endpoint and key.
    fn provider_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let model = default_model(self.provider(cx));
        self.model
            .update(cx, |input, cx| input.set_value(model, window, cx));
        self.base_url
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.api_key
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.api_format.update(cx, |select, cx| {
            select.set_selected_index(Some(IndexPath::new(0)), window, cx)
        });
        self.refresh_key_state(cx);
        cx.notify();
    }

    fn save_ai(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let provider = self.provider(cx);
        let config = AiConfig {
            provider,
            model: self.model.read(cx).value().trim().to_string(),
            base_url: self.base_url.read(cx).value().trim().to_string(),
            api_format: common::selected(&self.api_format, cx).and_then(|code| parse(&code)),
        };
        let api_key = self.api_key.read(cx).value().to_string();
        let app = self.store.read(cx).app();
        let Ok(value) = serde_json::to_string(&config) else {
            return;
        };
        self.store
            .update(cx, |store, cx| store.set_setting(AI_SETTING, value, cx));
        self.saving = true;
        self.ai_notice = None;
        self.ai_error = None;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let saved = backend(async move {
                if !api_key.trim().is_empty() {
                    crate::ai::ai_set_api_key(app.clone(), provider, Some(api_key)).await?;
                }
                crate::ai::ai_has_api_key(app, provider).await
            })
            .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.saving = false;
                match saved {
                    Ok(has_key) => {
                        this.has_key = has_key;
                        this.api_key
                            .update(cx, |input, cx| input.set_value("", window, cx));
                        this.ai_notice = Some(t("Saved"));
                    }
                    Err(error) => this.ai_error = Some(error.into()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn remove_key(&mut self, cx: &mut Context<Self>) {
        let provider = self.provider(cx);
        let app = self.store.read(cx).app();
        cx.spawn(async move |this, cx| {
            let removed = backend(crate::ai::ai_set_api_key(app, provider, None)).await;
            let _ = this.update(cx, |this, cx| match removed {
                Ok(()) => this.refresh_key_state(cx),
                Err(error) => {
                    this.ai_error = Some(error.into());
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn check_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.update = UpdateStatus::Checking;
        cx.notify();
        let check = updater::check(window, cx);
        self._update = Some(cx.spawn(async move |this, cx| {
            // A found update opens its own dialog; only say "up to date" when
            // there is none.
            let status = match check.await {
                Ok(true) => UpdateStatus::Idle,
                Ok(false) => UpdateStatus::UpToDate,
                Err(error) => UpdateStatus::Error(error),
            };
            let settled = status == UpdateStatus::Idle;
            let _ = this.update(cx, |this, cx| {
                this.update = status;
                cx.notify();
            });
            if !settled {
                cx.background_executor().timer(Duration::from_secs(3)).await;
                let _ = this.update(cx, |this, cx| {
                    this.update = UpdateStatus::Idle;
                    cx.notify();
                });
            }
        }));
    }

    fn back_up(&mut self, cx: &mut Context<Self>) {
        self.backing_up = true;
        self.backup_notice = None;
        self.backup_error = None;
        cx.notify();
        let app = self.store.read(cx).app();
        cx.spawn(async move |this, cx| {
            let saved = backend(async move { crate::store::db_backup(app.state()).await }).await;
            let _ = this.update(cx, |this, cx| {
                this.backing_up = false;
                match saved {
                    Ok(file_name) => {
                        this.backup_notice = Some(tf(
                            "Saved to Downloads/{fileName}",
                            &[("fileName", &file_name)],
                        ));
                    }
                    Err(error) => this.backup_error = Some(error.into()),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

/// A bordered block with an icon and a heading.
fn section(icon: &'static str, heading: impl IntoElement) -> Div {
    v_flex()
        .gap_4()
        .p_5()
        .rounded(theme::RADIUS)
        .border_1()
        .border_color(theme::hairline())
        .child(
            h_flex()
                .gap_3()
                .child(common::icon_box(icon))
                .child(heading),
        )
}

fn heading(title: SharedString) -> Div {
    div()
        .flex_1()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .child(title)
}

fn note(text: SharedString, color: Rgba) -> Div {
    div().text_xs().text_color(color).child(text)
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let provider = self.provider(cx);
        let row = |label: SharedString, control: AnyElement| {
            h_flex()
                .gap_3()
                .text_sm()
                .child(
                    div()
                        .w(px(96.))
                        .flex_shrink_0()
                        .text_color(theme::STENCIL_MUTE)
                        .child(label),
                )
                .child(div().flex_1().min_w_0().child(control))
        };
        // The placeholders follow the provider and the language, so they are
        // set here rather than once at creation.
        let model_example = match provider {
            AiProvider::OpenaiCompatible => "gpt-5-mini, llama3.2",
            other => default_model(other),
        };
        self.model.update(cx, |input, cx| {
            input.set_placeholder(
                tf("e.g. {example}", &[("example", model_example)]),
                window,
                cx,
            )
        });
        let key_placeholder = if self.has_key {
            t("Saved on this computer")
        } else {
            "".into()
        };
        self.api_key.update(cx, |input, cx| {
            input.set_placeholder(key_placeholder, window, cx)
        });

        let terminal_config = self.store.read(cx).terminal_config();
        let terminal = section("terminal", heading(t("Terminal"))).child(
            v_flex()
                .gap_2()
                .child(row(
                    t("Font size"),
                    Input::new(&self.font_size).small().into_any_element(),
                ))
                .child(row(
                    t("Scrollback"),
                    Input::new(&self.scrollback).small().into_any_element(),
                ))
                .child(row(
                    t("Cursor"),
                    Select::new(&self.cursor_style).small().into_any_element(),
                ))
                .child(row(
                    t("Cursor blink"),
                    Switch::new("cursor-blink")
                        .checked(terminal_config.cursor_blink)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            let checked = *checked;
                            this.set_terminal(|config| config.cursor_blink = checked, cx);
                        }))
                        .into_any_element(),
                ))
                .when(cfg!(target_os = "macos"), |rows| {
                    rows.child(row(
                        t("Option as Meta"),
                        Switch::new("option-as-meta")
                            .checked(terminal_config.option_as_meta)
                            .tooltip(t(
                                "Option+key sends ESC+key instead of typing special characters",
                            ))
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                let checked = *checked;
                                this.set_terminal(|config| config.option_as_meta = checked, cx);
                            }))
                            .into_any_element(),
                    ))
                }),
        );

        let ai = section("sparkles", heading(t("AI assistant")))
            .child(
                v_flex()
                    .gap_2()
                    .child(row(
                        t("Provider"),
                        Select::new(&self.provider).small().into_any_element(),
                    ))
                    .child(row(
                        t("Model"),
                        Input::new(&self.model).small().into_any_element(),
                    ))
                    .when(provider == AiProvider::OpencodeGo, |rows| {
                        rows.child(row(
                            t("API format"),
                            Select::new(&self.api_format).small().into_any_element(),
                        ))
                    })
                    .when(provider == AiProvider::OpenaiCompatible, |rows| {
                        rows.child(row(
                            t("Base URL"),
                            Input::new(&self.base_url).small().into_any_element(),
                        ))
                    })
                    .child(row(
                        t("API key"),
                        h_flex()
                            .gap_2()
                            .child(div().flex_1().child(Input::new(&self.api_key).small()))
                            .when(self.has_key, |key| {
                                key.child(
                                    Button::new("remove-key")
                                        .ghost()
                                        .small()
                                        .label(t("Remove"))
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.remove_key(cx)),
                                        ),
                                )
                            })
                            .into_any_element(),
                    )),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Button::new("save-ai")
                            .outline()
                            .small()
                            .label(t("Save"))
                            .disabled(self.saving)
                            .on_click(cx.listener(|this, _, window, cx| this.save_ai(window, cx))),
                    )
                    .children(
                        self.ai_notice
                            .clone()
                            .map(|notice| note(notice, theme::GREEN_TEXT)),
                    )
                    .children(
                        self.ai_error
                            .clone()
                            .map(|error| note(error, theme::RED_TEXT)),
                    ),
            );

        let check = Button::new("check-update").outline().small().w_full();
        let check = match &self.update {
            UpdateStatus::Checking => check.label(t("Checking...")).disabled(true),
            UpdateStatus::UpToDate => check
                .icon(lucide("circle-check").text_color(theme::GREEN_TEXT))
                .label(t("Up to date"))
                .text_color(theme::GREEN_TEXT),
            _ => check.icon(lucide("download")).label(t("Check for updates")),
        };
        let about = section(
            "settings",
            v_flex()
                .flex_1()
                .child(heading(t("About Noverterm")))
                .child(note(
                    tf(
                        "Version {version}",
                        &[("version", env!("CARGO_PKG_VERSION"))],
                    ),
                    theme::STENCIL_MUTE,
                )),
        )
        .child(check.on_click(cx.listener(|this, _, window, cx| this.check_update(window, cx))))
        .children(match &self.update {
            UpdateStatus::Error(error) => Some(note(
                format!("{}: {error}", t("Failed to check for updates")).into(),
                theme::RED_TEXT,
            )),
            _ => None,
        });

        let backup = section(
            "database-backup",
            v_flex().flex_1().child(heading(t("Backup"))).child(note(
                t("Connections, keys and snippets are stored in plain text — keep the file somewhere private."),
                theme::STENCIL_MUTE,
            )),
        )
        .child(
            Button::new("backup")
                .outline()
                .small()
                .w_full()
                .map(|button| {
                    if self.backing_up {
                        button.label(t("Backing up...")).disabled(true)
                    } else {
                        button.icon(lucide("database-backup")).label(t("Back up to Downloads"))
                    }
                })
                .on_click(cx.listener(|this, _, _, cx| this.back_up(cx))),
        )
        .children(self.backup_notice.clone().map(|notice| note(notice, theme::GREEN_TEXT)))
        .children(self.backup_error.clone().map(|error| note(error, theme::RED_TEXT)));

        v_flex()
            .id("settings")
            // The dialog does not scroll its content by itself.
            .max_h(window.viewport_size().height * 0.7)
            .overflow_y_scroll()
            .gap_6()
            .text_color(theme::STENCIL)
            .child(
                section("languages", heading(t("Language")))
                    .child(Select::new(&self.language).small()),
            )
            .child(terminal)
            .child(ai)
            .child(about)
            .child(backup)
    }
}
