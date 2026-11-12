//! The window: sidebar, session tabs, the current page, and the status bar.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use shared::SnippetRecord;

use crate::ui::i18n::{t, tf};
use crate::ui::icons::{lucide, pin_icon};
use crate::ui::stencil::{session_stencil, Stencil};
use crate::ui::store::{store, Route, Session, SessionStatus, Store};
use crate::ui::terminal::SessionKind;
use crate::ui::theme;
use crate::ui::views;

actions!(
    noverterm,
    [OpenSettings, ShowTerminal, CloseTab, NextTab, PreviousTab]
);

/// Jumps to the tab at this index.
#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = noverterm, no_json)]
pub struct SelectTab(pub usize);

pub fn bind_keys(cx: &mut App) {
    // Cmd on macOS; Ctrl elsewhere.
    let modifier = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    // Ctrl+T and Ctrl+W belong to the shell and to text fields on other platforms.
    let outside_text = (!cfg!(target_os = "macos")).then_some("!Terminal && !Input");
    let key = |keys: &str| -> &'static str { format!("{modifier}-{keys}").leak() };
    let mut bindings = vec![
        KeyBinding::new(key(","), OpenSettings, None),
        KeyBinding::new(key("t"), ShowTerminal, outside_text),
        KeyBinding::new(key("w"), CloseTab, outside_text),
        KeyBinding::new(key("shift-["), PreviousTab, None),
        KeyBinding::new(key("shift-]"), NextTab, None),
        KeyBinding::new("ctrl-tab", NextTab, None),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, None),
    ];
    for index in 0..9 {
        let digit = (index + 1).to_string();
        bindings.push(KeyBinding::new(key(&digit), SelectTab(index), None));
    }
    cx.bind_keys(bindings);
}

/// A session tab being dragged to a new position.
#[derive(Clone)]
struct DraggedTab {
    id: String,
    name: SharedString,
}

/// A pinned shortcut being dragged to a new position.
#[derive(Clone)]
struct DraggedShortcut {
    id: String,
    name: SharedString,
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

pub struct Shell {
    store: Entity<Store>,
    /// The page for the current route; rebuilt when the route changes.
    page: Option<(Route, AnyView)>,
    /// SFTP panes stay connected while the user visits other pages.
    sftp: Option<AnyView>,
    /// Lives for the whole window: status line, approval dialog, update prompt.
    overlays: views::Overlays,
    /// The tab the terminal last had focus for.
    focused_session: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl Shell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let changes = cx.observe_in(&store, window, |this, _, window, cx| {
            this.focus_active_terminal(window, cx);
            cx.notify();
        });
        Self {
            overlays: views::Overlays::new(window, cx),
            store,
            page: None,
            sftp: None,
            focused_session: None,
            _subscriptions: vec![changes],
        }
    }

