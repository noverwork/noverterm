//! Saved SSH hosts, grouped: connect, edit, move between groups, import.

use std::future::Future;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Sizable as _, WindowExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use shared::{AiAccess, HostGroupRecord, SshHostAuthMaterial, SshHostRecord};

use crate::store::hosts::SaveConnectionInput;
use crate::store::ssh_config::SshConfigHost;
use crate::ui::bridge::backend;
use crate::ui::i18n::{t, tf};
use crate::ui::icons::lucide;
use crate::ui::stencil::{group_prefix, line_color};
use crate::ui::store::{store, Route, Store};
use crate::ui::theme;
use crate::ui::views::common;

/// Settings key holding the JSON list of collapsed section keys.
const COLLAPSED_SETTING: &str = "noverterm.connections.collapsedGroups";
const UNGROUPED_KEY: &str = "ungrouped";
const SSH_CONFIG_PATH: &str = "~/.ssh/config";

pub const AI_ACCESS_OPTIONS: [(AiAccess, &str); 3] = [
    (AiAccess::Allow, "Always allow"),
    (AiAccess::Ask, "Ask every time"),
    (AiAccess::Deny, "Deny"),
];

/// The password saved with a host, if any.
pub fn saved_password(host: &SshHostRecord) -> Option<String> {
    match &host.auth {
        Some(SshHostAuthMaterial::Password { password })
        | Some(SshHostAuthMaterial::PublicKeyAndPassword { password, .. }) => {
            Some(password.clone())
        }
        _ => None,
    }
}

