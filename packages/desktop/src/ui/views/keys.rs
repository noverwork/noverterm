//! SSH keys: the list, and the form to add or edit one.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::{h_flex, v_flex, Sizable as _};
use gpui_kit::*;
use shared::SshKeyRecord;

use crate::store::keys::{KeyInput, KeyUpdateInput};
use crate::ui::bridge::backend;
use crate::ui::i18n::{t, tf};
use crate::ui::icons::lucide;
use crate::ui::store::{store, Route, Store};
use crate::ui::theme;
use crate::ui::views::common;

const KEY_PLACEHOLDER: &str = "-----BEGIN OPENSSH PRIVATE KEY-----";

// --- List --------------------------------------------------------------------

pub struct KeysView {
    store: Entity<Store>,
    search: Entity<InputState>,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl KeysView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(t("Search keys")));
        let subscriptions = vec![
            cx.observe(&store, |_, _, cx| cx.notify()),
            cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
        ];
        Self {
            store,
            search,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn edit(&mut self, id: &str, cx: &mut Context<Self>) {
        let route = Route::KeyForm(Some(id.to_string()));
        self.store.update(cx, |store, cx| store.navigate(route, cx));
    }

    fn request_delete(&mut self, key: &SshKeyRecord, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        cx.notify();
        let (view, id) = (cx.entity(), key.id.clone());
        common::confirm_delete(
            t("Delete SSH key?"),
            t("Saved connections using this key may no longer be able to authenticate. This action cannot be undone."),
            Some(key.name.clone().into()),
            t("Delete key"),
            window,
            cx,
            move |_, cx| view.update(cx, |this, cx| this.delete(id.clone(), cx)),
        );
    }

    fn delete(&mut self, id: String, cx: &mut Context<Self>) {
        let app = self.store.read(cx).app();
        cx.spawn(async move |this, cx| {
            let deleted =
                backend(async move { crate::store::keys::key_delete(id, app.state()).await }).await;
            let _ = this.update(cx, |this, cx| {
                this.error = deleted.err().map(SharedString::from);
                this.store.update(cx, |store, cx| store.refresh(cx));
                cx.notify();
            });
        })
        .detach();
    }

    fn render_card(&self, key: &SshKeyRecord, cx: &Context<Self>) -> impl IntoElement {
        let fingerprint: SharedString = match key.fingerprint.as_deref() {
            None | Some("") => "—".into(),
            Some(fingerprint) if fingerprint.chars().count() > 24 => {
                format!("{}…", fingerprint.chars().take(24).collect::<String>()).into()
            }
            Some(fingerprint) => fingerprint.to_string().into(),
        };
        let group: SharedString = format!("key-{}", key.id).into();
        let (edit, delete, menu_key) = (key.id.clone(), key.clone(), key.clone());
        let view = cx.entity();

        common::card(group.clone())
            .group(group.clone())
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.edit(&edit, cx)))
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(common::icon_box("key-round"))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                div()
                                    .truncate()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(key.name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(fingerprint),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .invisible()
                            .group_hover(group, |style| style.visible())
                            .child(
                                Button::new(SharedString::from(format!("delete-{}", key.id)))
                                    .ghost()
                                    .small()
                                    .icon(lucide("trash"))
                                    .tooltip(tf("Delete {name}", &[("name", &key.name)]))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.request_delete(&delete, window, cx);
                                    })),
                            ),
                    ),
            )
            .context_menu(move |menu, _, _| {
                let item = |label: SharedString, icon: &'static str| {
                    PopupMenuItem::new(label).icon(lucide(icon))
                };
                let (edit_view, delete_view) = (view.clone(), view.clone());
                let (edit, delete) = (menu_key.id.clone(), menu_key.clone());
                menu.label(menu_key.name.clone())
                    .separator()
                    .item(item(t("Edit"), "pencil").on_click(move |_, _, cx| {
                        edit_view.update(cx, |this, cx| this.edit(&edit, cx));
                    }))
                    .separator()
                    .item(item(t("Delete"), "trash").on_click(move |_, window, cx| {
                        delete_view.update(cx, |this, cx| this.request_delete(&delete, window, cx));
                    }))
            })
    }
}

