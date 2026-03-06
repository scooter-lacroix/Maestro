/**
 * TrackLens Server Utilities
 *
 * Common utility functions for TrackLens server modules.
 */

import { randomUUID } from "crypto";

// ============================================================================
// CORS Headers
// ============================================================================

/** Default CORS headers for API responses */
export const corsHeaders = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, Authorization",
};

/**
 * Create a JSON response with CORS headers
 */
export function jsonResponse<T>(data: T, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json",
      ...corsHeaders,
    },
  });
}

/**
 * Create an error response with CORS headers
 */
export function errorResponse(message: string, status = 500): Response {
  return jsonResponse({ success: false, error: message }, status);
}

// ============================================================================
// Authentication
// ============================================================================

/**
 * Generate a secure authentication token
 */
export function generateAuthToken(): string {
  return randomUUID();
}

/**
 * Validate authentication header against token
 */
export function validateAuthHeader(authHeader: string | null, expectedToken: string): boolean {
  if (!authHeader) return false;
  return authHeader === `Bearer ${expectedToken}`;
}

// ============================================================================
// Port Management
// ============================================================================

const DEFAULT_MAX_PORT_RETRIES = 5;
const PORT_RETRY_DELAY_MS = 500;

/**
 * Try to start a server with automatic port retry logic
 */
export async function startServerWithRetry(
  port: number,
  fetchHandler: (req: Request) => Promise<Response> | Response,
  maxRetries = DEFAULT_MAX_PORT_RETRIES
): Promise<ReturnType<typeof Bun.serve>> {
  let lastError: Error | undefined;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const server = Bun.serve({
        port,
        fetch: fetchHandler,
      });
      return server;
    } catch (err) {
      lastError = err instanceof Error ? err : new Error(String(err));

      const isAddressInUse = lastError.message.includes("EADDRINUSE");

      if (isAddressInUse && attempt < maxRetries) {
        await sleep(PORT_RETRY_DELAY_MS);
        continue;
      }

      throw lastError;
    }
  }

  throw lastError || new Error("Failed to start server after retries");
}

// ============================================================================
// Async Utilities
// ============================================================================

/**
 * Sleep for a specified number of milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Create a deferred promise that can be resolved externally
 */
export function createDeferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (error: Error) => void;
} {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;

  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return { promise, resolve, reject };
}

// ============================================================================
// URL Utilities
// ============================================================================

/**
 * Get the origin from a request URL
 */
export function getRequestOrigin(req: Request): string {
  const url = new URL(req.url);
  return `${url.protocol}//${url.host}`;
}

/**
 * Check if a request is for an API endpoint
 */
export function isApiRequest(pathname: string): boolean {
  return pathname.startsWith("/api/");
}

// ============================================================================
// Logging
// ============================================================================

/**
 * Log a server message with prefix
 */
export function log(message: string, level: "info" | "error" | "warn" = "info") {
  const prefix = "[TrackLens]";
  const timestamp = new Date().toISOString();

  switch (level) {
    case "error":
      console.error(`${prefix} [${timestamp}] ERROR: ${message}`);
      break;
    case "warn":
      console.warn(`${prefix} [${timestamp}] WARN: ${message}`);
      break;
    default:
      console.log(`${prefix} [${timestamp}] ${message}`);
  }
}

// ============================================================================
// HTML Injection
// ============================================================================

/**
 * Inject a script tag into HTML content (before closing head tag)
 */
export function injectScriptIntoHtml(html: string, scriptContent: string): string {
  const scriptTag = `<script>${scriptContent}</script>`;
  return html.replace("</head>", `${scriptTag}</head>`);
}

/**
 * Inject an auth token into HTML for client-side access
 */
export function injectAuthToken(html: string, authToken: string): string {
  const tokenScript = `window.TRACKLENS_AUTH_TOKEN = "${authToken}";`;
  return injectScriptIntoHtml(html, tokenScript);
}
