---
name: Noverterm
description: Native SSH terminal client, dark glass panels around a terminal that stays the main subject.
colors:
  signal-cyan: "#67e8f9"
  signal-cyan-hover: "#a5f3fc"
  signal-cyan-text: "#a5f3fc"
  signal-cyan-glow: "#ecfeff"
  midnight-canvas: "#05070b"
  midnight-canvas-deep: "#0b1220"
  terminal-ink: "#080c13"
  overlay-slate: "#020617"
  frost-border: "rgb(255 255 255 / 0.10)"
  frost-border-faint: "rgb(255 255 255 / 0.08)"
  frost-fill: "rgb(255 255 255 / 0.05)"
  field-well: "rgb(0 0 0 / 0.20)"
  text-bright: "#ffffff"
  text-primary: "#f1f5f9"
  text-secondary: "#cbd5e1"
  text-muted: "#94a3b8"
  text-faint: "#64748b"
  on-signal: "#020617"
  status-live: "#34d399"
  status-waiting: "#fcd34d"
  status-error: "#f87171"
  error-text: "#fca5a5"
typography:
  headline:
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, Courier New, monospace"
    fontSize: "1.5rem"
    fontWeight: 600
  title:
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, Courier New, monospace"
    fontSize: "1.25rem"
    fontWeight: 600
  body:
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, Courier New, monospace"
    fontSize: "0.875rem"
    fontWeight: 500
  label:
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, Courier New, monospace"
    fontSize: "0.75rem"
    fontWeight: 500
  section-label:
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, Courier New, monospace"
    fontSize: "11px"
    fontWeight: 600
    letterSpacing: "0.22em"
rounded:
  control: "10px"
  field: "16px"
  card: "1.35rem"
  panel: "1.75rem"
  dialog: "2rem"
  pill: "9999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  2xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.signal-cyan}"
    textColor: "{colors.on-signal}"
    typography: "{typography.body}"
    rounded: "{rounded.field}"
    height: "32px"
    padding: "0 10px"
  button-primary-hover:
    backgroundColor: "{colors.signal-cyan-hover}"
  button-ghost:
    backgroundColor: "{colors.frost-fill}"
    textColor: "{colors.text-bright}"
    rounded: "{rounded.control}"
    height: "32px"
    padding: "0 10px"
  input-field:
    backgroundColor: "{colors.field-well}"
    textColor: "{colors.text-bright}"
    typography: "{typography.body}"
    rounded: "{rounded.field}"
    height: "44px"
    padding: "0 12px"
  session-tab:
    textColor: "{colors.text-muted}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "4px 12px"
  session-tab-active:
    textColor: "{colors.text-bright}"
  list-card:
    backgroundColor: "{colors.frost-fill}"
    rounded: "{rounded.card}"
    padding: "16px"
  chip:
    textColor: "{colors.signal-cyan-text}"
    typography: "{typography.label}"
    rounded: "{rounded.pill}"
    padding: "4px 12px"
---

# Design System: Noverterm

## Overview

**Creative North Star: "The Glass Terminal"**

Noverterm is a dark, cool workspace where every piece of chrome is a thin sheet of frosted glass laid over a near-black blue gradient, and the terminal itself is the one opaque, solid surface. The glass is how the app says "light": panels are translucent white at single-digit opacity, edged by a one-pixel frost border, so the structure reads without adding visual weight.

Restraint governs the glass. Hierarchy comes from borders and tonal steps in white opacity, not from lift. Signal Cyan is the only accent with a voice: it marks the active tab, the primary action, the focused field. Everything else is slate text on midnight. Status lights (green, amber, red dots) are the only other color, and they carry information, never decoration.

The whole interface is set in one monospaced face, Sarasa Term, so chrome and terminal share a single typographic voice. Density is moderate: 14px text, 44px fields and top bar, generous rounding that softens the monospace grid.

