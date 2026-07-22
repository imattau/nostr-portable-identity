<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  interface ApprovalRequest {
    id: string;
    client_identity: string;
    method: string;
    description: string;
    risk_level: string;
    details: string;
  }

  let request = $state<ApprovalRequest | null>(null);
  const appWindow = getCurrentWebviewWindow();

  onMount(async () => {
    const req = await invoke<string>("get_pending_approval");
    if (req) {
      request = JSON.parse(req);
    }
  });

  async function approve() {
    await invoke("submit_approval", { approved: true });
    await appWindow.close();
  }

  async function reject() {
    await invoke("submit_approval", { approved: false });
    await appWindow.close();
  }
</script>

<div class="window-container">
  <h2>Signing Approval</h2>
  {#if request}
    <div class="details">
      <p><strong>Client:</strong> {request.client_identity}</p>
      <p><strong>Method:</strong> {request.method}</p>
      <p><strong>Description:</strong> {request.description}</p>
      <p><strong>Risk Level:</strong> {request.risk_level}</p>
    </div>
    <div class="button-row">
      <button class="approve" onclick={approve}>Approve</button>
      <button class="reject" onclick={reject}>Reject</button>
    </div>
  {:else}
    <p>No pending request.</p>
  {/if}
</div>

<style>
  .details {
    background: #f9f9f9;
    padding: 12px;
    border-radius: 6px;
  }

  .details p {
    margin: 4px 0;
  }

  .approve {
    background: #2ecc71;
    color: #fff;
    border-color: #27ae60;
  }

  .reject {
    background: #e74c3c;
    color: #fff;
    border-color: #c0392b;
  }
</style>
