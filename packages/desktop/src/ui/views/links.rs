//! Links: local HTTP services published at `https://<name>.<relay>` through
//! the user's own relay. The relay setup, the saved links with their state,
//! and the form to add or edit one.

use std::collections::{HashMap, HashSet};

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use shared::{LinkRecord, LinkWriteRequest};

use crate::app::AppHandle;
use crate::runtime::link::{
    parse_relay, LinkManager, LinkStartInput, LinkState, LinkStatus, RELAY_SETTING,
};
use crate::trust::SshTrustStore;
use crate::ui::bridge::backend;
use crate::ui::i18n::{t, tf};
use crate::ui::icons::lucide;
use crate::ui::store::{store, Route, Store, StoreEvent};
use crate::ui::theme;
use crate::ui::views::common;

// --- Runtime state -------------------------------------------------------------

/// The links the backend is running, kept current from its status events.
pub struct LinkRuntime {
    app: AppHandle,
    links: HashMap<String, LinkStatus>,
    _subscription: Subscription,
}

struct RuntimeGlobal(Entity<LinkRuntime>);

impl Global for RuntimeGlobal {}

fn runtime(cx: &mut App) -> Entity<LinkRuntime> {
    if let Some(global) = cx.try_global::<RuntimeGlobal>() {
        return global.0.clone();
    }
    let runtime = cx.new(LinkRuntime::new);
    cx.set_global(RuntimeGlobal(runtime.clone()));
    runtime
}

impl LinkRuntime {
    fn new(cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let subscription = cx.subscribe(&store, |this, _, event: &StoreEvent, cx| {
            if let StoreEvent::Backend("link_status", payload) = event {
                if let Ok(status) = serde_json::from_str(payload) {
                    this.apply(status, cx);
                }
            }
        });
        let app = store.read(cx).app();
        cx.spawn(async move |this, cx| {
            let listed = backend(async move { app.state::<LinkManager>().list().await }).await;
            let _ = this.update(cx, |this, cx| {
                for status in listed {
                    this.apply(status, cx);
                }
            });
        })
        .detach();
        Self {
            app: store.read(cx).app(),
            links: HashMap::new(),
            _subscription: subscription,
        }
    }

    fn apply(&mut self, status: LinkStatus, cx: &mut Context<Self>) {
        // A link an AI agent just created: load it so its card shows.
        let store = store(cx);
        if !store.read(cx).links.iter().any(|link| link.id == status.id) {
            store.update(cx, |store, cx| store.refresh(cx));
        }
        if status.state == LinkState::Stopped {
            self.links.remove(&status.id);
        } else {
            self.links.insert(status.id.clone(), status);
        }
        cx.notify();
    }

    fn start(
        &mut self,
        link: &LinkRecord,
        relay: String,
        cx: &mut Context<Self>,
    ) -> Task<Result<(), String>> {
        let app = self.app.clone();
        let input = LinkStartInput {
            id: link.id.clone(),
            subdomain: link.subdomain.clone(),
            target_host: link.target_host.clone(),
            target_port: link.target_port as u16,
            relay,
        };
        let started = backend(async move {
            let trust = app.state::<SshTrustStore>().inner().clone();
            app.state::<LinkManager>()
                .start(app.clone(), input, trust)
                .await
                .map(|_| ())
        });
        cx.spawn(async move |_, _| started.await)
    }

    /// Stops a running link; one that already failed is just dismissed.
    fn stop(&mut self, id: String, cx: &mut Context<Self>) -> Task<Result<(), String>> {
        if self
            .links
            .remove(&id)
            .is_some_and(|status| status.state == LinkState::Error)
        {
            cx.notify();
            return Task::ready(Ok(()));
        }
        let app = self.app.clone();
        let stopped =
            backend(async move { app.state::<LinkManager>().stop(app.clone(), &id).await });
        cx.spawn(async move |_, _| stopped.await)
    }
}

fn state_badge(state: Option<LinkState>) -> (Rgba, SharedString) {
    match state {
        Some(LinkState::Online) => (theme::GREEN_TEXT, t("Online")),
        Some(LinkState::Connecting) => (theme::AMBER_TEXT, t("Connecting")),
        Some(LinkState::Error) => (theme::RED_TEXT, t("Error")),
        Some(LinkState::Stopped) | None => (theme::STENCIL_MUTE, t("Idle")),
    }
}

