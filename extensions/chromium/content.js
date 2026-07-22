// Only accept messages from the NIP-07 provider injected into the same page
window.addEventListener("message", async (event) => {
  if (event.source !== window) return;
  if (!event.data || event.data.type !== "nostr-request") return;

  const { method, params } = event.data;

  try {
    const response = await chrome.runtime.sendMessage({ method, params, origin: window.location.origin });
    if (response.error) {
      window.postMessage({ type: "nostr-response", method, error: response.error }, window.location.origin);
    } else {
      window.postMessage({ type: "nostr-response", method, result: response.result }, window.location.origin);
    }
  } catch (error) {
    window.postMessage({ type: "nostr-response", method, error: error.message }, window.location.origin);
  }
});
