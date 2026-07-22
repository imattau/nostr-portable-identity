<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface IdentityInfo {
    name: string;
    provider_type: string;
    available: boolean;
  }

  // Vault creation
  let vaultType: "usb" | "local" = $state("local");
  let path = $state("");
  let name = $state("Primary identity");
  let passphrase = $state("");
  let confirmPassphrase = $state("");
  let nsec = $state("");
  let error = $state<string | null>(null);
  let success = $state<string | null>(null);

  // Identity management
  let identities = $state<IdentityInfo[]>([]);

  onMount(async () => {
    await loadIdentities();
  });

  async function loadIdentities() {
    try {
      identities = await invoke<IdentityInfo[]>("list_local_vaults");
    } catch {
      // Not available, ignored
    }
  }

  async function createVault() {
    error = null;
    success = null;
    if (passphrase !== confirmPassphrase) {
      error = "Passphrases do not match";
      return;
    }
    if (!name) {
      error = "Identity name is required";
      return;
    }
    if (vaultType === "usb" && !path) {
      error = "USB mount path is required";
      return;
    }
    try {
      if (vaultType === "usb") {
        await invoke("create_vault", {
          path,
          name,
          passphrase,
          nsec: nsec || null,
        });
        success = `USB vault '${name}' created at ${path}/NOSTR-SIGNER/nostr-vault.json`;
      } else {
        await invoke("create_local_vault", {
          name,
          passphrase,
          nsec: nsec || null,
        });
        success = `Local vault '${name}' created in ~/.nostr-portable-identity/vaults/`;
      }
      await loadIdentities();
    } catch (e) {
      error = String(e);
    }
  }

  async function switchIdentity(name: string) {
    try {
      await invoke("switch_local_identity", { name });
      await loadIdentities();
      success = `Switched to '${name}'`;
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

  <div class="vault-type-toggle">
    <button
      class:active={vaultType === "local"}
      onclick={() => (vaultType = "local")}
    >Local</button>
    <button
      class:active={vaultType === "usb"}
      onclick={() => (vaultType = "usb")}
    >USB Drive</button>
  </div>

  {#if vaultType === "usb"}
    <input type="text" placeholder="USB mount path (e.g. /media/usb)" bind:value={path} />
  {:else}
    <p class="hint">Stored in <code>~/.nostr-portable-identity/vaults/</code></p>
  {/if}

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
  <button onclick={createVault}>Create Vault</button>

  {#if identities.length > 0}
    <h3>Local Identities</h3>
    <div class="identity-list">
      {#each identities as id}
        <div class="identity-row">
          <span>
            <strong>{id.name}</strong>
            <span class="tag">{id.provider_type}</span>
          </span>
          {#if id.available}
            <span class="badge active">active</span>
          {:else}
            <button onclick={() => switchIdentity(id.name)}>Switch</button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
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

  .hint {
    font-size: 12px;
    color: #888;
    margin: 0 0 4px 0;
  }

  .hint code {
    background: #f0f0f0;
    padding: 2px 6px;
    border-radius: 4px;
  }

  input {
    margin-bottom: 4px;
  }

  .vault-type-toggle {
    display: flex;
    gap: 0;
    margin-bottom: 8px;
  }

  .vault-type-toggle button {
    flex: 1;
    border-radius: 0;
    border: 1px solid #ccc;
    background: #f8f8f8;
    padding: 8px;
    font-size: 13px;
  }

  .vault-type-toggle button:first-child {
    border-radius: 6px 0 0 6px;
  }

  .vault-type-toggle button:last-child {
    border-radius: 0 6px 6px 0;
  }

  .vault-type-toggle button.active {
    background: #396cd8;
    color: #fff;
    border-color: #396cd8;
  }

  .identity-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .identity-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: #fff;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
  }

  .tag {
    font-size: 11px;
    background: #e8e8e8;
    padding: 1px 6px;
    border-radius: 4px;
    margin-left: 6px;
  }

  .badge.active {
    color: #2ecc71;
    font-size: 12px;
    font-weight: bold;
  }
</style>
