<script lang="ts">
  import { t } from "$lib/i18n/index.svelte.js";
</script>

<div class="flex h-full items-center justify-center bg-port p-8">
  <div class="max-w-md text-center">
    <h1 class="text-2xl font-semibold tracking-tight text-stencil">
      {t("No terminal selected")}
    </h1>
    <p class="mt-3 text-sm leading-6 text-stencil-mute">
      {t("Open a local terminal or choose an active session from the sidebar.")}
    </p>
  </div>
</div>
