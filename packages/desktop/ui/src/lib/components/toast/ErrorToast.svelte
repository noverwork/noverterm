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
        return "border-amber-300/30 bg-amber-950/90 text-amber-100 shadow-amber-950/30";
      case "info":
        return "border-blue-300/30 bg-blue-950/90 text-blue-100 shadow-blue-950/30";
      default:
        return "border-red-300/30 bg-red-950/90 text-red-100 shadow-red-950/30";
    }
  });

  const iconClass = $derived.by(() => {
    switch (type) {
      case "warning":
        return "text-amber-200";
      case "info":
        return "text-blue-200";
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
  class="animate-[toast-slide-in_180ms_ease-out] rounded-2xl border px-4 py-3 shadow-2xl backdrop-blur {toneClass}"
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
      class="-mr-1 -mt-1 grid size-7 shrink-0 cursor-pointer place-items-center rounded-lg text-current opacity-70 transition hover:bg-white/10 hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/30"
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
