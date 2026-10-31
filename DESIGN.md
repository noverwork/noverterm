---
name: Noverterm
description: A native SSH terminal client marked like freight in a night port.
colors:
  port: "#101317"
  rail: "#0b0e11"
  rib: "#1a1f25"
  rib-groove: "#161a1f"
  terminal-ink: "#000000"
  stencil: "#e8e4d8"
  stencil-dim: "#c1beb5"
  stencil-mute: "#9f9d96"
  stencil-faint: "#82807b"
  hairline: "rgb(255 255 255 / 0.1)"
  seal: "#a33b28"
  seal-hover: "#b4432f"
  line-rust: "#a33b28"
  line-harbour: "#2a648f"
  line-evergreen: "#336a41"
  line-ochre: "#c9952e"
  line-ochre-hover: "#d6a443"
  ochre-text: "#e6c27a"
  line-graphite: "#2a2f36"
typography:
  headline:
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, Courier New, monospace"
    fontSize: "1.5rem"
    fontWeight: 600
    letterSpacing: "-0.025em"
  title:
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, Courier New, monospace"
    fontSize: "1.25rem"
    fontWeight: 600
    letterSpacing: "-0.025em"
  body:
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, Courier New, monospace"
    fontSize: "0.875rem"
    fontWeight: 600
  body-small:
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, Courier New, monospace"
    fontSize: "0.75rem"
    fontWeight: 600
  label:
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, Courier New, monospace"
    fontSize: "11px"
    fontWeight: 600
    letterSpacing: "0.14em"
rounded:
  corner: "2px"
  full: "9999px"
spacing:
  rail-width: "56px"
  top-bar: "36px"
  tab-height: "28px"
  tab-min: "176px"
  tab-max: "240px"
  status-line: "24px"
  field-height: "44px"
  terminal-pad: "14px"
components:
  button-primary:
    backgroundColor: "{colors.stencil}"
    textColor: "{colors.port}"
    rounded: "{rounded.corner}"
    padding: "0 16px"
  button-primary-hover:
    backgroundColor: "#ffffff"
  button-destructive:
    backgroundColor: "{colors.seal}"
    textColor: "{colors.stencil}"
    rounded: "{rounded.corner}"
    padding: "0 16px"
  button-destructive-hover:
    backgroundColor: "{colors.seal-hover}"
  button-trust:
    backgroundColor: "{colors.line-ochre}"
    textColor: "{colors.port}"
    rounded: "{rounded.corner}"
    padding: "4px 16px"
  button-trust-hover:
    backgroundColor: "{colors.line-ochre-hover}"
  button-outline:
    backgroundColor: "transparent"
    textColor: "{colors.stencil}"
    rounded: "{rounded.corner}"
    padding: "0 16px"
  input-field:
    backgroundColor: "{colors.rail}"
    textColor: "{colors.stencil}"
    rounded: "{rounded.corner}"
    height: "{spacing.field-height}"
    padding: "0 12px"
  session-tab-active:
    textColor: "{colors.stencil}"
    rounded: "{rounded.corner}"
    height: "{spacing.tab-height}"
    padding: "0 10px"
  held-seal:
    backgroundColor: "{colors.stencil}"
    textColor: "{colors.seal}"
    typography: "{typography.label}"
    padding: "0 4px"
  stencil-chip:
    backgroundColor: "{colors.rib}"
    textColor: "{colors.stencil-dim}"
    typography: "{typography.label}"
    rounded: "{rounded.corner}"
    padding: "0 6px"
  status-line:
    backgroundColor: "{colors.rail}"
    textColor: "{colors.stencil-mute}"
    typography: "{typography.label}"
    height: "{spacing.status-line}"
    padding: "0 12px"
---

# Design System: Noverterm

## Overview

**Creative North Star: "Container Stencil"**

Every session is a container end in a night port. The ground is cold slate, the tab strip is corrugated steel, and each session tab is a painted container door carrying its group's line colour and a stencilled code (`PRD·web-01`). A dropped session is sealed HELD the way a customs-held box is. The engineer reads environment by colour and code before they read the host name.

The system is dense, dark, and flat. One monospace face does everything. Every edge is a whole-pixel 1px hairline, every corner is 2px, nothing is blurred, glowing, or lifted by a soft shadow. Colour is rationed: the slate and stencil neutrals carry the chrome, and saturated colour appears only as a group's line colour, the seal of a held or destructive state, or the ochre of a trust decision. The terminal itself sits on pure black ink and is never tinted.

Motion is functional only: short opacity and border transitions on state, and a short flip when tabs reorder. Nothing animates for decoration, which keeps the client feeling as light as its claim.

**Key Characteristics:**
- Session identity is a line colour plus a stencilled `GRP·HOST` code in tracked caps.
- Exactly four line colours plus graphite for ungrouped; no fifth.
- One shape per session state, readable without colour.
- 1px hairlines, 2px corners, no soft shadows, no blur, no glass.
- Pure black terminal ink inside slate chrome.
- Sarasa Term in one weight everywhere.

