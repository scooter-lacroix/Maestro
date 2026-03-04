/**
 * TrackLens - Crypto Utility
 *
 * AES-256-GCM encryption for zero-knowledge storage.
 *
 * REBRANDED: Plannotator → TrackLens
 */

export async function encrypt(
  compressedData: string
): Promise<{ ciphertext: string; key: string }> {
  const cryptoKey = await crypto.subtle.generateKey(
    { name: 'AES-GCM', length: 256 },
    true,
    ['encrypt']
  );

  const iv = crypto.getRandomValues(new Uint8Array(12));
  const plaintext = new TextEncoder().encode(compressedData);

  const encrypted = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    cryptoKey,
    plaintext
  );

  const combined = new Uint8Array(iv.length + encrypted.byteLength);
  combined.set(iv, 0);
  combined.set(new Uint8Array(encrypted), iv.length);

  const rawKey = await crypto.subtle.exportKey('raw', cryptoKey);

  return {
    ciphertext: bytesToBase64url(combined),
    key: bytesToBase64url(new Uint8Array(rawKey)),
  };
}

export async function decrypt(
  ciphertext: string,
  key: string
): Promise<string> {
  const combined = base64urlToBytes(ciphertext);
  const rawKey = base64urlToBytes(key);

  const iv = combined.slice(0, 12);
  const encrypted = combined.slice(12);

  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    rawKey as any,
    { name: 'AES-GCM', length: 256 },
    false,
    ['decrypt']
  );

  const decrypted = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv },
    cryptoKey,
    encrypted
  );

  return new TextDecoder().decode(decrypted);
}

function bytesToBase64url(bytes: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=/g, '');
}

function base64urlToBytes(b64: string): Uint8Array {
  const base64 = b64.replace(/-/g, '+').replace(/_/g, '/');
  const binary = atob(base64);
  return Uint8Array.from(binary, c => c.charCodeAt(0));
}