fn dot(color: Rgba) -> Div {
    div().size_1p5().flex_shrink_0().rounded_full().bg(color)
}

/// The public URL of `subdomain` on the saved relay, if one is set.
fn public_url(store: &Store, subdomain: &str) -> Option<String> {
    let (host, _) = parse_relay(store.setting(RELAY_SETTING)?).ok()?;
    Some(format!("https://{subdomain}.{host}"))
}

// --- List --------------------------------------------------------------------

pub struct LinksView {
    store: Entity<Store>,
    runtime: Entity<LinkRuntime>,
    relay: Entity<InputState>,
    public_key: Option<String>,
    error: Option<SharedString>,
    /// Links with a start, stop or delete under way.
    busy: HashSet<String>,
    _subscriptions: Vec<Subscription>,
}

impl LinksView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let runtime = runtime(cx);
        let saved_relay = store
            .read(cx)
            .setting(RELAY_SETTING)
            .unwrap_or_default()
            .to_string();
        let relay = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(tf("e.g. {example}", &[("example", "link.example.com")]))
                .default_value(saved_relay)
        });
        let app = store.read(cx).app();
        cx.spawn(async move |this, cx| {
            let key =
                backend(async move { crate::runtime::link::link_public_key(&app).await }).await;
            let _ = this.update(cx, |this, cx| {
                match key {
                    Ok(key) => this.public_key = Some(key),
                    Err(error) => this.error = Some(error.into()),
                }
                cx.notify();
            });
        })
        .detach();
        Self {
            _subscriptions: vec![
                cx.observe(&store, |_, _, cx| cx.notify()),
                cx.observe(&runtime, |_, _, cx| cx.notify()),
            ],
            store,
            runtime,
            relay,
            public_key: None,
            error: None,
            busy: HashSet::new(),
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

    fn save_relay(&mut self, cx: &mut Context<Self>) {
        let value = self.relay.read(cx).value().trim().to_lowercase();
        if let Err(error) = parse_relay(&value) {
            self.error = Some(error.into());
            cx.notify();
            return;
        }
        self.error = None;
        self.store
            .update(cx, |store, cx| store.set_setting(RELAY_SETTING, value, cx));
    }

    fn toggle(&mut self, link: &LinkRecord, running: bool, cx: &mut Context<Self>) {
        let task = if running {
            let id = link.id.clone();
            self.runtime.update(cx, |runtime, cx| runtime.stop(id, cx))
        } else {
            match self.store.read(cx).setting(RELAY_SETTING) {
                Some(relay) => {
                    let relay = relay.to_string();
                    self.runtime
                        .update(cx, |runtime, cx| runtime.start(link, relay, cx))
                }
                None => Task::ready(Err(t("Set the relay domain first").to_string())),
            }
        };
        self.track(link.id.clone(), task, cx);
    }

    fn request_delete(&mut self, link: &LinkRecord, window: &mut Window, cx: &mut Context<Self>) {
        let (view, id) = (cx.entity(), link.id.clone());
        common::confirm_delete(
            t("Delete link?"),
            t("This removes the saved link."),
            Some(link.subdomain.clone().into()),
            t("Delete link"),
            window,
            cx,
            move |_, cx| view.update(cx, |this, cx| this.delete(id.clone(), cx)),
        );
    }

    fn delete(&mut self, id: String, cx: &mut Context<Self>) {
        let (app, store, link_id) = (self.store.read(cx).app(), self.store.clone(), id.clone());
        let task = cx.spawn(async move |_, cx| {
            let deleted =
                backend(async move { crate::store::links::link_delete(id, app.state()).await })
                    .await;
            store.update(cx, |store, cx| store.refresh(cx));
            deleted
        });
        self.track(link_id, task, cx);
    }

    fn render_relay(&self, cx: &Context<Self>) -> impl IntoElement {
        let key = self.public_key.clone();
        common::form_section("server", t("Relay"), None)
            .child(
                h_flex()
                    .items_end()
                    .gap_2()
                    .child(
                        common::field(t("Relay domain"), Input::new(&self.relay).small(), None)
                            .flex_1(),
                    )
                    .child(
                        Button::new("save-relay")
                            .outline()
                            .small()
                            .label(t("Save"))
                            .on_click(cx.listener(|this, _, _, cx| this.save_relay(cx))),
                    ),
            )
            .child(common::field(
                t("Link key"),
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .px_3()
                            .py_1p5()
                            .rounded(theme::RADIUS)
                            .border_1()
                            .border_color(theme::hairline())
                            .bg(theme::RAIL)
                            .text_xs()
                            .text_color(theme::STENCIL_DIM)
                            .child(key.clone().unwrap_or_default()),
                    )
                    .child(
                        Button::new("copy-key")
                            .ghost()
                            .small()
                            .icon(lucide("copy"))
                            .tooltip(t("Copy key"))
                            .disabled(key.is_none())
                            .on_click(move |_, _, cx| {
                                if let Some(key) = &key {
                                    cx.write_to_clipboard(ClipboardItem::new_string(key.clone()));
                                }
                            }),
                    ),
                None,
            ))
    }

    fn render_card(&self, link: &LinkRecord, cx: &Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        let status = self.runtime.read(cx).links.get(&link.id);
        let state = status.map(|status| status.state);
        let (badge_color, badge_label) = state_badge(state);
        let running = matches!(state, Some(LinkState::Connecting | LinkState::Online));
        let busy = self.busy.contains(&link.id);
        let url = status
            .map(|status| status.url.clone())
            .or_else(|| public_url(store, &link.subdomain));
        let error = status.and_then(|status| status.error.clone());
        let group: SharedString = format!("link-{}", link.id).into();
        let icon_button = |id: &'static str, icon: &'static str, label: SharedString| {
            Button::new(SharedString::from(format!("{id}-{}", link.id)))
                .ghost()
                .small()
                .icon(lucide(icon))
                .tooltip(label)
        };
        let (toggle, edit, delete) = (link.clone(), link.id.clone(), link.clone());
        let (copy_url, open_url) = (url.clone(), url.clone());

        common::card(group.clone())
            .group(group.clone())
            .cursor_pointer()
            .when(state.is_some(), |card| {
                card.border_color(badge_color.opacity(0.3))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.busy.contains(&toggle.id) {
                    this.toggle(&toggle, running, cx);
                }
            }))
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(common::icon_box(if busy {
                        "loader-circle"
                    } else {
                        "globe"
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
                                            .child(link.subdomain.clone()),
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
                            .child(
                                div()
                                    .truncate()
                                    .text_xs()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(format!("{}:{}", link.target_host, link.target_port)),
                            ),
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
                                        t("Stop the link to edit it")
                                    } else {
                                        t("Edit")
                                    },
                                )
                                .disabled(running)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        let route = Route::LinkForm(Some(edit.clone()));
                                        this.store
                                            .update(cx, |store, cx| store.navigate(route, cx));
                                    },
                                )),
                            )
                            .child(
                                icon_button(
                                    "delete",
                                    "trash",
                                    if running {
                                        t("Stop the link to delete it")
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
            .children(url.map(|url| {
                h_flex()
                    .mt_3()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .rounded(theme::RADIUS)
                    .border_1()
                    .border_color(theme::hairline())
                    .bg(theme::RAIL)
                    .text_xs()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(if state == Some(LinkState::Online) {
                                theme::STENCIL
                            } else {
                                theme::STENCIL_FAINT
                            })
                            .child(url),
                    )
                    .child(icon_button("copy-url", "copy", t("Copy URL")).on_click(
                        move |_, _, cx| {
                            cx.stop_propagation();
                            if let Some(url) = &copy_url {
                                cx.write_to_clipboard(ClipboardItem::new_string(url.clone()));
                            }
                        },
                    ))
                    .child(
                        icon_button("open-url", "external-link", t("Open in browser")).on_click(
                            move |_, _, cx| {
                                cx.stop_propagation();
                                if let Some(url) = &open_url {
                                    cx.open_url(url);
                                }
                            },
                        ),
                    )
            }))
            .children(error.map(|error| {
                h_flex()
                    .mt_2()
                    .items_start()
                    .gap_1p5()
                    .text_xs()
                    .text_color(if state == Some(LinkState::Error) {
                        theme::RED_TEXT
                    } else {
                        theme::AMBER_TEXT
                    })
                    .child(lucide("circle-alert").xsmall().flex_shrink_0())
                    .child(div().min_w_0().child(error))
            }))
    }
}

