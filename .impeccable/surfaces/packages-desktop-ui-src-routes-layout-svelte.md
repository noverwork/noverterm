---
version: 1
slug: "packages-desktop-ui-src-routes-layout-svelte"
primary_target: "packages/desktop/ui/src/routes/+layout.svelte"
related_targets: ["packages/desktop/ui/src/lib/components/sidebar.svelte","packages/desktop/ui/src/lib/components/shortcut-panel.svelte"]
---

# App shell

Scope: the persistent shell around every view: left rail (sidebar nav + pinned snippet shortcuts), top bar (page label + session tabs), terminal frame, and a bottom status line. Mode: Operate.

Audience and job: backend/DevOps engineers holding many SSH sessions for hours, switching between hosts and environments, keyboard first. Success: the active host and its environment read at a glance; dropped sessions are unmistakable; nothing moves from where the user's hands expect it (tabs stay on top, drag-reorder, right-click menu, sidebar order).

Constraints (user-pinned): dark; Sarasa Term mono only; must not read as a VS Code/IDE clone; must not be bland grey system UI; must not break existing habits or shortcuts.

## Direction contract

THESIS: Every session is a container end: identity stencilled as a group code, carried on its group's line colour, sealed when held. Refuses the category default of grey browser tabs over a grey host list.

OWN-WORLD: Night-port slate ground (#101317 / #0b0e11 rail), corrugated tab strip (1px vertical rib on #1a1f25), white stencil type (#e8e4d8) tracked 0.14em, line colours rust #b4432f, harbour blue #2f6f9f, evergreen #3f7d4f, ochre #c9952e, graphite #2a2f36 for ungrouped. Whole-pixel 1px hairlines only; no blur, no glass, no radial glows. Square corners (2px max).

STORY: The engineer sees which environment each tab belongs to by colour and code before reading the host, sees the live one outlined in stencil white, and sees a dropped one sealed HELD. They keep working exactly as before.

FIRST VIEWPORT: 56px slate rail left (nav items 40×28, active outlined 1px stencil white; pinned shortcuts below a rule). 36px top bar: page label cell, then tabs 176–240px wide, each a container end filled with its line colour, code `GRP·HOST` in tracked caps, state shape at right (filled dot live, ring connecting/waiting, HELD seal dropped/error). Inactive tabs at 55% opacity; active tab full opacity with 1px stencil-white outline. Terminal fills the rest on solid black ink. 24px status line at bottom: active code, user@host:port, state, session counts.

FORM: Container Stencil, position 6 of 7 on the grounded list (re-roll round 1). seed:0e673c982a0ba5b4
Raises: Pixel honesty (from Emigre): every rule a 1px whole-pixel hairline, nothing soft. One shape per state (from Labanotation): filled dot live, ring waiting, cross/seal dropped, readable without colour. Fixed anatomy (from Botanical folio): every tab identical anatomy and width rules; only state changes.
Signature move: line-colour + stencil group code on every session tab and the status line, derived from the connection's host group.
Motion grammar: none decorative; 120ms opacity/outline transitions on tab state; the HELD seal appears without animation; reduced motion removes transitions.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Recorded adaptations
- Line colours are darkened to rust #a33b28, harbour blue #2a648f, evergreen #336a41 so stencil text keeps >= 4.5:1; ochre #c9952e takes dark text when filled. No fifth colour.
- Inactive tabs mix their line colour 55% into the rib (instead of 55% opacity) so the stencil text keeps contrast; they carry a 1px white/10 edge so ungrouped tabs still read as containers.
- The rail keeps Lucide icons instead of the sketch's text codes, to preserve the user's existing habits.
- Pixel honesty applies app-wide: no soft shadows, 2px corners, terminal ink is pure black, secondary text is stencil-derived (#c1beb5 / #9f9d96 / #82807b).

## Unresolved
- Groups have no stored colour; line colour is derived deterministically from the group id. User-assigned colours are a later product decision.
