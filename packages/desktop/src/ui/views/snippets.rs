//! Command snippets: the list, and the form to add or edit one.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::select::Select;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use shared::{SnippetRecord, SnippetWriteRequest};

use crate::ui::bridge::backend;
use crate::ui::i18n::{t, tf};
use crate::ui::icons::{lucide, pin_color, pin_icon, PIN_COLORS, PIN_ICON_NAMES};
use crate::ui::store::{store, Route, Store};
use crate::ui::theme;
use crate::ui::views::common::{self, Choice, ChoiceSelect};

// --- List --------------------------------------------------------------------

pub struct SnippetsView {
    store: Entity<Store>,
    search: Entity<InputState>,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl SnippetsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(t("Search snippets")));
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

    fn run(&mut self, snippet: &SnippetRecord, cx: &mut Context<Self>) {
        let missing = snippet
            .host_id
            .as_deref()
            .is_some_and(|host_id| self.store.read(cx).host(host_id).is_none());
        if missing {
            self.error = Some(tf(
                "Connection \"{name}\" not found. It may have been deleted.",
                &[("name", &snippet.host_name)],
            ));
            cx.notify();
            return;
        }
        self.error = None;
        let (host_id, body) = (snippet.host_id.clone(), snippet.body.clone());
        self.store.update(cx, |store, cx| {
            store.run_snippet(host_id.as_deref(), body, cx)
        });
    }

    fn edit(&mut self, id: &str, cx: &mut Context<Self>) {
        let route = Route::SnippetForm {
            id: Some(id.to_string()),
            shortcut: false,
        };
        self.store.update(cx, |store, cx| store.navigate(route, cx));
    }

    fn request_delete(
        &mut self,
        snippet: &SnippetRecord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (view, id) = (cx.entity(), snippet.id.clone());
        common::confirm_delete(
            t("Delete snippet?"),
            t("This action cannot be undone."),
            Some(snippet.title.clone().into()),
            t("Delete snippet"),
            window,
            cx,
            move |_, cx| view.update(cx, |this, cx| this.delete(id.clone(), cx)),
        );
    }

    fn delete(&mut self, id: String, cx: &mut Context<Self>) {
        let app = self.store.read(cx).app();
        cx.spawn(async move |this, cx| {
            let deleted =
                backend(
                    async move { crate::store::snippets::snippet_delete(id, app.state()).await },
                )
                .await;
            let _ = this.update(cx, |this, cx| {
                this.error = deleted.err().map(SharedString::from);
                this.store.update(cx, |store, cx| store.refresh(cx));
                cx.notify();
            });
        })
        .detach();
    }

    fn render_card(&self, snippet: &SnippetRecord, cx: &Context<Self>) -> impl IntoElement {
        let lines = snippet
            .body
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count();
        let count = lines.to_string();
        let line_count = if lines == 1 {
            tf("{count} line", &[("count", &count)])
        } else {
            tf("{count} lines", &[("count", &count)])
        };
        let first_line = snippet.body.lines().find(|line| !line.trim().is_empty());
        let preview: SharedString = match first_line {
            None => "—".into(),
            Some(line) if line.chars().count() > 96 => {
                format!("{}…", line.chars().take(96).collect::<String>()).into()
            }
            Some(line) => line.to_string().into(),
        };
        let full_script: SharedString = snippet.body.clone().into();
        let group: SharedString = format!("snippet-{}", snippet.id).into();
        let (run, copy, edit, delete) = (
            snippet.clone(),
            snippet.body.clone(),
            snippet.id.clone(),
            snippet.clone(),
        );
        let menu_snippet = snippet.clone();
        let view = cx.entity();
        let icon_button = |id: &'static str, icon: &'static str, label: SharedString| {
            Button::new(SharedString::from(format!("{id}-{}", snippet.id)))
                .ghost()
                .small()
                .icon(lucide(icon))
                .tooltip(label)
        };

        common::card(SharedString::from(format!("snippet-{}", snippet.id)))
            .group(group.clone())
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.run(&run, cx)))
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(common::icon_box("file-text"))
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
                                    .child(snippet.title.clone()),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_xs()
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(tf("Runs on {name}", &[("name", &snippet.host_name)])),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::STENCIL_MUTE)
                                    .child(line_count),
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
                                    "copy",
                                    "copy",
                                    tf("Copy {title}", &[("title", &snippet.title)]),
                                )
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()));
                                }),
                            )
                            .child(
                                icon_button(
                                    "edit",
                                    "pencil",
                                    tf("Edit {title}", &[("title", &snippet.title)]),
                                )
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
                                    tf("Delete {title}", &[("title", &snippet.title)]),
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
            .child(
                h_flex()
                    .id(SharedString::from(format!(
                        "snippet-preview-{}",
                        snippet.id
                    )))
                    .mt_3()
                    .gap_1()
                    .px_2()
                    .py_1p5()
                    .rounded(theme::RADIUS)
                    .bg(theme::RAIL)
                    .text_size(px(11.))
                    .text_color(theme::STENCIL_DIM)
                    // The whole script shows on hover.
                    .tooltip(move |window, cx| Tooltip::new(full_script.clone()).build(window, cx))
                    .child(div().text_color(theme::STENCIL_FAINT).child("$"))
                    .child(div().min_w_0().truncate().child(preview)),
            )
            .context_menu(move |menu, _, _| {
                let snippet = menu_snippet.clone();
                let item = |label: SharedString, icon: &'static str| {
                    PopupMenuItem::new(label).icon(lucide(icon))
                };
                let (run_view, edit_view, delete_view) = (view.clone(), view.clone(), view.clone());
                let (run, copy, edit, delete) = (
                    snippet.clone(),
                    snippet.body.clone(),
                    snippet.id.clone(),
                    snippet.clone(),
                );
                menu.label(snippet.title.clone())
                    .separator()
                    .item(item(t("Run"), "play").on_click(move |_, _, cx| {
                        run_view.update(cx, |this, cx| this.run(&run, cx));
                    }))
                    .item(item(t("Copy"), "copy").on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()));
                    }))
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