    /// Hands the keyboard to the terminal when its tab comes to the front.
    fn focus_active_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let shown = (store.route == Route::Terminal)
            .then(|| store.active())
            .flatten()
            .and_then(|session| Some((session.id.clone(), session.terminal.clone()?)));
        let shown_id = shown.as_ref().map(|(id, _)| id.clone());
        if shown_id != self.focused_session {
            self.focused_session = shown_id;
            if let Some((_, terminal)) = shown {
                terminal.focus_handle(cx).focus(window, cx);
            }
        }
    }

    fn page(&mut self, route: &Route, window: &mut Window, cx: &mut Context<Self>) -> AnyView {
        if *route == Route::Sftp {
            return self
                .sftp
                .get_or_insert_with(|| views::build(route, window, cx))
                .clone();
        }
        match &self.page {
            Some((shown, view)) if shown == route => view.clone(),
            _ => {
                let view = views::build(route, window, cx);
                self.page = Some((route.clone(), view.clone()));
                view
            }
        }
    }

    fn select_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        self.store.update(cx, |store, cx| {
            if let Some(id) = store.sessions.get(index).map(|session| session.id.clone()) {
                store.activate_session(&id, cx);
            }
        });
    }

    fn cycle_tab(&mut self, offset: isize, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let count = store.sessions.len() as isize;
        if count == 0 {
            return;
        }
        let current = store
            .active_session
            .as_deref()
            .and_then(|id| store.sessions.iter().position(|session| session.id == id))
            .unwrap_or(0) as isize;
        self.select_tab(((current + offset + count) % count) as usize, cx);
    }

    // --- Sidebar ------------------------------------------------------------

    fn nav_item(
        &self,
        id: &'static str,
        label: SharedString,
        icon: &'static str,
        route: Route,
        active: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let store = self.store.clone();
        rail_button(id, active)
            .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
            .on_click(cx.listener(move |_, _, _, cx| {
                store.update(cx, |store, cx| store.navigate(route.clone(), cx));
            }))
            .child(lucide(icon).size_3p5())
    }

    fn render_sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        let route = store.route.clone();
        let with_count = |label: SharedString, count: usize| -> SharedString {
            format!("{label} ({count})").into()
        };
        let connections = with_count(t("Connections"), store.hosts.len());
        let keys = with_count(t("Keys"), store.keys.len());
        let forwards = with_count(t("Forwards"), store.forwards.len());
        let pinned = store.pinned_snippets();
        let store_handle = self.store.clone();

        let section = |route: &Route| match route {
            Route::Connections | Route::ConnectionForm(_) => "hosts",
            Route::Keys | Route::KeyForm(_) => "keys",
            Route::Forwards | Route::ForwardForm(_) => "forwards",
            Route::KnownHosts => "known-hosts",
            Route::Snippets | Route::SnippetForm { .. } => "snippets",
            Route::Sftp => "sftp",
            Route::Mcp => "mcp",
            Route::Terminal => "terminal",
        };
        let active = section(&route);

        v_flex()
            .w(px(56.))
            .h_full()
            .flex_shrink_0()
            .bg(theme::RAIL)
            .border_r_1()
            .border_color(theme::RIB)
            .child(
                h_flex().justify_center().px_2().pt_3().pb_2().child(
                    div()
                        .id("home")
                        .size_10()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(theme::RADIUS)
                        .border_1()
                        .border_color(theme::RIB)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::wash()))
                        .tooltip(|window, cx| {
                            Tooltip::new(t("Go to Connections")).build(window, cx)
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.store.update(cx, |store, cx| {
                                store.set_active(None, cx);
                                store.navigate(Route::Connections, cx);
                            });
                        }))
                        .child(img("icon.png").size_7().rounded(theme::RADIUS)),
                ),
            )
            .child(
                v_flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_3()
                    .child(self.nav_item(
                        "nav-hosts",
                        connections,
                        "server",
                        Route::Connections,
                        active == "hosts",
                        cx,
                    ))
                    .child(self.nav_item(
                        "nav-sftp",
                        "SFTP".into(),
                        "folder-open",
                        Route::Sftp,
                        active == "sftp",
                        cx,
                    ))
                    .child(self.nav_item(
                        "nav-keys",
                        keys,
                        "key-round",
                        Route::Keys,
                        active == "keys",
                        cx,
                    ))
                    .child(self.nav_item(
                        "nav-snippets",
                        t("Snippets"),
                        "file-text",
                        Route::Snippets,
                        active == "snippets",
                        cx,
                    ))
                    .child(self.nav_item(
                        "nav-forwards",
                        forwards,
                        "network",
                        Route::Forwards,
                        active == "forwards",
                        cx,
                    ))
                    .child(self.nav_item(
                        "nav-known-hosts",
                        t("Known Hosts"),
                        "server",
                        Route::KnownHosts,
                        active == "known-hosts",
                        cx,
                    ))
                    .child(self.nav_item(
                        "nav-mcp",
                        "MCP".into(),
                        "plug",
                        Route::Mcp,
                        active == "mcp",
                        cx,
                    )),
            )
            .child(div().mx_3().h(px(1.)).flex_shrink_0().bg(theme::RIB))
            .child(
                v_flex()
                    .id("shortcuts")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_3()
                    .child(
                        rail_button("local-terminal", active == "terminal")
                            .tooltip(|window, cx| {
                                Tooltip::new(t("Local terminal")).build(window, cx)
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.store
                                    .update(cx, |store, cx| store.connect_local(None, None, cx));
                            }))
                            .child(lucide("terminal").size_3p5()),
                    )
                    .children(
                        pinned
                            .iter()
                            .map(|snippet| self.render_shortcut(snippet, &pinned, cx)),
                    )
                    .child(
                        div()
                            .id("add-shortcut")
                            .h_7()
                            .w_10()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(theme::RADIUS)
                            .border_1()
                            .border_dashed()
                            .border_color(theme::hairline())
                            .text_color(theme::STENCIL_MUTE)
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme::STENCIL))
                            .tooltip(|window, cx| Tooltip::new(t("Add shortcut")).build(window, cx))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.store.update(cx, |store, cx| {
                                    store.navigate(
                                        Route::SnippetForm {
                                            id: None,
                                            shortcut: true,
                                        },
                                        cx,
                                    );
                                });
                            }))
                            .child(lucide("plus").size_3p5()),
                    ),
            )
            .child(
                h_flex()
                    .justify_center()
                    .p_2()
                    .border_t_1()
                    .border_color(theme::RIB)
                    .child(
                        rail_button("settings", false)
                            .border_color(gpui_kit::transparent_black())
                            .tooltip(|window, cx| Tooltip::new(t("Settings")).build(window, cx))
                            .on_click(move |_, _, cx| {
                                store_handle.update(cx, |store, cx| {
                                    store.show_settings = true;
                                    cx.notify();
                                });
                            })
                            .child(lucide("settings").size_3p5()),
                    ),
            )
    }

    fn render_shortcut(
        &self,
        snippet: &SnippetRecord,
        pinned: &[SnippetRecord],
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let label: SharedString = format!("{} · {}", snippet.title, snippet.host_name).into();
        let ids: Vec<String> = pinned.iter().map(|pinned| pinned.id.clone()).collect();
        let (launch, edit, unpin, target) = (
            snippet.clone(),
            snippet.id.clone(),
            snippet.clone(),
            snippet.id.clone(),
        );
        let (store, menu_store) = (self.store.clone(), self.store.clone());
        let title: SharedString = snippet.title.clone().into();
        let tooltip = label.clone();
        rail_button(
            SharedString::from(format!("shortcut-{}", snippet.id)),
            false,
        )
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.store
                .update(cx, |store, cx| store.launch_pinned_snippet(&launch, cx));
        }))
        .on_drag(
            DraggedShortcut {
                id: snippet.id.clone(),
                name: label,
            },
            |dragged, _, _, cx| cx.new(|_| DragChip(dragged.name.clone())),
        )
        .drag_over::<DraggedShortcut>(|style, _, _, _| style.border_color(theme::STENCIL))
        .on_drop(move |dragged: &DraggedShortcut, _, cx| {
            // Drop onto a shortcut to take its place.
            let mut ids = ids.clone();
            let from = ids.iter().position(|id| *id == dragged.id);
            let to = ids.iter().position(|id| *id == target);
            if let (Some(from), Some(to)) = (from, to) {
                let moved = ids.remove(from);
                ids.insert(to, moved);
                store.update(cx, |store, cx| store.reorder_pinned_snippets(ids, cx));
            }
        })
        .child(pin_icon(snippet.pin_icon.as_deref().unwrap_or_default()))
        .context_menu(move |menu, _, _| {
            let (edit_store, unpin_store) = (menu_store.clone(), menu_store.clone());
            let (edit, unpin) = (edit.clone(), unpin.clone());
            menu.label(title.clone())
                .separator()
                .item(
                    PopupMenuItem::new(t("Edit shortcut"))
                        .icon(lucide("pencil"))
                        .on_click(move |_, _, cx| {
                            let route = Route::SnippetForm {
                                id: Some(edit.clone()),
                                shortcut: false,
                            };
                            edit_store.update(cx, |store, cx| store.navigate(route, cx));
                        }),
                )
                .item(
                    PopupMenuItem::new(t("Remove from panel"))
                        .icon(lucide("pin-off"))
                        .on_click(move |_, _, cx| {
                            unpin_store.update(cx, |store, cx| store.unpin_snippet(&unpin, cx));
                        }),
                )
        })
    }

    // --- Tabs ---------------------------------------------------------------

    fn render_tab(
        &self,
        index: usize,
        session: &Session,
        stencil: &Stencil,
        ai_allowed: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let store = self.store.read(cx);
        let count = store.sessions.len();
        let active = store.route == Route::Terminal
            && store.active_session.as_deref() == Some(session.id.as_str());
        let id = session.id.clone();
        let name: SharedString = session.name.clone().into();
        let label: SharedString = format!("{}·{}", stencil.prefix, session.name)
            .to_uppercase()
            .into();
        let fill = if active {
            stencil.fill
        } else {
            // 55% line colour over the tab rib, like `color-mix`.
            let mix = |line: f32, rib: f32| line * 0.55 + rib * 0.45;
            Rgba {
                r: mix(stencil.fill.r, theme::RIB.r),
                g: mix(stencil.fill.g, theme::RIB.g),
                b: mix(stencil.fill.b, theme::RIB.b),
                a: 1.,
            }
        };
        let text = if active && stencil.dark_text {
            theme::PORT
        } else {
            theme::STENCIL
        };
        let can_duplicate = session.kind == SessionKind::Local || session.connection_id.is_some();
        let ids: Vec<String> = store
            .sessions
            .iter()
            .map(|session| session.id.clone())
            .collect();
        let (click_id, close_id, drop_id) = (id.clone(), id.clone(), id.clone());
        let (drop_store, menu_store) = (self.store.clone(), self.store.clone());
        let group: SharedString = format!("tab-{id}").into();

        h_flex()
            .id(SharedString::from(format!("tab-{id}")))
            .group(group.clone())
            .h_7()
            .min_w(px(176.))
            .max_w(px(240.))
            .flex_1()
            .flex_basis(px(0.))
            .gap_2()
            .px_2p5()
            .rounded(theme::RADIUS)
            .border_1()
            .border_color(if active {
                theme::STENCIL
            } else {
                theme::hairline()
            })
            .bg(fill)
            .text_color(text)
            .text_size(px(11.))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store
                    .update(cx, |store, cx| store.activate_session(&click_id, cx));
            }))
            .on_drag(
                DraggedTab {
                    id: id.clone(),
                    name: name.clone(),
                },
                |dragged, _, _, cx| cx.new(|_| DragChip(dragged.name.clone())),
            )
            .drag_over::<DraggedTab>(|style, _, _, _| style.border_color(theme::STENCIL))
            .on_drop(move |dragged: &DraggedTab, _, cx| {
                drop_store.update(cx, |store, cx| {
                    store.reorder_sessions(&dragged.id, &drop_id, cx)
                });
            })
            .child(div().flex_1().min_w_0().truncate().child(label))
            .when(ai_allowed, |tab| {
                tab.child(
                    div()
                        .id(SharedString::from(format!("tab-ai-{id}")))
                        .tooltip(|window, cx| {
                            Tooltip::new(t("AI agents run commands here without asking"))
                                .build(window, cx)
                        })
                        .child(lucide("bot").size_3p5()),
                )
            })
            .when(session.status.is_held(), |tab| {
                tab.child(
                    div()
                        .px_1()
                        .bg(theme::STENCIL)
                        .text_color(theme::SEAL)
                        .line_height(px(16.))
                        .child("HELD"),
                )
            })
            .when(
                !session.status.is_held() && session.status != SessionStatus::Connected,
                |tab| tab.child(div().size_2().rounded_full().border_2().border_color(text)),
            )
            .child(
                div()
                    .id(SharedString::from(format!("tab-close-{id}")))
                    .size_5()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .rounded(theme::RADIUS)
                    .when(!active, |close| {
                        close
                            .invisible()
                            .group_hover(group, |style| style.visible())
                    })
                    .hover(|style| style.bg(gpui_kit::black().opacity(0.25)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.store.update(cx, |store, cx| {
                            store.close_sessions(std::slice::from_ref(&close_id), cx)
                        });
                    }))
                    .child(lucide("x").size_3()),
            )
            .context_menu(move |menu, _, _| {
                let close = |label: SharedString, targets: Vec<String>, enabled: bool| {
                    let store = menu_store.clone();
                    PopupMenuItem::new(label)
                        .disabled(!enabled)
                        .on_click(move |_, _, cx| {
                            store.update(cx, |store, cx| store.close_sessions(&targets, cx));
                        })
                };
                let (duplicate_store, duplicate_id) = (menu_store.clone(), id.clone());
                menu.label(name.clone())
                    .separator()
                    .item(close(t("Close"), vec![id.clone()], true))
                    .item(
                        PopupMenuItem::new(t("Duplicate Tab"))
                            .disabled(!can_duplicate)
                            .on_click(move |_, _, cx| {
                                duplicate_store.update(cx, |store, cx| {
                                    store.duplicate_session(&duplicate_id, cx)
                                });
                            }),
                    )
                    .item(close(
                        t("Close Tabs to the Left"),
                        ids[..index].to_vec(),
                        index > 0,
                    ))
                    .item(close(
                        t("Close Tabs to the Right"),
                        ids[index + 1..].to_vec(),
                        index + 1 < count,
                    ))
                    .separator()
                    .item(close(t("Close All Tabs"), ids.clone(), true))
            })
    }

    fn render_tab_rib(&self, cx: &Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        let page_chip = match &store.route {
            Route::Connections | Route::ConnectionForm(_) => Some((t("Connections"), "server")),
            Route::Keys | Route::KeyForm(_) => Some((t("Keys"), "key-round")),
            Route::Forwards | Route::ForwardForm(_) => Some((t("Forwards"), "network")),
            Route::KnownHosts => Some((t("Known Hosts"), "server")),
            Route::Snippets | Route::SnippetForm { .. } => Some((t("Snippets"), "file-text")),
            Route::Sftp => Some(("SFTP".into(), "folder-open")),
            Route::Mcp => Some(("MCP".into(), "plug")),
            Route::Terminal => None,
        };
        let local_ai = views::local_terminal_ai_allowed(cx);
        let tabs: Vec<AnyElement> = store
            .sessions
            .iter()
            .enumerate()
            .map(|(index, session)| {
                let stencil = session_stencil(session, store);
                let ai_allowed = match session.kind {
                    SessionKind::Local => local_ai,
                    SessionKind::Ssh => session
                        .connection_id
                        .as_deref()
                        .and_then(|id| store.host(id))
                        .is_some_and(|host| host.ai_access == shared::AiAccess::Allow),
                };
                self.render_tab(index, session, &stencil, ai_allowed, cx)
                    .into_any_element()
            })
            .collect();

        h_flex()
            .h_9()
            .flex_shrink_0()
            .bg(theme::RIB)
            .border_b_1()
            .border_color(theme::RIB)
            .when_some(page_chip, |rib, (label, icon)| {
                rib.child(
                    h_flex()
                        .h_full()
                        .flex_shrink_0()
                        .px_2()
                        .bg(theme::RAIL)
                        .border_r_1()
                        .border_color(theme::RIB)
                        .child(
                            h_flex()
                                .h_7()
                                .gap_2()
                                .px_2p5()
                                .rounded(theme::RADIUS)
                                .border_1()
                                .border_color(theme::STENCIL)
                                .text_size(px(11.))
                                .text_color(theme::STENCIL)
                                .child(lucide(icon).size_3p5().text_color(theme::STENCIL_DIM))
                                .child(SharedString::from(label.to_uppercase())),
                        ),
                )
            })
            .child(
                h_flex()
                    .id("session-tabs")
                    .h_full()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .px_1p5()
                    .overflow_x_scroll()
                    // Tabs recede while another page is in front.
                    .when(store.route != Route::Terminal, |tabs| tabs.opacity(0.45))
                    .children(tabs),
            )
    }

    // --- Terminal area ------------------------------------------------------

    fn render_terminal_area(&self, cx: &Context<Self>) -> AnyElement {
        let store = self.store.read(cx);
        let Some(session) = store.active() else {
            return centered()
                .child(
                    v_flex()
                        .max_w(px(448.))
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .text_2xl()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(t("No terminal selected")),
                        )
                        .child(div().text_sm().text_color(theme::STENCIL_MUTE).child(t(
                            "Open a local terminal or choose an active session from the sidebar.",
                        ))),
                )
                .into_any_element();
        };
        let stencil = session_stencil(session, store);
        let code: SharedString = format!("{}·{}", stencil.prefix, session.name)
            .to_uppercase()
            .into();
        let saved = session.connection_id.is_some();
        let id = session.id.clone();
        let close = cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.store.update(cx, |store, cx| {
                store.close_sessions(std::slice::from_ref(&id), cx)
            });
        });
        let retry = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.store.update(cx, |store, cx| store.retry_active(cx));
        });
        let held_badge = || {
            div()
                .px_1()
                .bg(theme::STENCIL)
                .text_color(theme::SEAL)
                .line_height(px(16.))
                .child("HELD")
        };
        let code_line = |code: SharedString| {
            h_flex()
                .justify_center()
                .gap_2()
                .text_size(px(11.))
                .child(code)
                .child(held_badge())
        };
        let card = |accent: Rgba| {
            v_flex()
                .w_full()
                .max_w(px(512.))
                .p_8()
                .gap_4()
                .rounded(theme::RADIUS)
                .border_1()
                .border_color(accent.opacity(0.6))
                .bg(theme::RAIL)
        };

        match session.status {
            SessionStatus::Connected => match &session.terminal {
                Some(terminal) => terminal.clone().into_any_element(),
                None => div().into_any_element(),
            },
            SessionStatus::Connecting => centered()
                .gap_4()
                .child(Spinner::new().large().color(theme::STENCIL.into()))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(tf("Connecting to {name}", &[("name", &session.name)])),
                )
                .into_any_element(),
            SessionStatus::Disconnected => centered()
                .child(
                    card(theme::SEAL)
                        .items_center()
                        .child(code_line(code))
                        .child(
                            div()
                                .text_xl()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(t("Terminal disconnected")),
                        )
                        .child(
                            h_flex()
                                .gap_3()
                                .pt_2()
                                .when(saved, |row| {
                                    row.child(
                                        Button::new("retry")
                                            .danger()
                                            .label(t("Retry session"))
                                            .on_click(retry),
                                    )
                                })
                                .child(
                                    Button::new("close-tab")
                                        .outline()
                                        .label(t("Close tab"))
                                        .on_click(close),
                                ),
                        ),
                )
                .into_any_element(),
            SessionStatus::TrustRequired => {
                let Some(prompt) = session.trust_prompt.clone() else {
                    return div().into_any_element();
                };
                let confirming = store.trust_confirming;
                let fact = |label: SharedString, value: String| {
                    h_flex()
                        .justify_between()
                        .gap_4()
                        .px_4()
                        .py_3()
                        .rounded(theme::RADIUS)
                        .border_1()
                        .border_color(theme::hairline())
                        .bg(theme::RIB)
                        .text_sm()
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_color(theme::STENCIL_MUTE)
                                .child(label),
                        )
                        .child(div().min_w_0().child(value))
                };
                centered()
                    .child(
                        v_flex()
                            .w_full()
                            .max_w(px(576.))
                            .rounded(theme::RADIUS)
                            .border_1()
                            .border_color(theme::AMBER.opacity(0.6))
                            .bg(theme::RAIL)
                            .child(
                                h_flex()
                                    .gap_4()
                                    .px_6()
                                    .py_5()
                                    .border_b_1()
                                    .border_color(theme::hairline())
                                    .child(
                                        div()
                                            .size_11()
                                            .flex()
                                            .flex_shrink_0()
                                            .items_center()
                                            .justify_center()
                                            .rounded(theme::RADIUS)
                                            .border_1()
                                            .border_color(theme::AMBER.opacity(0.6))
                                            .text_color(theme::AMBER_TEXT)
                                            .child(lucide("circle-alert").size_5()),
                                    )
                                    .child(
                                        div()
                                            .text_xl()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(t("Verify SSH host identity")),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .gap_3()
                                    .px_6()
                                    .py_5()
                                    .child(fact(t("Host"), format!("{}:{}", prompt.host, prompt.port)))
                                    .child(fact(t("Algorithm"), prompt.algorithm.clone()))
                                    .child(
                                        v_flex()
                                            .gap_3()
                                            .p_4()
                                            .rounded(theme::RADIUS)
                                            .border_1()
                                            .border_color(theme::hairline())
                                            .bg(theme::RIB)
                                            .text_sm()
                                            .child(
                                                div()
                                                    .text_color(theme::STENCIL_MUTE)
                                                    .child(t("Fingerprint")),
                                            )
                                            .child(
                                                div()
                                                    .px_3()
                                                    .py_2p5()
                                                    .rounded(theme::RADIUS)
                                                    .border_1()
                                                    .border_color(theme::hairline())
                                                    .bg(theme::RAIL)
                                                    .child(prompt.fingerprint.clone()),
                                            ),
                                    )
                                    .child(div().text_xs().text_color(theme::STENCIL_MUTE).child(t(
                                        "Only continue if this fingerprint matches the server you expect. It will be saved locally in Known Hosts.",
                                    ))),
                            )
                            .children(store.trust_error.clone().map(|error| error_note(error).mx_6().mb_4()))
                            .child(
                                h_flex()
                                    .justify_end()
                                    .gap_3()
                                    .px_6()
                                    .py_4()
                                    .border_t_1()
                                    .border_color(theme::hairline())
                                    .bg(theme::PORT)
                                    .when(saved, |row| {
                                        row.child(
                                            Button::new("trust")
                                                .warning()
                                                .label(t("Trust host and retry"))
                                                .loading(confirming)
                                                .disabled(confirming)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.store
                                                        .update(cx, |store, cx| store.trust_active_host(cx));
                                                })),
                                        )
                                    })
                                    .when(!saved, |row| {
                                        row.child(div().text_sm().text_color(theme::STENCIL_MUTE).child(t(
                                            "Save this connection first to trust the host and retry automatically.",
                                        )))
                                    })
                                    .child(Button::new("cancel").outline().label(t("Cancel")).on_click(close)),
                            ),
                    )
                    .into_any_element()
            }
            SessionStatus::Error => {
                let confirming = store.trust_confirming;
                let error = session
                    .error
                    .clone()
                    .map_or_else(|| t("Unknown error"), SharedString::from);
                let fingerprint = |label: SharedString, value: String, alarming: bool| {
                    v_flex()
                        .gap_1()
                        .text_xs()
                        .child(div().text_color(theme::STENCIL_MUTE).child(label))
                        .child(
                            div()
                                .px_3()
                                .py_2()
                                .rounded(theme::RADIUS)
                                .bg(theme::RAIL)
                                .text_color(if alarming {
                                    theme::RED_TEXT
                                } else {
                                    theme::STENCIL_DIM
                                })
                                .child(value),
                        )
                };
                centered()
                    .child(
                        card(theme::SEAL)
                            .child(code_line(code))
                            .child(
                                div()
                                    .text_xl()
                                    .text_center()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(t("Connection failed")),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_center()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(error),
                            )
                            .children(session.trust_mismatch.clone().map(|mismatch| {
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
                                    .child(fingerprint(t("Expected"), mismatch.expected_fingerprint, false))
                                    .child(fingerprint(t("Presented"), mismatch.presented_fingerprint, true))
                                    .child(div().text_xs().text_color(theme::STENCIL_MUTE).child(t(
                                        "This may indicate the server changed keys or a man-in-the-middle risk. Not updating trust automatically.",
                                    )))
                            }))
                            .children(store.trust_error.clone().map(error_note))
                            .child(
                                h_flex()
                                    .justify_center()
                                    .gap_3()
                                    .pt_2()
                                    .when(session.trust_mismatch.is_some() && saved, |row| {
                                        row.child(
                                            Button::new("replace-trust")
                                                .danger()
                                                .label(t("Delete & trust new key"))
                                                .loading(confirming)
                                                .disabled(confirming)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.store.update(cx, |store, cx| {
                                                        store.trust_new_host_key(cx)
                                                    });
                                                })),
                                        )
                                    })
                                    .when(saved, |row| {
                                        let retry_button = Button::new("retry")
                                            .label(t("Retry session"))
                                            .disabled(confirming)
                                            .on_click(retry);
                                        row.child(if session.trust_mismatch.is_some() {
                                            retry_button.outline()
                                        } else {
                                            retry_button.danger()
                                        })
                                    })
                                    .child(Button::new("close-tab").outline().label(t("Close tab")).on_click(close)),
                            ),
                    )
                    .into_any_element()
            }
        }
    }

    // --- Footer -------------------------------------------------------------

    fn render_footer(&self, cx: &Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        let held = store
            .sessions
            .iter()
            .filter(|session| session.status.is_held())
            .count();
        let count = store.sessions.len().to_string();
        let mut counts = tf("{count} open", &[("count", &count)]).to_string();
        if held > 0 {
            let held = held.to_string();
            counts = format!("{counts} · {}", tf("{count} held", &[("count", &held)]));
        }
        let session = store.active().map(|session| {
            let stencil = session_stencil(session, store);
            let target: SharedString = match session.kind {
                SessionKind::Local => t("local shell"),
                SessionKind::Ssh => {
                    format!("{}@{}:{}", session.username, session.host, session.port).into()
                }
            };
            (
                stencil.fill,
                SharedString::from(format!("{}·{}", stencil.prefix, session.name).to_uppercase()),
                target,
                session.status,
            )
        });

        h_flex()
            .h_6()
            .flex_shrink_0()
            .gap_5()
            .px_3()
            .bg(theme::RAIL)
            .border_t_1()
            .border_color(theme::RIB)
            .text_size(px(11.))
            .text_color(theme::STENCIL_MUTE)
            .map(|footer| match session {
                Some((fill, code, target, status)) => footer
                    .child(
                        h_flex()
                            .min_w_0()
                            .gap_2()
                            .text_color(theme::STENCIL)
                            .child(div().h_2().w_3().flex_shrink_0().bg(fill))
                            .child(div().truncate().child(code)),
                    )
                    .child(div().truncate().child(target))
                    .child(
                        div()
                            .when(status.is_held(), |label| label.text_color(theme::RED_TEXT))
                            .child(status.label()),
                    ),
                None => footer.child(t("No session selected")),
            })
            .child(div().flex_1())
            .child(self.overlays.status_line())
            .child(div().flex_shrink_0().child(counts))
    }
}

