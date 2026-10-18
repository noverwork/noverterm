<script lang="ts">
  import { ChevronDown, FileText, Pin, Server } from "@lucide/svelte";

  import type { SnippetRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import FormSection from "$lib/components/form-section.svelte";
  import FormShell from "$lib/components/form-shell.svelte";
  import SnippetPinIcon, { PIN_ICON_NAMES } from "$lib/components/snippet-pin-icon.svelte";
  import { fieldClass, SELECT_CLASS, textareaClass } from "$lib/components/form-styles.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    hosts: ConnectionConfig[];
    snippet?: SnippetRecord | null;
    /// Pre-checks "Show on shortcut panel" for a new snippet.
    defaultShowOnPanel?: boolean;
    onSave: (
      hostId: string | null,
      title: string,
      body: string,
      pinIcon: string | null,
    ) => Promise<void>;
    onCancel: () => void;
  }

  let {
    hosts,
    snippet = null,
    defaultShowOnPanel = false,
    onSave,
    onCancel,
  }: Props = $props();

  let title = $state("");
  let body = $state("");
  // "" targets the local terminal.
  let hostId = $state("");
  let showOnPanel = $state(false);
  let pinIcon = $state("terminal");
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
      showOnPanel = snippet ? snippet.pin_icon !== null : defaultShowOnPanel;
      pinIcon = snippet?.pin_icon ?? "terminal";
      error = null;
    }
  });

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();

    if (!title.trim()) {
      error = "Title is required";
      return;
    }

    error = null;
    isSaving = true;

    try {
      await onSave(hostId || null, title.trim(), body, showOnPanel ? pinIcon : null);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to save snippet";
    } finally {
      isSaving = false;
    }
  }
</script>

{#snippet iconOption(value: string)}
  {@const selected = pinIcon === value}
  <button
    type="button"
    role="radio"
    aria-checked={selected}
    aria-label={value}
    title={value}
    disabled={isSaving}
    onclick={() => (pinIcon = value)}
    class="grid size-10 cursor-pointer place-items-center rounded-xl border transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/50 disabled:cursor-not-allowed disabled:opacity-50 {selected
      ? 'border-cyan-300/50 bg-cyan-300/12 text-cyan-50'
      : 'border-white/10 bg-white/[0.03] text-slate-400 hover:border-white/20 hover:bg-white/[0.06] hover:text-white'}"
  >
    <SnippetPinIcon name={value} />
  </button>
{/snippet}

<FormShell
  eyebrow="Snippets"
  title={pageTitle}
  description={pageDescription}
  formId="snippet-form"
  {submitLabel}
  {error}
  busy={isSaving}
  onsubmit={handleSubmit}
  {onCancel}
>
  <FormSection
    icon={Server}
    title="Host & title"
    hint="Choose the target host and give this snippet a name."
  >
    <div class="space-y-2">
      <label for="snippet-host" class="text-sm font-medium text-slate-100">Host</label>
      <div class="relative">
        <select
          id="snippet-host"
          bind:value={hostId}
          class={SELECT_CLASS}
          disabled={isSaving}
        >
          <option value="" class="bg-slate-900">Local Terminal</option>
          {#each hosts as host (host.id)}
            <option value={host.id} class="bg-slate-900">
              {host.name} ({host.host})
            </option>
          {/each}
        </select>
        <ChevronDown
          class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-slate-400"
        />
      </div>
    </div>

    <div class="space-y-2">
      <label for="snippet-title" class="text-sm font-medium text-slate-100">Title</label>
      <Input
        id="snippet-title"
        bind:value={title}
        placeholder="e.g. Restart Nginx"
        class={fieldClass()}
        disabled={isSaving}
      />
    </div>
  </FormSection>

  <FormSection
    icon={FileText}
    title="Command"
    hint="The command or script that will run on the selected host."
  >
    <div class="space-y-2">
      <label for="snippet-body" class="text-sm font-medium text-slate-100">Command</label>
      <textarea
        id="snippet-body"
        bind:value={body}
        placeholder="sudo systemctl restart nginx"
        rows="10"
        autocapitalize="none"
        spellcheck="false"
        class={textareaClass()}
        disabled={isSaving}
      ></textarea>
    </div>
  </FormSection>

  <FormSection
    icon={Pin}
    title="Shortcut panel"
    hint="Launch this snippet from the sidebar. Each click opens a new tab and runs the command."
  >
    <label
      class="flex cursor-pointer items-start gap-3 rounded-xl border border-white/10 bg-white/[0.03] px-3 py-3 transition-colors duration-150 hover:border-white/20"
    >
      <input
        type="checkbox"
        bind:checked={showOnPanel}
        disabled={isSaving}
        class="mt-0.5 size-4 shrink-0 cursor-pointer accent-cyan-300"
      />
      <span class="min-w-0">
        <span class="block text-sm font-medium text-slate-100">Show on shortcut panel</span>
        <span class="mt-0.5 block text-xs text-slate-400">
          Drag shortcuts in the sidebar to change their order.
        </span>
      </span>
    </label>

    {#if showOnPanel}
      <div class="space-y-2">
        <span id="snippet-icon-label" class="text-sm font-medium text-slate-100">Icon</span>
        <div
          role="radiogroup"
          aria-labelledby="snippet-icon-label"
          class="flex flex-wrap gap-2"
        >
          {#each PIN_ICON_NAMES as name (name)}
            {@render iconOption(name)}
          {/each}
        </div>
      </div>
    {/if}
  </FormSection>
</FormShell>
