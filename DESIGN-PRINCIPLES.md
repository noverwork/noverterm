# DESIGN-PRINCIPLES.md — Anti-AI-Taste Design Workflow

This document defines the workflow rules for **designing and implementing UI** in Noverterm, applicable to both humans and AI.

> **Why this document?**
> "AI taste" is fundamentally caused by **models falling back to training data averages when context is insufficient** — that average is the median of recent Dribbble + SaaS landing pages.
> The only way to avoid AI taste is to **flood context upfront, enforce reuse during, and self-audit after**, not filter after the fact.
>
> This document is the process; tokens and component specs are in [`DESIGN.md`](./DESIGN.md); implemented widgets live in [`packages/desktop/ui/src/lib/components/`](./packages/desktop/ui/src/lib/components/).

---

## A. Hard Rules (MUST / MUST NOT)

Violating these = PR rejected.

### A1. Pre-flight Checklist (Three Steps)

> Before any new screen, new component, or redesign of an existing screen, **complete these three steps in order**. Skipping = violation.

1. **Read `DESIGN.md`** — Confirm available tokens (color / type / spacing / radius).
2. **List widget inventory** — Scan `packages/desktop/ui/src/lib/components/`, note:
   - Which widgets will be used directly
   - Which widget variants will be modified (and why existing variants are insufficient)
   - Which new widgets must be added (and why existing ones cannot be extended)
3. **Verbalize the visual system** — Before writing code, articulate in text:
   - Which heading levels will be used (`headingTitle` / `headingLg` / ...)
   - Which accent colors (e.g. `cyan-300` for actions, `emerald` for success)
   - Primary radius pattern (card `rounded-[1.35rem]`, button `rounded-2xl`, pill `rounded-full`)
   - Spacing rhythm (page 16, section 12–16, card padding 16, icon gap 8)
   - **Card style follows connections-view** (flat border + subtle bg, NOT gradient + shadow)

> When working solo, write steps 1–3 in the PR description or task comment.
> When collaborating with AI, output steps 1–3 for review before writing any code.

### A2. Anti-Pattern List (MUST NOT)

These are the most common "training set averages" that AI falls back to, and directions this project deliberately avoids:

| # | MUST NOT | Why |
|---|---|---|
| 1 | Use gradient backgrounds (`LinearGradient`, CSS `gradient`) | `DESIGN.md` has no gradient tokens; appearing = inventing things |
| 2 | Use emoji as icons | Lucide is the designated icon set |
| 3 | Use framework default colors (e.g. Tailwind `blue-500` without reason) | Must use project-defined color tokens |
| 4 | Use raw hex / `#...` in widget code | Always use Tailwind utility classes or CSS variables |
| 5 | Hardcode color inside typography/style definitions | Styles should be colorless; color applied via composition |
| 6 | Create "left accent color bar" list rows | Use existing `StatusBadge` or icon patterns for status |
| 7 | Add decorative UI (floating particles, breathing glow, multi-layer shadows, "tech" decorations) | Use existing shadow tokens; do not add decorative effects |
| 8 | Add data slop (fake stats for layout, fake progress bars, meaningless icons) | Whitespace > filler; less is more |
| 9 | Add new widget when 90% of needs can be met by existing widget variants | Modify variant first; discuss in PR before duplicating |
| 10 | Add new design tokens (color / spacing / radius / font) | New tokens must be discussed in PR first |
| 11 | Hard-code inline SVG illustrations | Use placeholder; request real assets |
| 12 | Let padding/margin escape spacing scale `{4, 8, 12, 16, 24, 32, 48}` | Defined in `DESIGN.md` |
| 13 | Let radius escape radius scale `{8, 12, 14, 16, 31}` or project standard `rounded-[1.35rem]` | Defined in `DESIGN.md`; card/empty/form use `rounded-[1.35rem]` (connections-view baseline) |
| 14 | Skip `ContextMenu` / `Popover` components and write custom overlay logic | All dropdown/overlay UI must use existing shadcn-svelte primitives |

---

## B. Soft Principles (Workflow Suggestions)

Not hard rules, but skipping these will likely produce average-looking work.

### B1. Reference before designing
Before any new screen, browse [`docs/design-references/`](./docs/design-references/) or explicitly state "this flow references {app}/{screen}".

**Do not** start from adjectives ("modern", "clean", "elegant"). Adjectives = zero context = AI falls back to average.

### B2. Widget inventory first
Write out the A1-2 inventory before starting. This takes under 5 minutes but catches 80% of "reinventing the wheel" impulses.

### B3. Verbalize before coding
Articulate the system in text first. This lets you or others review direction before work begins, costing 10x less than post-hoc review.