impl Render for SnippetsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        let query = self.search.read(cx).value().trim().to_lowercase();
        let visible: Vec<&SnippetRecord> = store
            .snippets
            .iter()
            .filter(|snippet| {
                query.is_empty()
                    || [&snippet.title, &snippet.host_name, &snippet.body]
                        .iter()
                        .any(|text| text.to_lowercase().contains(&query))
            })
            .collect();
        let body = if store.snippets.is_empty() {
            common::empty_state(t("No snippets saved yet")).into_any_element()
        } else if visible.is_empty() {
            common::empty_state(t("No snippets match your search.")).into_any_element()
        } else {
            div()
                .grid()
                .grid_cols(3)
                .gap_3()
                .children(
                    visible
                        .into_iter()
                        .map(|snippet| self.render_card(snippet, cx)),
                )
                .into_any_element()
        };

        common::page(
            t("Command Snippets"),
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
                    Button::new("add-snippet")
                        .primary()
                        .small()
                        .icon(lucide("plus"))
                        .label(t("Add snippet"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            let route = Route::SnippetForm {
                                id: None,
                                shortcut: false,
                            };
                            this.store.update(cx, |store, cx| store.navigate(route, cx));
                        })),
                ),
            common::page_body("snippets")
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

pub struct SnippetForm {
    store: Entity<Store>,
    /// The snippet being edited; `None` adds a new one.
    id: Option<String>,
    /// Editing a snippet that no longer exists.
    missing: bool,
    title: Entity<InputState>,
    body: Entity<TextareaState>,
    /// Empty value targets the local terminal.
    host: Entity<ChoiceSelect>,
    show_on_panel: bool,
    pin_icon: String,
    /// `None` keeps the default icon color.
    pin_color: Option<String>,
    error: Option<SharedString>,
    saving: bool,
}

