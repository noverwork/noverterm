# DESIGN.md — Truley Sentinel Design System

This file defines the visual language for the Sentinel app. All UI code must
conform to these tokens and patterns. When in doubt, reference the existing
shared widgets in `packages/app/lib/shared/widgets/`.

---

## Color Tokens (`AppColorsTheme`)

Access via `context.colors.xxx`. Never use raw hex in widgets.

### Brand

Light 與 Dark 採不同色值：light 在白底上需要更高彩度與對比，dark 沿用品牌原色。

| Token       | Dark        | Light       | Usage              |
| ----------- | ----------- | ----------- | ------------------ |
| `brandCyan` | `#2EE8D4`  | `#06B6D4`  | Logo, brand accent |
| `brandBlue` | `#3B9EFF`  | `#2563EB`  | Brand secondary    |

### Accent (Semantic)

Light 變體針對白底調整為 contrast >= 4.5:1，確保可作為前景（文字、icon、status dot）使用，不只用於填色。

| Token          | Dark        | Light       | Usage                            |
| -------------- | ----------- | ----------- | -------------------------------- |
| `accentBlue`   | `#3B82F6`  | `#2563EB`  | Primary action, selected state   |
| `accentGreen`  | `#32D583`  | `#16A34A`  | Success, toggle-on, active badge |
| `accentIndigo` | `#6366F1`  | `#4F46E5`  | AI / assistant features          |
| `accentOrange` | `#F97316`  | `#EA580C`  | Warning, shadow overlay          |
| `accentRed`    | `#E85A4F`  | `#DC2626`  | Error, destructive action        |

### Background

Light 模式刻意讓 `bgCard` 與 `bgElevated` 同為白色，**透過 shadow（而非 color step）做層級**——這是 light 與 dark 的設計策略差異。

| Token        | Dark        | Light       | Usage                           |
| ------------ | ----------- | ----------- | ------------------------------- |
| `bgPage`     | `#0B0B0E`  | `#F2F2F7`  | Full-page background            |
| `bgCard`     | `#16161A`  | `#FFFFFF`  | Card, dialog, sheet surface     |
| `bgElevated` | `#1A1A1E`  | `#FFFFFF`  | Elevated surface, tab bar, chip (light 用 shadow 區分層級) |
| `bgInput`    | `#1E1E24`  | `#EFEFF4`  | Text field, search bar          |

### Border

| Token          | Dark        | Light       | Usage                    |
| -------------- | ----------- | ----------- | ------------------------ |
| `borderSubtle` | `#2A2A2E`  | `#D1D1D6`  | Default card/row border  |
| `borderStrong` | `#3A3A40`  | `#8E8E93`  | Hover border, outline btn|

### Text

Light 文字色經過 WCAG AA 校驗：primary 15.9:1、secondary 7.7:1、tertiary 5.1:1，皆通過小字標準。

| Token           | Dark        | Light       | Usage                        |
| --------------- | ----------- | ----------- | ---------------------------- |
| `textPrimary`   | `#FAFAF9`  | `#1C1C1E`  | Headings, body text          |
| `textSecondary` | `#A1A1A6`  | `#515159`  | Subtitle, metadata           |
| `textTertiary`  | `#6B6B70`  | `#6C6C70`  | Placeholder, hint, icon mute |

### Special

| Token            | Dark                  | Light                | Usage                             |
| ---------------- | --------------------- | -------------------- | --------------------------------- |
| `whiteOnColor`   | `#FFFFFF`             | `#FFFFFF`            | Text/icon on accent fill          |
| `hoverOverlay`   | `white 8%`            | `black 5%`           | General hover blend               |
| `hoverHighlight` | `white 12%`           | `black 8%`           | Emphasis hover (sidebar)          |
| `shadowOverlay`  | `accentOrange 15%`    | `accentOrange 8%`    | Glow / recording shadow           |

---

## Typography (`AppTypography`)

All styles are **colorless** — apply color via `.copyWith(color: ...)` at the widget layer.

### Headings (DM Sans — semibold 600)

| Style          | Size | Weight | Extra           | Usage                    |
| -------------- | ---- | ------ | --------------- | ------------------------ |
| `headingTitle` | 24   | w600   | ls: -0.8        | Page title               |
| `headingLg`    | 20   | w600   |                 | Section title            |
| `headingMd`    | 16   | w600   |                 | Card title, dialog title |
| `headingSm`    | 15   | w600   |                 | Button label (AppButton) |