### B4. Challenge the first output
AI's first pass is usually average. If the output looks like "Tailwind default + a brand color", **throw it away and restart with more constraints**, don't iterate on the first pass.
The only way to make "AI not look like AI" is to never give it the chance to fall back to average.

### B5. Cross-category references
Terminal/SSH apps shouldn't only look at terminal apps. Steal aesthetic DNA from excellent products across categories:
- **Writing/Reading**: Bear, Craft, Things
- **Productivity**: Linear, Height, Superhuman
- **Information Density**: Stripe Docs, Notion, Raycast
- **Emotional Warmth**: Day One, Overcast

Look at terminal apps only to **avoid clichés** (see `docs/design-references/same-category/`).

---

## C. Pre-Completion Self-Audit Checklist (Mandatory)

> **Trigger**: Before declaring any UI implementation or modification **complete** (including AI saying "done", or before committing) you must run through these 7 checks. This is an **implementation-stage gate**, not a code-review gate.

### When collaborating with AI

Before reporting "UI complete", the AI **must output pass/fail for each item** at the end of the turn, plus concrete evidence if failed (which file, which line).

Format:

```markdown
### Self-Audit (DESIGN-PRINCIPLES §C)
- [x] All colors use Tailwind classes — no raw hex
- [x] All text styles use project typography tokens
- [x] padding/margin within spacing scale `{4, 8, 12, 16, 24, 32, 48}`
- [x] radius within radius scale `{8, 12, 14, 16, 31}` or project standard `rounded-[1.35rem]` (card / empty state / form)
- [x] No gradients / emoji icons / decorative glow (cards use flat border pattern, see §E5)
- [x] No fake data, fake stats, or meaningless icons for layout padding
- [x] New/modified widgets trace back to existing shared component patterns
- [x] Card / icon / button style matches connections-view (§E), not snippets-view
```

Any failure → **fix it on the spot before declaring complete**, don't leave it for the next turn. Skipping audit and declaring complete = rules don't work.

### When working solo

Run through each UI change before committing. If something fails → fix it now, don't bank on coming back to it later.

### Exceptions

A2 anti-pattern #10 (adding tokens) or any item explicitly exempted at task start may note `(exempt: <reason>)` after the checkbox. But "forgot" or "not enough time" are **not** valid exceptions.

---

## D. Reference Asset Library

Location: [`docs/design-references/`](./docs/design-references/)

### When to browse
- Before any new screen
- Before redesigning an existing screen
- When exploring variants / visual directions

### Folder structure
- `same-category/` — Terminal/SSH app screenshots (**warning use**: avoid clichés)
- `cross-category/` — Cross-category aesthetic references (Things, Craft, Bear, Linear, Stripe...)
- `type-hierarchy/` — Typography-level screenshots (editorial, docs, IDE)
- `empty-states/` — Empty state studies (most commonly botched)
- `onboarding/` — Onboarding flow studies

### Naming convention
`{app}-{screen}.png` — e.g. `bear-settings.png`, `things-today-empty.png`.

### Adding assets
Drop in inspiration when you find it. Alongside each screenshot, optionally add `{filename}.notes.md` noting "why this screenshot, what to reference, what to avoid".
Also collect anti-patterns — put AI-taste-heavy screenshots in `same-category/` with `AVOID-` prefix.

---

## E. Gold Standard — Connections Page

`connections-view.svelte` and `connection-form.svelte` are the visual baseline for list/form screens in this project.
New screens and components should **prioritize matching this pattern** rather than inventing new ones.

### E1. Page Shell (shared by all list views)

```html
<div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6">
```

- Outer `workspace-canvas` + inner `ide-panel` is a fixed pairing
- Page padding: `px-5 py-6 lg:px-8`
- Section padding: `p-5 sm:p-6`

### E2. Page Header

```html
<div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
  <div>
    <p class="section-title text-cyan-200/70">{Category}</p>
    <h1 class="mt-2 text-2xl font-semibold tracking-tight">{Page Title}</h1>
    <p class="mt-2 text-sm text-slate-500">{One-line description.}</p>
  </div>
  <Button ... class="gap-2 self-start rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200">
    <Plus class="size-3.5" /> Add {item}
  </Button>
</div>
```

- section-title: `text-cyan-200/70` (small category label)
- Title: `text-2xl font-semibold tracking-tight`
- Subtitle: `text-sm text-slate-500`
- Divider: `border-b border-white/10 pb-5`
- Primary button: `rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200`

### E3. Error Banner

```html
<div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
```

- Always `rounded-2xl`, never `rounded-xl`

### E4. Empty State

```html
<div class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
```

