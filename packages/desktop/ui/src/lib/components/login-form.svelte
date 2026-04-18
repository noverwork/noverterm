<script lang="ts">
  import { AlertCircle, Loader2, LockKeyhole, UserRound } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  let {
    onLogin,
    isLoading,
    error,
  }: {
    onLogin: (username: string, password: string) => void;
    isLoading: boolean;
    error: string | null;
  } = $props();

  let username = $state("");
  let password = $state("");
  let submitted = $state(false);
  let touched = $state<Record<string, boolean>>({});

  const errors = $derived.by(() => {
    const errs: Record<string, string> = {};
    if (!username.trim()) errs.username = "Username is required";
    if (!password) errs.password = "Password is required";
    return errs;
  });

  const isValid = $derived(Object.keys(errors).length === 0);

  function markTouched(field: string) {
    touched[field] = true;
  }

  function showError(field: string) {
    return (submitted || touched[field]) && errors[field];
  }

  function handleSubmit() {
    submitted = true;
    if (!isValid || isLoading) return;
    onLogin(username.trim(), password);
  }
</script>

<div class="rounded-[1.4rem] bg-slate-950/85 p-6 text-white sm:p-7">
  <div class="space-y-2">
    <p class="section-title text-slate-400">Workspace sign-in</p>
    <h3 class="text-2xl font-semibold tracking-tight">Access your terminal workspace</h3>
    <p class="text-sm leading-6 text-slate-400">
      Sign in with the account provisioned for your infrastructure workspace. Your recent sessions,
      saved hosts, and runtime preferences will be restored automatically.
    </p>
  </div>

  <div class="mt-6 space-y-5">
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
        <label for="login-username" class="text-sm font-medium text-slate-100">Username</label>
        <div class="relative">
          <UserRound class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
          <Input
            id="login-username"
            bind:value={username}
            placeholder="admin"
            disabled={isLoading}
            onblur={() => markTouched("username")}
            onkeydown={(event) => event.key === "Enter" && handleSubmit()}
            class={showError("username") ? "pl-9 border-destructive bg-white/5 text-white placeholder:text-slate-500" : "pl-9 border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
          />
        </div>
        {#if showError("username")}
          <p class="text-xs text-destructive" role="alert">{showError("username")}</p>
        {/if}
      </div>

      <div class="space-y-2">
        <label for="login-password" class="text-sm font-medium text-slate-100">Password</label>
        <div class="relative">
          <LockKeyhole class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
          <Input
            id="login-password"
            type="password"
            bind:value={password}
            placeholder="Enter your workspace password"
            disabled={isLoading}
            onblur={() => markTouched("password")}
            onkeydown={(event) => event.key === "Enter" && handleSubmit()}
            class={showError("password") ? "pl-9 border-destructive bg-white/5 text-white placeholder:text-slate-500" : "pl-9 border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
          />
        </div>
        {#if showError("password")}
          <p class="text-xs text-destructive" role="alert">{showError("password")}</p>
        {/if}
      </div>
    </div>

    <div class="rounded-2xl border border-white/8 bg-white/[0.04] px-4 py-3 text-sm text-slate-300">
      If you’re new here, use the sign up tab to create your operator profile first. Workspace activation
      completes after backend provisioning.
    </div>

    <Button class="w-full gap-2" disabled={isLoading || !isValid} onclick={handleSubmit}>
      {#if isLoading}
        <Loader2 class="size-4 animate-spin" />
        Signing in…
      {:else}
        Continue to workspace
      {/if}
    </Button>
  </div>
</div>