/// A 40x28 icon button in the sidebar rail.
fn rail_button(id: impl Into<ElementId>, active: bool) -> Stateful<Div> {
    div()
        .id(id)
        .h_7()
        .w_10()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(theme::RADIUS)
        .border_1()
        .cursor_pointer()
        .map(|button| {
            if active {
                button
                    .border_color(theme::STENCIL)
                    .text_color(theme::STENCIL)
            } else {
                button
                    .border_color(gpui_kit::transparent_black())
                    .text_color(theme::STENCIL_MUTE)
                    .hover(|style| {
                        style
                            .border_color(theme::hairline())
                            .bg(theme::wash())
                            .text_color(theme::STENCIL)
                    })
            }
        })
}

fn centered() -> Div {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .p_8()
        .bg(theme::PORT)
}

fn error_note(error: String) -> Div {
    div()
        .px_4()
        .py_3()
        .rounded(theme::RADIUS)
        .border_1()
        .border_color(theme::SEAL.opacity(0.6))
        .text_sm()
        .text_color(theme::RED_TEXT)
        .child(error)
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (loading, load_error, route) = {
            let store = self.store.read(cx);
            (store.loading, store.load_error.clone(), store.route.clone())
        };
        if loading {
            return centered()
                .gap_4()
                .child(Spinner::new().large().color(theme::STENCIL.into()))
                .child(
                    div()
                        .text_sm()
                        .text_color(theme::STENCIL_MUTE)
                        .child(t("Loading...")),
                )
                .into_any_element();
        }
        if let Some(error) = load_error {
            return centered()
                .child(
                    v_flex()
                        .w_full()
                        .max_w(px(672.))
                        .items_center()
                        .gap_4()
                        .p_8()
                        .rounded(theme::RADIUS)
                        .border_1()
                        .border_color(theme::hairline())
                        .bg(theme::RAIL)
                        .child(lucide("circle-alert").size_8().text_color(theme::RED_TEXT))
                        .child(
                            div()
                                .text_2xl()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(t("Backend connection unavailable")),
                        )
                        .child(div().text_sm().text_color(theme::STENCIL_DIM).child(error))
                        .child(Button::new("retry").outline().label(t("Retry")).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.store.update(cx, |store, cx| store.refresh(cx));
                            }),
                        )),
                )
                .into_any_element();
        }

        let content = if route == Route::Terminal {
            self.render_terminal_area(cx)
        } else {
            self.page(&route, window, cx).into_any_element()
        };

        h_flex()
            .size_full()
            .items_start()
            .bg(theme::PORT)
            .text_color(theme::STENCIL)
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| {
                this.store.update(cx, |store, cx| {
                    store.show_settings = true;
                    cx.notify();
                });
            }))
            .on_action(cx.listener(|this, _: &ShowTerminal, _, cx| {
                this.store
                    .update(cx, |store, cx| store.navigate(Route::Terminal, cx));
            }))
            .on_action(cx.listener(|this, _: &CloseTab, _, cx| {
                this.store.update(cx, |store, cx| {
                    if let Some(active) = store.active_session.clone() {
                        store.close_sessions(&[active], cx);
                    }
                });
            }))
            .on_action(cx.listener(|this, _: &NextTab, _, cx| this.cycle_tab(1, cx)))
            .on_action(cx.listener(|this, _: &PreviousTab, _, cx| this.cycle_tab(-1, cx)))
            .on_action(cx.listener(|this, action: &SelectTab, _, cx| this.select_tab(action.0, cx)))
            .child(self.render_sidebar(cx))
            .child(
                v_flex()
                    .h_full()
                    .flex_1()
                    .min_w_0()
                    .child(self.render_tab_rib(cx))
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .overflow_hidden()
                            .child(content),
                    )
                    .child(self.render_footer(cx)),
            )
            .children(self.overlays.layers(window, cx))
            .into_any_element()
    }
}
