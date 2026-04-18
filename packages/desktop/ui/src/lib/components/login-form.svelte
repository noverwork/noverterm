<script lang="ts">
  import { Loader2, Terminal } from "@lucide/svelte";
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

  const errors = $derived.by(() => {
    const errs: Record<string, string> = {};
    if (!username.trim()) errs.username = "Username is required";
    if (!password) errs.password = "Password is required";
    return errs;
  });

  const isValid = $derived(Object.keys(errors).length === 0);

  function handleSubmit() {
    submitted = true;
    if (!isValid || isLoading) return;
    onLogin(username.trim(), password);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && isValid && !isLoading) {
      handleSubmit();
    }
  }
</script>

<div class="flex min-h-screen items-center justify-center bg-background">
  <div class="w-full max-w-sm space-y-6 px-4">
    <div class="flex flex-col items-center text-center">
      <div class="relative mb-4">
        <div class="absolute inset-0 blur-3xl bg-primary/20 rounded-full"></div>
        <Terminal class="relative size-12 text-primary" />
      </div>
      <h1 class="text-2xl font-bold tracking-tight">Noverterm</h1>
      <p class="text-sm text-muted-foreground mt-1">Sign in to your account</p>
    </div>

    <div class="space-y-4">
      {#if error}
        <div class="rounded-lg border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      {/if}

      <div class="space-y-1">
        <label for="login-username" class="text-sm font-medium">Username</label>
        <Input
          id="login-username"
          bind:value={username}
          placeholder="Enter your username"
          disabled={isLoading}
          onkeydown={handleKeydown}
          class={submitted && errors.username ? 'border-destructive' : ''}
        />
        {#if submitted && errors.username}
          <p class="text-xs text-destructive">{errors.username}</p>
        {/if}
      </div>

      <div class="space-y-1">
        <label for="login-password" class="text-sm font-medium">Password</label>
        <Input
          id="login-password"
          type="password"
          bind:value={password}
          placeholder="Enter your password"
          disabled={isLoading}
          onkeydown={handleKeydown}
          class={submitted && errors.password ? 'border-destructive' : ''}
        />
        {#if submitted && errors.password}
          <p class="text-xs text-destructive">{errors.password}</p>
        {/if}
      </div>

      <Button
        class="w-full gap-2"
        disabled={isLoading}
        onclick={handleSubmit}
      >
        {#if isLoading}
          <Loader2 class="size-4 animate-spin" />
          Signing in...
        {:else}
          Sign In
        {/if}
      </Button>
    </div>
  </div>
</div>