### Body (DM Sans)

| Style           | Size | Weight | Extra    | Usage                    |
| --------------- | ---- | ------ | -------- | ------------------------ |
| `bodyMd`        | 14   | w500   |          | Default body text        |
| `bodyMdRegular` | 14   | w400   | h: 1.5   | Long-form paragraph      |

### Labels (Inter)

| Style             | Size | Weight | Usage                          |
| ----------------- | ---- | ------ | ------------------------------ |
| `labelLg`         | 14   | w400   | Search input text              |
| `labelMd`         | 13   | w600   | Active chip, filter label      |
| `labelMdRegular`  | 13   | w400   | Card metadata (time, duration) |
| `labelSm`         | 12   | w600   | Section header, badge text     |
| `labelSmRegular`  | 12   | w400   | Footnote                       |
| `labelXs`         | 11   | w600   | Tiny label                     |
| `labelXsRegular`  | 11   | w500   | Timestamp, micro text          |

### Brand (Fraunces)

| Style   | Size | Weight | Extra    | Usage     |
| ------- | ---- | ------ | -------- | --------- |
| `brand` | 20   | w600   | ls: -0.3 | App title |

### Font Stack

| Role     | Family    | Bundled |
| -------- | --------- | ------- |
| Heading  | DM Sans   | Yes     |
| Label    | Inter     | Yes     |
| Brand    | Fraunces  | Yes     |

---

## Spacing Scale

Only use these values: **4, 8, 10, 12, 16, 24, 32, 48**

`10` 僅作為 icon-to-text 緊密 gap 使用（如 `AppSearchBar` 的 search icon 與輸入框之間），其它情境請優先選用 8 或 12。

Common patterns from existing widgets:

| Context                     | Value |
| --------------------------- | ----- |
| Icon-to-text gap            | 8     |
| Inline element gap          | 4     |
| Row internal gap            | 10–12 |
| Card padding                | 16    |
| Section vertical spacing    | 12–16 |
| Page horizontal padding     | 16    |
| Button horizontal padding   | 24 (AppButton) / 16 (PillButton) |

---

## Border Radius

| Context                | Radius |
| ---------------------- | ------ |
| Pill chip / tab bar    | 31     |
| Card                   | 16     |
| Button / Input / Sheet | 12     |
| Badge                  | 8      |
| Segment item           | 8      |
| Toggle                 | 14     |

---

## Iconography

- Icon set: **Lucide** (`lucide_icons` package)
- Default size: **18**
- Color: semantic token (`textTertiary` for muted, `textPrimary` for active, `whiteOnColor` on accent fill)

---

## Component Patterns

### Hover (Desktop)

All interactive elements must use `HoverBuilder`:

```
HoverBuilder wraps OUTSIDE GestureDetector
Hover effect = Color.alphaBlend(hoverOverlay, baseColor)
Primary button: fill.withValues(alpha: 0.85)
Non-primary: alphaBlend(hoverOverlay, fill)
```

### AppButton (Primary Action)

- Height: **48**
- Radius: **12**
- Horizontal padding: **24**
- Label style: `headingSm`
- Variants: `primary` (accentBlue fill), `secondary` (bgElevated fill), `outline` (transparent + borderStrong)

### PillButton (Header / Toolbar)

- Height: **42**
- Radius: **12**
- Horizontal padding: **16**
- Label: DM Sans 14 w600
- Variants: `primary`, `outline`, `destructive` (accentRed text), `ghost` (textSecondary text)
- Disabled: opacity 0.5

### StatusBadge

- Padding: h10 v4
- Radius: **8**
- Fill: caller-provided accent at 20% opacity（預設 `accentGreen`，可由 caller 傳入任意 accent token）
- Text: `labelSm` in same accent color

### AppFilterChip

- Padding: h16 v8
- Radius: **31** (pill)
- Active: accentBlue fill, whiteOnColor text
- Inactive: bgElevated fill, textSecondary text

### SessionCard

- Padding: **16**
- Radius: **16**
- Fill: bgCard
- Border: borderSubtle (default), borderStrong (hover)
- Title: `headingMd` / textPrimary
- Metadata: `labelMdRegular` / textSecondary

### SegmentControl

- Outer height: **44**, padding: **4**
- Outer fill: bgElevated, radius: **12**
- Item fill (selected): bgCard, radius: **8**
- Text: `bodyMd`, selected w600, unselected w400

### AppToggle

