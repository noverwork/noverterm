<script lang="ts">
  import { FileText } from "@lucide/svelte";

  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    hosts: ConnectionConfig[];
    onSave: (hostId: string, title: string, body: string) => Promise<void>;
    onCancel: () => void;
  }

  let { hosts, onSave, onCancel }: Props = $props();

  let title = $state("");
  let body = $state("");
  let hostId = $state("");
  let error = $state<string | null>(null);
  let isSaving = $state(false);

  $effect(() => {
    if (!hostId && hosts.length > 0) {
      hostId = hosts[0].id;
    }
  });

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();

    if (!title.trim()) {
      error = "Title is required";
      return;
    }

    if (!hostId) {
      error = "Please select a host";
      return;
    }

    error = null;
    isSaving = true;

    try {
      await onSave(hostId, title.trim(), body);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to save snippet";
    } finally {
      isSaving = false;
    }
  }
</script>

<div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6">
    <div class="flex items-start gap-4 border-b border-white/10 pb-5">
      <div class="flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
        <FileText class="size-5" />
      </div>

      <div class="flex-1">
        <h1 class="text-2xl font-semibold tracking-tight">New Snippet</h1>
        <p class="mt-1 text-sm text-slate-500">Create a reusable command template for a host.</p>
      </div>
    </div>

    {#if error}
      <div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {error}
      </div>
    {/if}

    <form class="mt-6 min-h-0 flex-1 space-y-5 overflow-y-auto pr-1" onsubmit={handleSubmit}>
      {#if hosts.length > 1}
        <div class="space-y-2">
          <label for="snippet-host" class="text-sm font-medium text-slate-300">Host</label>
          <select
            id="snippet-host"
            bind:value={hostId}
            class="w-full rounded-xl border border-white/10 bg-white/[0.05] px-3 py-2 text-sm text-white focus:border-cyan-300/40 focus:outline-none focus:ring-1 focus:ring-cyan-300/20"
          >
            {#each hosts as host (host.id)}
              <option value={host.id}>{host.name} ({host.host})</option>
            {/each}
          </select>
        </div>
      {:else if hosts.length === 1}
        <input type="hidden" bind:value={hostId} />
      {/if}

      <div class="space-y-2">
        <label for="snippet-title" class="text-sm font-medium text-slate-300">Title</label>
        <Input
          id="snippet-title"
          bind:value={title}
          placeholder="e.g. Restart Nginx"
          class="rounded-xl border-white/10 bg-white/[0.05] text-white placeholder:text-slate-500"
        />
      </div>

      <div class="space-y-2">
        <label for="snippet-body" class="text-sm font-medium text-slate-300">Command</label>
        <textarea
          id="snippet-body"
          bind:value={body}
          placeholder="sudo systemctl restart nginx"
          rows={6}
          class="w-full rounded-xl border border-white/10 bg-white/[0.05] px-3 py-2 font-mono text-sm text-white placeholder:text-slate-500 focus:border-cyan-300/40 focus:outline-none focus:ring-1 focus:ring-cyan-300/20 resize-none"
        ></textarea>
      </div>

      <div class="flex items-center gap-3 pt-2">
        <Button
          type="submit"
          variant="default"
          size="sm"
          class="gap-2 rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200"
          disabled={isSaving}
        >
          {isSaving ? "Saving…" : "Save snippet"}
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          class="rounded-2xl text-slate-400 hover:bg-white/10 hover:text-white"
          onclick={onCancel}
          disabled={isSaving}
        >
          Cancel
        </Button>
      </div>
    </form>
  </section>
</div>
