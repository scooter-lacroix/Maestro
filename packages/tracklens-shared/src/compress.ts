/**
 * TrackLens - Compression Utility
 *
 * Portable deflate-raw + base64url compression.
 * Uses only Web APIs (CompressionStream, TextEncoder, btoa).
 *
 * REBRANDED: Plannotator → TrackLens
 */

export async function compress(data: unknown): Promise<string> {
  const json = JSON.stringify(data);
  const byteArray = new TextEncoder().encode(json);

  const stream = new CompressionStream('deflate-raw');
  const writer = stream.writable.getWriter();
  writer.write(byteArray);
  writer.close();

  const buffer = await new Response(stream.readable).arrayBuffer();
  const compressed = new Uint8Array(buffer);

  // Optimized: Use Buffer.from in Node/Bun environments for direct base64url conversion
  // Falls back to char-by-char for browser-only environments
  let base64: string;
  if (typeof Buffer !== 'undefined') {
    // Node/Bun: Direct conversion from Uint8Array to base64url
    base64 = Buffer.from(compressed).toString('base64url');
  } else {
    // Browser fallback: Char-by-char conversion (unavoidable without Buffer)
    let binary = '';
    for (let i = 0; i < compressed.length; i++) {
      binary += String.fromCharCode(compressed[i]);
    }
    base64 = btoa(binary)
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=/g, '');
  }

  return base64;
}

export async function decompress(b64: string): Promise<unknown> {
  const base64 = b64
    .replace(/-/g, '+')
    .replace(/_/g, '/');

  const binary = atob(base64);
  const byteArray = Uint8Array.from(binary, c => c.charCodeAt(0));

  const stream = new DecompressionStream('deflate-raw');
  const writer = stream.writable.getWriter();
  writer.write(byteArray);
  writer.close();

  const buffer = await new Response(stream.readable).arrayBuffer();
  const json = new TextDecoder().decode(buffer);

  return JSON.parse(json);
}
