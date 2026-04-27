<script lang="ts">
  import SshKeysView from "$lib/components/ssh-keys-view.svelte";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();
</script>

<SshKeysView
  keys={app.keys}
  onSave={app.saveKey}
  onUpdate={app.updateKey}
  onDelete={app.deleteKey}
/>
