//! Building blocks every page shares: the page frame, form sections, fields,
//! banners, and the delete confirmation.

use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::select::{SelectItem, SelectState};
use gpui_kit::component::{h_flex, v_flex, Disableable as _, IndexPath, WindowExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::ui::i18n::t;
use crate::ui::icons::lucide;
use crate::ui::theme;

/// The frame of a list page: a title row with actions, then scrolling content.
pub fn page(title: SharedString, actions: impl IntoElement, body: impl IntoElement) -> Div {
    v_flex()
        .size_full()
        .px_8()
        .py_6()
        .bg(theme::PORT)
        .text_color(theme::STENCIL)
        .child(
            v_flex()
                .flex_1()
                .min_h_0()
                .p_6()
                .child(
                    h_flex()
                        .items_start()
                        .justify_between()
                        .gap_4()
                        .pb_5()
                        .border_b_1()
                        .border_color(theme::hairline())
                        .child(
                            div()
                                .text_2xl()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(h_flex().gap_2().child(actions)),
                )
                .child(body),
        )
}

/// The scrolling area under a page's title row.
pub fn page_body(id: &'static str) -> Stateful<Div> {
    div().id(id).mt_6().flex_1().min_h_0().overflow_y_scroll()
}

/// The frame of a form page: title, scrolling fields, then Cancel and Save.
pub fn form_page(
    title: SharedString,
    error: Option<SharedString>,
    fields: impl IntoElement,
    footer: impl IntoElement,
) -> Div {
    v_flex()
        .size_full()
        .px_8()
        .py_6()
        .bg(theme::PORT)
        .text_color(theme::STENCIL)
        .child(
            v_flex()
                .flex_1()
                .min_h_0()
                .p_6()
                .child(
                    div()
                        .pb_4()
                        .border_b_1()
                        .border_color(theme::hairline())
                        .text_2xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .child(
                    div()
                        .id("form-fields")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .child(
                            v_flex()
                                .mx_auto()
                                .w_full()
                                .max_w(px(768.))
                                .gap_4()
                                .py_4()
                                .children(error.map(error_banner))
                                .child(fields),
                        ),
                )
                .child(
                    h_flex()
                        .flex_shrink_0()
                        .justify_end()
                        .gap_2()
                        .pt_4()
                        .border_t_1()
                        .border_color(theme::hairline())
                        .child(footer),
                ),
        )
}

/// Cancel and submit, for [`form_page`]'s footer.
pub fn form_buttons(
    submit_label: SharedString,
    busy: bool,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_submit: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    h_flex()
        .gap_2()
        .child(
            Button::new("form-cancel")
                .ghost()
                .label(t("Cancel"))
                .disabled(busy)
                .on_click(on_cancel),
        )
        .child(
            Button::new("form-submit")
                .primary()
                .label(submit_label)
                .loading(busy)
                .disabled(busy)
                .on_click(on_submit),
        )
}

/// A bordered group of fields with an icon and a title.
pub fn form_section(icon: &'static str, title: SharedString, hint: Option<SharedString>) -> Div {
    v_flex()
        .gap_3()
        .p_4()
        .rounded(theme::RADIUS)
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::RIB)
        .child(
            h_flex().gap_3().pb_1().child(icon_box(icon)).child(
                v_flex()
                    .gap_1()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                    .children(
                        hint.map(|hint| {
                            div().text_xs().text_color(theme::STENCIL_MUTE).child(hint)
                        }),
                    ),
            ),
        )
}

/// The 40px bordered square that holds a row's or section's icon.
pub fn icon_box(icon: &'static str) -> Div {
    div()
        .size_10()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(theme::RADIUS)
        .border_1()
        .border_color(theme::hairline())
        .text_color(theme::STENCIL_DIM)
        .child(lucide(icon).size_5())
}

/// A label over a control, with an optional validation message under it.
pub fn field(label: SharedString, control: impl IntoElement, error: Option<SharedString>) -> Div {
    v_flex()
        .gap_2()
        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(label))
        .child(control)
        .children(error.map(|error| div().text_xs().text_color(theme::RED_TEXT).child(error)))
}

/// The small uppercase heading over a group of rows.
pub fn section_title(title: SharedString) -> Div {
    div()
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::STENCIL_MUTE)
        .child(SharedString::from(title.to_uppercase()))
}

pub fn error_banner(error: SharedString) -> Div {
    div()
        .px_4()
        .py_3()
        .rounded(theme::RADIUS)
        .border_1()
        .border_color(theme::SEAL.opacity(0.6))
        .bg(theme::SEAL.opacity(0.1))
        .text_sm()
        .text_color(theme::RED_TEXT)
        .child(error)
}

/// The dashed box shown when a list has nothing in it.
pub fn empty_state(message: SharedString) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .min_h(px(256.))
        .px_4()
        .py_8()
        .rounded(theme::RADIUS)
        .border_1()
        .border_dashed()
        .border_color(theme::hairline())
        .text_sm()
        .text_color(theme::STENCIL_MUTE)
        .child(message)
}

/// A card in a list: bordered, and brighter under the pointer.
pub fn card(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .px_4()
        .py_4()
        .rounded(theme::RADIUS)
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::RIB)
        .hover(|style| {
            style
                .border_color(theme::STENCIL.opacity(0.2))
                .bg(theme::RIB_HOVER)
        })
}

/// Asks before deleting `item_name`; `on_confirm` runs when the user agrees.
pub fn confirm_delete(
    title: SharedString,
    description: SharedString,
    item_name: Option<SharedString>,
    confirm_label: SharedString,
    window: &mut Window,
    cx: &mut App,
    on_confirm: impl Fn(&mut Window, &mut App) + 'static,
) {
    let on_confirm = std::rc::Rc::new(on_confirm);
    window.open_alert_dialog(cx, move |alert, _, _| {
        let on_confirm = on_confirm.clone();
        let description = v_flex().gap_3().child(description.clone()).when_some(
            item_name.clone(),
            |body, name| {
                body.child(
                    v_flex()
                        .gap_1()
                        .px_4()
                        .py_3()
                        .rounded(theme::RADIUS)
                        .border_1()
                        .border_color(theme::hairline())
                        .bg(theme::RIB)
                        .child(section_title(t("Target")))
                        .child(
                            div()
                                .truncate()
                                .text_sm()
                                .text_color(theme::STENCIL)
                                .child(name),
                        ),
                )
            },
        );
        alert
            .title(title.clone())
            .description(description)
            .confirm()
            .ok_text(confirm_label.clone())
            .ok_variant(ButtonVariant::Danger)
            .cancel_text(t("Cancel"))
            .on_ok(move |_, window, cx| {
                on_confirm(window, cx);
                true
            })
    });
}

/// One option in a [`ChoiceSelect`].
#[derive(Clone)]
pub struct Choice {
    pub value: String,
    pub label: SharedString,
}

impl Choice {
    pub fn new(value: impl Into<String>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

impl SelectItem for Choice {
    type Value = String;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &String {
        &self.value
    }
}

/// A dropdown over string values. Render with `Select::new(&state)`; read with
/// [`selected`].
pub type ChoiceSelect = SelectState<Vec<Choice>>;

pub fn new_select(
    choices: Vec<Choice>,
    selected: &str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<ChoiceSelect> {
    let index = choices.iter().position(|choice| choice.value == selected);
    cx.new(|cx| SelectState::new(choices, index.map(IndexPath::new), window, cx))
}

pub fn selected(select: &Entity<ChoiceSelect>, cx: &App) -> Option<String> {
    select.read(cx).selected_value().cloned()
}