## Colors

Night-port slate and off-white stencil paint, with four freight line colours and a seal red held in reserve.

### Primary
- **Stencil White** (stencil): all primary text, the active outline on tabs and rail items, focus rings, and the fill of primary buttons and the HELD seal. It is paint, not pure white; pure white appears only as the primary button's hover.

### Secondary
- **Freight Line Colours** (line-rust, line-harbour, line-evergreen, line-ochre): the group signature. Groups take them in group-name order (sorted by name, index modulo four), so neighbouring groups in the Connections list never share a colour until there are more than four groups. Rust, harbour blue, and evergreen carry stencil text; ochre carries dark port text. They fill session tabs, the swatch in the status line, and the swatch beside each group heading in Connections.
- **Graphite** (line-graphite): the line colour for ungrouped SSH sessions and local shells (coded `SSH` and `LOC`).

### Tertiary
- **Seal Red** (seal, hover seal-hover): the HELD seal's text, destructive confirm buttons, and the 60%-alpha border on error toasts and the disconnected panel. It shares its value with the rust line colour; the meaning comes from context (a seal is a stencil chip with red letters, a line is a fill).
- **Trust Ochre** (line-ochre, hover line-ochre-hover, text ochre-text): host-trust and conflict decisions. Ochre borders at 60% alpha, ochre-text for warning copy and icons, solid ochre for the confirm action.

### Neutral
- **Port Slate** (port): the workspace ground behind views and the top bar's content area; also the text colour on stencil and ochre fills.
- **Rail Black** (rail): the sidebar, status line, popovers, dialogs, fields, and the page-label cell.
- **Rib Steel** (rib, groove rib-groove): the corrugated tab strip, chips, and divider rules between shell regions.
- **Terminal Ink** (terminal-ink): the terminal surface and its padding. Pure black, never a tint.
- **Stencil Dim / Mute / Faint** (stencil-dim, stencil-mute, stencil-faint): secondary text, tracked labels and inactive icons, and placeholders, in that order of prominence. All are stencil-derived, never cool grey.
- **Hairline** (hairline): the default 1px edge on panels, chips, fields, inactive tabs, and separators. Hover steps it to white/20 or white/25.

### Named Rules
**The Four Lines Rule.** Group identity uses exactly the four line colours plus graphite, assigned by group-name order. Never add a fifth line colour or hand-pick one per screen.

**The Inactive Mix Rule.** An inactive tab mixes its line colour 55% into rib (`color-mix(in srgb, <line> 55%, rib)`) with a 1px hairline edge. Never fade the tab with opacity; the stencil text has to keep its contrast.

**The Seal Is Rare Rule.** Seal red means held, failed, or about to be destroyed. It never decorates.

## Typography

**Display Font:** Sarasa Term TC SemiBold (with Menlo, Monaco, Courier New, monospace)
**Body Font:** the same
**Label/Mono Font:** the same

**Character:** One semibold terminal face for chrome, content, and the terminal alike, so the chrome reads as part of the same machine. Hierarchy comes from size and from tracked caps, not from a second family or weight.

### Hierarchy
- **Headline** (600, 24px, tracking -0.025em): the page title of each view (SSH Connections, SSH Keys, Command Snippets).
- **Title** (600, 20px, tracking -0.025em): dialog and overlay titles.
- **Body** (600, 14px): descriptions, list rows, field text.
- **Body Small** (600, 12px): metadata such as algorithms and fingerprints.
- **Label** (600, 11px, tracking 0.14em, uppercase): stencil codes on tabs, the status line, chips, tooltips, field labels, definition terms, and column headers.

### Named Rules
**The Stencil Code Rule.** Anything that identifies a container (group code, session code, HELD) is set in the label style: 11px uppercase tracked 0.14em, joined with a middle dot (`PRD·HOST`).

**The Label Below Rule.** Tracked labels name the thing they sit on (a field, a column, a term). They never float above a heading as an eyebrow.

## Layout

A fixed shell frames every view: a 56px rail on the left (logo cell, five icon nav items at 40x28, a rule, pinned snippet shortcuts, settings at the foot), a 36px corrugated top bar (a page-label cell on rail, then session tabs), the content area, and a 24px status line at the bottom on rail with a rib top rule.

Session tabs are 28px tall, flex between 176px and 240px, and sit 4px apart with 6px inset from the strip's ends; when the strip overflows it scrolls horizontally with chevron edge indicators. Every tab has identical anatomy: code at left, state shape at right, a close control that appears on hover or when active. While a non-terminal view is open, the tab strip drops to 45% opacity and returns on hover.

Views are padded 20px (32px from the lg breakpoint) and separate their header from content with a 1px hairline rule and 20px of space. Fields are 44px tall. The terminal fills the rest on black ink with 14px padding. Density is high; spacing follows the Tailwind 4px step.

