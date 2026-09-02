<script lang="ts">
  import { Copy, FileText, Loader2, Pencil, Play, Plus, Search, Trash2 } from "@lucide/svelte";

  import type { SnippetRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    snippets: SnippetRecord[];
    connections: ConnectionConfig[];
    onNew: () => void;
    onRun: (connection: ConnectionConfig, command: string) => Promise<boolean>;
    onEdit: (snippet: SnippetRecord) => void;
    onDelete: (snippet: SnippetRecord) => Promise<void>;
  }

  let { snippets, connections, onNew, onRun, onEdit, onDelete }: Props = $props();

  let error = $state<string | null>(null);
  let pendingDeleteSnippet = $state<SnippetRecord | null>(null);
  let deletingSnippetId = $state<string | null>(null);
  let runningSnippetId = $state<string | null>(null);
  let searchQuery = $state("");

  let visibleSnippets = $derived.by(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return snippets;
    }
    return snippets.filter(
      (snippet) =>
        snippet.title.toLowerCase().includes(query) ||
        snippet.host_name.toLowerCase().includes(query) ||
        snippet.body.toLowerCase().includes(query),
    );
  });

  function requestDelete(snippet: SnippetRecord) {
    pendingDeleteSnippet = snippet;
    error = null;
  }

  async function confirmDelete() {
    if (!pendingDeleteSnippet) {
      return;
    }

    const snippet = pendingDeleteSnippet;
    error = null;
    deletingSnippetId = snippet.id;

    try {
      await onDelete(snippet);
      pendingDeleteSnippet = null;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to delete snippet";
    } finally {
      deletingSnippetId = null;
    }
  }

  async function copyToClipboard(text: string) {
    await navigator.clipboard.writeText(text);
  }

  async function handleRun(snippet: SnippetRecord) {
    if (runningSnippetId === snippet.id) {
      return;
    }

    const connection = connections.find((c) => c.id === snippet.host_id);
    if (!connection) {
      error = `Connection "${snippet.host_name}" not found. It may have been deleted.`;
      return;
    }

    error = null;
    runningSnippetId = snippet.id;

    try {
      const success = await onRun(connection, snippet.body);
      if (!success) {
        error = `Failed to connect to ${snippet.host_name}`;
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to run snippet";
    } finally {
      runningSnippetId = null;
    }
  }

  function bodyPreview(body: string): string {
    if (!body) return "—";
    const firstLine = body.split("\n").find((line) => line.trim()) ?? "";
    return firstLine.length > 96 ? `${firstLine.slice(0, 96)}…` : firstLine;
  }

  function commandLineCount(body: string): number {
    return body.split("\n").filter((line) => line.trim()).length;
  }

  function extraLineLabel(body: string): string | null {
    const hiddenLineCount = commandLineCount(body) - 1;
    if (hiddenLineCount <= 0) {
      return null;
    }

    return `+${hiddenLineCount} more ${hiddenLineCount === 1 ? "line" : "lines"}`;
  }
</script>

<div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6">
    <div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <p class="section-title text-cyan-200/70">Snippets</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">Command Snippets</h1>
        <p class="mt-2 text-sm text-slate-500">Reusable command templates organized by host.</p>
      </div>

      <div class="flex items-center gap-2 self-start">
        <div class="relative">
          <Search
            class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-slate-500"
          />
          <Input
            type="search"
            bind:value={searchQuery}
            placeholder="Search snippets"
            aria-label="Search snippets"
            class="h-8 w-48 rounded-2xl border-white/10 bg-white/[0.03] pl-8 text-sm text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40 focus-visible:ring-cyan-300/20"
          />
        </div>

        <Button onclick={onNew} variant="default" size="sm" class="gap-2 rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200">
          <Plus class="size-3.5" />
          Add snippet
        </Button>
      </div>
    </div>

    {#if error}
      <div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {error}
      </div>
    {/if}

    <div class="mt-6 min-h-0 flex-1 overflow-y-auto pr-1">
      {#if snippets.length === 0}
        <div class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
          No snippets saved yet
        </div>
      {:else if visibleSnippets.length === 0}
        <div class="rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
          No snippets match your search.
        </div>
      {:else}
        <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {#each visibleSnippets as snippet (snippet.id)}
            <ContextMenu.Root>
              <ContextMenu.Trigger class="contents">
                <div
                  role="button"
                  tabindex="0"
                  aria-label={`Run ${snippet.title}`}
                  onclick={() => void handleRun(snippet)}
                  onkeydown={(event) => {
                    if (event.key === "Enter") {
                      void handleRun(snippet);
                    }
                  }}
                  class="group cursor-pointer rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-cyan-300/30 hover:bg-white/[0.055] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40"
                >
                  <div class="flex items-start gap-3">
                    <div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                      {#if runningSnippetId === snippet.id}
                        <Loader2 class="size-5 animate-spin" />
                      {:else}
                        <FileText class="size-5" />
                      {/if}
                    </div>

                    <div class="min-w-0 flex-1">
                      <p class="truncate text-sm font-medium text-white">{snippet.title}</p>
                      <p class="mt-1 truncate text-xs text-slate-400">Runs on {snippet.host_name}</p>
                      <div class="mt-1 flex items-center gap-2 text-[10px] text-slate-500">
                        <span>{commandLineCount(snippet.body)} {commandLineCount(snippet.body) === 1 ? "line" : "lines"}</span>
                        {#if extraLineLabel(snippet.body)}
                          <span>{extraLineLabel(snippet.body)}</span>
                        {/if}
                      </div>
                    </div>

                    <div
                      class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
                    >
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        class="rounded-xl text-slate-400 hover:bg-white/8 hover:text-white"
                        aria-label={`Copy ${snippet.title}`}
                        onclick={(event) => {
                          event.stopPropagation();
                          void copyToClipboard(snippet.body);
                        }}
                      >
                        <Copy class="size-3.5" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        class="rounded-xl text-slate-400 hover:bg-white/8 hover:text-white"
                        aria-label={`Edit ${snippet.title}`}
                        onclick={(event) => {
                          event.stopPropagation();
                          onEdit(snippet);
                        }}
                      >
                        <Pencil class="size-3.5" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        class="rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                        aria-label={`Delete ${snippet.title}`}
                        disabled={deletingSnippetId === snippet.id}
                        onclick={(event) => {
                          event.stopPropagation();
                          requestDelete(snippet);
                        }}
                      >
                        <Trash2 class="size-3.5" />
                      </Button>
                    </div>
                  </div>

                  <div class="group/preview relative mt-3 rounded-lg bg-black/20 px-2 py-1.5 font-mono text-[11px] text-slate-300 transition hover:bg-black/30">
                    <p class="break-all">
                      <span class="select-none text-cyan-300/70">$</span>
                      {bodyPreview(snippet.body)}
                    </p>

                    <div class="pointer-events-none absolute left-0 right-0 top-[calc(100%+0.5rem)] z-30 translate-y-1 opacity-0 transition duration-150 group-hover/preview:translate-y-0 group-hover/preview:opacity-100">
                      <div class="rounded-2xl border border-white/10 bg-slate-950/95 p-3 shadow-[0_24px_70px_rgb(0_0_0/0.45)] ring-1 ring-cyan-300/10 backdrop-blur-xl">
                        <div class="mb-2 flex items-center justify-between gap-3 font-sans">
                          <span class="text-[10px] font-semibold uppercase tracking-[0.2em] text-slate-500">Full script</span>
                          <span class="text-[10px] text-slate-500">{commandLineCount(snippet.body)} {commandLineCount(snippet.body) === 1 ? "line" : "lines"}</span>
                        </div>
                        <pre class="max-h-64 overflow-auto whitespace-pre-wrap break-words rounded-xl bg-black/35 p-3 font-mono text-xs leading-5 text-slate-100">{snippet.body || "—"}</pre>
                      </div>
                    </div>
                  </div>
                </div>
              </ContextMenu.Trigger>

              <ContextMenu.Content
                class="min-w-44 border-white/10 bg-slate-950/96 text-slate-100 shadow-2xl shadow-black/45"
              >
                <ContextMenu.Label class="max-w-56 truncate text-slate-400">
                  {snippet.title}
                </ContextMenu.Label>
                <ContextMenu.Separator class="bg-white/10" />
                <ContextMenu.Item
                  class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                  disabled={runningSnippetId === snippet.id}
                  onclick={() => void handleRun(snippet)}
                >
                  <Play class="size-3.5" />
                  Run
                </ContextMenu.Item>
                <ContextMenu.Item
                  class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                  onclick={() => void copyToClipboard(snippet.body)}
                >
                  <Copy class="size-3.5" />
                  Copy
                </ContextMenu.Item>
                <ContextMenu.Item
                  class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                  onclick={() => onEdit(snippet)}
                >
                  <Pencil class="size-3.5" />
                  Edit
                </ContextMenu.Item>
                <ContextMenu.Separator class="bg-white/10" />
                <ContextMenu.Item
                  class="cursor-pointer text-red-300 focus:bg-red-400/10 focus:text-red-200"
                  disabled={deletingSnippetId === snippet.id}
                  onclick={() => requestDelete(snippet)}
                >
                  <Trash2 class="size-3.5" />
                  Delete
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Root>
          {/each}
        </div>
      {/if}
    </div>
  </section>
</div>

<DeleteConfirmDialog
  open={pendingDeleteSnippet !== null}
  title="Delete snippet?"
  description="This action cannot be undone."
  itemName={pendingDeleteSnippet?.title}
  confirmLabel="Delete snippet"
  isDeleting={deletingSnippetId !== null}
  onConfirm={confirmDelete}
  onCancel={() => (pendingDeleteSnippet = null)}
/>
