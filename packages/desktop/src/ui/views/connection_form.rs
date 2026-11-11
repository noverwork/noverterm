//! The form to add or edit a saved SSH host.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::select::Select;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{h_flex, v_flex, Disableable as _, IndexPath};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use shared::AiAccess;

use crate::store::hosts::SaveConnectionInput;
use crate::ui::bridge::backend;
use crate::ui::i18n::{t, tf};
use crate::ui::icons::lucide;
use crate::ui::store::{store, Route, Store};
use crate::ui::theme;
use crate::ui::views::common::{self, Choice, ChoiceSelect};
use crate::ui::views::connections::{saved_password, AI_ACCESS_OPTIONS};

/// Validation messages, one per field that can be wrong.
#[derive(Debug, Default, PartialEq)]
struct Errors {
    name: Option<&'static str>,
    host: Option<&'static str>,
    port: Option<&'static str>,
    username: Option<&'static str>,
    private_key: Option<&'static str>,
}

/// Checks the trimmed field values; `key_required` is set when a new key is
/// being pasted. Returns the parsed port when everything is valid.
fn validate(
    name: &str,
    host: &str,
    port: &str,
    username: &str,
    private_key: &str,
    key_required: bool,
) -> Result<i32, Errors> {
    let port = port
        .parse::<i32>()
        .ok()
        .filter(|port| (1..=65535).contains(port));
    let errors = Errors {
        name: name.is_empty().then_some("Connection name is required"),
        host: host.is_empty().then_some("Host is required"),
        port: port.is_none().then_some("Port must be between 1 and 65535"),
        username: username.is_empty().then_some("Username is required"),
        private_key: (key_required && private_key.is_empty())
            .then_some("Paste a private key or select a saved key"),
    };
    match port {
        Some(port) if errors == Errors::default() => Ok(port),
        _ => Err(errors),
    }
}

pub struct ConnectionForm {
    store: Entity<Store>,
    /// The host being edited; `None` adds a new one.
    id: Option<String>,
    /// Editing a host that no longer exists.
    missing: bool,
    /// Kept as is: hosts change group from the list, not from this form.
    group_id: Option<String>,
    /// The saved password, kept when the field is left empty.
    saved_password: Option<String>,
    has_keys: bool,
    name: Entity<InputState>,
    host: Entity<InputState>,
    port: Entity<InputState>,
    username: Entity<InputState>,
    /// Empty value connects directly.
    jump_host: Entity<ChoiceSelect>,
    has_jump_hosts: bool,
    password: Entity<InputState>,
    use_key: bool,
    /// Pasting a new key rather than picking a saved one.
    new_key: bool,
    saved_key: Entity<ChoiceSelect>,
    key_name: Entity<InputState>,
    private_key: Entity<TextareaState>,
    passphrase: Entity<InputState>,
    ai_access: AiAccess,
    errors: Errors,
    error: Option<SharedString>,
    saving: bool,
}

