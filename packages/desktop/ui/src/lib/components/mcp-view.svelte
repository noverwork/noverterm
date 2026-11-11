<script lang="ts">
  import { Check, Copy, Eye, EyeOff, RefreshCw } from "@lucide/svelte";

  import { commands, type McpSettings, type McpStatus } from "../../bindings.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { t } from "$lib/i18n/index.svelte.js";
  import {
    activity,
    activityLabel,
    activityTone,
    aiAccess,
  } from "$lib/mcp/mcp-events.svelte.js";

  type Client = "claude-code" | "cursor" | "vscode" | "claude-desktop";

  const CLIENTS: { id: Client; label: string; file: string | null }[] = [
    { id: "claude-code", label: "Claude Code", file: null },
    { id: "cursor", label: "Cursor", file: "~/.cursor/mcp.json" },
    { id: "vscode", label: "VS Code", file: ".vscode/mcp.json" },
    { id: "claude-desktop", label: "Claude Desktop", file: "claude_desktop_config.json" },
  ];

  const TOOLS: [name: string, description: string][] = [
    ["list_hosts", "Lists saved hosts."],
    ["run_command", "Runs a command on a host."],
    ["run_command_multi", "Runs a command on several hosts at once."],
    ["upload_file", "Uploads a file to a host."],
    ["download_file", "Downloads a file from a host."],
    ["list_dir", "Lists a directory on a host."],
    ["read_file", "Reads a text file on a host."],
    ["write_file", "Writes a text file on a host."],
    ["list_snippets", "Lists saved snippets."],
    ["run_snippet", "Runs a saved snippet on a host."],
    ["list_port_forwards", "Lists running port forwards."],
    ["start_port_forward", "Opens a port forward through a host."],
    ["stop_port_forward", "Stops a port forward the AI opened."],
    ["start_command", "Starts a long-running or interactive command."],
    ["read_command", "Reads a started command's new output."],
    ["write_command", "Sends input to a started command."],
    ["stop_command", "Stops a started command."],
    ["list_terminals", "Lists open terminal tabs."],
    ["read_terminal", "Reads a terminal tab's recent output."],
  ];

  const MASK = "••••••••";

  let status = $state<McpStatus | null>(null);
  let port = $state(47321);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let client = $state<Client>("claude-code");
  let reveal = $state(false);
  let copied = $state<string | null>(null);
  let confirmingRegenerate = $state(false);

  const running = $derived(status?.running && status.url && status.token ? status : null);
  const currentClient = $derived(CLIENTS.find((entry) => entry.id === client)!);

  $effect(() => {
    void commands.mcpStatus().then((result) => {
      if (result.status === "ok") setStatus(result.data);
    });
  });

  function setStatus(next: McpStatus) {
    status = next;
    port = next.settings.port;
    error = next.error;
  }

  async function configure(change: Partial<McpSettings>) {
    if (!status) return;
    saving = true;
    error = null;
    const result = await commands.mcpConfigure({ ...status.settings, ...change });
    if (result.status === "ok") {
      setStatus(result.data);
      aiAccess.localTerminal = result.data.settings.allow_local_terminal ?? false;
    } else error = result.error;
    saving = false;
  }

  async function regenerateToken() {
    confirmingRegenerate = false;
    saving = true;
    error = null;
    const result = await commands.mcpRegenerateToken();
    if (result.status === "ok") setStatus(result.data);
    else error = result.error;
    saving = false;
  }

  function snippet(target: Client, url: string, token: string): string {
    const auth = `Bearer ${token}`;
    const json = (value: unknown) => JSON.stringify(value, null, 2);
    switch (target) {
      case "claude-code":
        return `claude mcp add --transport http noverterm ${url} --header "Authorization: ${auth}"`;
      case "cursor":
        return json({ mcpServers: { noverterm: { url, headers: { Authorization: auth } } } });
      case "vscode":
        return json({
          servers: { noverterm: { type: "http", url, headers: { Authorization: auth } } },
        });
      case "claude-desktop":
        return json({
          mcpServers: {
            noverterm: {
              command: "npx",
              args: ["-y", "mcp-remote", url, "--header", "Authorization:${AUTH_HEADER}"],
              env: { AUTH_HEADER: auth },
            },
          },
        });
    }
  }

  async function copy(key: string, text: string) {
    await navigator.clipboard.writeText(text);
    copied = key;
    setTimeout(() => (copied = null), 2000);
  }
