<script lang="ts">
  import { goto } from "$app/navigation";

  import SshKeysView from "$lib/components/ssh-keys-view.svelte";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();
</script>

<SshKeysView
  keys={app.keys}
  onNew={() => goto("/keys/new")}
  onEdit={(key) => goto(`/keys/${key.id}/edit`)}
  onDelete={app.deleteKey}
/>