impl ConnectionForm {
    pub fn new(id: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let (host, jump_hosts, keys) = {
            let store = store.read(cx);
            let host = id.as_deref().and_then(|id| store.host(id)).cloned();
            let mut jump_hosts = vec![Choice::new("", t("None"))];
            jump_hosts.extend(
                store
                    .hosts
                    .iter()
                    .filter(|other| Some(&other.id) != id.as_ref())
                    .map(|other| {
                        Choice::new(
                            other.id.clone(),
                            format!("{} ({}@{})", other.name, other.username, other.host),
                        )
                    }),
            );
            let mut keys = vec![Choice::new("", t("— Select a saved key —"))];
            keys.extend(store.keys.iter().map(|key| {
                let fingerprint = key
                    .fingerprint
                    .as_deref()
                    .map(|fingerprint| {
                        format!(" — {}…", fingerprint.chars().take(16).collect::<String>())
                    })
                    .unwrap_or_default();
                Choice::new(
                    key.id.clone(),
                    format!("{} ({}){fingerprint}", key.name, key.kind),
                )
            }));
            (host, jump_hosts, keys)
        };
        let saved_password = host.as_ref().and_then(saved_password);
        let text = |value: Option<&String>| value.cloned().unwrap_or_default();
        let (has_keys, has_jump_hosts) = (keys.len() > 1, jump_hosts.len() > 1);
        let mut input = |placeholder: SharedString, value: String| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(placeholder)
                    .default_value(value)
            })
        };
        let example = |example: &str| tf("e.g. {example}", &[("example", example)]);
        Self {
            missing: id.is_some() && host.is_none(),
            name: input(
                example("prod-web-01"),
                text(host.as_ref().map(|host| &host.name)),
            ),
            host: input(
                example("prod.example.com"),
                text(host.as_ref().map(|host| &host.host)),
            ),
            port: input(
                "22".into(),
                host.as_ref().map_or(22, |host| host.port).to_string(),
            ),
            username: input(
                example("ubuntu"),
                text(host.as_ref().map(|host| &host.username)),
            ),
            key_name: input(example("GitHub key"), String::new()),
            jump_host: common::new_select(
                jump_hosts,
                host.as_ref()
                    .and_then(|host| host.jump_host_id.as_deref())
                    .unwrap_or_default(),
                window,
                cx,
            ),
            saved_key: common::new_select(
                keys,
                host.as_ref()
                    .and_then(|host| host.ssh_key_id.as_deref())
                    .unwrap_or_default(),
                window,
                cx,
            ),
            password: cx.new(|cx| {
                let state = InputState::new(window, cx)
                    .masked(true)
                    .default_value(saved_password.clone().unwrap_or_default());
                if saved_password.is_some() {
                    state.placeholder(t("Saved password"))
                } else {
                    state
                }
            }),
            private_key: cx.new(|cx| {
                TextareaState::new(window, cx)
                    .rows(8)
                    .placeholder("-----BEGIN OPENSSH PRIVATE KEY-----")
            }),
            passphrase: cx.new(|cx| InputState::new(window, cx).masked(true)),
            use_key: host.as_ref().is_some_and(|host| host.ssh_key_id.is_some()),
            // With no saved key to pick, the only way to use one is to paste it.
            new_key: !has_keys,
            ai_access: host.as_ref().map_or(AiAccess::Ask, |host| host.ai_access),
            group_id: host.and_then(|host| host.group_id),
            saved_password,
            has_keys,
            has_jump_hosts,
            store,
            id,
            errors: Errors::default(),
            error: None,
            saving: false,
        }
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        self.store
            .update(cx, |store, cx| store.navigate(Route::Connections, cx));
    }

    /// Switching the key off forgets what was entered for it.
    fn set_use_key(&mut self, use_key: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.use_key = use_key;
        if !use_key {
            self.new_key = !self.has_keys;
            self.saved_key.update(cx, |select, cx| {
                select.set_selected_index(Some(IndexPath::new(0)), window, cx)
            });
            self.key_name
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.private_key
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.passphrase
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
        cx.notify();
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let value = |input: &Entity<InputState>| input.read(cx).value().trim().to_string();
        let non_empty = |value: String| (!value.is_empty()).then_some(value);
        let (name, host, username) = (value(&self.name), value(&self.host), value(&self.username));
        let pasting = self.use_key && self.new_key;
        let private_key = self.private_key.read(cx).value().trim().to_string();
        let port = match validate(
            &name,
            &host,
            &value(&self.port),
            &username,
            &private_key,
            pasting,
        ) {
            Ok(port) => port,
            Err(errors) => {
                self.errors = errors;
                cx.notify();
                return;
            }
        };
        let request = SaveConnectionInput {
            id: self.id.clone(),
            name,
            host,
            port,
            username,
            group_id: self.group_id.clone(),
            jump_host_id: common::selected(&self.jump_host, cx).and_then(non_empty),
            password: non_empty(value(&self.password)).or_else(|| self.saved_password.clone()),
            private_key: pasting.then_some(private_key),
            passphrase: non_empty(value(&self.passphrase)).filter(|_| pasting),
            key_name: non_empty(value(&self.key_name)),
            existing_key_id: common::selected(&self.saved_key, cx)
                .and_then(non_empty)
                .filter(|_| self.use_key && !self.new_key),
            ai_access: Some(self.ai_access),
        };
        self.errors = Errors::default();
        self.error = None;
        self.saving = true;
        cx.notify();
        let app = self.store.read(cx).app();
        cx.spawn(async move |this, cx| {
            let saved =
                backend(async move { crate::store::hosts::host_save(request, app.state()).await })
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

    /// A row of mutually exclusive options; `on_pick` gets the option's index.
    fn segments(
        &self,
        id: &'static str,
        options: Vec<(SharedString, bool)>,
        cx: &Context<Self>,
        on_pick: impl Fn(&mut Self, usize, &mut Context<Self>) + Clone + 'static,
    ) -> Div {
        let saving = self.saving;
        h_flex()
            .gap_1()
            .p_1()
            .rounded(theme::RADIUS)
            .border_1()
            .border_color(theme::hairline())
            .bg(theme::RAIL)
            .children(
                options
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, active))| {
                        let on_pick = on_pick.clone();
                        div()
                            .id((id, index))
                            .flex_1()
                            .px_3()
                            .py_2()
                            .rounded(theme::RADIUS)
                            .border_1()
                            .text_sm()
                            .text_center()
                            .map(|segment| {
                                if active {
                                    segment
                                        .border_color(theme::STENCIL)
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme::STENCIL)
                                } else {
                                    segment
                                        .border_color(transparent_black())
                                        .text_color(theme::STENCIL_MUTE)
                                        .hover(|style| {
                                            style.bg(theme::wash()).text_color(theme::STENCIL)
                                        })
                                }
                            })
                            .when(!saving, |segment| {
                                segment.cursor_pointer().on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        on_pick(this, index, cx);
                                        cx.notify();
                                    },
                                ))
                            })
                            .child(label)
                    }),
            )
    }
}

