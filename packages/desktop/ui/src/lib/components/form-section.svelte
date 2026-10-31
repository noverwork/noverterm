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

<fieldset class="rounded-[2px] border border-white/10 bg-rib p-4">
  <legend class="sr-only">{title}</legend>

  <div class="flex items-center gap-3">
    <div
      class="flex size-10 shrink-0 items-center justify-center rounded-[2px] border border-white/10 bg-transparent text-stencil-dim"
    >
      <Icon class="size-5" />
    </div>
    <div>
      <h2 class="text-sm font-medium text-stencil">{title}</h2>
      <p class="mt-1 text-xs text-stencil-mute">{hint}</p>
    </div>
  </div>

  <div class="mt-4 space-y-3">
    {@render children()}
  </div>
</fieldset>
