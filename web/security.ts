const base64url = (value: ArrayBuffer) =>
  btoa(String.fromCharCode(...new Uint8Array(value))).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
let keyPromise: Promise<CryptoKeyPair> | undefined;
export function signingKey(): Promise<CryptoKeyPair> {
  return keyPromise ??= (async () => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open("prm-security", 1);
      req.onupgradeneeded = () => req.result.createObjectStore("keys");
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
    try {
      const existing = await new Promise<CryptoKeyPair | undefined>((resolve, reject) => {
        const req = db.transaction("keys").objectStore("keys").get("device");
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
      });
      if (existing) return existing;
      const pair = await crypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign", "verify"]) as CryptoKeyPair;
      // add, rather than put: a second tab cannot overwrite an enrolled key.
      await new Promise<void>((resolve, reject) => {
        const tx = db.transaction("keys", "readwrite");
        tx.objectStore("keys").add(pair, "device");
        tx.oncomplete = () => resolve();
        tx.onabort = () => reject(new Error("设备密钥创建冲突，请刷新页面"));
      });
      return pair;
    } finally { db.close(); }
  })();
}
export async function devicePublicKey() {
  return base64url(await crypto.subtle.exportKey("raw", (await signingKey()).publicKey));
}
export async function signatureHeaders(path: string, method: string, body: string) {
  const timestamp = Math.floor(Date.now() / 1000).toString();
  const nonce = base64url(crypto.getRandomValues(new Uint8Array(24)).buffer);
  const digest = base64url(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(body)));
  const message = `PRM-REQUEST-V1\n${method}\n${path}\n${timestamp}\n${nonce}\n${digest}`;
  const signature = base64url(await crypto.subtle.sign("Ed25519", (await signingKey()).privateKey, new TextEncoder().encode(message)));
  return { "X-PRM-Timestamp": timestamp, "X-PRM-Nonce": nonce, "X-PRM-Signature": signature };
}
