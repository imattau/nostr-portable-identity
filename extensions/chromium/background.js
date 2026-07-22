const NATIVE_HOST = "com.nostr.portable.identity";

let pendingRequests = new Map();
let nextId = 1;

async function sendNativeMessage(message) {
  return new Promise((resolve, reject) => {
    const id = String(nextId++);
    message.id = id;

    pendingRequests.set(id, { resolve, reject });

    chrome.runtime.sendNativeMessage(NATIVE_HOST, message, (response) => {
      if (chrome.runtime.lastError) {
        const pending = pendingRequests.get(id);
        if (pending) {
          pendingRequests.delete(id);
          reject(new Error(chrome.runtime.lastError.message));
        }
        return;
      }

      if (response && response.id === id) {
        const pending = pendingRequests.get(id);
        if (pending) {
          pendingRequests.delete(id);
          if (response.error) {
            reject(new Error(response.error));
          } else {
            resolve(response.result);
          }
        }
      }
    });

    setTimeout(() => {
      const pending = pendingRequests.get(id);
      if (pending) {
        pendingRequests.delete(id);
        reject(new Error("Request timed out"));
      }
    }, 30000);
  });
}

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (!request || !request.method) {
    sendResponse({ error: "invalid request" });
    return false;
  }

  const origin = sender?.origin || request.origin || "unknown";

  const message = {
    method: request.method,
    origin: origin,
    params: request.params || {},
  };

  sendNativeMessage(message)
    .then((result) => sendResponse({ result }))
    .catch((error) => sendResponse({ error: error.message }));

  return true;
});