**Key Characteristics:**
- Near-black blue canvas with faint cyan and blue radial glows at the edges.
- Translucent panels (white 4–10%) with a one-pixel white 8–10% border.
- One accent, Signal Cyan, reserved for active and primary states.
- A single monospace typeface for every text role.
- Large, soft radii: 16px controls, ~22px cards, 28–32px panels and dialogs.

## Colors

A cold midnight palette: blue-black surfaces, slate text, one cyan signal, and three status lights.

### Primary
- **Signal Cyan** (`signal-cyan`): primary action fill (Save, New, Connect), with Midnight text on top; active-tab border and wash at 30% and 10%; focus borders at 40%; drag-insert markers.
- **Pale Signal** (`signal-cyan-hover`): primary hover fill, and the text/icon color for active labels and chips.

### Neutral
- **Midnight Canvas** (`midnight-canvas` to `midnight-canvas-deep`): the 135° workspace gradient, under 9–11% cyan and blue radial glows.
- **Terminal Ink** (`terminal-ink`): the terminal background and the solid tab bar behind it. The only fully opaque working surface.
- **Overlay Slate** (`overlay-slate`): context menus and popovers at 95–96% opacity.
- **Frost Border** (`frost-border`, `frost-border-faint`): every panel, card, field and divider edge.
- **Frost Fill** (`frost-fill`): resting card and ghost-button fill; hover raises it to white 8–10%.
- **Field Well** (`field-well`): inputs, selects and textareas sit *below* the surface as a darker well.
- **Text ramp**: Bright White for headings and values, Slate 100/300 for body, Slate 400 for inactive tabs and metadata, Slate 500 for placeholders and hints.

### Status
- **Live Green** (`status-live`), **Waiting Amber** (`status-waiting`), **Fault Red** (`status-error`): 8px session dots with a 10px glow of the same hue. Amber pulses only while connecting. Red text and red-tinted fills (red 400 at 10–12%) carry errors and destructive actions.

### Named Rules
**The One Signal Rule.** Cyan means "active, focused, or primary". Never use it for decoration, headings, or illustration.

**The Lights Mean Something Rule.** Green, amber and red appear only as connection or result state. A colored dot that does not report state is a bug.

The shadcn theme tokens in `app.css` (`--primary` is a blue `oklch(0.68 0.16 255)`) back the stock `Button` default variant, but product screens override primary actions to Signal Cyan. Treat Signal Cyan as the primary.

## Typography

**Body Font:** Sarasa Term TC SemiBold (with Menlo, Monaco, Courier New, monospace)

**Character:** One monospaced CJK-capable face for everything. Chrome reads like part of the terminal, and Chinese hostnames or paths align with Latin text.

Only the 600 weight is bundled, so `font-medium` and `font-semibold` both render at the same SemiBold weight. Hierarchy comes from size, color and case, not weight.

### Hierarchy
- **Headline** (600, 24px): page or dialog titles such as error screens.
- **Title** (600, 20px): view headers (Connections, Snippets, Forwards).
- **Body** (14px): default for tabs, list rows, form values, buttons.
- **Label** (12px): metadata, helper text, chips, compact buttons.
- **Section Label** (600, 11px, 0.22em tracking, uppercase, Slate 400): group headers above lists and form sections.

### Named Rules
**The One Voice Rule.** Never introduce a proportional sans for chrome. If text needs more emphasis, change size or color.

## Layout

A fixed desktop shell: a collapsible sidebar on the left, a 44px top bar holding the page label and session tabs, and the active view filling the rest. The window opens maximized at a 1200×800 minimum, and nothing reflows for mobile.

Spacing follows Tailwind's 4px grid. The working steps are 8px (icon-to-text, tab gap), 12px (row internals, form gaps), 16px (card padding, section spacing) and 24px (view padding). Session tabs flex between 128px and 240px and scroll horizontally, with edge fades and hover auto-scroll.

## Elevation & Depth

