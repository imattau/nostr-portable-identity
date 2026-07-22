window.addEventListener("message", async (event) => {
  if (event.source !== window) return;
  if (!event.data || event.data.type !== "nostr-request") return;

  const { method, params } = event.data;

  try {
    const response = await chrome.runtime.sendMessage({ method, params, origin: window.location.origin });
    if (response.error) {
      window.postMessage({ type: "nostr-response", method, error: response.error }, "*");
    } else {
      window.postMessage({ type: "nostr-response", method, result: response.result }, "*");
    }
  } catch (error) {
    window.postMessage({ type: "nostr-response", method, error: error.message }, "*");
  }
});
