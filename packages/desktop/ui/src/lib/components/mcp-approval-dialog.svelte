<script lang="ts">
  import { ShieldAlert } from "@lucide/svelte";

  import type { McpDecision } from "../../bindings.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { t } from "$lib/i18n/index.svelte.js";
  import { approvals, respond } from "$lib/mcp/mcp-events.svelte.js";

  const request = $derived(approvals[0]);

  function decide(decision: McpDecision) {
    if (request) void respond(request, decision);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (request && event.key === "Escape") {
      event.preventDefault();
      decide("deny");
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if request}
  <div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/70 px-4 py-6">
    <div
      class="w-full max-w-lg overflow-hidden rounded-[2px] border border-[#c9952e]/60 bg-rail text-stencil"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="mcp-approval-title"
    >
      <div class="flex items-center gap-3 border-b border-white/10 px-6 py-5">
        <ShieldAlert class="size-5 shrink-0 text-[#e6c27a]" />
        <h2 id="mcp-approval-title" class="text-lg font-semibold tracking-tight">
          {t("AI wants to run a command on {host}", { host: request.host_name })}
        </h2>
      </div>

      <div class="space-y-3 px-6 py-5">
        <p class="font-mono text-xs text-stencil-mute">{request.target}</p>
        <pre
          class="max-h-64 overflow-auto whitespace-pre-wrap rounded-[2px] border border-white/10 bg-port px-4 py-3 font-mono text-sm text-stencil">{request.command}</pre>
        {#if approvals.length > 1}
          <p class="text-xs text-stencil-mute">
            {t("{count} more waiting", { count: approvals.length - 1 })}
          </p>
        {/if}
      </div>

      <div class="flex flex-wrap items-center justify-end gap-3 border-t border-white/10 bg-port px-6 py-4">
        <Button
          variant="outline"
          class="rounded-[2px] border-white/10 bg-transparent px-4 text-stencil hover:border-white/20 hover:bg-white/5"
          onclick={() => decide("deny")}
        >
          {t("Deny")}
        </Button>
        <Button
          variant="outline"
          class="rounded-[2px] border-white/10 bg-transparent px-4 text-stencil hover:border-white/20 hover:bg-white/5"
          onclick={() => decide("allow_host")}
        >
          {t("Always allow on this host")}
        </Button>
        <Button
          class="rounded-[2px] bg-[#c9952e] px-4 text-port hover:bg-[#d6a443]"
          onclick={() => decide("allow")}
        >
          {t("Allow")}
        </Button>
      </div>
    </div>
  </div>
{/if}
