<script lang="ts">
  import { AlertCircle, Loader2 } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import type { HostTrustMismatch, HostTrustPrompt } from "../../bindings";

  interface Props {
    status: "connecting" | "error" | "trust_required";
    name?: string;
    /** Stencil code of the session, e.g. "PRO·API-01". */
    code?: string;
    protocol?: "terminal" | "sftp";
    error?: string | null;
    trustPrompt?: HostTrustPrompt | null;
    trustMismatch?: HostTrustMismatch | null;
    trustError?: string | null;
    trustConfirming?: boolean;
    canTrust?: boolean;
    onRetry?: () => unknown;
    onTrust?: () => unknown;
    onReplaceTrust?: () => unknown;
    onCancel?: () => unknown;
  }

  let {
    status,
    name,
    code,
    protocol = "terminal",
    error = null,
    trustPrompt = null,
    trustMismatch = null,
    trustError = null,
    trustConfirming = false,
    canTrust = true,
    onRetry,
    onTrust,
    onReplaceTrust,
    onCancel,
  }: Props = $props();
</script>

{#if status === "connecting"}
  <div
    class="absolute inset-0 z-20 flex flex-col items-center justify-center overflow-y-auto bg-port/90 p-4 text-center [overflow-wrap:anywhere]"
    role="status"
    aria-live="polite"
  >
    <Loader2 class="mb-4 size-8 shrink-0 animate-spin text-stencil motion-reduce:animate-none" />
    <p class="text-sm font-semibold text-stencil">
      {name ? `Connecting to ${name}` : "Connecting…"}
    </p>
    <p class="mt-2 text-xs text-stencil-mute">
      Negotiating {protocol === "sftp" ? "SFTP" : "terminal"} session…
    </p>
    {#if onCancel}
      <Button
        variant="outline"
        onclick={() => onCancel?.()}
        class="mt-6 rounded-[2px] border-white/10 bg-transparent px-4 text-stencil hover:border-white/20 hover:bg-white/5"
      >
        Cancel
      </Button>
    {/if}
  </div>
{:else}
  <div
    class="absolute inset-0 z-20 flex h-full min-w-0 flex-col items-center justify-center p-4 sm:p-8"
    aria-live="polite"
  >
    <div
      role="dialog"
      aria-label={trustPrompt ? "Verify SSH host identity" : "Connection failed"}
      class={status === "trust_required"
        ? "max-h-full w-full min-w-0 max-w-xl overflow-y-auto rounded-[2px] border border-[#c9952e]/60 bg-rail text-left [overflow-wrap:anywhere]"
        : "max-h-full min-w-0 max-w-lg overflow-y-auto rounded-[2px] border border-seal/60 bg-rail p-8 [overflow-wrap:anywhere]"}
    >
      {#if trustPrompt}
        <div class="border-b border-white/10 px-6 py-5 sm:px-7">
          <div class="flex items-start gap-4">
            <div
              class="grid size-11 shrink-0 place-items-center rounded-[2px] border border-[#c9952e]/60 bg-transparent text-[#e6c27a]"
            >
              <AlertCircle class="size-5" />
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span
                  class="rounded-[2px] border border-white/10 bg-rib px-1.5 text-[11px] uppercase tracking-[0.14em] text-stencil-dim"
                >
                  SSH host identity
                </span>
              </div>
              <h2 class="mt-3 text-xl font-semibold tracking-tight text-stencil">
                Verify SSH host identity
              </h2>
              <p class="mt-2 text-sm leading-6 text-stencil-mute">
                Confirm this fingerprint before opening {protocol === "sftp" ? "an SFTP" : "a terminal"}
                session.
              </p>
            </div>
          </div>
        </div>

        <div class="px-6 py-5 sm:px-7">
          <dl class="grid gap-3 text-sm">
            <div
              class="flex items-center justify-between gap-4 rounded-[2px] border border-white/10 bg-rib px-4 py-3"
            >
              <dt class="shrink-0 section-title">
                Host
              </dt>
              <dd class="min-w-0 break-all text-right font-mono text-stencil">
                {trustPrompt.host}:{trustPrompt.port}
              </dd>
            </div>
            <div
              class="flex items-center justify-between gap-4 rounded-[2px] border border-white/10 bg-rib px-4 py-3"
            >
              <dt class="shrink-0 section-title">
                Algorithm
              </dt>
              <dd class="min-w-0 break-all text-right font-mono text-stencil">
                {trustPrompt.algorithm}
              </dd>
            </div>
            <div class="rounded-[2px] border border-white/10 bg-rib p-4">
              <dt class="section-title">
                Fingerprint
              </dt>
              <dd
                class="mt-3 break-all rounded-[2px] border border-white/10 bg-rail px-3 py-2.5 font-mono text-sm leading-6 text-stencil"
              >
                {trustPrompt.fingerprint}
              </dd>
            </div>
          </dl>
          <p class="mt-4 text-xs leading-5 text-stencil-mute">
            Only continue if this fingerprint matches the server you expect. It
            will be saved locally in Known Hosts.
          </p>
        </div>

        {#if trustError}
          <p
            class="mx-6 mb-4 rounded-[2px] border border-seal/60 bg-transparent px-4 py-3 text-sm text-red-200 sm:mx-7"
          >
            {trustError}
          </p>
        {/if}
        <div
          class="flex flex-wrap items-center justify-end gap-3 border-t border-white/10 bg-port px-6 py-4 sm:px-7"
        >
          {#if canTrust && onTrust}
            <Button
              onclick={() => onTrust?.()}
              disabled={trustConfirming}
              class="order-2 h-auto min-h-8 max-w-full gap-2 whitespace-normal rounded-[2px] bg-[#c9952e] px-4 py-1 text-port hover:bg-[#d6a443] sm:order-none"
            >
              {#if trustConfirming}
                <Loader2 class="size-4 animate-spin" />
              {/if}
              Trust host and retry
            </Button>
          {:else if !canTrust}
            <p class="max-w-md text-sm leading-6 text-stencil-mute">
              Save this connection first to trust the host and retry automatically.
            </p>
          {/if}
          {#if onCancel}
            <Button
              variant="outline"
              onclick={() => onCancel?.()}
              class="rounded-[2px] border-white/10 bg-transparent px-4 text-stencil hover:border-white/20 hover:bg-white/5"
            >
              Cancel
            </Button>
          {/if}
        </div>
      {:else}
        <div
          class="flex items-center justify-center gap-2 text-[11px] uppercase tracking-[0.14em] text-stencil"
        >
          {#if code}<span>{code}</span>{/if}
          <span class="bg-stencil px-1 leading-4 text-seal">HELD</span>
        </div>
        <h2 class="mt-4 text-center text-xl font-semibold text-stencil">
          Connection failed
        </h2>
        <p class="mx-auto mt-2 max-w-md text-center text-sm leading-6 text-stencil-mute">
          {error ?? "Unknown error"}
        </p>
        {#if trustMismatch}
          <div
            class="mt-5 rounded-[2px] border border-seal/60 bg-rib p-4 text-left"
          >
            <p class="text-sm font-semibold text-red-200">
              Saved fingerprint does not match.
            </p>
            <dl class="mt-3 grid gap-2 text-xs text-stencil-dim">
              <div class="space-y-1">
                <dt class="text-stencil-mute">Expected</dt>
                <dd class="break-all rounded-[2px] bg-rail px-3 py-2 font-mono">
                  {trustMismatch.expected_fingerprint}
                </dd>
              </div>
              <div class="space-y-1">
                <dt class="text-stencil-mute">Presented</dt>
                <dd
                  class="break-all rounded-[2px] bg-rail px-3 py-2 font-mono text-red-100"
                >
                  {trustMismatch.presented_fingerprint}
                </dd>
              </div>
            </dl>
            <p class="mt-3 text-xs leading-5 text-stencil-mute">
              This may indicate the server changed keys or a man-in-the-middle risk.
              Not updating trust automatically.
            </p>
          </div>
          {#if trustError}
            <p
              class="mt-4 rounded-[2px] border border-seal/60 bg-transparent px-4 py-3 text-sm text-red-200"
            >
              {trustError}
            </p>
          {/if}
          <div class="mt-6 flex flex-wrap justify-center gap-3">
            {#if canTrust && onReplaceTrust}
              <Button
                onclick={() => onReplaceTrust?.()}
                disabled={trustConfirming}
                class="h-auto min-h-8 max-w-full gap-2 whitespace-normal rounded-[2px] bg-seal px-5 py-1 text-stencil hover:bg-[#b4432f]"
              >
                {#if trustConfirming}
                  <Loader2 class="size-4 animate-spin" />
                {/if}
                Delete &amp; trust new key
              </Button>
            {/if}
            {#if onRetry}
              <Button
                variant="outline"
                onclick={() => onRetry?.()}
                disabled={trustConfirming}
                class="h-auto min-h-8 max-w-full whitespace-normal rounded-[2px] border-white/10 bg-transparent px-5 py-1 text-stencil hover:border-white/20 hover:bg-white/5"
              >
                Retry session
              </Button>
            {/if}
            {#if onCancel}
              <Button
                variant="outline"
                onclick={() => onCancel?.()}
                class="rounded-[2px] border-white/10 bg-transparent px-4 text-stencil hover:border-white/20 hover:bg-white/5"
              >
                Cancel
              </Button>
            {/if}
          </div>
        {:else if onRetry || onCancel}
          <div class="mt-6 flex flex-wrap justify-center gap-3">
            {#if onRetry}
              <Button
                onclick={() => onRetry?.()}
                class="h-auto min-h-8 max-w-full gap-2 whitespace-normal rounded-[2px] bg-seal px-5 py-1 text-stencil hover:bg-[#b4432f]"
              >
                Retry session
              </Button>
            {/if}
            {#if onCancel}
              <Button
                variant="outline"
                onclick={() => onCancel?.()}
                class="rounded-[2px] border-white/10 bg-transparent px-4 text-stencil hover:border-white/20 hover:bg-white/5"
              >
                Cancel
              </Button>
            {/if}
          </div>
        {/if}
      {/if}
    </div>
  </div>
{/if}
