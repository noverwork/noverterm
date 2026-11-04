<script lang="ts">
  import { Check, Copy, Eye, EyeOff } from "@lucide/svelte";

  import { commands, type McpStatus } from "../../bindings.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { t } from "$lib/i18n/index.svelte.js";
  import { activity } from "$lib/mcp/mcp-events.svelte.js";

  type Client = "claude-code" | "cursor" | "vscode" | "claude-desktop";

  const CLIENTS: { id: Client; label: string; file: string | null }[] = [
    { id: "claude-code", label: "Claude Code", file: null },
    { id: "cursor", label: "Cursor", file: "~/.cursor/mcp.json" },
    { id: "vscode", label: "VS Code", file: ".vscode/mcp.json" },
    { id: "claude-desktop", label: "Claude Desktop", file: "claude_desktop_config.json" },
  ];

  const MASK = "••••••••";

  let status = $state<McpStatus | null>(null);
  let port = $state(47321);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let client = $state<Client>("claude-code");
  let reveal = $state(false);
  let copied = $state<string | null>(null);

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

  async function configure() {
    saving = true;
    error = null;
    const result = await commands.mcpConfigure({ port });
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
      <div class="grid grid-cols-[6rem_1fr] items-center gap-x-3 gap-y-3 text-sm">
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
              onclick={() => void configure()}
              disabled={saving}
            >
              {t("Apply")}
            </Button>
          {/if}
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
          <dl class="mt-3 grid grid-cols-[8rem_1fr] gap-x-3 gap-y-2 text-sm">
            <dt class="font-mono text-xs text-stencil">list_hosts</dt>
            <dd class="text-stencil-dim">{t("Lists saved hosts, without credentials.")}</dd>
            <dt class="font-mono text-xs text-stencil">run_command</dt>
            <dd class="text-stencil-dim">{t("Runs a command on a host after you allow it here.")}</dd>
            <dt class="font-mono text-xs text-stencil">upload_file</dt>
            <dd class="text-stencil-dim">{t("Copies a local file to a host after you allow it here.")}</dd>
            <dt class="font-mono text-xs text-stencil">download_file</dt>
            <dd class="text-stencil-dim">{t("Copies a file from a host after you allow it here.")}</dd>
          </dl>
        </div>
      {/if}

      {#if activity.length > 0}
        <div>
          <h2 class="section-title">{t("Recent MCP commands")}</h2>
          <ul class="mt-3 space-y-1">
            {#each activity as entry, index (index)}
              <li class="flex items-center gap-2 text-xs">
                <span
                  class={entry.status === "finished" && (entry.exit_status ?? 0) === 0
                    ? "text-emerald-400"
                    : entry.status === "denied"
                      ? "text-stencil-mute"
                      : "text-red-300"}
                >
                  {entry.status === "finished"
                    ? entry.exit_status === null
                      ? t("Done")
                      : t("exit {code}", { code: entry.exit_status })
                    : entry.status === "denied"
                      ? t("Denied")
                      : t("Failed")}
                </span>
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
