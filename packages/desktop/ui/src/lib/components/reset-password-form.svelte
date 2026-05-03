<script lang="ts">
  import { AlertCircle, CheckCircle2, Loader2, LockKeyhole } from "@lucide/svelte";
  import { superForm } from "sveltekit-superforms";
  import { zod4 } from "sveltekit-superforms/adapters";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import {
    resetPasswordSchema,
    type ResetPasswordForm,
  } from "$lib/schemas/index.js";

  interface Props {
    token: string;
    onSubmit: (token: string, password: string) => Promise<void>;
    onBack: () => void;
  }

  let { token, onSubmit, onBack }: Props = $props();

  const form = superForm<ResetPasswordForm>(
    { password: "", confirmPassword: "" },
    { validators: zod4(resetPasswordSchema) },
  );

  const { form: formData, errors, submitting } = form;
  let error = $state<string | null>(null);
  let resetComplete = $state(false);

  async function handleSubmit() {
    const result = resetPasswordSchema.safeParse($formData);
    if (!result.success || $submitting || !token) return;

    error = null;
    try {
      await onSubmit(token, result.data.password);
      resetComplete = true;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to reset password";
    }
  }
</script>

<div class="rounded-[1.4rem] bg-slate-950/85 p-6 text-white sm:p-7">
  <div class="space-y-5">
    {#if resetComplete}
      <div class="rounded-2xl border border-emerald-300/30 bg-emerald-300/10 p-4 text-sm text-emerald-100" role="status">
        <div class="flex items-start gap-3">
          <CheckCircle2 class="mt-0.5 size-4 shrink-0" />
          <div>
            <p class="font-medium">Password reset</p>
            <p class="mt-1 text-emerald-100/85">You can now sign in with your new password.</p>
          </div>
        </div>
      </div>
    {:else if !token}
      <div class="rounded-2xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive" role="alert">
        <div class="flex items-start gap-3">
          <AlertCircle class="mt-0.5 size-4 shrink-0" />
          <div>
            <p class="font-medium">Invalid reset link</p>
            <p class="mt-1 text-destructive/90">This reset link is missing a token.</p>
          </div>
        </div>
      </div>
    {/if}

    {#if error}
      <div class="rounded-2xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive" role="alert">
        <div class="flex items-start gap-3">
          <AlertCircle class="mt-0.5 size-4 shrink-0" />
          <div>
            <p class="font-medium">Unable to reset password</p>
            <p class="mt-1 text-destructive/90">{error}</p>
          </div>
        </div>
      </div>
    {/if}

    <div class="space-y-4">
      <div class="space-y-2">
        <label for="reset-password" class="text-sm font-medium text-slate-100">New password</label>
        <div class="relative">
          <LockKeyhole class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
          <Input
            id="reset-password"
            type="password"
            bind:value={$formData.password}
            placeholder="new password"
            disabled={$submitting || resetComplete || !token}
            onkeydown={(event) => event.key === "Enter" && handleSubmit()}
            class={$errors.password ? "pl-9 border-destructive bg-white/5 text-white placeholder:text-slate-500" : "pl-9 border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
          />
        </div>
        {#if $errors.password}
          <p class="text-xs text-destructive" role="alert">{$errors.password}</p>
        {/if}
      </div>

      <div class="space-y-2">
        <label for="reset-confirm" class="text-sm font-medium text-slate-100">Confirm password</label>
        <div class="relative">
          <LockKeyhole class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
          <Input
            id="reset-confirm"
            type="password"
            bind:value={$formData.confirmPassword}
            placeholder="re-enter password"
            disabled={$submitting || resetComplete || !token}
            onkeydown={(event) => event.key === "Enter" && handleSubmit()}
            class={$errors.confirmPassword ? "pl-9 border-destructive bg-white/5 text-white placeholder:text-slate-500" : "pl-9 border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
          />
        </div>
        {#if $errors.confirmPassword}
          <p class="text-xs text-destructive" role="alert">{$errors.confirmPassword}</p>
        {/if}
      </div>
    </div>

    <Button class="w-full gap-2" disabled={$submitting || resetComplete || !token} onclick={handleSubmit}>
      {#if $submitting}
        <Loader2 class="size-4 animate-spin" />
        Resetting…
      {:else}
        Reset password
      {/if}
    </Button>

    <button type="button" class="w-full text-sm font-medium text-slate-400 transition hover:text-white" onclick={onBack}>
      Back to sign in
    </button>
  </div>
</div>
