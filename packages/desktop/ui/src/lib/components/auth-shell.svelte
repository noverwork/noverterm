<script lang="ts">
  import { Command } from "@lucide/svelte";

  import LoginForm from "$lib/components/login-form.svelte";
  import SignupForm from "$lib/components/signup-form.svelte";

  let {
    onLogin,
    onSignup,
    isLoading,
    error,
  }: {
    onLogin: (email: string, password: string) => void;
    onSignup: (email: string, password: string) => void;
    isLoading: boolean;
    error: string | null;
  } = $props();

  let view = $state<"login" | "signup">("login");
</script>

<div class="auth-shell flex h-screen items-center justify-center overflow-y-auto overflow-x-hidden">
  <div class="w-full max-w-md px-4 py-8">
    <div class="mb-8 text-center">
      <div class="hero-chip mx-auto mb-4 w-fit">
        <Command class="size-3.5" />
        Noverterm
      </div>
      {#if view === "login"}
        <h1 class="text-2xl font-semibold tracking-tight text-white">Sign in</h1>
        <p class="mt-2 text-sm text-slate-400">Enter your workspace credentials to continue.</p>
      {:else}
        <h1 class="text-2xl font-semibold tracking-tight text-white">Create account</h1>
        <p class="mt-2 text-sm text-slate-400">Set up your workspace to get started.</p>
      {/if}
    </div>

    {#if view === "login"}
      <LoginForm onLogin={onLogin} {isLoading} {error} />
      <div class="mt-6 text-center text-sm text-slate-400">
        Don't have an account?
        <button class="ml-1 font-medium text-primary hover:underline" onclick={() => (view = "signup")}>Sign up</button>
      </div>
    {:else}
      <SignupForm onSignup={onSignup} {isLoading} {error} />
      <div class="mt-6 text-center text-sm text-slate-400">
        Already have an account?
        <button class="ml-1 font-medium text-primary hover:underline" onclick={() => (view = "login")}>Sign in</button>
      </div>
    {/if}
  </div>
</div>
