<script lang="ts">
  import { AlertCircle, AlertTriangle, Info, X } from "@lucide/svelte";

  type ErrorToastType = "error" | "warning" | "info";

  interface Props {
    message: string;
    type?: ErrorToastType;
    duration?: number;
    onDismiss: () => void;
  }

  let { message, type = "error", duration = 5000, onDismiss }: Props = $props();

  const icon = $derived.by(() => {
    switch (type) {
      case "warning":
        return AlertTriangle;
      case "info":
        return Info;
      default:
        return AlertCircle;
    }
  });

  const toneClass = $derived.by(() => {
    switch (type) {
      case "warning":
        return "border-[#c9952e]/60 bg-rail text-[#e6c27a]";
      case "info":
        return "border-white/10 bg-rail text-stencil";
      default:
        return "border-seal/60 bg-rail text-red-200";
    }
  });

  const iconClass = $derived.by(() => {
    switch (type) {
      case "warning":
        return "text-[#e6c27a]";
      case "info":
        return "text-stencil-dim";
      default:
        return "text-red-200";
    }
  });

  $effect(() => {
    if (duration <= 0) {
      return;
    }

    const timeout = window.setTimeout(onDismiss, duration);
    return () => window.clearTimeout(timeout);
  });
</script>

<div
  class="animate-[toast-slide-in_180ms_ease-out] rounded-[2px] border px-4 py-3 {toneClass}"
  role="alert"
  data-testid="error-toast"
  data-toast-type={type}
>
  <div class="flex items-start gap-3">
    {#if icon === AlertTriangle}
      <AlertTriangle class="mt-0.5 size-4 shrink-0 {iconClass}" aria-hidden="true" />
    {:else if icon === Info}
      <Info class="mt-0.5 size-4 shrink-0 {iconClass}" aria-hidden="true" />
    {:else}
      <AlertCircle class="mt-0.5 size-4 shrink-0 {iconClass}" aria-hidden="true" />
    {/if}

    <p class="min-w-0 flex-1 text-sm leading-5" data-testid="error-toast-message">
      {message}
    </p>

    <button
      type="button"
      class="-mr-1 -mt-1 grid size-7 shrink-0 cursor-pointer place-items-center rounded-[2px] text-current opacity-70 transition hover:bg-white/10 hover:opacity-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil"
      aria-label="Dismiss notification"
      onclick={onDismiss}
      data-testid="error-toast-dismiss"
    >
      <X class="size-4" aria-hidden="true" />
    </button>
  </div>
</div>

<style>
  @keyframes toast-slide-in {
    from {
      opacity: 0;
      transform: translateX(1rem);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }
</style>
