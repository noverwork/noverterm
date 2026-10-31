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
    "grid h-7 w-10 cursor-pointer place-items-center rounded-[2px] border transition-colors duration-150 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil";

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
  class="flex flex-col items-center gap-1"
  role="list"
  aria-label="Shortcuts"
  onpointerleave={() => (tooltip = null)}
>
  {#if onLocalTerminal}
    <button
      type="button"
      class="{BUTTON_CLASS} {localTerminalActive
        ? 'border-stencil text-stencil hover:bg-white/5'
        : 'border-transparent text-stencil-mute hover:border-white/10 hover:bg-white/5 hover:text-stencil'}"
      aria-label="Local terminal"
      onclick={onLocalTerminal}
      onpointerenter={(event) => showTooltip(event, "Local terminal")}
      onfocus={(event) => showTooltip(event, "Local terminal")}
      onblur={() => (tooltip = null)}
    >
      <Terminal class="size-3.5" />
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
          class="pointer-events-none absolute -top-[3px] left-1 right-1 h-0.5 rounded-none bg-stencil"
        ></span>
      {/if}
      <ContextMenu.Root>
        <ContextMenu.Trigger class="contents">
          <button
            type="button"
            draggable="true"
            class="{BUTTON_CLASS} border-transparent text-stencil-mute hover:border-white/10 hover:bg-white/5 hover:text-stencil {draggingId ===
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
          class="min-w-44 rounded-[2px] border-white/10 bg-rail text-stencil"
        >
          <ContextMenu.Label class="max-w-56 truncate text-stencil-mute">
            {snippet.title}
          </ContextMenu.Label>
          <ContextMenu.Separator class="bg-white/10" />
          <ContextMenu.Item
            class="cursor-pointer rounded-[2px] focus:bg-white/8 focus:text-stencil"
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
          class="pointer-events-none absolute -bottom-[3px] left-1 right-1 h-0.5 rounded-none bg-stencil"
        ></span>
      {/if}
    </div>
  {/each}

  <button
    type="button"
    class="{BUTTON_CLASS} border-dashed border-white/10 text-stencil-mute hover:border-white/20 hover:text-stencil"
    aria-label="Add shortcut"
    onclick={onAdd}
    onpointerenter={(event) => showTooltip(event, "Add shortcut")}
    onfocus={(event) => showTooltip(event, "Add shortcut")}
    onblur={() => (tooltip = null)}
  >
    <Plus class="size-3.5" />
  </button>
</div>

{#if tooltip}
  <span
    class="pointer-events-none fixed z-50 -translate-y-1/2 whitespace-nowrap rounded-[2px] border border-white/10 bg-rail px-2 py-1 text-[11px] uppercase tracking-[0.14em] text-stencil"
    style="top: {tooltip.top}px; left: {tooltip.left}px"
    aria-hidden="true"
  >
    {tooltip.label}
  </span>
{/if}