/// A host card being dragged onto a group.
#[derive(Clone)]
struct DraggedHost {
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

pub struct ConnectionsView {
    store: Entity<Store>,
    search: Entity<InputState>,
    group_name: Entity<InputState>,
    creating_group: bool,
    saving_group: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl ConnectionsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(t("Search hosts")));
        let group_name = cx.new(|cx| InputState::new(window, cx).placeholder(t("Group name")));
        let subscriptions = vec![
            cx.observe(&store, |_, _, cx| cx.notify()),
            cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe(&group_name, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.create_group(cx);
                }
            }),
        ];
        Self {
            store,
            search,
            group_name,
            creating_group: false,
            saving_group: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    /// Runs a backend change and reloads the saved data; `done` runs only when
    /// it succeeded, a failure shows in the banner.
    fn change<T: Send + 'static>(
        &mut self,
        work: impl Future<Output = Result<T, String>> + Send + 'static,
        cx: &mut Context<Self>,
        done: impl FnOnce(&mut Self, &mut Context<Self>) + 'static,
    ) {
        self.error = None;
        cx.spawn(async move |this, cx| {
            let result = backend(work).await;
            let _ = this.update(cx, |this, cx| {
                this.saving_group = false;
                match result {
                    Ok(_) => done(this, cx),
                    Err(error) => this.error = Some(error.into()),
                }
                this.store.update(cx, |store, cx| store.refresh(cx));
                cx.notify();
            });
        })
        .detach();
    }

    fn connect(&mut self, id: &str, cx: &mut Context<Self>) {
        self.store
            .update(cx, |store, cx| store.connect_host(id, None, cx));
    }

    fn navigate(&mut self, route: Route, cx: &mut Context<Self>) {
        self.store.update(cx, |store, cx| store.navigate(route, cx));
    }

    fn collapsed(&self, cx: &App) -> Vec<String> {
        self.store
            .read(cx)
            .setting(COLLAPSED_SETTING)
            .and_then(|value| serde_json::from_str(value).ok())
            .unwrap_or_default()
    }

    fn toggle_collapsed(&mut self, key: &str, cx: &mut Context<Self>) {
        let mut collapsed = self.collapsed(cx);
        match collapsed.iter().position(|collapsed| collapsed == key) {
            Some(index) => {
                collapsed.remove(index);
            }
            None => collapsed.push(key.to_string()),
        }
        let value = serde_json::json!(collapsed).to_string();
        self.store.update(cx, |store, cx| {
            store.set_setting(COLLAPSED_SETTING, value, cx)
        });
    }

    fn move_to_group(&mut self, host_id: &str, group_id: Option<String>, cx: &mut Context<Self>) {
        let Some(host) = self.store.read(cx).host(host_id).cloned() else {
            return;
        };
        if host.group_id == group_id {
            return;
        }
        let input = SaveConnectionInput {
            password: saved_password(&host),
            id: Some(host.id),
            name: host.name,
            host: host.host,
            port: host.port,
            username: host.username,
            group_id,
            jump_host_id: host.jump_host_id,
            private_key: None,
            passphrase: None,
            key_name: None,
            existing_key_id: host.ssh_key_id,
            ai_access: None,
        };
        let app = self.store.read(cx).app();
        self.change(
            async move { crate::store::hosts::host_save(input, app.state()).await },
            cx,
            |_, _| {},
        );
    }

    fn set_ai_access(&mut self, host: &SshHostRecord, access: AiAccess, cx: &mut Context<Self>) {
        if host.ai_access == access {
            return;
        }
        let (app, id) = (self.store.read(cx).app(), host.id.clone());
        self.change(
            async move { crate::store::hosts::host_set_ai_access(id, access, app.state()).await },
            cx,
            |_, _| {},
        );
    }

    fn create_group(&mut self, cx: &mut Context<Self>) {
        let name = self.group_name.read(cx).value().trim().to_string();
        if name.is_empty() || self.saving_group {
            return;
        }
        self.saving_group = true;
        cx.notify();
        let app = self.store.read(cx).app();
        self.change(
            async move { crate::store::groups::host_group_create(name, app.state()).await },
            cx,
            |this, _| this.creating_group = false,
        );
    }

    fn request_delete_group(
        &mut self,
        group: &HostGroupRecord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (view, id) = (cx.entity(), group.id.clone());
        common::confirm_delete(
            t("Delete host group?"),
            t("Hosts in this group will stay saved and move back to Ungrouped. This action cannot be undone."),
            Some(group.name.clone().into()),
            t("Delete group"),
            window,
            cx,
            move |_, cx| {
                let id = id.clone();
                view.update(cx, |this, cx| {
                    let app = this.store.read(cx).app();
                    this.change(
                        async move { crate::store::groups::host_group_delete(id, app.state()).await },
                        cx,
                        |_, _| {},
                    );
                });
            },
        );
    }

    fn request_delete(
        &mut self,
        host: &SshHostRecord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (view, id, key_id) = (cx.entity(), host.id.clone(), host.ssh_key_id.clone());
        common::confirm_delete(
            t("Delete saved connection?"),
            t("This removes the saved host profile and disconnects any sessions that were opened from it. This action cannot be undone."),
            Some(host.name.clone().into()),
            t("Delete connection"),
            window,
            cx,
            move |_, cx| {
                let (id, key_id) = (id.clone(), key_id.clone());
                view.update(cx, |this, cx| {
                    let (app, deleted) = (this.store.read(cx).app(), id.clone());
                    this.change(
                        async move { crate::store::hosts::host_delete(id, key_id, app.state()).await },
                        cx,
                        move |this, cx| {
                            this.store.update(cx, |store, cx| {
                                store.forget_recent_connection(&deleted, cx);
                                store.close_connection_sessions(&deleted, cx);
                            });
                        },
                    );
                });
            },
        );
    }

    fn open_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let import = cx.new(SshConfigImport::new);
        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .title(tf("Import from {path}", &[("path", SSH_CONFIG_PATH)]))
                .w(px(512.))
                .child(import.clone())
        });
    }

    fn import_button(&self, id: &'static str, label: SharedString, cx: &Context<Self>) -> Button {
        Button::new(id)
            .outline()
            .small()
            .icon(lucide("file-input"))
            .label(label)
            .on_click(cx.listener(|this, _, window, cx| this.open_import(window, cx)))
    }

    fn render_card(
        &self,
        host: &SshHostRecord,
        groups: &[HostGroupRecord],
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let (fill, dark_text) = line_color(host.group_id.as_deref().unwrap_or_default(), groups);
        let auth_label = match host.auth {
            Some(SshHostAuthMaterial::PublicKeyAndPassword { .. }) => t("Key + Password"),
            Some(SshHostAuthMaterial::PublicKey { .. }) => t("SSH Key"),
            _ => t("Password"),
        };
        let ai_mark = match host.ai_access {
            AiAccess::Allow => Some((
                "bot",
                theme::AMBER_TEXT,
                t("AI agents run commands on this host without asking."),
            )),
            AiAccess::Deny => Some((
                "bot-off",
                theme::STENCIL_FAINT,
                t("AI agents cannot use this host."),
            )),
            AiAccess::Ask => None,
        };
        let hover: SharedString = format!("host-{}", host.id).into();
        let view = cx.entity();
        let drag_view = view.clone();
        let (connect, edit, delete, menu_host) =
            (host.id.clone(), host.id.clone(), host.clone(), host.clone());
        let mut menu_groups: Vec<HostGroupRecord> = groups.to_vec();
        menu_groups.sort_by_key(|group| group.name.to_lowercase());
        let icon_button = |id: &'static str, icon: &'static str, label: SharedString| {
            Button::new(SharedString::from(format!("{id}-{}", host.id)))
                .ghost()
                .small()
                .icon(lucide(icon))
                .tooltip(label)
        };

        common::card(hover.clone())
            .group(hover.clone())
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.connect(&connect, cx)))
            .on_drag(
                DraggedHost {
                    id: host.id.clone(),
                    name: host.name.clone().into(),
                },
                move |dragged, _, _, cx| {
                    // Redraw so the drop targets show while the drag is live.
                    drag_view.update(cx, |_, cx| cx.notify());
                    cx.new(|_| DragChip(dragged.name.clone()))
                },
            )
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(
                        div()
                            .size_10()
                            .flex()
                            .flex_shrink_0()
                            .items_center()
                            .justify_center()
                            .rounded(theme::RADIUS)
                            .bg(fill)
                            .text_color(if dark_text {
                                theme::PORT
                            } else {
                                theme::STENCIL
                            })
                            .child(lucide("server").size_5()),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                h_flex()
                                    .min_w_0()
                                    .gap_1p5()
                                    .child(
                                        div()
                                            .truncate()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(host.name.clone()),
                                    )
                                    .children(ai_mark.map(|(icon, color, hint)| {
                                        div()
                                            .id(SharedString::from(format!("ai-{}", host.id)))
                                            .flex_shrink_0()
                                            .text_color(color)
                                            .tooltip(move |window, cx| {
                                                Tooltip::new(hint.clone()).build(window, cx)
                                            })
                                            .child(lucide(icon).size_3p5())
                                    })),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .truncate()
                                    .text_xs()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(format!(
                                        "{}@{}:{}",
                                        host.username, host.host, host.port
                                    )),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(auth_label),
                            ),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .gap_1()
                            .invisible()
                            .group_hover(hover, |style| style.visible())
                            .child(
                                icon_button(
                                    "edit",
                                    "pencil",
                                    tf("Edit {name}", &[("name", &host.name)]),
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.navigate(
                                            Route::ConnectionForm(Some(edit.clone())),
                                            cx,
                                        );
                                    },
                                )),
                            )
                            .child(
                                icon_button(
                                    "delete",
                                    "trash",
                                    tf("Delete {name}", &[("name", &host.name)]),
                                )
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.request_delete(&delete, window, cx);
                                    },
                                )),
                            ),
                    ),
            )
            .context_menu(move |menu, window, cx| {
                let host = menu_host.clone();
                let (connect_view, edit_view, delete_view) =
                    (view.clone(), view.clone(), view.clone());
                let (connect, edit, delete) = (host.id.clone(), host.id.clone(), host.clone());
                let (group_view, group_host, groups) =
                    (view.clone(), host.clone(), menu_groups.clone());
                let (ai_view, ai_host) = (view.clone(), host.clone());
                menu.label(host.name.clone())
                    .separator()
                    .item(PopupMenuItem::new(t("Connect")).on_click(move |_, _, cx| {
                        connect_view.update(cx, |this, cx| this.connect(&connect, cx));
                    }))
                    .item(
                        PopupMenuItem::new(t("Edit"))
                            .icon(lucide("pencil"))
                            .on_click(move |_, _, cx| {
                                edit_view.update(cx, |this, cx| {
                                    this.navigate(Route::ConnectionForm(Some(edit.clone())), cx)
                                });
                            }),
                    )
                    .submenu_with_icon(
                        Some(lucide("folder-input")),
                        t("Move to group"),
                        window,
                        cx,
                        move |menu, _, _| {
                            let targets =
                                std::iter::once((None, t("Ungrouped"))).chain(groups.iter().map(
                                    |group| (Some(group.id.clone()), group.name.clone().into()),
                                ));
                            targets.fold(
                                menu,
                                |menu, (group_id, label): (Option<String>, SharedString)| {
                                    let (view, host_id) =
                                        (group_view.clone(), group_host.id.clone());
                                    menu.item(
                                        PopupMenuItem::new(label)
                                            .checked(group_host.group_id == group_id)
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.move_to_group(
                                                        &host_id,
                                                        group_id.clone(),
                                                        cx,
                                                    )
                                                });
                                            }),
                                    )
                                },
                            )
                        },
                    )
                    .submenu_with_icon(
                        Some(lucide("bot")),
                        t("MCP access"),
                        window,
                        cx,
                        move |menu, _, _| {
                            AI_ACCESS_OPTIONS
                                .into_iter()
                                .fold(menu, |menu, (access, label)| {
                                    let (view, host) = (ai_view.clone(), ai_host.clone());
                                    menu.item(
                                        PopupMenuItem::new(t(label))
                                            .checked(ai_host.ai_access == access)
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.set_ai_access(&host, access, cx)
                                                });
                                            }),
                                    )
                                })
                        },
                    )
                    .separator()
                    .item(
                        PopupMenuItem::new(t("Delete"))
                            .icon(lucide("trash"))
                            .on_click(move |_, window, cx| {
                                delete_view.update(cx, |this, cx| {
                                    this.request_delete(&delete, window, cx)
                                });
                            }),
                    )
            })
    }

    fn render_section(
        &self,
        group: Option<&HostGroupRecord>,
        hosts: Vec<&SshHostRecord>,
        groups: &[HostGroupRecord],
        collapsed: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let key = group
            .map_or(UNGROUPED_KEY, |group| group.id.as_str())
            .to_string();
        let hover: SharedString = format!("section-{key}").into();
        let (fill, _) = line_color(&key, groups);
        let badge = || {
            div()
                .px_1p5()
                .rounded(theme::RADIUS)
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::RIB)
                .text_size(px(11.))
        };
        let (toggle_key, drop_group) = (key.clone(), group.map(|group| group.id.clone()));
        let delete_group = group.cloned();

        v_flex()
            .group(hover.clone())
            .mb_2()
            .px_2()
            .pb_1()
            .rounded(theme::RADIUS)
            .border_1()
            .border_color(transparent_black())
            .drag_over::<DraggedHost>(|style, _, _, _| {
                style.bg(theme::wash()).border_color(theme::STENCIL)
            })
            .on_drop(cx.listener(move |this, dragged: &DraggedHost, _, cx| {
                this.move_to_group(&dragged.id, drop_group.clone(), cx);
            }))
            .child(
                h_flex()
                    .gap_1()
                    .py_2()
                    .border_b_1()
                    .border_color(theme::hairline())
                    .child(
                        h_flex()
                            .id(SharedString::from(format!("toggle-{key}")))
                            .min_w_0()
                            .gap_2()
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_collapsed(&toggle_key, cx)
                            }))
                            .child(
                                lucide(if collapsed {
                                    "chevron-right"
                                } else {
                                    "chevron-down"
                                })
                                .size_3p5()
                                .text_color(theme::STENCIL_MUTE),
                            )
                            .child(div().h_2().w_3().flex_shrink_0().bg(fill))
                            .child(
                                common::section_title(group.map_or_else(
                                    || t("Ungrouped"),
                                    |group| group.name.clone().into(),
                                ))
                                .truncate()
                                .text_color(theme::STENCIL_DIM),
                            )
                            .children(group.map(|group| {
                                badge()
                                    .text_color(theme::STENCIL_DIM)
                                    .child(group_prefix(&group.name))
                            }))
                            .child(
                                badge()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(hosts.len().to_string()),
                            ),
                    )
                    .children(delete_group.map(|group| {
                        div()
                            .ml_auto()
                            .invisible()
                            .group_hover(hover, |style| style.visible())
                            .child(
                                Button::new(SharedString::from(format!(
                                    "delete-group-{}",
                                    group.id
                                )))
                                .ghost()
                                .xsmall()
                                .icon(lucide("trash"))
                                .tooltip(tf("Delete group {name}", &[("name", &group.name)]))
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.request_delete_group(&group, window, cx)
                                    },
                                )),
                            )
                    })),
            )
            .when(!collapsed, |section| {
                if hosts.is_empty() {
                    section.child(
                        div()
                            .my_3()
                            .px_4()
                            .py_4()
                            .rounded(theme::RADIUS)
                            .border_1()
                            .border_dashed()
                            .border_color(theme::hairline())
                            .text_center()
                            .text_xs()
                            .text_color(theme::STENCIL_MUTE)
                            .child(if cx.has_active_drag() {
                                t("Drop here to move host")
                            } else {
                                t("No hosts in this group — drag one here")
                            }),
                    )
                } else {
                    section.child(
                        div().grid().grid_cols(3).gap_3().py_4().children(
                            hosts
                                .into_iter()
                                .map(|host| self.render_card(host, groups, cx)),
                        ),
                    )
                }
            })
    }

    fn render_new_group(&self, cx: &Context<Self>) -> AnyElement {
        if !self.creating_group {
            return h_flex()
                .id("new-group")
                .mt_2()
                .gap_1p5()
                .cursor_pointer()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme::STENCIL_MUTE)
                .hover(|style| style.text_color(theme::STENCIL))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.creating_group = true;
                    this.group_name.update(cx, |input, cx| {
                        input.set_value("", window, cx);
                        input.focus(window, cx);
                    });
                    cx.notify();
                }))
                .child(lucide("plus").size_3p5())
                .child(t("New group"))
                .into_any_element();
        }
        let saving = self.saving_group;
        h_flex()
            .mt_2()
            .gap_2()
            .child(
                div()
                    .w(px(144.))
                    .child(Input::new(&self.group_name).small().disabled(saving)),
            )
            .child(
                Button::new("add-group")
                    .primary()
                    .small()
                    .label(t("Add"))
                    .disabled(saving)
                    .on_click(cx.listener(|this, _, _, cx| this.create_group(cx))),
            )
            .child(
                Button::new("cancel-group")
                    .ghost()
                    .small()
                    .label(t("Cancel"))
                    .disabled(saving)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.creating_group = false;
                        cx.notify();
                    })),
            )
            .into_any_element()
    }
}

