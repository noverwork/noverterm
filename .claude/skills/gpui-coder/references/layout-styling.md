# Layout, Styling, and Interface Copy

Styling is a Tailwind-like fluent API on elements. It is not CSS: there are no selectors, no
cascade, no stylesheet. Each call sets a field on that one element.

## Flex

```rust
use gpui_kit::component::{h_flex, v_flex};

h_flex().gap_2().child(icon).child(label)          // row, children centered on the cross axis
v_flex().gap_3().child(header).child(body)         // column, children stretch
div().flex().items_start().justify_between()       // explicit
div().grid().grid_cols(3).gap_3()                  // grid
```

`h_flex()` and `v_flex()` are not mirror images. `h_flex()` centers its children vertically, which
is right for an icon and a label and wrong for full-height columns:

```rust
h_flex().items_stretch().size_full().child(sidebar).child(content)
```

Without `items_stretch()` (or `h_full()` on the column) a column taller than the row is centered
and its header is clipped off the top.

## Shrinking and scrolling

```rust
v_flex()
    .size_full()
    .child(header)                      // takes its content height
    .child(
        div()
            .id("list")                 // scrolling needs an id
            .flex_1()
            .min_h_0()                  // allow it to shrink below its content
            .overflow_y_scroll()
            .child(rows),
    )
    .child(footer.flex_shrink_0())
```

- A flex child keeps its content size until told otherwise: `.min_h_0()` for a scrolling column,
  `.min_w_0()` for a truncating row (`name.min_w_0().truncate()`).
- One owner per scroll region. Put padding **inside** the scroller, not around it, so the scrollbar
  sits on the region's edge.
- No accidental nested scrolling.
- Large or unbounded lists: `uniform_list(...)` with `.track_scroll(&handle)` (see the SFTP pane),
  not a `div` with thousands of children. Keep the scroll handle on the view.

## Sizes

Theme sets the rem to 16px, so the scale matches Tailwind: `p_4()` = 16px, `gap_3()` = 12px,
`size_10()` = 40px, `text_sm()` = 14px, `text_xs()` = 12px.

Prefer scale helpers. `px(...)` is for a real fixed geometry (`max_w(px(768.))` for the form
column, the window size, measured terminal cells) — not for nudging something into place. If an
edge is misaligned, find the duplicated padding or border that causes it.

## Theme

```rust
use crate::ui::theme;

div()
    .bg(theme::RIB)
    .text_color(theme::STENCIL_MUTE)
    .border_1()
    .border_color(theme::hairline())
    .rounded(theme::RADIUS)
    .hover(|style| style.bg(theme::wash()))
```

| Constant | Role |
|---|---|
| `PORT` / `RAIL` / `RIB` | Page background / sidebar and inset / raised surface |
| `STENCIL` / `_DIM` / `_MUTE` / `_FAINT` | Text, from primary to faintest |
| `hairline()` / `wash()` | Border / hover wash |
| `SEAL` / `SEAL_HOVER` | Danger |
| `RED_TEXT` / `AMBER_TEXT` / `GREEN_TEXT` | Status text |
| `RADIUS` | The only corner radius |

- Raw `rgb(...)`, `rgba(...)`, `hsla(...)`, and hex belong in `ui/theme.rs` only. Need a new
  color: add a named constant there. The palette is defined in `DESIGN.md`.
- `theme::apply` also maps the palette onto the component theme, so `Button`, `Input`, `Select`
  already match. Change component colors through `Theme::update(cx, |theme| ...)`, never
  `Theme::global_mut`.
- Color carries meaning, not decoration. Ordinary metadata stays neutral; do not give every enum
  case its own color.
- State styling: `.hover(...)`, `.active(...)`, `.focus_visible(...)` on elements; the component's
  own `.selected(...)`, `.disabled(...)`, `.loading(...)` for semantic states.
- An ancestor with `overflow_hidden()` clips focus rings. Leave room.

