<script lang="ts">
  import { goto } from "$app/navigation";

  import SshKeyForm from "$lib/components/ssh-key-form.svelte";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();

  async function handleSave(name: string, privateKey: string, passphrase: string) {
    await app.saveKey(name, privateKey, passphrase);
    await goto("/keys");
  }
</script>

<SshKeyForm
  onSave={handleSave}
  onUpdate={app.updateKey}
  onCancel={() => goto("/keys")}
/>
