<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebviewWindow, WebviewWindow } from "@tauri-apps/api/webviewWindow";

  interface SignerStatus {
    state: string;
    vault_present: boolean;
    public_key: string | null;
    unlocked_at: number | null;
  }

  let status: SignerStatus | null = $state(null);
  let error: string | null = $state(null);
  let passphrase: string = $state("");
  let vaultPath: string = $state("");
  let signingContent: string = $state("");
  let signedEvent: string | null = $state(null);
  let approvedRequest: string | null = $state(null);

  const appWindow = getCurrentWebviewWindow();

  onMount(async () => {
    await refreshStatus();

    const unlisten = await listen<string>("approval-request", (event) => {
      approvedRequest = event.payload;
    });

    return () => {
      unlisten();
    };
  });

  async function refreshStatus() {
    try {
      status = await invoke<SignerStatus>("get_status");
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  async function unlock() {
    try {
      await invoke("unlock_vault", {
        path: vaultPath || "/media/usb",
        passphrase,
        timeout: 300,
      });
      passphrase = "";
      await refreshStatus();
    } catch (e) {
      error = String(e);
    }
  }

  async function lock() {
    try {
      await invoke("lock_signer");
      await refreshStatus();
    } catch (e) {
      error = String(e);
    }
  }

  async function getPubkey() {
    try {
      const pk = await invoke<string>("get_public_key");
      await navigator.clipboard.writeText(pk);
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  async function signEvent() {
    try {
      const result = await invoke<string>("sign_text_note", {
        content: signingContent,
      });
      signedEvent = result;
      signingContent = "";
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  async function approveRequest() {
    try {
      await invoke("submit_approval", { approved: true });
      approvedRequest = null;
      await refreshStatus();
    } catch (e) {
      error = String(e);
    }
  }

  async function rejectRequest() {
    try {
      await invoke("submit_approval", { approved: false });
      approvedRequest = null;
    } catch (e) {
      error = String(e);
    }
  }

  function openSettings() {
    const settingsWindow = new WebviewWindow("settings", {
      url: "index.html",
      title: "Settings - Nostr Portable Identity",
      width: 500,
      height: 600,
      resizable: true,
    });
  }
</script>

<div class="window-container">
  <h2>Nostr Portable Identity</h2>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if status}
    <div class="status-bar">
      <span
        class="status-indicator"
        class:status-locked={status.state === "locked"}
        class:status-unlocked={status.state === "unlocked"}
        class:status-absent={!status.vault_present}
      ></span>
      <strong>{status.state}</strong>
      {#if status.public_key}
        <span class="pubkey">npub: {status.public_key.slice(0, 16)}...</span>
      {/if}
    </div>
  {/if}

  {#if approvedRequest}
    <div class="approval-panel">
      <h3>Approval Required</h3>
      <p>{approvedRequest}</p>
      <div class="button-row">
        <button onclick={approveRequest}>Approve</button>
        <button onclick={rejectRequest}>Reject</button>
      </div>
    </div>
  {/if}

  {#if !status || status.state === "locked"}
    <div class="unlock-panel">
      <h3>Unlock Vault</h3>
      <input
        type="text"
        placeholder="Vault path (e.g. /media/usb)"
        bind:value={vaultPath}
      />
      <input
        type="password"
        placeholder="Passphrase"
        bind:value={passphrase}
      />
      <button onclick={unlock}>Unlock</button>
      <button class="secondary" onclick={openSettings}>+ Create New Vault</button>
    </div>
  {:else}
    <div class="actions-panel">
      <h3>Signer Active</h3>
      <div class="button-row">
        <button onclick={getPubkey}>Copy Public Key</button>
        <button onclick={lock}>Lock</button>
      </div>
      <div class="sign-panel">
        <input
          type="text"
          placeholder="Enter text to sign..."
          bind:value={signingContent}
        />
        <button onclick={signEvent}>Sign Text Note</button>
      </div>
      {#if signedEvent}
        <div class="result">
          <strong>Signed:</strong> {signedEvent}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .error {
    color: #e74c3c;
    background: #fdf0ef;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 13px;
  }

  .status-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: #fff;
    border-radius: 6px;
    border: 1px solid #e0e0e0;
  }

  .pubkey {
    color: #666;
    font-size: 12px;
    font-family: monospace;
  }

  .button-row {
    display: flex;
    gap: 8px;
    margin: 8px 0;
  }

  .unlock-panel,
  .actions-panel,
  .approval-panel {
    background: #fff;
    padding: 16px;
    border-radius: 8px;
    border: 1px solid #e0e0e0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .sign-panel {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .result {
    background: #f0fdf4;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-family: monospace;
    word-break: break-all;
  }

  .secondary {
    background: #f0f0f0;
    border-color: #ccc;
    font-size: 13px;
  }
</style>
