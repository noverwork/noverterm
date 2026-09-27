# WebGL glyph corruption in the terminal

## Symptom

Some characters in a terminal render as fragments of other glyphs (for
example `m` shows as `⸢` in `Volumes`, `noverterm`, `limits`; `(`, `)`, `·`
are also broken). CJK-heavy output gets much worse. Copying the text out of
the terminal gives the correct characters, so the buffer is fine and only the
rendering is wrong.

- Switching to another session tab and back made it **worse**.
- Resizing the window **fixed** it.

## Diagnosis (2026-09-27)

`@xterm/addon-webgl` 0.19 shares one texture atlas between all terminals whose
renderer config (font, size, theme, DPR, ...) is identical — see the
`ownedBy` cache in `addon-webgl.mjs`. Every session tab uses the same config,
so all tabs share one atlas.

`WebglAddon.clearTextureAtlas()` empties that shared atlas, but only the
calling terminal clears its render model (the cached per-cell atlas
coordinates). Every other terminal keeps coordinates that now point at
whatever glyph is later rasterized into that slot.

`refresh()` in `packages/desktop/ui/src/lib/terminal/xterm.ts` called
`clearTextureAtlas()` on every tab reveal (added in `4c9112b`, on the theory
that the GPU reclaims the atlas while the window is idle). So each tab switch
corrupted the other tabs.

Resize fixes it because it rebuilds the terminal's model against the current
atlas without touching the atlas itself. That shows the atlas contents were
fine and the stale model was the problem, which also argues against the
"GPU reclaimed the atlas" theory (a resize would not re-upload a lost
texture).

## Fix

Removed the `clearTextureAtlas()` call from `refresh()`; it now only calls
`terminal.refresh(0, rows - 1)`.

## Upstream status (checked 2026-09-27)

Both are known `@xterm/addon-webgl` 0.19.0 defects, fixed upstream only on the
0.20.0 beta line (`0.20.0-beta.300`, needs `@xterm/xterm` `6.1.0-beta`). No
stable release yet (`latest` is still `@xterm/xterm` 6.0.0 / addon 0.19.0).

1. xterm.js #6014 — `clearTextureAtlas()` wipes the shared atlas without
   clearing sibling renderers' models. **Worked around here** by no longer
   calling it.
2. xterm.js #6038 — atlas page merge corruption. When the atlas hits the
   texture-unit cap (16 on WKWebView, i.e. our macOS Tauri webview) it merges
   4 pages into one. Page version counters are per page, so the merged page
   can land in a slot whose old page had the same version, the GPU upload is
   skipped, and the slot keeps a stale bitmap. Triggered by many unique glyphs
   (CJK-heavy sessions). **Backported** (2026-09-27, after corruption came
   back in a CJK-heavy session with the first fix in place):
   `scripts/patch-xterm-webgl.js` runs on `postinstall` and rewrites every
   `AtlasPage.version` write in `lib/addon-webgl.mjs` to draw from one global
   counter, so a page moved into a slot always differs from the texture bound
   there. The script refuses to run on any version other than 0.19.0 and
   fails if a pattern is not found exactly once.

   The page-overflow half of #6038 (more pages than bound textures) is not
   backported.

Upstream tracking: #6014 is fixed by PR #6055; #6038 is still open (draft PR
#6033); both are on the 7.0.0 milestone.

Do not jump to the beta line casually: `@xterm/xterm` `6.1.0-beta.304` has a
regression (xterm.js #6154) where `resize()` calls `WriteBuffer.flushSync()`
and re-applies already-parsed output. Apps that replay a transcript and then
resize (as we do) leaked terminal query replies into the PTY — the exact
thing `terminal-query-replies.test.ts` guards against.

References:
- https://github.com/omnigent-ai/omnigent/issues/7388 (summary of both bugs)
- https://github.com/xiaolai/vmark/pull/1430 (backport of #6038 via package
  patch: one global monotonic `nextAtlasPageVersion()` for all page versions)
- https://github.com/hay-kot/hive-desktop/issues/337

## If it still happens

Both fixes are based on reading the addon source, not on a reproduced test.
If corruption comes back:

1. Note what happened just before (tab switch, sleep/wake, window occluded,
   font/theme change, WebGL context loss warning `[xterm:webgl] context lost`
   in the console).
2. Check whether a resize still fixes it. If yes, the model is stale again:
   look for any other path that clears or mutates the shared atlas.
3. Confirm the #6038 patch is actually applied: `lib/addon-webgl.mjs` must
   start with `/*noverterm:atlas-version*/`. `cargo make frontend:install`
   skips `npm install` when `node_modules` exists, so run
   `node scripts/patch-xterm-webgl.js` by hand after a fresh checkout of this
   change, and delete `node_modules/.vite` so the dev server re-bundles.
4. If it is patched and still happens, look at the rest of #6038 (page
   overflow, merge-count guard — see omnigent PR #7397) or fall back to the
   DOM renderer.
5. If you ever need to clear the atlas again, clear it on **all** live
   terminals together — keep a module-level `Set` of `WebglAddon` instances
   and call `clearTextureAtlas()` on each, so every terminal drops its model at
   the same time.
