<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let path = $state("");
  let name = $state("Primary identity");
  let passphrase = $state("");
  let confirmPassphrase = $state("");
  let nsec = $state("");
  let error = $state<string | null>(null);
  let success = $state<string | null>(null);

  async function createVault() {
    error = null;
    success = null;
    if (passphrase !== confirmPassphrase) {
      error = "Passphrases do not match";
      return;
    }
    if (!path) {
      error = "Vault path is required";
      return;
    }
    try {
      await invoke("create_vault", {
        path,
        name,
        passphrase,
        nsec: nsec || null,
      });
      success = "Vault created successfully!";
    } catch (e) {
      error = String(e);
    }
  }

  async function showVaultInfo() {
    error = null;
    success = null;
    if (!path) {
      error = "Vault path is required";
      return;
    }
    try {
      const info = await invoke<string>("vault_info", { path });
      success = info;
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="window-container">
  <h2>Settings</h2>

  <h3>Create or Import Vault</h3>
  {#if error}
    <div class="error">{error}</div>
  {/if}
  {#if success}
    <div class="success">{success}</div>
  {/if}

  <input type="text" placeholder="USB mount path" bind:value={path} />
  <input type="text" placeholder="Identity name" bind:value={name} />
  <input type="password" placeholder="Passphrase" bind:value={passphrase} />
  <input
    type="password"
    placeholder="Confirm passphrase"
    bind:value={confirmPassphrase}
  />
  <input
    type="text"
    placeholder="nsec to import (optional)"
    bind:value={nsec}
  />
  <div class="button-row">
    <button onclick={createVault}>Create Vault</button>
    <button onclick={showVaultInfo}>Show Vault Info</button>
  </div>
</div>

<style>
  .error {
    color: #e74c3c;
    background: #fdf0ef;
    padding: 8px;
    border-radius: 6px;
  }

  .success {
    color: #27ae60;
    background: #f0fdf4;
    padding: 8px;
    border-radius: 6px;
    white-space: pre-wrap;
    font-family: monospace;
    font-size: 12px;
  }

  input {
    margin-bottom: 4px;
  }
</style>