impl Render for KeysView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        let query = self.search.read(cx).value().trim().to_lowercase();
        let visible: Vec<&SshKeyRecord> = store
            .keys
            .iter()
            .filter(|key| {
                query.is_empty()
                    || key.name.to_lowercase().contains(&query)
                    || key
                        .fingerprint
                        .as_deref()
                        .is_some_and(|fingerprint| fingerprint.to_lowercase().contains(&query))
            })
            .collect();
        let body = if store.keys.is_empty() {
            common::empty_state(t("No SSH keys saved yet")).into_any_element()
        } else if visible.is_empty() {
            common::empty_state(t("No keys match your search.")).into_any_element()
        } else {
            div()
                .grid()
                .grid_cols(3)
                .gap_3()
                .children(visible.into_iter().map(|key| self.render_card(key, cx)))
                .into_any_element()
        };

        common::page(
            t("SSH Keys"),
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
                    Button::new("add-key")
                        .primary()
                        .small()
                        .icon(lucide("plus"))
                        .label(t("Add key"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.store
                                .update(cx, |store, cx| store.navigate(Route::KeyForm(None), cx));
                        })),
                ),
            common::page_body("keys")
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

pub struct KeyForm {
    store: Entity<Store>,
    /// The key being edited; `None` adds a new one.
    key: Option<SshKeyRecord>,
    /// Editing a key that no longer exists.
    missing: bool,
    name: Entity<InputState>,
    passphrase: Entity<InputState>,
    private_key: Entity<TextareaState>,
    error: Option<SharedString>,
    saving: bool,
    /// The saved secret is still being read into the fields.
    loading_secret: bool,
}

