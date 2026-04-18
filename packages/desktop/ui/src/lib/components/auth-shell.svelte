<script lang="ts">
  import { ArrowRight, Command, ShieldCheck, Server, TerminalSquare } from "@lucide/svelte";

  import LoginForm from "$lib/components/login-form.svelte";
  import RequestAccessPanel from "$lib/components/request-access-panel.svelte";
  import { Button } from "$lib/components/ui/button/index.js";

  let {
    onLogin,
    isLoading,
    error,
  }: {
    onLogin: (username: string, password: string) => void;
    isLoading: boolean;
    error: string | null;
  } = $props();

  let activeView = $state<"login" | "signup">("login");

  const highlights = [
    {
      icon: TerminalSquare,
      title: "Secure SSH workspace",
      body: "Manage saved hosts, issue scoped connection material, and keep local terminal sessions in one place.",
    },
    {
      icon: Server,
      title: "Operational command center",
      body: "Launch local or remote sessions fast, monitor active terminals, and keep infrastructure access organized.",
    },
    {
      icon: ShieldCheck,
      title: "Provisioned access",
      body: "Accounts, hosts, and keys are activated centrally so credentials stay controlled and auditable.",
    },
  ];
</script>

<div class="auth-shell flex min-h-screen overflow-hidden">
  <div class="hidden xl:flex xl:w-[46%] xl:flex-col xl:justify-between xl:border-r xl:border-white/8 xl:px-12 xl:py-10">
    <div class="space-y-10">
      <div class="space-y-4">
        <div class="hero-chip w-fit">
          <Command class="size-3.5" />
          Noverterm workspace
        </div>
        <div class="space-y-3">
          <h1 class="max-w-lg text-5xl font-semibold tracking-tight text-white">
            Secure terminal access for your infrastructure.
          </h1>
          <p class="max-w-xl text-base leading-7 text-slate-300">
            A focused desktop workspace for SSH operations, local terminals, saved host management, and
            session handoff across your environment.
          </p>
        </div>
      </div>

      <div class="grid gap-4">
        {#each highlights as item}
          <div class="panel-glass rounded-2xl p-5">
            <div class="flex items-start gap-4">
              <div class="flex size-11 items-center justify-center rounded-2xl bg-primary/15 text-primary">
                <item.icon class="size-5" />
              </div>
              <div class="space-y-2">
                <h2 class="text-base font-semibold text-white">{item.title}</h2>
                <p class="text-sm leading-6 text-slate-300">{item.body}</p>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>

    <div class="rounded-2xl border border-white/8 bg-white/5 px-5 py-4 text-sm text-slate-300 backdrop-blur-md">
      Sign in with your provisioned workspace account, or start the signup flow on the right and finish
      activation once your backend workspace is provisioned.
    </div>
  </div>

  <div class="flex min-h-screen flex-1 items-center justify-center px-4 py-8 sm:px-8 xl:px-14">
    <div class="w-full max-w-3xl rounded-[2rem] border border-white/10 bg-slate-950/70 p-4 shadow-2xl backdrop-blur-2xl sm:p-6">
      <div class="mb-6 flex flex-col gap-4 border-b border-white/10 pb-6 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <p class="section-title text-slate-400">Access workspace</p>
          <h2 class="mt-2 text-2xl font-semibold text-white">
            {#if activeView === "login"}
              Sign in to continue
            {:else}
              Create your Noverterm account
            {/if}
          </h2>
          <p class="mt-2 max-w-xl text-sm leading-6 text-slate-400">
            {#if activeView === "login"}
              Continue into your terminal workspace, recent sessions, and centrally managed SSH connections.
            {:else}
              Create your operator profile now, then complete workspace activation once backend provisioning finishes.
            {/if}
          </p>
        </div>

        <div class="flex items-center gap-2 rounded-full border border-white/10 bg-white/5 p-1">
          <Button
            type="button"
            variant={activeView === "login" ? "default" : "ghost"}
            class={activeView === "login" ? "rounded-full" : "rounded-full text-slate-300 hover:text-white"}
            onclick={() => (activeView = "login")}
          >
            Sign in
          </Button>
          <Button
            type="button"
            variant={activeView === "signup" ? "default" : "ghost"}
            class={activeView === "signup" ? "rounded-full" : "rounded-full text-slate-300 hover:text-white"}
            onclick={() => (activeView = "signup")}
          >
            Sign up
          </Button>
        </div>
      </div>

      {#if activeView === "login"}
        <div class="grid gap-5 xl:grid-cols-[1fr_0.82fr]">
          <div class="rounded-2xl border border-white/10 bg-white/5 p-1 shadow-lg backdrop-blur-sm">
            <LoginForm {onLogin} {isLoading} {error} />
          </div>
          <div class="space-y-4 rounded-2xl border border-white/10 bg-white/5 p-5 shadow-lg backdrop-blur-sm">
            <div class="space-y-2">
              <p class="section-title text-slate-400">Before you sign in</p>
              <h3 class="text-lg font-semibold text-white">What you get inside the workspace</h3>
            </div>

            <div class="space-y-3">
              <div class="metric-card">
                <div class="flex items-start gap-3">
                  <div class="flex size-10 items-center justify-center rounded-2xl bg-primary/15 text-primary">
                    <TerminalSquare class="size-5" />
                  </div>
                  <div>
                    <p class="font-medium text-white">Saved connections and quick launch</p>
                    <p class="mt-1 text-sm leading-6 text-slate-400">Jump back into production, staging, or local sessions without rebuilding auth each time.</p>
                  </div>
                </div>
              </div>

              <div class="metric-card">
                <div class="flex items-start gap-3">
                  <div class="flex size-10 items-center justify-center rounded-2xl bg-primary/15 text-primary">
                    <ShieldCheck class="size-5" />
                  </div>
                  <div>
                    <p class="font-medium text-white">Scoped backend-issued connection material</p>
                    <p class="mt-1 text-sm leading-6 text-slate-400">Desktop runtime connects locally, while host, key, and access metadata stay under backend control.</p>
                  </div>
                </div>
              </div>
            </div>

            <Button type="button" variant="ghost" class="w-fit gap-2 px-0 text-slate-300 hover:text-white" onclick={() => (activeView = "signup") }>
              Need an account first?
              <ArrowRight class="size-4" />
            </Button>
          </div>
        </div>
      {:else}
        <RequestAccessPanel />
      {/if}
    </div>
  </div>
</div>
