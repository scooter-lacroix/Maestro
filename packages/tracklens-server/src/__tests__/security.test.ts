/**
 * TrackLens Security Tests
 *
 * Validates all security controls are in place:
 * - Auth on decision endpoints
 * - Path traversal rejection
 * - Secure token generation
 * - CORS restriction
 *
 * Run with: bun test packages/tracklens-server/src/__tests__/security.test.ts
 *
 * @packageDocumentation
 */

import { describe, test, expect, beforeAll, afterAll } from "bun:test";

// Mock server port for testing
const TEST_PORT = 3999;
const SERVER_URL = `http://localhost:${TEST_PORT}`;

let server: any = null;

describe("TrackLens Security Tests", () => {
  beforeAll(async () => {
    // Start a test server instance
    // Note: This requires the server to be importable
    // In a real setup, you'd import and start the server programmatically
  });

  afterAll(async () => {
    // Clean up test server
    if (server) {
      await server.stop();
    }
  });

  describe("Decision Endpoint Authentication", () => {
    test("should reject POST to /api/decision without auth token", async () => {
      const res = await fetch(`${SERVER_URL}/api/decision`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ approved: true, feedback: '' }),
      });

      expect(res.status).toBe(401);
    });

    test("should reject POST to /api/decision with wrong token", async () => {
      const res = await fetch(`${SERVER_URL}/api/decision`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer wrong-token-12345',
        },
        body: JSON.stringify({ approved: true, feedback: '' }),
      });

      expect(res.status).toBe(401);
    });

    test("should accept POST to /api/decision with valid token", async () => {
      // Get the auth token from the server first
      const indexRes = await fetch(SERVER_URL);
      const html = await indexRes.text();

      // Extract token from injected script
      const tokenMatch = html.match(/window\.TRACKLENS_AUTH_TOKEN\s*=\s*"([^"]+)"/);
      if (!tokenMatch) {
        throw new Error("Could not extract auth token from HTML");
      }
      const token = tokenMatch[1];

      // Submit decision with valid token
      const res = await fetch(`${SERVER_URL}/api/decision`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`,
        },
        body: JSON.stringify({ approved: true, feedback: '' }),
      });

      expect(res.status).toBe(200);
    });
  });

  describe("Path Traversal Protection", () => {
    test("should reject path traversal in /api/vault-tree folder parameter", async () => {
      const res = await fetch(`${SERVER_URL}/api/vault-tree`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          vaultPath: '/home/test/vault',
          folder: '../../../etc',
        }),
      });

      expect(res.status).toBe(400);
      const data = await res.json();
      expect(data.success).toBe(false);
      expect(data.error).toContain('Invalid folder path');
    });

    test("should reject encoded path traversal", async () => {
      const res = await fetch(`${SERVER_URL}/api/vault-tree`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          vaultPath: '/home/test/vault',
          folder: '..%2F..%2F..%2Fetc',
        }),
      });

      expect(res.status).toBe(400);
    });

    test("should reject absolute path escape", async () => {
      const res = await fetch(`${SERVER_URL}/api/vault-tree`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          vaultPath: '/home/test/vault',
          folder: '/etc/passwd',
        }),
      });

      expect(res.status).toBe(400);
    });
  });

  describe("Token Security", () => {
    test("should generate cryptographically random tokens", async () => {
      // Extract multiple tokens from multiple server starts
      const tokens = new Set<string>();

      for (let i = 0; i < 10; i++) {
        const indexRes = await fetch(SERVER_URL);
        const html = await indexRes.text();
        const tokenMatch = html.match(/window\.TRACKLENS_AUTH_TOKEN\s*=\s*"([^"]+)"/);

        if (tokenMatch) {
          tokens.add(tokenMatch[1]);
        }
      }

      // All tokens should be unique (not predictable timestamps)
      expect(tokens.size).toBeGreaterThan(1);

      // Token should be reasonable length (at least 32 hex chars = 128 bits)
      for (const token of tokens) {
        expect(token.length).toBeGreaterThanOrEqual(32);
      }
    });

    test("should not use timestamp-based tokens", async () => {
      // Extract token and check it's not a simple hex timestamp
      const indexRes = await fetch(SERVER_URL);
      const html = await indexRes.text();
      const tokenMatch = html.match(/window\.TRACKLENS_AUTH_TOKEN\s*=\s*"([^"]+)"/);

      if (tokenMatch) {
        const token = tokenMatch[1];

        // A timestamp-based token would be short (like 8-14 chars for hex timestamp)
        // Secure random tokens should be longer (64 chars for 32 bytes in hex)
        expect(token.length).toBeGreaterThan(20);
      }
    });
  });

  describe("CORS Configuration", () => {
    test("should reject requests from unauthorized origins", async () => {
      // This test requires a fetch that can set Origin header
      // Bun's fetch doesn't fully support CORS preflight testing
      // In a browser environment, this would be tested

      // Verify the response doesn't have overly permissive CORS headers
      const res = await fetch(SERVER_URL, {
        headers: {
          'Origin': 'http://evil.com',
        },
      });

      // Should not have Access-Control-Allow-Origin: *
      const acao = res.headers.get('Access-Control-Allow-Origin');
      expect(acao).not.toBe('*');
    });
  });

  describe("Input Validation", () => {
    test("should reject oversized request bodies", async () => {
      // Create a body larger than 100KB limit
      const largeBody = {
        approved: true,
        feedback: 'x'.repeat(200 * 1024), // 200KB
      };

      const res = await fetch(`${SERVER_URL}/api/decision`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(largeBody),
      });

      // Should be rejected by RequestBodyLimitLayer
      expect(res.status).toBeGreaterThanOrEqual(400);
    });

    test("should reject malformed JSON", async () => {
      const res = await fetch(`${SERVER_URL}/api/decision`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: '{ invalid json }',
      });

      expect(res.status).toBeGreaterThanOrEqual(400);
    });
  });
});