- `rounded-[1.35rem]` is the project standard for empty states
- `border-dashed border-white/10`
- `bg-white/[0.025]`
- `min-h-[16rem]`

### E5. List Card (card baseline)

```html
<article class="group rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-white/14 hover:bg-white/[0.055]">
```

- **Card radius**: `rounded-[1.35rem]` (≈21.6px, project card standard)
- **Border**: `border-white/8` default → `hover:border-white/14`
- **Background**: `bg-white/[0.03]` default → `hover:bg-white/[0.055]`
- **No gradient, no shadow** — flat color + border for hierarchy
- Padding: `px-4 py-4`
- Grid: `grid gap-3 px-1 py-5 md:grid-cols-2 xl:grid-cols-3`

### E6. Card Icon

```html
<div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
  <Icon class="size-5" />
</div>
```

- Icon container: `size-10` + `rounded-2xl`
- Border: `border-cyan-300/14`
- Background: `bg-cyan-300/8` (very subtle cyan)
- Text: `text-cyan-200`
- **No glow shadow** — no `shadow-[0_0_24px_...]`

### E7. Card Action Buttons

```html
<Button variant="ghost" size="xs" class="gap-1.5 rounded-xl bg-white/[0.035] text-slate-200 hover:bg-cyan-300/10 hover:text-white">
<Button variant="ghost" size="xs" class="gap-1.5 rounded-xl text-slate-400 hover:bg-white/7 hover:text-white">
<Button variant="ghost" size="xs" class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300">
```

- Primary action: `bg-white/[0.035]` base → `hover:bg-cyan-300/10`
- Secondary action: transparent base → `hover:bg-white/7`
- Destructive: transparent base → `hover:bg-red-400/10 hover:text-red-300`
- Button radius: `rounded-xl`

### E8. Tab Bar (filter tabs)

```html
<button class="border-b-2 border-cyan-300/40 px-1 pb-2 pt-2 text-sm font-medium text-white">  <!-- active -->
<button class="border-b-2 border-transparent px-1 pb-2 pt-2 text-sm font-medium text-slate-400 transition hover:text-white">  <!-- inactive -->
```

- Active: `border-cyan-300/40` + `text-white`
- Inactive: `border-transparent` + `text-slate-400` + `hover:text-white`
- Badge: `rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] text-slate-400`

### E9. Form Container

```html
<form class="mt-5 rounded-[1.35rem] border border-cyan-300/24 bg-cyan-300/8 p-5 shadow-[0_16px_42px_rgb(34_211_238/0.08)]">
```

- Form outer frame: `rounded-[1.35rem] border border-cyan-300/24 bg-cyan-300/8`
- Cyan shadow: `shadow-[0_16px_42px_rgb(34_211_238/0.08)]` (very subtle, form focus only)
- Section card: `rounded-2xl border border-white/8 bg-white/[0.035] p-4`
- Section icon: `rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200`

### E10. Input Fields

```html
<Input class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40" />
```

- Default: `border-white/10 bg-black/20`
- Focus: `focus-visible:border-cyan-300/40`
- Error: `border-destructive`

### E11. Context Menu / Dropdown

```html
<div class="fixed z-[9999] min-w-[14rem] rounded-xl border border-white/12 bg-[#0d1117] p-1 shadow-2xl shadow-black/50">
```

- Background: `bg-[#0d1117]` (GitHub dark)
- Border: `border-white/12`
- Shadow: `shadow-2xl shadow-black/50`
- Item radius: `rounded-lg`
- Active item: `bg-cyan-300/10 text-cyan-100`
- Hover item: `hover:bg-white/6`

### E12. Differences from snippets-view (deliberately NOT followed)

snippets-view uses the following patterns, which are **NOT in the gold standard**:
- `bg-gradient-to-b from-white/[0.07] to-white/[0.025]` — gradient background (violates A2-1)
- `shadow-[0_18px_50px_rgb(0_0_0/0.16)]` — heavy shadow
- `rounded-[1.45rem]` — inconsistent with card standard `rounded-[1.35rem]`
- Icon glow: `shadow-[0_0_24px_rgb(34_211_238/0.08)]` — decorative glow (violates A2-7)

New screens should follow connections-view's flat + border pattern, not snippets-view's gradient + shadow pattern.

---

## When to Relax These Rules

- **Experimental spike** — When exploring entirely new interactions (not redesigning existing screens), the A2 anti-pattern list may be temporarily exempted, but spike results must conform to rules before merging.
- **A/B visual tests** — Deliberately creating off-brand versions to test user reactions must be explicitly marked with `// spike: anti-pattern on purpose` comment.

Otherwise, follow the rules without exception.
