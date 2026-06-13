<script lang="ts">
  import {
    ArrowDown,
    ArrowUp,
    ArrowUpDown,
    FileQuestion,
    FileText,
    Folder,
    Link2,
    Loader2,
  } from "@lucide/svelte";
  import { tick } from "svelte";

  import type { FileEntry, FileType } from "$lib/types/sftp.js";

  type SortKey = "name" | "size" | "modified";
  type SortDirection = "asc" | "desc";

  interface Props {
    files: FileEntry[];
    selected: FileEntry | null;
    loading: boolean;
    panelId?: "local" | "remote";
    scrollKey?: string;
    onSelect: (entry: FileEntry) => void;
    onNavigate: (entry: FileEntry) => void;
    onDragStart?: (entry: FileEntry, panel: "local" | "remote") => void;
  }

  let {
    files,
    selected,
    loading,
    panelId = "local",
    scrollKey = panelId,
    onSelect,
    onNavigate,
    onDragStart,
  }: Props = $props();

  let sortBy = $state<SortKey>("name");
  let sortDir = $state<SortDirection>("asc");
  let scrollContainer: HTMLDivElement | null = $state(null);
  let rememberedScrollTop = $state(0);
  let previousScrollKey = $state<string | null>(null);

  let sortedFiles = $derived.by(() => {
    const sorted = [...files];
    const dir = sortDir === "asc" ? 1 : -1;

    sorted.sort((left, right) => {
      if (left.file_type === "Dir" && right.file_type !== "Dir") {
        return -1;
      }
      if (left.file_type !== "Dir" && right.file_type === "Dir") {
        return 1;
      }

      if (sortBy === "name") {
        return left.name.localeCompare(right.name) * dir;
      }
      if (sortBy === "size") {
        return (left.size - right.size) * dir;
      }
      const leftModified = left.modified ?? 0;
      const rightModified = right.modified ?? 0;
      return (leftModified - rightModified) * dir;
    });

    return sorted;
  });

  function isSelected(entry: FileEntry): boolean {
    return selected?.name === entry.name && selected?.file_type === entry.file_type;
  }

  function toggleSort(key: SortKey): void {
    if (sortBy === key) {
      sortDir = sortDir === "asc" ? "desc" : "asc";
    } else {
      sortBy = key;
      sortDir = "asc";
    }
  }

  function nameIndicator(key: SortKey) {
    if (sortBy !== key) {
      return ArrowUpDown;
    }
    return sortDir === "asc" ? ArrowUp : ArrowDown;
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) {
      return `${bytes} B`;
    }
    const units = ["KB", "MB", "GB", "TB"];
    let value = bytes / 1024;
    let unitIndex = 0;
    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex += 1;
    }
    return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unitIndex]}`;
  }

  function formatModified(timestamp: number | null): string {
    if (timestamp === null) {
      return "—";
    }
    const date = new Date(timestamp * 1000);
    if (Number.isNaN(date.getTime())) {
      return "—";
    }
    return date.toLocaleString();
  }

  function fileIcon(type: FileType) {
    switch (type) {
      case "Dir":
        return Folder;
      case "File":
        return FileText;
      case "Symlink":
        return Link2;
      default:
        return FileQuestion;
    }
  }

  function fileIconClass(type: FileType): string {
    switch (type) {
      case "Dir":
        return "text-cyan-200";
      case "File":
        return "text-slate-300";
      case "Symlink":
        return "text-amber-200";
      default:
        return "text-slate-500";
    }
  }

  function rowKey(entry: FileEntry, index: number): string {
    return `${entry.file_type}:${entry.name}:${index}`;
  }

  function displayName(name: string): string {
    if (name.length <= 40) {
      return name;
    }

    return `${name.slice(0, 37)}...`;
  }

  function handleRowClick(entry: FileEntry): void {
    onSelect(entry);
  }

  function handleRowDoubleClick(entry: FileEntry): void {
    if (entry.file_type === "Dir") {
      onNavigate(entry);
    }
  }

  function handleNameClick(event: MouseEvent, entry: FileEntry): void {
    event.stopPropagation();
    if (entry.file_type === "Dir") {
      onNavigate(entry);
    } else {
      onSelect(entry);
    }
  }

  function handleRowDragStart(event: DragEvent, entry: FileEntry): void {
    if (!event.dataTransfer) {
      return;
    }
    if (entry.file_type !== "File") {
      event.preventDefault();
      return;
    }
    const payload = JSON.stringify({ panel: panelId, entry });
    event.dataTransfer.setData("application/x-sftp-entry", payload);
    event.dataTransfer.setData("text/plain", `${panelId}:${entry.name}`);
    event.dataTransfer.effectAllowed = "copy";
    if (event.currentTarget instanceof HTMLElement) {
      event.currentTarget.classList.add("opacity-50");
    }
    onDragStart?.(entry, panelId);
  }

  function handleRowDragEnd(event: DragEvent): void {
    if (event.currentTarget instanceof HTMLElement) {
      event.currentTarget.classList.remove("opacity-50");
    }
  }

  function handleScroll(): void {
    if (scrollContainer) {
      rememberedScrollTop = scrollContainer.scrollTop;
    }
  }

  $effect(() => {
    const key = scrollKey;
    const isLoading = loading;
    const fileCount = sortedFiles.length;

    if (previousScrollKey === null) {
      previousScrollKey = key;
      return;
    }

    if (key !== previousScrollKey) {
      previousScrollKey = key;
      rememberedScrollTop = 0;
      void tick().then(() => {
        if (scrollContainer) {
          scrollContainer.scrollTop = 0;
        }
      });
      return;
    }

    if (!isLoading && fileCount > 0) {
      const targetScrollTop = rememberedScrollTop;
      void tick().then(() => {
        if (scrollContainer && scrollKey === key) {
          scrollContainer.scrollTop = targetScrollTop;
        }
      });
    }
  });
</script>

<div
  class="ide-panel flex h-full min-h-0 flex-col overflow-hidden p-3 text-white sm:p-4"
  data-testid="file-list"
>
  <div
    class="grid grid-cols-[minmax(0,1fr)_6rem_10rem] items-center gap-3 border-b border-white/10 px-3 py-2 text-[11px] font-medium uppercase tracking-[0.14em] text-slate-500"
  >
    <button
      type="button"
      class="flex items-center gap-1.5 text-left transition-colors hover:text-slate-200"
      onclick={() => toggleSort("name")}
      aria-label="Sort by name"
    >
      <span>Name</span>
      {#if nameIndicator("name") === ArrowUp}
        <ArrowUp class="size-3" />
      {:else if nameIndicator("name") === ArrowDown}
        <ArrowDown class="size-3" />
      {:else}
        <ArrowUpDown class="size-3" />
      {/if}
    </button>
    <button
      type="button"
      class="flex items-center justify-end gap-1.5 text-right transition-colors hover:text-slate-200"
      onclick={() => toggleSort("size")}
      aria-label="Sort by size"
    >
      <span>Size</span>
      {#if nameIndicator("size") === ArrowUp}
        <ArrowUp class="size-3" />
      {:else if nameIndicator("size") === ArrowDown}
        <ArrowDown class="size-3" />
      {:else}
        <ArrowUpDown class="size-3" />
      {/if}
    </button>
    <button
      type="button"
      class="flex items-center justify-end gap-1.5 text-right transition-colors hover:text-slate-200"
      onclick={() => toggleSort("modified")}
      aria-label="Sort by modified date"
    >
      <span>Modified</span>
      {#if nameIndicator("modified") === ArrowUp}
        <ArrowUp class="size-3" />
      {:else if nameIndicator("modified") === ArrowDown}
        <ArrowDown class="size-3" />
      {:else}
        <ArrowUpDown class="size-3" />
      {/if}
    </button>
  </div>

  <div
    class="relative min-h-0 flex-1 overflow-y-auto"
    bind:this={scrollContainer}
    onscroll={handleScroll}
    data-testid="file-list-scroll"
  >
    {#if loading && sortedFiles.length === 0}
      <div
        class="flex h-full min-h-[12rem] items-center justify-center gap-2 text-sm text-slate-400"
        data-testid="file-list-loading"
      >
        <Loader2 class="size-4 animate-spin" />
        <span>Loading…</span>
      </div>
    {:else if sortedFiles.length === 0}
      <div
        class="flex h-full min-h-[12rem] flex-col items-center justify-center gap-2 rounded-2xl border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-slate-400"
        data-testid="file-list-empty"
      >
        <Folder class="size-6 text-slate-500" aria-hidden="true" />
        <span>No files</span>
      </div>
    {:else}
      <ul class="flex flex-col gap-0.5 py-1" data-testid="file-list-rows">
        {#each sortedFiles as entry, index (rowKey(entry, index))}
          <li>
            <div
              class="grid w-full grid-cols-[minmax(0,1fr)_6rem_10rem] items-center gap-3 rounded-xl px-3 py-2 text-left text-sm transition-colors {isSelected(
                entry,
              )
                ? 'bg-cyan-300/15 text-white ring-1 ring-cyan-300/30'
                : 'text-slate-200 hover:bg-white/[0.05]'}"
              data-testid="file-row"
              data-file-name={entry.name}
              data-file-type={entry.file_type}
              data-selected={isSelected(entry)}
              data-panel={panelId}
              draggable="true"
              role="button"
              tabindex="0"
              onclick={() => handleRowClick(entry)}
              ondblclick={() => handleRowDoubleClick(entry)}
              onkeydown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  handleRowClick(entry);
                }
              }}
              ondragstart={(event) => handleRowDragStart(event, entry)}
              ondragend={handleRowDragEnd}
            >
              <span class="flex min-w-0 items-center gap-2">
                {#if entry.file_type === "Dir"}
                  <Folder class="size-4 shrink-0 {fileIconClass(entry.file_type)}" />
                {:else if entry.file_type === "File"}
                  <FileText class="size-4 shrink-0 {fileIconClass(entry.file_type)}" />
                {:else if entry.file_type === "Symlink"}
                  <Link2 class="size-4 shrink-0 {fileIconClass(entry.file_type)}" />
                {:else}
                  <FileQuestion class="size-4 shrink-0 {fileIconClass(entry.file_type)}" />
                {/if}
                {#if entry.file_type === "Dir"}
                  <button
                    type="button"
                    class="min-w-0 flex-1 cursor-pointer truncate text-left font-medium text-white transition-colors hover:text-cyan-200"
                    onclick={(event) => handleNameClick(event, entry)}
                    title={entry.name}
                  >
                    {displayName(entry.name)}
                  </button>
                {:else}
                  <button
                    type="button"
                    class="min-w-0 flex-1 cursor-pointer truncate text-left text-slate-200 transition-colors hover:text-white"
                    onclick={(event) => handleNameClick(event, entry)}
                    title={entry.name}
                  >
                    {displayName(entry.name)}
                  </button>
                {/if}
              </span>
              <span class="text-right font-mono text-xs text-slate-400">
                {entry.file_type === "Dir" ? "—" : formatSize(entry.size)}
              </span>
              <span class="text-right text-xs text-slate-400">
                {formatModified(entry.modified)}
              </span>
            </div>
          </li>
        {/each}
      </ul>

      {#if loading}
        <div
          class="pointer-events-none sticky bottom-2 ml-auto mr-2 flex w-fit items-center gap-2 rounded-full border border-cyan-300/20 bg-slate-950/90 px-3 py-1.5 text-xs text-cyan-100 shadow-lg shadow-black/30 backdrop-blur"
          data-testid="file-list-refreshing"
        >
          <Loader2 class="size-3.5 animate-spin" />
          <span>Refreshing…</span>
        </div>
      {/if}
    {/if}
  </div>
</div>
