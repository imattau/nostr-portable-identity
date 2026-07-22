(function () {
  if (window.nostr) {
    return;
  }

  const provider = {
    getPublicKey: async () => {
      return callExtension("getPublicKey", {});
    },

    signEvent: async (event) => {
      return callExtension("signEvent", { event });
    },

    nip44: {
      encrypt: async (pubkey, plaintext) => {
        return callExtension("nip44Encrypt", { pubkey, plaintext });
      },
      decrypt: async (pubkey, ciphertext) => {
        return callExtension("nip44Decrypt", { pubkey, ciphertext });
      },
    },
  };

  function callExtension(method, params) {
    return new Promise((resolve, reject) => {
      const handler = (event) => {
        if (event.source !== window) return;
        if (event.data && event.data.type === "nostr-response" && event.data.method === method) {
          window.removeEventListener("message", handler);
          if (event.data.error) {
            reject(new Error(event.data.error));
          } else {
            resolve(event.data.result);
          }
        }
      };
      window.addEventListener("message", handler);
      window.postMessage(
        { type: "nostr-request", method, params },
        window.location.origin
      );
    });
  }

  Object.defineProperty(window, "nostr", {
    value: provider,
    writable: false,
    configurable: false,
  });
})();