impl Render for ConnectionsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        let query = self.search.read(cx).value().trim().to_lowercase();
        let searching = !query.is_empty();
        let collapsed = self.collapsed(cx);

        let mut hosts: Vec<&SshHostRecord> = store
            .hosts
            .iter()
            .filter(|host| {
                !searching
                    || [&host.name, &host.host, &host.username]
                        .iter()
                        .any(|text| text.to_lowercase().contains(&query))
            })
            .collect();
        hosts.sort_by_key(|host| host.name.to_lowercase());
        let mut groups: Vec<&HostGroupRecord> = store.groups.iter().collect();
        groups.sort_by_key(|group| group.name.to_lowercase());

        let sections = groups
            .into_iter()
            .map(Some)
            .chain([None])
            .filter_map(|group| {
                let id = group.map(|group| group.id.as_str());
                let members: Vec<&SshHostRecord> = hosts
                    .iter()
                    .copied()
                    .filter(|host| host.group_id.as_deref() == id)
                    .collect();
                // A search hides empty sections; Ungrouped otherwise shows only
                // when it has hosts or a dragged host needs somewhere to land.
                let hidden =
                    members.is_empty() && (searching || (group.is_none() && !cx.has_active_drag()));
                (!hidden).then(|| {
                    let key = id.unwrap_or(UNGROUPED_KEY);
                    let collapsed =
                        !searching && collapsed.iter().any(|collapsed| collapsed == key);
                    self.render_section(group, members, &store.groups, collapsed, cx)
                })
            });

        let body = if store.hosts.is_empty() {
            common::empty_state(t("No saved connections yet"))
                .flex_col()
                .gap_3()
                .child(self.import_button(
                    "import-empty",
                    tf("Import from {path}", &[("path", SSH_CONFIG_PATH)]),
                    cx,
                ))
                .into_any_element()
        } else {
            v_flex()
                .when(searching && hosts.is_empty(), |list| {
                    list.child(common::empty_state(t("No connections match your search.")))
                })
                .children(sections)
                .when(!searching, |list| list.child(self.render_new_group(cx)))
                .into_any_element()
        };

        common::page(
            t("SSH Connections"),
            h_flex()
                .gap_2()
                .child(
                    div().w(px(192.)).child(
                        Input::new(&self.search)
                            .small()
                            .prefix(lucide("search").xsmall().text_color(theme::STENCIL_FAINT)),
                    ),
                )
                .child(self.import_button("import", t("Import"), cx))
                .child(
                    Button::new("add-connection")
                        .primary()
                        .small()
                        .icon(lucide("plus"))
                        .label(t("Add connection"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.navigate(Route::ConnectionForm(None), cx)
                        })),
                ),
            common::page_body("connections")
                .children(
                    self.error
                        .clone()
                        .map(|error| common::error_banner(error).mb_4()),
                )
                .child(body),
        )
    }
}

