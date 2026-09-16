<script lang="ts">
  import { AlertCircle, Loader2 } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import type { HostTrustMismatch, HostTrustPrompt } from "../../bindings";

  interface Props {
    status: "connecting" | "error" | "trust_required";
    name?: string;
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
    class="absolute inset-0 z-20 flex flex-col items-center justify-center overflow-y-auto bg-[#080c13]/90 p-4 text-center [overflow-wrap:anywhere]"
    role="status"
    aria-live="polite"
  >
    <Loader2 class="mb-4 size-8 shrink-0 animate-spin text-amber-200 motion-reduce:animate-none" />
    <p class="text-sm font-semibold text-white">
      {name ? `Connecting to ${name}` : "Connecting…"}
    </p>
    <p class="mt-2 text-xs text-slate-500">
      Negotiating {protocol === "sftp" ? "SFTP" : "terminal"} session…
    </p>
    {#if onCancel}
      <Button
        variant="outline"
        onclick={() => onCancel?.()}
        class="mt-6 rounded-2xl border-white/10 bg-white/4 px-4 text-white hover:bg-white/8"
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
        ? "max-h-full w-full min-w-0 max-w-xl overflow-y-auto rounded-[1.75rem] border border-white/10 bg-slate-950/88 text-left shadow-2xl shadow-black/40 ring-1 ring-amber-300/10 backdrop-blur-xl [overflow-wrap:anywhere]"
        : "max-h-full min-w-0 max-w-lg overflow-y-auto rounded-[2rem] border border-red-300/20 bg-red-400/8 p-8 shadow-2xl shadow-black/30 [overflow-wrap:anywhere]"}
    >
      {#if trustPrompt}
        <div class="border-b border-white/10 px-6 py-5 sm:px-7">
          <div class="flex items-start gap-4">
            <div
              class="grid size-11 shrink-0 place-items-center rounded-2xl border border-amber-300/20 bg-amber-300/10 text-amber-200 shadow-[0_0_24px_rgb(252_211_77/0.08)]"
            >
              <AlertCircle class="size-5" />
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span
                  class="rounded-full border border-amber-300/20 bg-amber-300/10 px-2.5 py-1 text-[11px] font-medium uppercase tracking-[0.18em] text-amber-200"
                >
                  SSH host identity
                </span>
              </div>
              <h2 class="mt-3 text-xl font-semibold tracking-tight text-white">
                Verify SSH host identity
              </h2>
              <p class="mt-2 text-sm leading-6 text-slate-400">
                Confirm this fingerprint before opening {protocol === "sftp" ? "an SFTP" : "a terminal"}
                session.
              </p>
            </div>
          </div>
        </div>

        <div class="px-6 py-5 sm:px-7">
          <dl class="grid gap-3 text-sm">
            <div
              class="flex items-center justify-between gap-4 rounded-2xl border border-white/8 bg-white/[0.035] px-4 py-3"
            >
              <dt class="shrink-0 text-xs font-medium uppercase tracking-[0.16em] text-slate-500">
                Host
              </dt>
              <dd class="min-w-0 break-all text-right font-mono text-slate-100">
                {trustPrompt.host}:{trustPrompt.port}
              </dd>
            </div>
            <div
              class="flex items-center justify-between gap-4 rounded-2xl border border-white/8 bg-white/[0.035] px-4 py-3"
            >
              <dt class="shrink-0 text-xs font-medium uppercase tracking-[0.16em] text-slate-500">
                Algorithm
              </dt>
              <dd class="min-w-0 break-all text-right font-mono text-slate-100">
                {trustPrompt.algorithm}
              </dd>
            </div>
            <div class="rounded-2xl border border-white/8 bg-black/25 p-4">
              <dt class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500">
                Fingerprint
              </dt>
              <dd
                class="mt-3 break-all rounded-xl border border-amber-300/10 bg-amber-300/[0.06] px-3 py-2.5 font-mono text-sm leading-6 text-amber-100"
              >
                {trustPrompt.fingerprint}
              </dd>
            </div>
          </dl>
          <p class="mt-4 text-xs leading-5 text-slate-500">
            Only continue if this fingerprint matches the server you expect. It
            will be saved locally in Known Hosts.
          </p>
        </div>

        {#if trustError}
          <p
            class="mx-6 mb-4 rounded-2xl border border-red-400/20 bg-red-400/10 px-4 py-3 text-sm text-red-200 sm:mx-7"
          >
            {trustError}
          </p>
        {/if}
        <div
          class="flex flex-wrap items-center justify-end gap-3 border-t border-white/10 bg-white/[0.025] px-6 py-4 sm:px-7"
        >
          {#if canTrust && onTrust}
            <Button
              onclick={() => onTrust?.()}
              disabled={trustConfirming}
              class="order-2 h-auto min-h-8 max-w-full gap-2 whitespace-normal rounded-2xl bg-amber-300 px-4 py-1 text-amber-950 hover:bg-amber-200 sm:order-none"
            >
              {#if trustConfirming}
                <Loader2 class="size-4 animate-spin" />
              {/if}
              Trust host and retry
            </Button>
          {:else if !canTrust}
            <p class="max-w-md text-sm leading-6 text-slate-400">
              Save this connection first to trust the host and retry automatically.
            </p>
          {/if}
          {#if onCancel}
            <Button
              variant="outline"
              onclick={() => onCancel?.()}
              class="rounded-2xl border-white/10 bg-white/4 px-4 text-white hover:bg-white/8"
            >
              Cancel
            </Button>
          {/if}
        </div>
      {:else}
        <div
          class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300"
        >
          <AlertCircle class="size-7" />
        </div>
        <h2 class="mt-5 text-center text-xl font-semibold text-white">
          Connection failed
        </h2>
        <p class="mx-auto mt-2 max-w-md text-center text-sm leading-6 text-slate-400">
          {error ?? "Unknown error"}
        </p>
        {#if trustMismatch}
          <div
            class="mt-5 rounded-2xl border border-red-300/25 bg-red-300/8 p-4 text-left"
          >
            <p class="text-sm font-semibold text-red-100">
              Saved fingerprint does not match.
            </p>
            <dl class="mt-3 grid gap-2 text-xs text-slate-300">
              <div class="space-y-1">
                <dt class="text-slate-500">Expected</dt>
                <dd class="break-all rounded-xl bg-black/30 px-3 py-2 font-mono">
                  {trustMismatch.expected_fingerprint}
                </dd>
              </div>
              <div class="space-y-1">
                <dt class="text-slate-500">Presented</dt>
                <dd
                  class="break-all rounded-xl bg-black/30 px-3 py-2 font-mono text-red-100"
                >
                  {trustMismatch.presented_fingerprint}
                </dd>
              </div>
            </dl>
            <p class="mt-3 text-xs leading-5 text-slate-400">
              This may indicate the server changed keys or a man-in-the-middle risk.
              Not updating trust automatically.
            </p>
          </div>
          {#if trustError}
            <p
              class="mt-4 rounded-2xl border border-red-400/20 bg-red-400/10 px-4 py-3 text-sm text-red-200"
            >
              {trustError}
            </p>
          {/if}
          <div class="mt-6 flex flex-wrap justify-center gap-3">
            {#if canTrust && onReplaceTrust}
              <Button
                onclick={() => onReplaceTrust?.()}
                disabled={trustConfirming}
                class="h-auto min-h-8 max-w-full gap-2 whitespace-normal rounded-2xl bg-red-300 px-5 py-1 text-red-950 hover:bg-red-200"
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
                class="h-auto min-h-8 max-w-full whitespace-normal rounded-2xl border-white/10 bg-white/4 px-5 py-1 text-white hover:bg-white/8"
              >
                Retry session
              </Button>
            {/if}
            {#if onCancel}
              <Button
                variant="outline"
                onclick={() => onCancel?.()}
                class="rounded-2xl border-white/10 bg-white/4 px-4 text-white hover:bg-white/8"
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
                class="h-auto min-h-8 max-w-full gap-2 whitespace-normal rounded-2xl bg-red-300 px-5 py-1 text-red-950 hover:bg-red-200"
              >
                Retry session
              </Button>
            {/if}
            {#if onCancel}
              <Button
                variant="outline"
                onclick={() => onCancel?.()}
                class="rounded-2xl border-white/10 bg-white/4 px-4 text-white hover:bg-white/8"
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
