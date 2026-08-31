const FIELD_BASE =
  "h-11 rounded-2xl bg-black/20 text-white placeholder:text-slate-500";
const FIELD_OK = "border-white/10 focus-visible:border-cyan-300/40";
const FIELD_ERROR = "border-destructive";

/** Input class for a form field; `hasError` swaps the border to destructive. */
export function fieldClass(hasError: unknown = false, extra = ""): string {
  return `${FIELD_BASE} ${hasError ? FIELD_ERROR : FIELD_OK} ${extra}`;
}

export const SELECT_CLASS =
  "flex h-11 w-full appearance-none rounded-2xl border border-white/10 bg-black/20 px-3 pr-10 text-sm text-white transition-colors hover:bg-white/[0.06] focus-visible:border-cyan-300/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-300/20 disabled:cursor-not-allowed disabled:opacity-50";

/** Textarea class; `hasError` swaps the border to destructive. */
export function textareaClass(hasError: unknown = false): string {
  return hasError
    ? "flex min-h-48 w-full resize-y rounded-2xl border border-destructive bg-black/20 px-3 py-2 font-mono text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
    : "flex min-h-48 w-full resize-y rounded-2xl border border-white/10 bg-black/20 px-3 py-2 font-mono text-sm text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-300/20";
}