impl SnippetForm {
    pub fn new(
        id: Option<String>,
        shortcut: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let store = store(cx);
        let (snippet, hosts) = {
            let store = store.read(cx);
            let snippet = id
                .as_deref()
                .and_then(|id| store.snippets.iter().find(|snippet| snippet.id == id))
                .cloned();
            let mut hosts = vec![Choice::new("", t("Local Terminal"))];
            hosts.extend(store.hosts.iter().map(|host| {
                Choice::new(host.id.clone(), format!("{} ({})", host.name, host.host))
            }));
            (snippet, hosts)
        };
        let title_value = snippet
            .as_ref()
            .map(|snippet| snippet.title.clone())
            .unwrap_or_default();
        let body_value = snippet
            .as_ref()
            .map(|snippet| snippet.body.clone())
            .unwrap_or_default();
        let host_value = snippet
            .as_ref()
            .and_then(|snippet| snippet.host_id.clone())
            .unwrap_or_default();
        Self {
            missing: id.is_some() && snippet.is_none(),
            title: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(tf("e.g. {example}", &[("example", "Restart Nginx")]))
                    .default_value(title_value)
            }),
            body: cx.new(|cx| {
                TextareaState::new(window, cx)
                    .rows(10)
                    .placeholder(tf(
                        "e.g. {example}",
                        &[("example", "sudo systemctl restart nginx")],
                    ))
                    .default_value(body_value)
            }),
            host: common::new_select(hosts, &host_value, window, cx),
            show_on_panel: snippet
                .as_ref()
                .map_or(shortcut, |snippet| snippet.pin_icon.is_some()),
            pin_color: snippet
                .as_ref()
                .and_then(|snippet| snippet.pin_color.clone()),
            pin_icon: snippet
                .and_then(|snippet| snippet.pin_icon)
                .unwrap_or_else(|| "terminal".to_string()),
            store,
            id,
            error: None,
            saving: false,
        }
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        self.store
            .update(cx, |store, cx| store.navigate(Route::Snippets, cx));
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let title = self.title.read(cx).value().trim().to_string();
        if title.is_empty() {
            self.error = Some(t("Title is required"));
            cx.notify();
            return;
        }
        let request = SnippetWriteRequest {
            host_id: common::selected(&self.host, cx).filter(|host| !host.is_empty()),
            title,
            body: self.body.read(cx).value().to_string(),
            pin_icon: self.show_on_panel.then(|| self.pin_icon.clone()),
            pin_color: self.show_on_panel.then(|| self.pin_color.clone()).flatten(),
        };
        self.error = None;
        self.saving = true;
        cx.notify();
        let (app, id) = (self.store.read(cx).app(), self.id.clone());
        cx.spawn(async move |this, cx| {
            let saved = backend(async move {
                match id {
                    Some(id) => {
                        crate::store::snippets::snippet_update(id, request, app.state()).await
                    }
                    None => crate::store::snippets::snippet_create(request, app.state()).await,
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

impl Render for SnippetForm {
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
                        .child(t("Snippet not found")),
                )
                .child(
                    Button::new("back")
                        .primary()
                        .label(t("Back to snippets"))
                        .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
                )
                .into_any_element();
        }
        let editing = self.id.is_some();
        let submit_label = match (self.saving, editing) {
            (true, true) => t("Updating…"),
            (true, false) => t("Saving…"),
            (false, true) => t("Update snippet"),
            (false, false) => t("Save snippet"),
        };
        let saving = self.saving;
        let icons = PIN_ICON_NAMES.map(|name| {
            let selected = self.pin_icon == name;
            div()
                .id(name)
                .size_10()
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::RADIUS)
                .border_1()
                .cursor_pointer()
                .map(|option| {
                    if selected {
                        option
                            .border_color(theme::STENCIL)
                            .bg(theme::STENCIL.opacity(0.06))
                            .text_color(theme::STENCIL)
                    } else {
                        option
                            .border_color(theme::hairline())
                            .text_color(theme::STENCIL_MUTE)
                            .hover(|style| style.text_color(theme::STENCIL))
                    }
                })
                .tooltip(move |window, cx| Tooltip::new(name).build(window, cx))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.pin_icon = name.to_string();
                    cx.notify();
                }))
                .child(pin_icon(name))
        });
        let color_swatch =
            |id: &'static str, value: Option<&'static str>, cx: &mut Context<Self>| {
                let selected = self.pin_color.as_deref() == value;
                div()
                    .id(id)
                    .size_10()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(theme::RADIUS)
                    .border_1()
                    .cursor_pointer()
                    .border_color(if selected {
                        theme::STENCIL
                    } else {
                        theme::hairline()
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.pin_color = value.map(str::to_string);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .size_5()
                            .rounded_full()
                            .bg(value.and_then(pin_color).unwrap_or(theme::STENCIL_MUTE)),
                    )
            };
        let colors: Vec<_> = std::iter::once(color_swatch("color-default", None, cx))
            .chain(PIN_COLORS.map(|color| color_swatch(color, Some(color), cx)))
            .collect();

        common::form_page(
            if editing {
                t("Edit Snippet")
            } else {
                t("New Snippet")
            },
            self.error.clone(),
            v_flex()
                .gap_4()
                .child(
                    common::form_section("server", t("Host & title"), None)
                        .child(common::field(
                            t("Host"),
                            Select::new(&self.host).disabled(saving),
                            None,
                        ))
                        .child(common::field(
                            t("Title"),
                            Input::new(&self.title).disabled(saving),
                            None,
                        )),
                )
                .child(
                    common::form_section("file-text", t("Command"), None).child(common::field(
                        t("Command"),
                        Textarea::new(&self.body).disabled(saving),
                        None,
                    )),
                )
                .child(
                    common::form_section("pin", t("Shortcut panel"), None)
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    Checkbox::new("show-on-panel")
                                        .label(t("Show on shortcut panel"))
                                        .checked(self.show_on_panel)
                                        .disabled(saving)
                                        .on_change(cx.listener(|this, checked: &bool, _, cx| {
                                            this.show_on_panel = *checked;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    div()
                                        .pl_6()
                                        .text_xs()
                                        .text_color(theme::STENCIL_MUTE)
                                        .child(t(
                                            "Drag shortcuts in the sidebar to change their order.",
                                        )),
                                ),
                        )
                        .when(self.show_on_panel, |section| {
                            section
                                .child(common::field(
                                    t("Icon"),
                                    h_flex().flex_wrap().gap_2().children(icons),
                                    None,
                                ))
                                .child(common::field(
                                    t("Color"),
                                    h_flex().flex_wrap().gap_2().children(colors),
                                    None,
                                ))
                        }),
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
