<script lang="ts">
  import { Settings, X } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";

  let {
    open,
    onClose,
  }: {
    open: boolean;
    onClose: () => void;
  } = $props();
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/76 px-4 backdrop-blur-xl"
    onclick={(event) => event.target === event.currentTarget && onClose()}
    onkeydown={(event) => event.key === "Escape" && onClose()}
    role="dialog"
    aria-modal="true"
    aria-label="Settings"
    tabindex="-1"
  >
    <div
      class="ide-panel flex w-full max-w-lg flex-col overflow-hidden text-white shadow-[0_32px_100px_rgb(0_0_0/0.62)]"
    >
      <div
        class="flex shrink-0 items-center justify-between border-b border-white/10 px-6 py-4"
      >
        <div>
          <p class="section-title text-cyan-200/70">Workspace preferences</p>
          <h2 class="mt-1 text-xl font-semibold tracking-tight">Settings</h2>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          class="rounded-2xl text-slate-400 hover:bg-white/8 hover:text-white"
          onclick={onClose}
          aria-label="Close settings"
          title="Close settings"
        >
          <X class="size-4" />
        </Button>
      </div>

      <div class="px-6 py-8">
        <div
          class="rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-5 py-8 text-center"
        >
          <div
            class="mx-auto grid size-12 place-items-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200"
          >
            <Settings class="size-5" />
          </div>
          <p class="mt-4 text-sm font-semibold text-white">
            Manage SSH trust from Known Hosts
          </p>
          <p class="mx-auto mt-2 max-w-sm text-sm leading-6 text-slate-500">
            Trusted SSH fingerprints are available from the Known Hosts page in the sidebar.
          </p>
        </div>
      </div>
    </div>
  </div>
{/if}
