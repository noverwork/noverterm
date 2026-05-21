<script lang="ts">
  import { Copy, FileText, Pencil, Play, Plus, Trash2 } from "@lucide/svelte";

  import type { SnippetRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    snippets: SnippetRecord[];
    connections: ConnectionConfig[];
    onNew: () => void;
    onRun: (connection: ConnectionConfig, command: string) => Promise<boolean>;
    onEdit: (snippet: SnippetRecord, title: string, body: string) => Promise<void>;
    onDelete: (snippet: SnippetRecord) => Promise<void>;
  }

  let { snippets, connections, onNew, onRun, onEdit, onDelete }: Props = $props();

  let error = $state<string | null>(null);
  let pendingDeleteSnippet = $state<SnippetRecord | null>(null);
  let deletingSnippetId = $state<string | null>(null);
  let runningSnippetId = $state<string | null>(null);
  let editingSnippet = $state<SnippetRecord | null>(null);
  let editTitle = $state("");
  let editBody = $state("");
  let savingEdit = $state(false);

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

  function startEdit(snippet: SnippetRecord) {
    editingSnippet = snippet;
    editTitle = snippet.title;
    editBody = snippet.body;
    error = null;
  }

  function cancelEdit() {
    editingSnippet = null;
    editTitle = "";
    editBody = "";
  }

  async function saveEdit() {
    if (!editingSnippet || !editTitle.trim()) {
      error = "Title is required";
      return;
    }

    savingEdit = true;
    error = null;

    try {
      await onEdit(editingSnippet, editTitle.trim(), editBody);
      editingSnippet = null;
      editTitle = "";
      editBody = "";
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to save changes";
    } finally {
      savingEdit = false;
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
    const firstLine = body.split("\n")[0];
    return firstLine.length > 100 ? `${firstLine.slice(0, 100)}…` : firstLine;
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
        <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {#each snippets as snippet (snippet.id)}
            <article class="group rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-white/14 hover:bg-white/[0.055]">
              {#if editingSnippet?.id === snippet.id}
                <div class="flex flex-col gap-3">
                  <Input
                    bind:value={editTitle}
                    placeholder="Snippet title"
                    class="rounded-xl border-white/10 bg-white/[0.05] text-white placeholder:text-slate-500"
                  />
                  <textarea
                    bind:value={editBody}
                    placeholder="Command or text..."
                    rows={4}
                    class="w-full rounded-xl border border-white/10 bg-white/[0.05] px-3 py-2 font-mono text-sm text-white placeholder:text-slate-500 focus:border-cyan-300/40 focus:outline-none focus:ring-1 focus:ring-cyan-300/20 resize-none"
                  ></textarea>
                  <div class="flex items-center gap-2">
                    <Button
                      variant="default"
                      size="xs"
                      class="gap-1.5 rounded-xl bg-cyan-300 text-slate-950 hover:bg-cyan-200"
                      onclick={saveEdit}
                      disabled={savingEdit}
                    >
                      {savingEdit ? "Saving…" : "Save"}
                    </Button>
                    <Button
                      variant="ghost"
                      size="xs"
                      class="gap-1.5 rounded-xl text-slate-400 hover:bg-white/10"
                      onclick={cancelEdit}
                      disabled={savingEdit}
                    >
                      Cancel
                    </Button>
                  </div>
                </div>
              {:else}
                <div class="flex items-start gap-3">
                  <div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                    <FileText class="size-5" />
                  </div>

                  <div class="min-w-0 flex-1">
                    <p class="truncate text-sm font-medium text-white">{snippet.title}</p>
                    <p class="mt-1 text-xs text-slate-500">{snippet.host_name}</p>
                    <p class="mt-2 rounded-lg bg-black/20 px-2 py-1.5 font-mono text-[11px] text-slate-300 break-all">
                      {bodyPreview(snippet.body)}
                    </p>
                  </div>
                </div>

                <div class="mt-4 flex items-center gap-2">
                  <Button
                    variant="default"
                    size="xs"
                    class="gap-1.5 rounded-xl bg-emerald-300/20 text-emerald-200 hover:bg-emerald-300/30 hover:text-emerald-100"
                    onclick={() => handleRun(snippet)}
                    disabled={runningSnippetId === snippet.id}
                  >
                    <Play class="size-3" />
                    {runningSnippetId === snippet.id ? "Running…" : "Run"}
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    class="gap-1.5 rounded-xl bg-white/[0.035] text-slate-200 hover:bg-cyan-300/10 hover:text-white"
                    onclick={() => copyToClipboard(snippet.body)}
                  >
                    <Copy class="size-3" />
                    Copy
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    class="gap-1.5 rounded-xl text-slate-400 hover:bg-cyan-300/10 hover:text-white"
                    onclick={() => startEdit(snippet)}
                  >
                    <Pencil class="size-3" />
                    Edit
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                    onclick={() => requestDelete(snippet)}
                    disabled={deletingSnippetId === snippet.id}
                  >
                    <Trash2 class="size-3" />
                    Delete
                  </Button>
                </div>
              {/if}
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