## Elevation & Depth

The system is flat. Depth is tonal: rail sits darker than port, rib sits lighter, and every boundary is a 1px hairline or a rib-coloured rule. There are no drop shadows and no blur. The only box-shadow is a 1px inset top hairline (white at 6%) on the terminal frame, which is an edge, not elevation. Dialogs and popovers separate from the page by sitting on rail with a hairline border.

### Named Rules
**The Pixel Honesty Rule.** Every edge is a whole-pixel 1px line. Nothing is soft: no drop shadows, no glows, no backdrop blur, no glass.

## Shapes

Square freight geometry. Every container, button, field, chip, tab, tooltip, menu, and dialog uses a 2px corner (the global radius scale resolves to 2px). Full rounding is reserved for the session state dots and the toggle switch track and knob. Scrollbar thumbs are square. The corrugation of the tab strip is a repeating 8px pattern: 7px of rib, then a 1px groove.

## Components

### Buttons
Solid, square, and blunt.
- **Shape:** 2px corner.
- **Primary:** stencil fill with port text; hover goes to pure white.
- **Destructive confirm:** solid seal fill with stencil text; hover seal-hover.
- **Trust confirm:** solid ochre fill with port text; hover line-ochre-hover. Used where the user accepts a host key or overwrites.
- **Outline / Cancel:** transparent with a hairline edge and stencil text; hover steps the edge to white/20 with a white/5 wash.
- **Icon delete actions:** stencil-mute icons that wash to seal at 20% on hover.

### Chips
- **Style:** rib fill, hairline edge, 2px corner, 6px horizontal padding, label type in stencil-dim (codes) or stencil-mute (counts).
- **HELD seal:** a stencil chip with seal-red letters, no border, no animation.

### Cards / Containers
- **Corner Style:** 2px.
- **Background:** rail for dialogs, popovers, and inset panels; rib for list cards and chips.
- **Shadow Strategy:** none (see Elevation & Depth).
- **Border:** 1px hairline; seal at 60% for disconnected and error panels, ochre at 60% for trust and warning panels.
- **Internal Padding:** 24-32px for dialogs and overlays, with a footer band on port separated by a hairline.

### Inputs / Fields
- **Style:** 44px tall, rail fill, hairline edge, 2px corner, stencil text, stencil-faint placeholder.
- **Focus:** the edge turns solid stencil; no glow.
- **Error / Disabled:** error swaps the edge to the destructive red; disabled drops to 50% opacity.
- **Toggle:** on is a stencil track with a port knob; off is a rail track with a hairline edge and a stencil-mute knob.

### Navigation
- **Rail items:** 40x28 icon buttons in stencil-mute; hover gains a hairline and white/5 wash; the active item has a 1px stencil outline and stencil icon. Tooltips sit on rail with a hairline, in label type.
- **Context menus:** rail with hairline edge, 2px items, white/8 focus wash.

### Session Tab (signature)
The container end. Active: full line-colour fill with a 1px stencil outline. Inactive: the line colour mixed 55% into rib with a 1px hairline edge, hover white/25. Text is stencil, or port on an active ochre tab. Border and background transition over 120ms, removed under reduced motion; reordering flips over 140ms.

State shapes, one per state:
- **Live** (connected): filled dot, 8px.
- **Waiting / connecting:** 8px ring, 2px stroke.
- **Held** (disconnected or failed): the HELD seal.

### Status Line (signature)
24px rail band: an 8x12 line-colour swatch and the active `GRP·HOST` code in stencil, then user@host:port, the state label (red when held), and on the right the open count with a held count when any are held.

### Group Heading (signature)
In Connections, each group heading carries the same 8x12 line-colour swatch, the group name, its stencil code chip, and a count chip.

## Do's and Don'ts

### Do:
- **Do** mark every session with its group's line colour and `GRP·HOST` stencil code, on the tab and in the status line.
- **Do** assign line colours from the four-colour set in group-name order, with graphite for ungrouped and local sessions.
- **Do** mix inactive tab fills 55% into rib and keep a 1px hairline edge on them.
- **Do** use filled dot for live, ring for waiting, and the HELD seal for dropped or failed.
- **Do** make destructive confirms solid seal and trust confirms solid ochre; make primary actions solid stencil with port text.
- **Do** keep every corner at 2px; use full rounding only for state dots and toggle parts.
- **Do** keep the terminal on pure black ink (#000000).

### Don't:
- **Don't** use soft shadows, glows, backdrop blur, or glass surfaces anywhere.
- **Don't** add a fifth line colour or tint a tab with opacity instead of the rib mix.
- **Don't** put eyebrow labels above headings.
- **Don't** tint the terminal background or use cool grey for secondary text; secondary text is stencil-derived.
- **Don't** animate the HELD seal or add decorative motion.
- **Don't** introduce a second typeface or weight.
