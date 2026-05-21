<script lang="ts">
  import { FileText } from "@lucide/svelte";

  import type { SnippetRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    hosts: ConnectionConfig[];
    snippet?: SnippetRecord | null;
    onSave: (hostId: string, title: string, body: string) => Promise<void>;
    onCancel: () => void;
  }

  let { hosts, snippet = null, onSave, onCancel }: Props = $props();

  let title = $state("");
  let body = $state("");
  let hostId = $state("");
  let error = $state<string | null>(null);
  let isSaving = $state(false);
  let initializedSnippetId = $state<string | null>(null);

  const isEditing = $derived(snippet !== null);
  const pageTitle = $derived(isEditing ? "Edit Snippet" : "New Snippet");
  const pageDescription = $derived(
    isEditing
      ? "Update the host, title, or command for this snippet."
      : "Create a reusable command template for a host.",
  );
  const submitLabel = $derived.by(() => {
    if (isSaving) {
      return isEditing ? "Updating…" : "Saving…";
    }

    return isEditing ? "Update snippet" : "Save snippet";
  });

  $effect(() => {
    const nextSnippetId = snippet?.id ?? "new";
    if (initializedSnippetId !== nextSnippetId) {
      initializedSnippetId = nextSnippetId;
      title = snippet?.title ?? "";
      body = snippet?.body ?? "";
      hostId = snippet?.host_id ?? "";
      error = null;
    }

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
    <div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <p class="section-title text-cyan-200/70">Snippets</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">{pageTitle}</h1>
        <p class="mt-2 text-sm text-slate-500">{pageDescription}</p>
      </div>

      <Button type="button" variant="ghost" size="sm" class="rounded-2xl text-slate-300 hover:bg-white/8 hover:text-white" onclick={onCancel} disabled={isSaving}>
        Cancel
      </Button>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto pr-1">
      {#if error}
        <div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
          {error}
        </div>
      {/if}

      <form class="mt-5 rounded-[1.35rem] border border-cyan-300/24 bg-cyan-300/8 p-5 shadow-[0_16px_42px_rgb(34_211_238/0.08)]" onsubmit={handleSubmit}>
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-3">
            <div class="flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
              <FileText class="size-5" />
            </div>
            <h2 class="text-sm font-semibold text-cyan-100">Command Snippet</h2>
          </div>
          <button type="button" class="cursor-pointer text-xs text-slate-400 transition-colors hover:text-white" onclick={onCancel} disabled={isSaving}>
            Cancel
          </button>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)]">
          <div class="space-y-4">
            {#if hosts.length > 1}
              <div class="space-y-2">
                <label for="snippet-host" class="text-sm font-medium text-slate-100">Host</label>
                <select
                  id="snippet-host"
                  bind:value={hostId}
                  class="w-full rounded-xl border border-white/10 bg-black/20 px-3 py-2 text-sm text-white focus:border-cyan-300/40 focus:outline-none focus:ring-1 focus:ring-cyan-300/20"
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
              <label for="snippet-title" class="text-sm font-medium text-slate-100">Title</label>
              <Input
                id="snippet-title"
                bind:value={title}
                placeholder="e.g. Restart Nginx"
                class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                disabled={isSaving}
              />
            </div>

            <div class="flex flex-wrap items-center gap-2 pt-1">
              <Button type="submit" class="rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200" disabled={isSaving}>
                {submitLabel}
              </Button>
              <Button type="button" variant="ghost" size="sm" class="rounded-2xl text-slate-300 hover:bg-white/8 hover:text-white" onclick={onCancel} disabled={isSaving}>
                Cancel
              </Button>
            </div>
          </div>

          <div class="space-y-2">
            <label for="snippet-body" class="text-sm font-medium text-slate-100">Command</label>
            <textarea
              id="snippet-body"
              bind:value={body}
              placeholder="sudo systemctl restart nginx"
              rows="10"
              class="flex min-h-[15rem] w-full resize-none rounded-2xl border border-white/10 bg-black/20 px-3 py-2 font-mono text-sm text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-300/20"
              disabled={isSaving}
            ></textarea>
          </div>
        </div>
      </form>
    </div>
  </section>
</div>