impl Render for ConnectionForm {
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
                        .child(t("Connection not found")),
                )
                .child(
                    Button::new("back")
                        .primary()
                        .label(t("Back to connections"))
                        .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
                )
                .into_any_element();
        }
        let editing = self.id.is_some();
        let saving = self.saving;
        let submit_label = match (saving, editing) {
            (true, _) => t("Saving…"),
            (false, true) => t("Save changes"),
            (false, false) => t("Save"),
        };
        let optional =
            |label: &'static str| SharedString::from(format!("{} {}", t(label), t("(optional)")));
        let ai_hint = match self.ai_access {
            AiAccess::Allow => t("AI agents run commands on this host without asking."),
            AiAccess::Ask => t("Every AI request on this host waits for your approval."),
            AiAccess::Deny => t("AI agents cannot use this host."),
        };
        let selected_key = common::selected(&self.saved_key, cx)
            .filter(|id| !id.is_empty())
            .and_then(|id| self.store.read(cx).keys.iter().find(|key| key.id == id))
            .map(|key| key.name.clone());

        let key_fields = v_flex()
            .gap_3()
            .when(self.has_keys, |fields| {
                fields.child(self.segments(
                    "key-mode",
                    vec![
                        (t("Saved key"), !self.new_key),
                        (t("Paste new key"), self.new_key),
                    ],
                    cx,
                    |this, index, _| this.new_key = index == 1,
                ))
            })
            .map(|fields| {
                if self.new_key {
                    fields
                        .child(common::field(
                            optional("Key name"),
                            Input::new(&self.key_name).disabled(saving),
                            None,
                        ))
                        .child(common::field(
                            t("Private key"),
                            Textarea::new(&self.private_key).disabled(saving),
                            self.errors.private_key.map(t),
                        ))
                        .child(common::field(
                            optional("Key passphrase"),
                            Input::new(&self.passphrase).mask_toggle().disabled(saving),
                            None,
                        ))
                } else {
                    fields.child(
                        common::field(
                            t("Saved key"),
                            Select::new(&self.saved_key).disabled(saving),
                            None,
                        )
                        .children(selected_key.map(|name| {
                            div()
                                .text_xs()
                                .text_color(theme::GREEN_TEXT)
                                .child(tf("Using saved key: {name}", &[("name", &name)]))
                        })),
                    )
                }
            });

        common::form_page(
            if editing {
                t("Edit connection")
            } else {
                t("New connection")
            },
            self.error.clone(),
            v_flex()
                .gap_4()
                .child(
                    common::form_section("server", t("Endpoint"), None)
                        .child(common::field(
                            t("Name"),
                            Input::new(&self.name).disabled(saving),
                            self.errors.name.map(t),
                        ))
                        .child(
                            h_flex()
                                .items_start()
                                .gap_3()
                                .child(
                                    common::field(
                                        t("Hostname"),
                                        Input::new(&self.host).disabled(saving),
                                        self.errors.host.map(t),
                                    )
                                    .flex_1()
                                    .min_w_0(),
                                )
                                .child(
                                    common::field(
                                        t("Port"),
                                        Input::new(&self.port).disabled(saving),
                                        self.errors.port.map(t),
                                    )
                                    .w(px(112.)),
                                ),
                        )
                        .child(common::field(
                            t("Username"),
                            Input::new(&self.username).disabled(saving),
                            self.errors.username.map(t),
                        ))
                        .child(common::field(
                            t("Jump host"),
                            Select::new(&self.jump_host).disabled(saving || !self.has_jump_hosts),
                            None,
                        )),
                )
                .child(
                    common::form_section("key-round", t("Authentication"), None)
                        .child(common::field(
                            t("Password"),
                            Input::new(&self.password).mask_toggle().disabled(saving),
                            None,
                        ))
                        .child(
                            v_flex()
                                .gap_4()
                                .pt_3()
                                .border_t_1()
                                .border_color(theme::hairline())
                                .child(
                                    h_flex()
                                        .justify_between()
                                        .gap_4()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::MEDIUM)
                                                .child(t("SSH key")),
                                        )
                                        .child(
                                            Switch::new("use-ssh-key")
                                                .accessibility_label(t("Use SSH key"))
                                                .checked(self.use_key)
                                                .disabled(saving)
                                                .on_change(cx.listener(
                                                    |this, checked: &bool, window, cx| {
                                                        this.set_use_key(*checked, window, cx)
                                                    },
                                                )),
                                        ),
                                )
                                .when(self.use_key, |section| section.child(key_fields)),
                        ),
                )
                .child(
                    common::form_section("bot", t("MCP access"), Some(ai_hint)).child(
                        self.segments(
                            "ai-access",
                            AI_ACCESS_OPTIONS
                                .iter()
                                .map(|(access, label)| (t(label), *access == self.ai_access))
                                .collect(),
                            cx,
                            |this, index, _| this.ai_access = AI_ACCESS_OPTIONS[index].0,
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
    use super::{validate, Errors};

    #[test]
    fn validation_reports_each_bad_field() {
        assert_eq!(
            validate("web", "example.com", "22", "root", "", false),
            Ok(22)
        );
        assert_eq!(
            validate("", "", "70000", "", "", true),
            Err(Errors {
                name: Some("Connection name is required"),
                host: Some("Host is required"),
                port: Some("Port must be between 1 and 65535"),
                username: Some("Username is required"),
                private_key: Some("Paste a private key or select a saved key"),
            })
        );
        assert_eq!(
            validate("web", "example.com", "abc", "root", "key", true)
                .unwrap_err()
                .port,
            Some("Port must be between 1 and 65535")
        );
    }
}