impl Render for LinksView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        let mut links: Vec<&LinkRecord> = store.links.iter().collect();
        links.sort_by_key(|link| link.subdomain.clone());
        let body = if store.setting(RELAY_SETTING).is_none() {
            common::empty_state(t("Set the relay domain first")).into_any_element()
        } else if links.is_empty() {
            common::empty_state(t("No links yet")).into_any_element()
        } else {
            div()
                .grid()
                .grid_cols(3)
                .gap_3()
                .children(links.into_iter().map(|link| self.render_card(link, cx)))
                .into_any_element()
        };

        common::page(
            t("Links"),
            Button::new("add-link")
                .primary()
                .small()
                .icon(lucide("plus"))
                .label(t("New Link"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.store
                        .update(cx, |store, cx| store.navigate(Route::LinkForm(None), cx));
                })),
            common::page_body("links")
                .children(
                    self.error
                        .clone()
                        .map(|error| common::error_banner(error).mb_4()),
                )
                .child(div().mb_4().child(self.render_relay(cx)))
                .child(body),
        )
    }
}

// --- Form --------------------------------------------------------------------

pub struct LinkForm {
    store: Entity<Store>,
    /// The link being edited; `None` adds a new one.
    id: Option<String>,
    subdomain: Entity<InputState>,
    target_host: Entity<InputState>,
    target_port: Entity<InputState>,
    error: Option<SharedString>,
    saving: bool,
}

