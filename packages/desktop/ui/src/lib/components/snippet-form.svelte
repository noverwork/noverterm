<script lang="ts">
  import { ChevronDown, FileText, Server } from "@lucide/svelte";

  import type { SnippetRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import FormSection from "$lib/components/form-section.svelte";
  import FormShell from "$lib/components/form-shell.svelte";
  import { fieldClass, SELECT_CLASS, textareaClass } from "$lib/components/form-styles.js";
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
  const selectedHost = $derived(hosts.find((host) => host.id === hostId) ?? null);
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
    {#if hosts.length > 1}
      <div class="space-y-2">
        <label for="snippet-host" class="text-sm font-medium text-slate-100">Host</label>
        <div class="relative">
          <select
            id="snippet-host"
            bind:value={hostId}
            class={SELECT_CLASS}
            disabled={isSaving}
          >
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
    {:else if hosts.length === 1}
      <input type="hidden" bind:value={hostId} />
      <div class="rounded-2xl border border-white/8 bg-black/15 px-3 py-2.5">
        <p class="text-[11px] font-medium uppercase tracking-[0.16em] text-slate-500">Host</p>
        <p class="mt-1 truncate font-mono text-sm text-slate-300">
          {selectedHost?.name ?? ""} ({selectedHost?.host ?? ""})
        </p>
      </div>
    {/if}

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
</FormShell>
