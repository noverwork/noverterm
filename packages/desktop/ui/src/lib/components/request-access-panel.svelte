<script lang="ts">
  import { CheckCircle2, Copy, Mail, ShieldCheck } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { buildSignupSummary, validateSignupDraft } from "$lib/view-models/auth-and-sessions.js";

  let fullName = $state("");
  let email = $state("");
  let team = $state("");
  let useCase = $state("");
  let submitted = $state(false);
  let feedback = $state<string | null>(null);
  let signupPrepared = $state(false);

  const errors = $derived.by(() => {
    return validateSignupDraft({ fullName, email, team, useCase });
  });

  const signupSummary = $derived.by(() => {
    return buildSignupSummary({ fullName, email, team, useCase });
  });

  const isValid = $derived(Object.keys(errors).length === 0);

  function prepareSignup() {
    submitted = true;
    if (!isValid) return false;
    signupPrepared = true;
    feedback = "Account setup created. Final activation still completes once your backend workspace is provisioned.";
    return true;
  }

  function handleCreateAccount() {
    prepareSignup();
  }

  async function copySummary() {
    if (!prepareSignup()) return;

    try {
      await navigator.clipboard.writeText(signupSummary);
      feedback = "Account setup summary copied. Share it with your workspace admin if activation still needs approval.";
    } catch {
      feedback = "Unable to access your clipboard. Copy the account setup summary manually.";
    }
  }

  function emailSummary() {
    if (!prepareSignup()) return;
    const subject = encodeURIComponent("Noverterm account setup");
    const body = encodeURIComponent(signupSummary);
    window.location.href = `mailto:?subject=${subject}&body=${body}`;
  }
</script>

<div class="space-y-6">
  <div class="space-y-2">
    <div class="hero-chip w-fit">
      <ShieldCheck class="size-3.5" />
      Create your account
    </div>
    <div>
      <h2 class="text-2xl font-semibold tracking-tight text-white">Sign up for Noverterm</h2>
      <p class="mt-2 max-w-md text-sm leading-6 text-slate-300">
        Set up your operator profile, team context, and intended access scope. You will create your
        account here, then finish activation once your backend workspace is provisioned.
      </p>
    </div>
  </div>

  <div class="grid gap-5 lg:grid-cols-[1.1fr_0.9fr]">
    <div class="space-y-4 rounded-2xl border border-white/10 bg-white/5 p-5 shadow-lg backdrop-blur-md">
      <div class="grid gap-4 sm:grid-cols-2">
        <div class="space-y-2 sm:col-span-2">
          <label for="request-name" class="text-sm font-medium text-slate-100">Full name</label>
          <Input id="request-name" bind:value={fullName} placeholder="Alex Chen" class={submitted && errors.fullName ? "border-destructive" : "bg-white/5 border-white/10 text-white placeholder:text-slate-400"} />
          {#if submitted && errors.fullName}
            <p class="text-xs text-destructive" role="alert">{errors.fullName}</p>
          {/if}
        </div>

        <div class="space-y-2 sm:col-span-2">
          <label for="request-email" class="text-sm font-medium text-slate-100">Work email</label>
          <Input id="request-email" bind:value={email} placeholder="alex@company.com" class={submitted && errors.email ? "border-destructive" : "bg-white/5 border-white/10 text-white placeholder:text-slate-400"} />
          {#if submitted && errors.email}
            <p class="text-xs text-destructive" role="alert">{errors.email}</p>
          {/if}
        </div>

        <div class="space-y-2 sm:col-span-2">
          <label for="request-team" class="text-sm font-medium text-slate-100">Team or workspace</label>
          <Input id="request-team" bind:value={team} placeholder="Platform Engineering" class={submitted && errors.team ? "border-destructive" : "bg-white/5 border-white/10 text-white placeholder:text-slate-400"} />
          {#if submitted && errors.team}
            <p class="text-xs text-destructive" role="alert">{errors.team}</p>
          {/if}
        </div>

        <div class="space-y-2 sm:col-span-2">
          <label for="request-use-case" class="text-sm font-medium text-slate-100">What do you need access to?</label>
          <textarea
            id="request-use-case"
            bind:value={useCase}
            rows="4"
            placeholder="Describe the environments, hosts, or operational tasks you need to access."
            class="flex w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm text-white shadow-sm transition-colors placeholder:text-slate-400 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          ></textarea>
          {#if submitted && errors.useCase}
            <p class="text-xs text-destructive" role="alert">{errors.useCase}</p>
          {/if}
        </div>
      </div>

      <div class="flex flex-wrap gap-3">
        <Button class="gap-2" onclick={handleCreateAccount}>
          <ShieldCheck class="size-4" />
          Create account
        </Button>
        <Button variant="outline" class="gap-2 border-white/12 bg-white/4 text-white hover:bg-white/8" onclick={copySummary}>
          <Copy class="size-4" />
          Copy summary
        </Button>
        <Button variant="outline" class="gap-2 border-white/12 bg-white/4 text-white hover:bg-white/8" onclick={emailSummary}>
          <Mail class="size-4" />
          Email summary
        </Button>
      </div>

      {#if feedback}
        <div class="rounded-xl border border-emerald-400/20 bg-emerald-400/10 px-3 py-2 text-sm text-emerald-100" role="status">
          {feedback}
        </div>
      {/if}
    </div>

    <div class="rounded-2xl border border-white/10 bg-white/5 p-5 shadow-lg backdrop-blur-md">
      <p class="section-title">Account summary</p>
      <pre class="mt-4 overflow-auto rounded-xl border border-white/8 bg-slate-950/60 p-4 text-xs leading-6 text-slate-200 whitespace-pre-wrap">{signupSummary}</pre>

      <div class="mt-5 rounded-xl border border-white/8 bg-slate-950/35 p-4 text-sm text-slate-300">
        <div class="flex items-start gap-3">
          <CheckCircle2 class="mt-0.5 size-4 shrink-0 text-emerald-300" />
          <div class="space-y-2">
            <p class="font-medium text-slate-100">What happens next?</p>
            <ul class="space-y-1 leading-6">
              <li>• Your account profile is ready from the UI side as soon as you submit it.</li>
              <li>• Backend provisioning then activates workspace access and connection permissions.</li>
              <li>• Once activated, you can sign in here and manage saved hosts securely.</li>
            </ul>
          </div>
        </div>
      </div>

      {#if signupPrepared}
        <div class="mt-5 rounded-xl border border-primary/25 bg-primary/10 p-4 text-sm text-slate-100">
          Account creation has been prepared. The only remaining step is backend activation for your
          workspace.
        </div>
      {/if}
    </div>
  </div>
</div>