Depth is tonal and translucent. Resting surfaces separate by white-opacity steps (4%, 5–6%, 8–10%) and a one-pixel frost border, not by lift. Panels use backdrop blur so the canvas glow shows through.

### Shadow Vocabulary
- **Overlay** (`box-shadow: 0 25px 50px -12px rgb(0 0 0 / 0.45)`, Tailwind `shadow-2xl shadow-black/45`): context menus, dialogs, popovers.
- **Panel** (`box-shadow: 0 24px 70px rgb(0 0 0 / 0.36), inset 0 1px 0 rgb(255 255 255 / 0.06)`): the large `ide-panel` container. The inset line is the glass top edge.
- **Status glow** (`box-shadow: 0 0 10px <status hue / 0.45–0.55>`): session dots only.

### Named Rules
**The Flat Glass Rule.** Resting surfaces get a border and a tonal step, not a drop shadow. Shadows belong to things that float above the workspace: menus, dialogs, the outer panel.

## Shapes

Soft and generous. Every rectangle is rounded, and radius grows with the size of the surface: 10px for tabs and small controls, 16px for fields and primary buttons, about 22px (1.35rem) for list cards and empty states, 28px (1.75rem) for panels, 32px (2rem) for full dialogs. Pills (fully round) are for chips and status dots. Empty states use a dashed frost border.

## Components

### Buttons
Quiet, compact, rounded.
- **Shape:** 16px radius on primary actions, 10px on stock buttons. 32px tall.
- **Primary:** Signal Cyan fill, Midnight text, Pale Signal on hover.
- **Ghost / Outline:** Frost Fill with a frost border and white text; hover raises the fill to white 8%.
- **Destructive:** red-tinted fill (10%) with red text, stronger tint on hover.
- **Focus / Press:** 3px ring at ring/50, a 1px press-down on active, 50% opacity when disabled.

### Inputs / Fields
Wells sunk into the glass.
- **Style:** 44px tall, 16px radius, Field Well background, frost border, white text, Slate 500 placeholder. Textareas use the same treatment, monospace, 192px minimum height.
- **Focus:** border shifts to cyan 40% plus a 1px cyan 20% ring.
- **Error:** border switches to Fault Red; the message sits below in red.

### Session Tabs (signature)
The workspace's primary navigation.
- **Inactive:** transparent with Slate 400 text; hover adds a frost border, white 4.5% fill and white text.
- **Active:** cyan 30% border, cyan 10% wash, white text.
- **Content:** status dot, truncated session name, and a 24px close button that appears on hover.
- **Interaction:** drag to reorder (the dragged tab drops to 40% opacity, the target gets a cyan ring), right-click for the context menu.

### Cards / List Rows
- **Corner Style:** about 22px (1.35rem).
- **Background:** white 3% at rest, 5.5% on hover.
- **Border:** frost border; a dashed frost border for empty states.
- **Internal Padding:** 16px.

### Chips
- **Style:** pill, cyan 8% fill, cyan 14% border, Pale Signal text, 12px label.

### Context Menu
- **Style:** Overlay Slate at 96%, frost border, Slate 100 text, Overlay shadow. Items highlight with a cyan 10% fill; separators are white 10%.

## Do's and Don'ts

### Do:
- **Do** use Signal Cyan only for the active, focused or primary element on a screen.
- **Do** separate surfaces with a frost border (white 8–10%) and a white-opacity fill step.
- **Do** keep the terminal on solid Terminal Ink, with no translucency behind text.
- **Do** report connection state with the 8px status dot: green live, amber waiting, red failed.
- **Do** match radius to surface size: 10px controls, 16px fields, ~22px cards, 28–32px panels.

### Don't:
- **Don't** add drop shadows to resting cards or rows; shadows are for floating layers.
- **Don't** introduce a second accent hue or a non-monospace font for chrome.
- **Don't** use colored dots, glows or tints that don't report state.
- **Don't** use raw hex in components where an existing token or Tailwind class covers it.