impl LinkForm {
    pub fn new(id: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let link = id.as_deref().and_then(|id| {
            store
                .read(cx)
                .links
                .iter()
                .find(|link| link.id == id)
                .cloned()
        });
        let mut input = |value: String, placeholder: &str| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(tf("e.g. {example}", &[("example", placeholder)]))
                    .default_value(value)
            })
        };
        Self {
            subdomain: input(
                link.as_ref()
                    .map(|l| l.subdomain.clone())
                    .unwrap_or_default(),
                "myapp",
            ),
            target_host: input(
                link.as_ref()
                    .map_or_else(|| "127.0.0.1".to_string(), |l| l.target_host.clone()),
                "127.0.0.1",
            ),
            target_port: input(
                link.as_ref()
                    .map(|l| l.target_port.to_string())
                    .unwrap_or_default(),
                "3000",
            ),
            store,
            id,
            error: None,
            saving: false,
        }
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        self.store
            .update(cx, |store, cx| store.navigate(Route::Links, cx));
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let value = |input: &Entity<InputState>| input.read(cx).value().trim().to_string();
        let Some(target_port) = value(&self.target_port)
            .parse::<i32>()
            .ok()
            .filter(|port| (1..=65535).contains(port))
        else {
            self.error = Some(t("Port must be a number from 1 to 65535"));
            cx.notify();
            return;
        };
        let request = LinkWriteRequest {
            subdomain: value(&self.subdomain),
            target_host: value(&self.target_host),
            target_port,
        };
        self.error = None;
        self.saving = true;
        cx.notify();
        let (app, id) = (self.store.read(cx).app(), self.id.clone());
        cx.spawn(async move |this, cx| {
            let saved = backend(async move {
                match id {
                    Some(id) => crate::store::links::link_update(id, request, app.state()).await,
                    None => crate::store::links::link_create(request, app.state()).await,
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

impl Render for LinkForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let saving = self.saving;
        common::form_page(
            if self.id.is_some() {
                t("Edit link")
            } else {
                t("New link")
            },
            self.error.clone(),
            v_flex()
                .gap_4()
                .child(
                    common::form_section("globe", t("Public name"), None).child(common::field(
                        t("Name"),
                        Input::new(&self.subdomain).disabled(saving),
                        None,
                    )),
                )
                .child(
                    common::form_section("network", t("Local service"), None).child(
                        h_flex()
                            .items_start()
                            .gap_3()
                            .child(
                                common::field(
                                    t("Target host"),
                                    Input::new(&self.target_host).disabled(saving),
                                    None,
                                )
                                .flex_1(),
                            )
                            .child(
                                common::field(
                                    t("Target port"),
                                    Input::new(&self.target_port).disabled(saving),
                                    None,
                                )
                                .w(px(112.)),
                            ),
                    ),
                ),
            common::form_buttons(
                if saving {
                    t("Saving…")
                } else {
                    t("Save link")
                },
                saving,
                cx.listener(|this, _, _, cx| this.back(cx)),
                cx.listener(|this, _, _, cx| this.save(cx)),
            ),
        )
    }
}
