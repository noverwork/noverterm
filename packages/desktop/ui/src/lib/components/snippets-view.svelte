<script lang="ts">
  import { Copy, FileText, Pencil, Play, Plus, Trash2 } from "@lucide/svelte";

  import type { SnippetRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";

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

      <Button onclick={onNew} variant="default" size="sm" class="gap-2 self-start rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200">
        <Plus class="size-3.5" />
        Add snippet
      </Button>
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
      {:else}
        <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {#each snippets as snippet (snippet.id)}
            <article class="group relative rounded-[1.45rem] border border-white/10 bg-gradient-to-b from-white/[0.07] to-white/[0.025] shadow-[0_18px_50px_rgb(0_0_0/0.16)] transition hover:z-20 hover:border-cyan-300/24 hover:from-white/[0.09] hover:to-white/[0.035]">
                <div class="p-4">
                  <div class="flex items-start gap-3">
                    <div class="mt-0.5 flex size-11 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/20 bg-cyan-300/10 text-cyan-200 shadow-[0_0_24px_rgb(34_211_238/0.08)]">
                      <FileText class="size-5" />
                    </div>

                    <div class="min-w-0 flex-1">
                      <p class="truncate text-base font-semibold text-white">{snippet.title}</p>
                      <div class="mt-2 flex flex-wrap items-center gap-2 text-[11px]">
                        <span class="rounded-full border border-cyan-300/15 bg-cyan-300/10 px-2 py-0.5 font-medium text-cyan-100">
                          Runs on {snippet.host_name}
                        </span>
                        <span class="rounded-full border border-white/10 bg-white/[0.035] px-2 py-0.5 text-slate-400">
                          {commandLineCount(snippet.body)} {commandLineCount(snippet.body) === 1 ? "line" : "lines"}
                        </span>
                      </div>
                    </div>
                  </div>

                  <div class="group/preview relative mt-4 rounded-2xl border border-white/10 bg-black/25 p-3 transition hover:border-cyan-300/25 hover:bg-black/35">
                    <div class="mb-2 flex items-center justify-between gap-3">
                      <span class="text-[10px] font-semibold uppercase tracking-[0.22em] text-slate-500">Command preview</span>
                      {#if extraLineLabel(snippet.body)}
                        <span class="text-[10px] font-medium text-cyan-200/70">{extraLineLabel(snippet.body)}</span>
                      {/if}
                    </div>
                    <p class="flex min-h-6 items-start gap-2 font-mono text-xs leading-5 text-slate-200">
                      <span class="select-none text-cyan-300/70">$</span>
                      <span class="break-all">{bodyPreview(snippet.body)}</span>
                    </p>

                    <div class="pointer-events-none absolute left-0 right-0 top-[calc(100%+0.5rem)] z-30 translate-y-1 opacity-0 transition duration-150 group-hover/preview:translate-y-0 group-hover/preview:opacity-100">
                      <div class="rounded-2xl border border-cyan-300/20 bg-slate-950/95 p-3 shadow-[0_24px_70px_rgb(0_0_0/0.45)] ring-1 ring-white/8 backdrop-blur-xl">
                        <div class="mb-2 flex items-center justify-between gap-3">
                          <span class="text-[10px] font-semibold uppercase tracking-[0.22em] text-cyan-200/80">Full script</span>
                          <span class="text-[10px] text-slate-500">{commandLineCount(snippet.body)} {commandLineCount(snippet.body) === 1 ? "line" : "lines"}</span>
                        </div>
                        <pre class="max-h-64 overflow-auto whitespace-pre-wrap break-words rounded-xl bg-black/35 p-3 font-mono text-xs leading-5 text-slate-100">{snippet.body || "—"}</pre>
                      </div>
                    </div>
                  </div>
                </div>

                <div class="flex items-center gap-2 border-t border-white/8 bg-white/[0.018] px-4 py-3">
                  <Button
                    variant="default"
                    size="xs"
                    class="min-w-20 gap-1.5 rounded-xl bg-emerald-300/22 text-emerald-100 hover:bg-emerald-300/32 hover:text-white"
                    onclick={() => handleRun(snippet)}
                    disabled={runningSnippetId === snippet.id}
                  >
                    <Play class="size-3" />
                    {runningSnippetId === snippet.id ? "Running…" : "Run"}
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    class="gap-1.5 rounded-xl text-slate-300 hover:bg-cyan-300/10 hover:text-white"
                    onclick={() => copyToClipboard(snippet.body)}
                  >
                    <Copy class="size-3" />
                    Copy
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    class="gap-1.5 rounded-xl text-slate-400 hover:bg-cyan-300/10 hover:text-white"
                    onclick={() => onEdit(snippet)}
                  >
                    <Pencil class="size-3" />
                    Edit
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    class="ml-auto gap-1.5 rounded-xl text-slate-500 hover:bg-red-400/10 hover:text-red-300"
                    onclick={() => requestDelete(snippet)}
                    disabled={deletingSnippetId === snippet.id}
                  >
                    <Trash2 class="size-3" />
                    Delete
                  </Button>
                </div>
            </article>
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
