<script lang="ts">
  import { Command } from "@lucide/svelte";

  import LoginForm from "$lib/components/login-form.svelte";

  let {
    onLogin,
    isLoading,
    error,
  }: {
    onLogin: (username: string, password: string) => void;
    isLoading: boolean;
    error: string | null;
  } = $props();
</script>

<div class="auth-shell flex h-screen items-center justify-center overflow-y-auto overflow-x-hidden">
  <div class="w-full max-w-md px-4 py-8">
    <div class="mb-8 text-center">
      <div class="hero-chip mx-auto mb-4 w-fit">
        <Command class="size-3.5" />
        Noverterm
      </div>
      <h1 class="text-2xl font-semibold tracking-tight text-white">Sign in</h1>
      <p class="mt-2 text-sm text-slate-400">Enter your workspace credentials to continue.</p>
    </div>

    <LoginForm {onLogin} {isLoading} {error} />
  </div>
</div>