impl KeyForm {
    pub fn new(id: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let key = id
            .as_deref()
            .and_then(|id| store.read(cx).keys.iter().find(|key| key.id == id))
            .cloned();
        let name_value = key.as_ref().map(|key| key.name.clone()).unwrap_or_default();
        let loading_secret = key.is_some();
        if let Some(key) = &key {
            Self::load_secret(key.id.clone(), store.read(cx).app(), window, cx);
        }
        Self {
            missing: id.is_some() && key.is_none(),
            name: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(tf("e.g. {example}", &[("example", "GitHub key")]))
                    .default_value(name_value)
            }),
            passphrase: cx.new(|cx| {
                let state = InputState::new(window, cx).masked(true);
                if loading_secret {
                    state.placeholder(t("Loading saved passphrase…"))
                } else {
                    state
                }
            }),
            private_key: cx.new(|cx| {
                TextareaState::new(window, cx)
                    .rows(10)
                    .placeholder(if loading_secret {
                        t("Loading saved private key…")
                    } else {
                        KEY_PLACEHOLDER.into()
                    })
            }),
            store,
            key,
            error: None,
            saving: false,
            loading_secret,
        }
    }

    /// Fills the fields with the saved key, so editing starts from what is stored.
    fn load_secret(
        id: String,
        app: crate::app::AppHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |this, cx| {
            let secret =
                backend(async move { crate::store::keys::key_secret(id, app.state()).await }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.loading_secret = false;
                this.passphrase
                    .update(cx, |input, cx| input.set_placeholder("", window, cx));
                this.private_key.update(cx, |input, cx| {
                    input.set_placeholder(KEY_PLACEHOLDER, window, cx)
                });
                match secret {
                    Ok(secret) => {
                        this.private_key.update(cx, |input, cx| {
                            input.set_value(secret.private_key, window, cx)
                        });
                        this.passphrase.update(cx, |input, cx| {
                            input.set_value(secret.passphrase.unwrap_or_default(), window, cx)
                        });
                    }
                    Err(error) => this.error = Some(error.into()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        self.store
            .update(cx, |store, cx| store.navigate(Route::Keys, cx));
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        if self.loading_secret {
            return;
        }
        let name = self.name.read(cx).value().trim().to_string();
        let private_key = self.private_key.read(cx).value().trim().to_string();
        let passphrase = self.passphrase.read(cx).value().trim().to_string();
        let editing = self.key.is_some();
        let invalid = if name.is_empty() {
            Some(t("Key name is required"))
        } else if !editing && private_key.is_empty() {
            Some(t("Private key is required"))
        } else if editing && private_key.is_empty() && !passphrase.is_empty() {
            Some(t(
                "Paste a replacement private key before changing the passphrase",
            ))
        } else {
            None
        };
        if invalid.is_some() {
            self.error = invalid;
            cx.notify();
            return;
        }
        self.error = None;
        self.saving = true;
        cx.notify();
        let passphrase = Some(passphrase).filter(|passphrase| !passphrase.is_empty());
        let (app, id) = (
            self.store.read(cx).app(),
            self.key.as_ref().map(|key| key.id.clone()),
        );
        cx.spawn(async move |this, cx| {
            let saved = backend(async move {
                match id {
                    Some(id) => {
                        let private_key = Some(private_key).filter(|key| !key.is_empty());
                        let key = KeyUpdateInput {
                            name,
                            kind: "inline".to_string(),
                            fingerprint: None,
                            // The passphrase belongs to the pasted key, not the stored one.
                            passphrase: private_key.as_ref().and(passphrase),
                            private_key,
                        };
                        crate::store::keys::key_update(id, key, app.state()).await
                    }
                    None => {
                        let key = KeyInput {
                            name,
                            kind: "inline".to_string(),
                            fingerprint: None,
                            private_key,
                            passphrase,
                        };
                        crate::store::keys::key_create(key, app.state()).await
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
}

impl Render for KeyForm {
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
                        .child(t("SSH key not found")),
                )
                .child(
                    Button::new("back")
                        .primary()
                        .label(t("Back to SSH keys"))
                        .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
                )
                .into_any_element();
        }
        let editing = self.key.is_some();
        let submit_label = match (self.loading_secret, self.saving, editing) {
            (true, ..) => t("Loading key…"),
            (_, true, true) => t("Updating…"),
            (_, true, false) => t("Saving…"),
            (_, false, true) => t("Update key"),
            (_, false, false) => t("Save key"),
        };
        let disabled = self.saving || self.loading_secret;
        let fingerprint = self.key.as_ref().map(|key| {
            v_flex()
                .gap_1()
                .px_3()
                .py_2p5()
                .rounded(theme::RADIUS)
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::RAIL)
                .child(common::section_title(t("Current fingerprint")))
                .child(
                    div()
                        .truncate()
                        .text_sm()
                        .text_color(theme::STENCIL_DIM)
                        .child(key.fingerprint.clone().unwrap_or_else(|| "—".to_string())),
                )
        });

        common::form_page(
            if editing {
                t("Edit SSH key")
            } else {
                t("New SSH key")
            },
            self.error.clone(),
            v_flex()
                .gap_4()
                .child(
                    common::form_section("key-round", t("Key identity"), None)
                        .children(fingerprint)
                        .child(common::field(
                            t("Key name"),
                            Input::new(&self.name).disabled(disabled),
                            None,
                        ))
                        .child(common::field(
                            SharedString::from(format!("{} {}", t("Passphrase"), t("(optional)"))),
                            Input::new(&self.passphrase).disabled(disabled),
                            None,
                        )),
                )
                .child(
                    common::form_section("key-round", t("Private key"), None).child(common::field(
                        t("Private key"),
                        Textarea::new(&self.private_key).disabled(disabled),
                        None,
                    )),
                ),
            common::form_buttons(
                submit_label,
                self.saving,
                cx.listener(|this, _, _, cx| this.back(cx)),
                cx.listener(|this, _, _, cx| this.save(cx)),
            ),
        )
        .into_any_element()
    }
}
