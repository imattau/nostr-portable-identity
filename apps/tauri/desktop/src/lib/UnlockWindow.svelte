<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  let passphrase = $state("");
  let path = $state("/media/usb");
  let error = $state<string | null>(null);

  const appWindow = getCurrentWebviewWindow();

  async function submit() {
    error = null;
    try {
      await invoke("unlock_vault", {
        path,
        passphrase,
        timeout: 300,
      });
      await appWindow.close();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="window-container">
  <h2>Unlock Vault</h2>
  {#if error}
    <div class="error">{error}</div>
  {/if}
  <input type="text" placeholder="Vault path" bind:value={path} />
  <input
    type="password"
    placeholder="Passphrase"
    bind:value={passphrase}
    onkeydown={(e) => { if (e.key === "Enter") submit(); }}
  />
  <button onclick={submit}>Unlock</button>
</div>
