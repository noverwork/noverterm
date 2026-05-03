<script lang="ts">
  import { AlertCircle, Loader2, LockKeyhole, Mail } from "@lucide/svelte";
  import { superForm } from "sveltekit-superforms";
  import { zod4 } from "sveltekit-superforms/adapters";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { loginSchema, type LoginForm } from "$lib/schemas/index.js";

  let {
    onLogin,
    onForgotPassword,
    isLoading,
    error,
  }: {
    onLogin: (email: string, password: string) => void;
    onForgotPassword: () => void;
    isLoading: boolean;
    error: string | null;
  } = $props();

  const form = superForm<LoginForm>(
    { email: "", password: "" },
    { validators: zod4(loginSchema) },
  );

  const { form: formData, errors, submitting } = form;

  async function handleSubmit() {
    const result = loginSchema.safeParse($formData);
    if (!result.success || isLoading) return;
    onLogin(result.data.email.trim(), result.data.password);
  }
</script>

<div class="rounded-[1.4rem] bg-slate-950/85 p-6 text-white sm:p-7">
  <div class="space-y-5">
    {#if error}
      <div class="rounded-2xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive" role="alert">
        <div class="flex items-start gap-3">
          <AlertCircle class="mt-0.5 size-4 shrink-0" />
          <div>
            <p class="font-medium">Unable to sign in</p>
            <p class="mt-1 text-destructive/90">{error}</p>
          </div>
        </div>
      </div>
    {/if}

    <div class="space-y-4">
      <div class="space-y-2">
        <label for="login-email" class="text-sm font-medium text-slate-100">Email</label>
        <div class="relative">
          <Mail class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
          <Input
            id="login-email"
            type="email"
            bind:value={$formData.email}
            placeholder="you@example.com"
            disabled={isLoading || $submitting}
            onkeydown={(event) => event.key === "Enter" && handleSubmit()}
            class={$errors.email ? "pl-9 border-destructive bg-white/5 text-white placeholder:text-slate-500" : "pl-9 border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
          />
        </div>
        {#if $errors.email}
          <p class="text-xs text-destructive" role="alert">{$errors.email}</p>
        {/if}
      </div>

      <div class="space-y-2">
        <div class="flex items-center justify-between gap-3">
          <label for="login-password" class="text-sm font-medium text-slate-100">Password</label>
          <button type="button" class="text-xs font-medium text-cyan-200 transition hover:text-cyan-100" onclick={onForgotPassword}>Forgot password?</button>
        </div>
        <div class="relative">
          <LockKeyhole class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
          <Input
            id="login-password"
            type="password"
            bind:value={$formData.password}
            placeholder="password"
            disabled={isLoading || $submitting}
            onkeydown={(event) => event.key === "Enter" && handleSubmit()}
            class={$errors.password ? "pl-9 border-destructive bg-white/5 text-white placeholder:text-slate-500" : "pl-9 border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
          />
        </div>
        {#if $errors.password}
          <p class="text-xs text-destructive" role="alert">{$errors.password}</p>
        {/if}
      </div>
    </div>

    <Button class="w-full gap-2" disabled={isLoading || $submitting} onclick={handleSubmit}>
      {#if isLoading || $submitting}
        <Loader2 class="size-4 animate-spin" />
        Signing in…
      {:else}
        Sign in
      {/if}
    </Button>
  </div>
</div>
