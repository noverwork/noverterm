<script lang="ts">
  import { AlertCircle, CheckCircle2, Loader2, Mail } from "@lucide/svelte";
  import { superForm } from "sveltekit-superforms";
  import { zod4 } from "sveltekit-superforms/adapters";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import {
    forgotPasswordSchema,
    type ForgotPasswordForm,
  } from "$lib/schemas/index.js";

  interface Props {
    onSubmit: (email: string) => Promise<void>;
    onBack: () => void;
  }

  let { onSubmit, onBack }: Props = $props();

  const form = superForm<ForgotPasswordForm>(
    { email: "" },
    { validators: zod4(forgotPasswordSchema) },
  );

  const { form: formData, errors, submitting } = form;
  let error = $state<string | null>(null);
  let sent = $state(false);

  async function handleSubmit() {
    const result = forgotPasswordSchema.safeParse($formData);
    if (!result.success || $submitting) return;

    error = null;
    try {
      await onSubmit(result.data.email.trim());
      sent = true;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to send reset email";
    }
  }
</script>

<div class="rounded-[1.4rem] bg-slate-950/85 p-6 text-white sm:p-7">
  <div class="space-y-5">
    {#if sent}
      <div class="rounded-2xl border border-emerald-300/30 bg-emerald-300/10 p-4 text-sm text-emerald-100" role="status">
        <div class="flex items-start gap-3">
          <CheckCircle2 class="mt-0.5 size-4 shrink-0" />
          <div>
            <p class="font-medium">Check your email</p>
            <p class="mt-1 text-emerald-100/85">If that account exists, a password reset link has been sent.</p>
          </div>
        </div>
      </div>
    {/if}

    {#if error}
      <div class="rounded-2xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive" role="alert">
        <div class="flex items-start gap-3">
          <AlertCircle class="mt-0.5 size-4 shrink-0" />
          <div>
            <p class="font-medium">Unable to send reset email</p>
            <p class="mt-1 text-destructive/90">{error}</p>
          </div>
        </div>
      </div>
    {/if}

    <div class="space-y-2">
      <label for="forgot-email" class="text-sm font-medium text-slate-100">Email</label>
      <div class="relative">
        <Mail class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
        <Input
          id="forgot-email"
          type="email"
          bind:value={$formData.email}
          placeholder="you@example.com"
          disabled={$submitting || sent}
          onkeydown={(event) => event.key === "Enter" && handleSubmit()}
          class={$errors.email ? "pl-9 border-destructive bg-white/5 text-white placeholder:text-slate-500" : "pl-9 border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
        />
      </div>
      {#if $errors.email}
        <p class="text-xs text-destructive" role="alert">{$errors.email}</p>
      {/if}
    </div>

    <Button class="w-full gap-2" disabled={$submitting || sent} onclick={handleSubmit}>
      {#if $submitting}
        <Loader2 class="size-4 animate-spin" />
        Sending…
      {:else}
        Send reset link
      {/if}
    </Button>

    <button type="button" class="w-full text-sm font-medium text-slate-400 transition hover:text-white" onclick={onBack}>
      Back to sign in
    </button>
  </div>
</div>