</script>

{#snippet copyButton(key: string, text: string)}
  <Button
    variant="ghost"
    size="sm"
    class="h-6 gap-1.5 rounded-[2px] px-2 text-xs text-stencil-mute hover:bg-white/10 hover:text-stencil"
    onclick={() => void copy(key, text)}
  >
    {#if copied === key}
      <Check class="size-3 text-emerald-400" />
    {:else}
      <Copy class="size-3" />
    {/if}
    {t("Copy")}
  </Button>
{/snippet}

<div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-stencil sm:p-6">
    <div class="flex items-center gap-4 border-b border-white/10 pb-5">
      <h1 class="flex-1 text-2xl font-semibold tracking-tight">MCP</h1>
    </div>

    <div class="mt-6 min-h-0 flex-1 space-y-6 overflow-y-auto pr-1">
      <div class="grid grid-cols-[8rem_1fr] items-center gap-x-3 gap-y-3 text-sm">
        <label for="mcp-port" class="text-stencil-mute">{t("Port")}</label>
        <div class="flex items-center gap-2">
          <input
            id="mcp-port"
            type="number"
            min="1024"
            max="65535"
            class="h-8 w-28 rounded-[2px] border border-white/10 bg-rail px-2 text-sm text-stencil focus:border-stencil focus:outline-none"
            bind:value={port}
            disabled={saving}
          />
          {#if status && port !== status.settings.port}
            <Button
              variant="outline"
              size="sm"
              class="rounded-[2px] border-white/10 bg-transparent text-stencil hover:border-white/20 hover:bg-white/5"
              onclick={() => void configure({ port })}
              disabled={saving}
            >
              {t("Apply")}
            </Button>
          {/if}
        </div>

        <span class="text-stencil-mute">{t("Local terminal")}</span>
        <div class="flex items-center gap-3">
          <button
            type="button"
            role="switch"
            aria-checked={status?.settings.allow_local_terminal ?? false}
            aria-label={t("Let AI read local terminals without asking")}
            onclick={() => void configure({ allow_local_terminal: !status?.settings.allow_local_terminal })}
            disabled={saving || !status}
            class={status?.settings.allow_local_terminal
              ? "relative h-6 w-11 shrink-0 cursor-pointer rounded-full bg-[#c9952e] transition-colors disabled:cursor-not-allowed disabled:opacity-50"
              : "relative h-6 w-11 shrink-0 cursor-pointer rounded-full border border-white/10 bg-rail transition-colors hover:border-white/20 disabled:cursor-not-allowed disabled:opacity-50"}
          >
            <span
              class={status?.settings.allow_local_terminal
                ? "absolute left-0.5 top-0.5 size-5 translate-x-5 rounded-full bg-port transition-transform"
                : "absolute left-0.5 top-0.5 size-5 rounded-full bg-stencil-mute transition-transform"}
            ></span>
          </button>
          <span class="text-xs text-stencil-mute">
            {status?.settings.allow_local_terminal
              ? t("AI reads local terminals without asking")
              : t("Ask before AI reads a local terminal")}
          </span>
        </div>

        {#if running}
          <span class="text-stencil-mute">URL</span>
          <div class="flex items-center gap-2">
            <code class="font-mono text-xs text-stencil">{running.url}</code>
            {@render copyButton("url", running.url!)}
          </div>

          <span class="text-stencil-mute">Token</span>
          <div class="flex items-center gap-2">
            <code class="truncate font-mono text-xs text-stencil">
              {reveal ? running.token : MASK}
            </code>
            <Button
              variant="ghost"
              size="sm"
              class="h-6 rounded-[2px] px-2 text-stencil-mute hover:bg-white/10 hover:text-stencil"
              aria-label={reveal ? t("Hide") : t("Show")}
              onclick={() => (reveal = !reveal)}
            >
              {#if reveal}<EyeOff class="size-3" />{:else}<Eye class="size-3" />{/if}
            </Button>
            {@render copyButton("token", running.token!)}
            {#if confirmingRegenerate}
              <span class="text-xs text-stencil-mute">
                {t("Clients using the current token will stop working. Regenerate it?")}
              </span>
              <Button
                variant="ghost"
                size="sm"
                class="h-6 rounded-[2px] px-2 text-xs text-stencil-mute hover:bg-white/10 hover:text-stencil"
                onclick={() => (confirmingRegenerate = false)}
              >
                {t("Cancel")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="h-6 rounded-[2px] px-2 text-xs text-red-300 hover:bg-white/10"
                onclick={() => void regenerateToken()}
                disabled={saving}
              >
                {t("Regenerate")}
              </Button>
            {:else}
              <Button
                variant="ghost"
                size="sm"
                class="h-6 gap-1.5 rounded-[2px] px-2 text-xs text-stencil-mute hover:bg-white/10 hover:text-stencil"
                onclick={() => (confirmingRegenerate = true)}
                disabled={saving}
              >
                <RefreshCw class="size-3" />
                {t("Regenerate")}
              </Button>
            {/if}
          </div>
        {/if}
      </div>

      {#if error}
        <p class="text-xs text-red-300" role="alert">{error}</p>
      {/if}

      {#if running}
        <div>
          <h2 class="section-title">{t("Connect a client")}</h2>
          <div class="mt-3 flex flex-wrap gap-1" role="tablist">
            {#each CLIENTS as entry (entry.id)}
              <button
                type="button"
                role="tab"
                aria-selected={client === entry.id}
                class="h-7 rounded-[2px] border px-3 text-xs transition-colors {client === entry.id
                  ? 'border-stencil text-stencil'
                  : 'border-white/10 text-stencil-mute hover:border-white/20 hover:text-stencil'}"
                onclick={() => (client = entry.id)}
              >
                {entry.label}
              </button>
            {/each}
          </div>
          <div class="mt-3 flex items-center justify-between gap-2">
            <code class="font-mono text-xs text-stencil-mute">{currentClient.file ?? t("Terminal")}</code>
            {@render copyButton("snippet", snippet(client, running.url!, running.token!))}
          </div>
          <pre
            class="mt-1 overflow-x-auto rounded-[2px] border border-white/10 bg-port px-3 py-2 font-mono text-[11px] text-stencil-dim">{snippet(
              client,
              running.url!,
              reveal ? running.token! : MASK,
            )}</pre>
        </div>

        <div>
          <h2 class="section-title">{t("Tools")}</h2>
          <dl class="mt-3 grid grid-cols-[10rem_1fr] gap-x-3 gap-y-2 text-sm">
            {#each TOOLS as [name, description] (name)}
              <dt class="font-mono text-xs text-stencil">{name}</dt>
              <dd class="text-stencil-dim">{t(description)}</dd>
            {/each}
          </dl>
        </div>
      {/if}

      {#if activity.length > 0}
        <div>
          <h2 class="section-title">{t("Recent MCP commands")}</h2>
          <ul class="mt-3 space-y-1">
            {#each activity as entry, index (index)}
              <li class="flex items-center gap-2 text-xs">
                <span class={activityTone(entry)}>{activityLabel(entry)}</span>
                <span class="shrink-0 text-stencil-mute">{entry.host_name}</span>
                <span class="truncate font-mono text-stencil">{entry.command}</span>
              </li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  </section>
</div>