// --- Import from ~/.ssh/config -------------------------------------------------

/// The body of the import dialog: scans the config, then imports the ticked hosts.
struct SshConfigImport {
    store: Entity<Store>,
    hosts: Vec<SshConfigHost>,
    selected: Vec<String>,
    scanning: bool,
    importing: bool,
    error: Option<SharedString>,
}

impl SshConfigImport {
    fn new(cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let app = store.read(cx).app();
        cx.spawn(async move |this, cx| {
            let scanned =
                backend(
                    async move { crate::store::ssh_config::ssh_config_scan(app.state()).await },
                )
                .await;
            let _ = this.update(cx, |this, cx| {
                this.scanning = false;
                match scanned {
                    Ok(hosts) => {
                        this.selected = hosts
                            .iter()
                            .filter(|host| !host.exists)
                            .map(|host| host.alias.clone())
                            .collect();
                        this.hosts = hosts;
                    }
                    Err(error) => this.error = Some(error.into()),
                }
                cx.notify();
            });
        })
        .detach();
        Self {
            store,
            hosts: Vec::new(),
            selected: Vec::new(),
            scanning: true,
            importing: false,
            error: None,
        }
    }

    fn import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.importing = true;
        self.error = None;
        cx.notify();
        let (app, aliases, store) = (
            self.store.read(cx).app(),
            self.selected.clone(),
            self.store.clone(),
        );
        cx.spawn_in(window, async move |this, cx| {
            let imported = backend(async move {
                crate::store::ssh_config::ssh_config_import(aliases, app.state()).await
            })
            .await;
            // The list reloads even when the dialog was closed meanwhile.
            store.update(cx, |store, cx| store.refresh(cx));
            let _ = this.update_in(cx, |this, window, cx| {
                this.importing = false;
                match imported {
                    Ok(_) => window.close_dialog(cx),
                    Err(error) => this.error = Some(error.into()),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for SshConfigImport {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let note = |text: SharedString| div().text_sm().text_color(theme::STENCIL_MUTE).child(text);
        let body = if self.scanning {
            note(t("Reading config…")).into_any_element()
        } else if let Some(error) = self.error.clone() {
            div()
                .text_sm()
                .text_color(theme::RED_TEXT)
                .child(error)
                .into_any_element()
        } else if self.hosts.is_empty() {
            note(tf(
                "No hosts found in {path}.",
                &[("path", SSH_CONFIG_PATH)],
            ))
            .into_any_element()
        } else {
            let importing = self.importing;
            v_flex()
                .id("ssh-config-hosts")
                .max_h(px(360.))
                .overflow_y_scroll()
                .gap_1()
                .children(self.hosts.iter().map(|host| {
                    let alias = host.alias.clone();
                    let via = host
                        .jump_alias
                        .as_deref()
                        .map(|alias| tf(" via {alias}", &[("alias", alias)]))
                        .unwrap_or_default();
                    h_flex()
                        .gap_3()
                        .px_2()
                        .py_2()
                        .when(host.exists, |row| row.opacity(0.5))
                        .child(
                            Checkbox::new(SharedString::from(format!("import-{alias}")))
                                .checked(self.selected.contains(&alias))
                                .disabled(host.exists || importing)
                                .on_change(cx.listener(move |this, checked: &bool, _, cx| {
                                    this.selected.retain(|selected| *selected != alias);
                                    if *checked {
                                        this.selected.push(alias.clone());
                                    }
                                    cx.notify();
                                })),
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
                                        .child(host.alias.clone()),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_xs()
                                        .text_color(theme::STENCIL_MUTE)
                                        .child(format!(
                                            "{}@{}:{}{via}",
                                            host.username, host.host, host.port
                                        )),
                                ),
                        )
                        .when(host.exists, |row| {
                            row.child(
                                div()
                                    .text_xs()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(t("Already saved")),
                            )
                        })
                }))
                .into_any_element()
        };
        let count = match self.selected.len() {
            0 => String::new(),
            count => count.to_string(),
        };

        v_flex()
            .gap_4()
            .text_color(theme::STENCIL)
            .child(note(t("Key files are copied into Noverterm.")))
            .child(body)
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("import-cancel")
                            .outline()
                            .label(t("Cancel"))
                            .disabled(self.importing)
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    )
                    .child(
                        Button::new("import-confirm")
                            .primary()
                            .icon(lucide("file-input"))
                            .label(tf("Import {count}", &[("count", &count)]))
                            .loading(self.importing)
                            .disabled(self.importing || self.scanning || self.selected.is_empty())
                            .on_click(cx.listener(|this, _, window, cx| this.import(window, cx))),
                    ),
            )
    }
}
