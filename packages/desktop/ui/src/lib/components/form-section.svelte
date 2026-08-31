<script lang="ts">
  import type { LucideIcon } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  let {
    icon: Icon,
    title,
    hint,
    children,
  }: {
    icon: LucideIcon;
    title: string;
    hint: string;
    children: Snippet;
  } = $props();
</script>

<fieldset class="rounded-[1.35rem] border border-white/8 bg-white/[0.03] p-4">
  <legend class="sr-only">{title}</legend>

  <div class="flex items-center gap-3">
    <div
      class="flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200"
    >
      <Icon class="size-5" />
    </div>
    <div>
      <h2 class="text-sm font-medium text-white">{title}</h2>
      <p class="mt-1 text-xs text-slate-400">{hint}</p>
    </div>
  </div>

  <div class="mt-4 space-y-3">
    {@render children()}
  </div>
</fieldset>
