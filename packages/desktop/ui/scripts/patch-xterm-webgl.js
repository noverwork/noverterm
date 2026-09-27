// Backport of xterm.js #6038 (globally monotonic atlas page versions) into
// @xterm/addon-webgl 0.19.0. See src/lib/terminal/WEBGL-GLYPH-CORRUPTION.md.
// Remove once a stable addon-webgl ships the fix.
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const require = createRequire(import.meta.url);
const pkgDir = dirname(require.resolve("@xterm/addon-webgl/package.json"));
const { version } = JSON.parse(readFileSync(join(pkgDir, "package.json"), "utf8"));
const file = join(pkgDir, "lib/addon-webgl.mjs");
const MARKER = "/*noverterm:atlas-version*/";
const NEXT = `(globalThis.__noverAtlasPageVersion=(globalThis.__noverAtlasPageVersion||0)+1)`;

// Every write to AtlasPage.version, in bundle order.
const replacements = [
  ["l=this._mergePages(o,a);l.version++;", `l=this._mergePages(o,a);l.version=${NEXT};`],
  ["s.texturePage--;n.version++}", `s.texturePage--;n.version=${NEXT}}`],
  ["_.addGlyph(m),_.version++,m}", `_.addGlyph(m),_.version=${NEXT},m}`],
  ["this._glyphs=[];this.version=0;", `this._glyphs=[];this.version=${NEXT};`],
  ["this.fixedRows.length=0,this.version++}", `this.fixedRows.length=0,this.version=${NEXT}}`],
];

let src = readFileSync(file, "utf8");
if (src.startsWith(MARKER)) process.exit(0);
if (version !== "0.19.0") {
  throw new Error(`patch-xterm-webgl: expected @xterm/addon-webgl 0.19.0, got ${version}; recheck whether the patch is still needed`);
}
for (const [from, to] of replacements) {
  if (src.split(from).length !== 2) {
    throw new Error(`patch-xterm-webgl: pattern not found exactly once: ${from}`);
  }
  src = src.replace(from, to);
}
writeFileSync(file, MARKER + src);
console.log("patch-xterm-webgl: patched @xterm/addon-webgl atlas page versions");
