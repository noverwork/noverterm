const FIELD_BASE =
  "h-11 rounded-[2px] bg-rail text-stencil placeholder:text-stencil-faint";
const FIELD_OK = "border-white/10 focus-visible:border-stencil";
const FIELD_ERROR = "border-destructive";

/** Input class for a form field; `hasError` swaps the border to destructive. */
export function fieldClass(hasError: unknown = false, extra = ""): string {
  return `${FIELD_BASE} ${hasError ? FIELD_ERROR : FIELD_OK} ${extra}`;
}

export const SELECT_CLASS =
  "flex h-11 w-full appearance-none rounded-[2px] border border-white/10 bg-rail px-3 pr-10 text-sm text-stencil transition-colors hover:border-white/20 focus-visible:border-stencil focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50";

/** Textarea class; `hasError` swaps the border to destructive. */
export function textareaClass(hasError: unknown = false): string {
  return hasError
    ? "flex min-h-48 w-full resize-y rounded-[2px] border border-destructive bg-rail px-3 py-2 font-mono text-sm text-stencil placeholder:text-stencil-faint focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
    : "flex min-h-48 w-full resize-y rounded-[2px] border border-white/10 bg-rail px-3 py-2 font-mono text-sm text-stencil placeholder:text-stencil-faint focus-visible:border-stencil focus-visible:outline-none";
}