- Size: 48 x 28
- Knob: 22 x 22 circle
- On: accentGreen fill, white knob
- Off: bgElevated fill + borderStrong, textSecondary knob

### AppSearchBar

- Padding: h16 v12
- Radius: **12**
- Fill: bgInput
- Icon: Lucide search, 18, textTertiary
- Text: `labelLg` / textPrimary, hint: textTertiary

### SectionLabel

- Style: `labelSm`, uppercase, letterSpacing 1.5
- Color: textSecondary

### AppDivider

- Height: 1px
- Color: borderSubtle

---

## Layout Rules

### Platform Detection

Layout is decided by **host OS**, not window width:

| Host                       | Layout    |
| -------------------------- | --------- |
| macOS / Windows / Linux    | Desktop   |
| iOS / Android (incl. iPad) | Mobile    |
| Web < 600px                | Mobile    |
| Web >= 600px               | Desktop   |

### Desktop

- Minimum window: **960 x 640**
- Sidebar: expanded **220px** / collapsed **72px**
- All pages render inside Sidebar shell

### Mobile

- Full-width, horizontal padding: **16**
- Recording/SessionDetail/Summary/Transcription: full-screen push (no tab bar)
- Tab bar: floating pill, radius 31, bgElevated, border borderSubtle

---

## Desktop Navigation

### Back triggers

Any page that exposes a back button (`onBack` on `DesktopPageHeader` or `SearchableHeader`) must be reachable via:

| Trigger              | Platform      | Behavior                                                                  |
| -------------------- | ------------- | ------------------------------------------------------------------------- |
| Back chevron click   | All           | Invokes `onBack`                                                          |
| **Esc**              | All desktop   | Invokes `onBack`. **Does not fire while a `TextField` has focus** — Flutter's `EditableText` absorbs Esc, so the user can never accidentally navigate away mid-typing |
| **⌘ + [**            | macOS         | Invokes `onBack`                                                          |
| **Alt + ←**          | Windows / Linux | Invokes `onBack`                                                        |
| **Mouse back button** | All desktop  | Invokes `onBack` (`kBackMouseButton` / XButton1)                          |

Wired automatically by `DesktopBackHandler` (`packages/app/lib/core/navigation/desktop_back_handler.dart`) wrapping `MaterialApp.router`. Pages do not need their own keyboard handling — passing `onBack` to a shared header is enough; it self-registers via `pageBackRegistryProvider`.

Hidden-branch pages must keep using `context.go(...)` inside `onBack` (not `Navigator.pop()`); the shortcut layer simply invokes the same callback, so this is preserved.

### AppBackButton (mandatory)

**All back affordances must use `AppBackButton`** (`packages/app/lib/shared/widgets/app_back_button.dart`). Self-rolled `IconButton(arrow_back)`, `GestureDetector + chevronLeft`, `BackButton()`, or `AppBar(leading: ...)` with a custom icon are **forbidden**.

| Style | Icon | Size | Hover | Use in |
| --- | --- | --- | --- | --- |
| `desktop` (default) | arrowLeft | 24 | textSecondary → textPrimary | `DesktopPageHeader` (drill-in pages) |
| `desktopCompact` | arrowLeft | 20 | textSecondary → textPrimary | `DesktopFocusHeader` (tighter padding) |
| `mobile` | chevronLeft | 24 | none (always textPrimary) | `SearchableHeader`, `NavHeader`, mobile pages |

Two reasons it is mandatory:

1. **Visual consistency** — three sizes / two icons / multiple hover behaviors used to coexist; the variants above are the only sanctioned forms.
2. **Auto-wires desktop shortcuts** — the widget self-registers with `pageBackRegistryProvider` on mount and unregisters on dispose. A self-rolled back button visually works but **silently breaks Esc / ⌘+[ / Alt+← / mouse back button** for that page.

If you need a non-standard back affordance (e.g. a "Back" pill with a label), extend `AppBackButton` rather than rolling your own — that way the registration logic lives in one place.

---

## Do / Don't

**DO:**
- Use `context.colors.xxx` for all colors
- Use `AppTypography.xxx` for all text styles, add color via `.copyWith()`
- Wrap all clickable desktop elements in `HoverBuilder`
- Use spacing scale values only
- Use existing shared widgets before creating new ones

**DON'T:**
- Use raw hex / Color() in widget code
- Put color in typography definitions
- Use Material default colors (Colors.blue, Colors.grey)
- Create custom hover logic outside HoverBuilder
- Use padding/margin values outside the spacing scale