## Conditionals

```rust
use gpui_kit::prelude::FluentBuilder as _;

div()
    .when(self.selected, |el| el.bg(theme::wash()))
    .when_some(self.error.clone(), |el, error| el.child(common::error_banner(error)))
    .children(self.hint.clone().map(|hint| div().text_xs().child(hint)))
```

Small refinements use `.when`. Substantially different interfaces use ordinary `if`/`match` with
`.into_any_element()` on each branch.

## Identity

```rust
Button::new(SharedString::from(format!("remove-{}", host.id)))
div().id(("row", index))      // only for a list that never reorders
div().id("form-fields")       // a singleton
```

`.id(...)` turns `Div` into `Stateful<Div>` and unlocks `on_click`, scrolling, tooltips, and keyed
state. The id is the element's identity: a changed id resets hover, scroll, and focus state; two
elements sharing one overwrite each other. Namespace repeated controls with their owner's id.

## Components first

Before composing `div`s, check `gpui_kit::component`: `button`, `input`, `select`, `switch`,
`checkbox`, `dialog`, `notification`, `tooltip`, `menu`, `tab`, `table`, `list`, `tree`,
`scroll`, `sidebar`, `badge`, `tag`, `kbd`, `skeleton`, `spinner`, and more. A custom clickable
`div` loses focus, keyboard activation, disabled state, and accessibility.

- `Button` is a command, even a quiet one (`.ghost()`, `.outline()`, icon-only with `.tooltip`).
- `Link` is for something a browser or mail client opens.
- `.primary()` marks the surface's real default action. One per decision area.
- Sizes come from `Sizable`: `.xsmall()`, `.small()`, `.large()`.

Then check `views/common.rs` for the project's own frame: `page`, `page_body`, `form_page`,
`form_buttons`, `form_section`, `field`, `card`, `icon_box`, `section_title`, `empty_state`,
`error_banner`.

## Hierarchy

Use a small number of levels: page title, section title, body, muted metadata, control labels.
Reach for size, weight, spacing, and a hairline before adding color or a container. Do not nest
cards in cards. Emphasis is a budget: if everything is bold, badged, boxed, or colored, nothing
reads as important.

## Interface copy

**Avoid unnecessary secondary labels and descriptions; let the layout do the explaining.**

- Do not repeat what the surrounding surface already says. A sidebar item is the object itself:
  `Keys`, not `Key Management`. A column of buttons needs no `Actions` heading. A dialog titled
  `Remove Known Host?` does not need a body asking the same question.
- A description, hint, or subtitle must change a decision: scope, an irreversible consequence, an
  unusual prerequisite, a way to recover. If the layout already answers it, delete the text.
- Before adding helper text under a field or a subtitle under a title, try grouping, order,
  proximity, a placeholder example, or the control's own state instead.
- Nouns for destinations (`Connections`), verbs for commands (`Save`, `Remove`), short adjectives
  for states (`live`, `held · disconnected`). No `Management`, `Module`, `Page`, `Operation`.
- One term per concept across the app.

### i18n

```rust
t("Remove host")
tf("e.g. {example}", &[("example", "prod-web-01")])
```

English is the key. Add the translation to `packages/desktop/src/ui/i18n/zh-tw.json`; a missing key
falls back to English. Never assemble a sentence from translated fragments — use one `tf` string
with placeholders so each locale controls word order. Check long translations against the layout.

## Text

Text kept across frames or moved into a closure is `SharedString` (cloning does not copy the
bytes). `String` is for a buffer being built; convert with `.into()` when done. `&'static str` and
`String` both convert into a child directly.

## Images and icons

`lucide("folder-open")` returns an `Icon` (`.xsmall()`, `.small()`, `.size_5()`,
`.text_color(...)`). App-specific SVGs are registered in `ui::Assets` (`ui/mod.rs`); a missing
asset path renders nothing and reports no error.
