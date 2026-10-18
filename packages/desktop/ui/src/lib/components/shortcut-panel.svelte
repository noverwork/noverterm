<script lang="ts">
  import { Pencil, PinOff, Plus, Terminal } from "@lucide/svelte";

  import type { SnippetRecord } from "$lib/api/types.js";
  import SnippetPinIcon from "$lib/components/snippet-pin-icon.svelte";
  import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
  import { moveToInsertionIndex } from "$lib/reorder.js";

  let {
    snippets,
    localTerminalActive = false,
    onLocalTerminal,
    onLaunch,
    onReorder,
    onEdit,
    onUnpin,
    onAdd,
  }: {
    snippets: SnippetRecord[];
    localTerminalActive?: boolean;
    onLocalTerminal?: () => void;
    onLaunch: (snippet: SnippetRecord) => void;
    onReorder: (ids: string[]) => void;
    onEdit: (snippet: SnippetRecord) => void;
    onUnpin: (snippet: SnippetRecord) => void;
    onAdd: () => void;
  } = $props();

  const BUTTON_CLASS =
    "grid size-10 cursor-pointer place-items-center rounded-2xl border transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/50";

  let draggingId = $state<string | null>(null);
  // Insertion index in `snippets` (0..length) where the dragged shortcut would land.
  let dropIndex = $state<number | null>(null);
  // The panel scrolls, which would clip CSS tooltips, so one fixed tooltip follows the hover.
  let tooltip = $state<{ label: string; top: number; left: number } | null>(null);

  function showTooltip(event: PointerEvent | FocusEvent, label: string) {
    if (draggingId) return;
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    tooltip = { label, top: rect.top + rect.height / 2, left: rect.right + 8 };
  }

  function shortcutLabel(snippet: SnippetRecord): string {
    return `${snippet.title} · ${snippet.host_name}`;
  }

  function handleDragStart(event: DragEvent, snippet: SnippetRecord) {
    draggingId = snippet.id;
    tooltip = null;
    if (event.dataTransfer) {
      event.dataTransfer.setData("text/plain", snippet.id);
      event.dataTransfer.effectAllowed = "move";
    }
  }

  function handleDragOver(event: DragEvent, index: number) {
    if (!draggingId) return;
    event.preventDefault();
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    dropIndex = event.clientY < rect.top + rect.height / 2 ? index : index + 1;
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    const from = snippets.findIndex((snippet) => snippet.id === draggingId);
    const to = dropIndex;
    resetDrag();
    if (from === -1 || to === null) return;

    const ids = moveToInsertionIndex(
      snippets.map((snippet) => snippet.id),
      from,
      to,
    );
    if (ids) onReorder(ids);
  }

  function resetDrag() {
    draggingId = null;
    dropIndex = null;
  }
</script>

<div
  class="flex flex-col items-center gap-2"
  role="list"
  aria-label="Shortcuts"
  onpointerleave={() => (tooltip = null)}
>
  {#if onLocalTerminal}
    <button
      type="button"
      class="{BUTTON_CLASS} {localTerminalActive
        ? 'border-emerald-300/24 bg-emerald-300/12 text-emerald-100 hover:border-emerald-300/35 hover:bg-emerald-300/16 hover:text-white'
        : 'border-emerald-300/12 bg-emerald-300/[0.045] text-emerald-200/85 hover:border-emerald-300/22 hover:bg-emerald-300/8 hover:text-emerald-50'}"
      aria-label="Local terminal"
      onclick={onLocalTerminal}
      onpointerenter={(event) => showTooltip(event, "Local terminal")}
      onfocus={(event) => showTooltip(event, "Local terminal")}
      onblur={() => (tooltip = null)}
    >
      <Terminal class="size-4" />
    </button>
  {/if}

  {#each snippets as snippet, index (snippet.id)}
    <div
      role="listitem"
      class="relative"
      ondragover={(event) => handleDragOver(event, index)}
      ondrop={handleDrop}
    >
      {#if dropIndex === index}
        <span
          class="pointer-events-none absolute -top-[5px] left-1 right-1 h-0.5 rounded-full bg-cyan-300"
        ></span>
      {/if}
      <ContextMenu.Root>
        <ContextMenu.Trigger class="contents">
          <button
            type="button"
            draggable="true"
            class="{BUTTON_CLASS} border-white/10 bg-white/[0.035] text-slate-300 hover:border-cyan-300/30 hover:bg-cyan-300/8 hover:text-cyan-50 {draggingId ===
            snippet.id
              ? 'opacity-40'
              : ''}"
            aria-label={shortcutLabel(snippet)}
            onclick={() => onLaunch(snippet)}
            ondragstart={(event) => handleDragStart(event, snippet)}
            ondragend={resetDrag}
            onpointerenter={(event) => showTooltip(event, shortcutLabel(snippet))}
            onfocus={(event) => showTooltip(event, shortcutLabel(snippet))}
            onblur={() => (tooltip = null)}
          >
            <SnippetPinIcon name={snippet.pin_icon ?? ""} />
          </button>
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
            onclick={() => onEdit(snippet)}
          >
            <Pencil class="size-3.5" />
            Edit shortcut
          </ContextMenu.Item>
          <ContextMenu.Item
            class="cursor-pointer text-red-300 focus:bg-red-400/10 focus:text-red-200"
            onclick={() => onUnpin(snippet)}
          >
            <PinOff class="size-3.5" />
            Remove from panel
          </ContextMenu.Item>
        </ContextMenu.Content>
      </ContextMenu.Root>
      {#if dropIndex === snippets.length && index === snippets.length - 1}
        <span
          class="pointer-events-none absolute -bottom-[5px] left-1 right-1 h-0.5 rounded-full bg-cyan-300"
        ></span>
      {/if}
    </div>
  {/each}

  <button
    type="button"
    class="{BUTTON_CLASS} border-dashed border-white/12 text-slate-500 hover:border-cyan-300/35 hover:text-cyan-100"
    aria-label="Add shortcut"
    onclick={onAdd}
    onpointerenter={(event) => showTooltip(event, "Add shortcut")}
    onfocus={(event) => showTooltip(event, "Add shortcut")}
    onblur={() => (tooltip = null)}
  >
    <Plus class="size-4" />
  </button>
</div>

{#if tooltip}
  <span
    class="pointer-events-none fixed z-50 -translate-y-1/2 whitespace-nowrap rounded-lg border border-white/10 bg-slate-950/95 px-2 py-1 text-[11px] font-medium text-slate-100 shadow-[0_8px_24px_rgb(0_0_0/0.45)] backdrop-blur"
    style="top: {tooltip.top}px; left: {tooltip.left}px"
    aria-hidden="true"
  >
    {tooltip.label}
  </span>
{/if}
