/**
 * Tests for TrackLens recent document tracker
 *
 * Covers:
 * - Recording documents
 * - Retrieval (last, all)
 * - Expiration (10-minute TTL)
 * - Max entries cap (5)
 * - Clearing
 */

import { describe, test, expect, beforeEach } from "bun:test";
import {
  recordRecentDocument,
  getLastGeneratedDocument,
  getRecentDocuments,
  hasRecentDocument,
  clearRecentDocuments,
} from "../recentDoc";

describe("recordRecentDocument", () => {
  beforeEach(() => {
    clearRecentDocuments();
  });

  test("records a document and makes it available", () => {
    recordRecentDocument({
      trackId: "test-track",
      type: "spec.md",
      content: "# Test Spec",
    });

    expect(hasRecentDocument()).toBe(true);
    const doc = getLastGeneratedDocument();
    expect(doc).toBeDefined();
    expect(doc!.trackId).toBe("test-track");
    expect(doc!.type).toBe("spec.md");
    expect(doc!.content).toBe("# Test Spec");
    expect(doc!.timestamp).toBeGreaterThan(0);
  });

  test("records with optional filePath", () => {
    recordRecentDocument({
      trackId: "test-track",
      type: "plan.md",
      content: "# Plan",
      filePath: "maestro/tracks/test-track/plan.md",
    });

    const doc = getLastGeneratedDocument();
    expect(doc!.filePath).toBe("maestro/tracks/test-track/plan.md");
  });

  test("most recent document is returned first", () => {
    recordRecentDocument({ trackId: "first", type: "spec.md", content: "# First" });
    recordRecentDocument({ trackId: "second", type: "plan.md", content: "# Second" });

    const doc = getLastGeneratedDocument();
    expect(doc!.trackId).toBe("second");
  });

  test("caps at MAX_ENTRIES (5)", () => {
    for (let i = 0; i < 8; i++) {
      recordRecentDocument({
        trackId: `track-${i}`,
        type: "document",
        content: `# Doc ${i}`,
      });
    }

    const docs = getRecentDocuments();
    expect(docs.length).toBe(5);
    // Most recent first
    expect(docs[0]!.trackId).toBe("track-7");
  });
});

describe("getLastGeneratedDocument", () => {
  beforeEach(() => {
    clearRecentDocuments();
  });

  test("returns undefined when no documents recorded", () => {
    expect(getLastGeneratedDocument()).toBeUndefined();
  });

  test("returns undefined when all documents expired", () => {
    recordRecentDocument({
      trackId: "expired",
      type: "spec.md",
      content: "# Expired",
    });

    // Use 0ms max age — everything is expired
    expect(getLastGeneratedDocument({ maxAgeMs: 0 })).toBeUndefined();
  });

  test("respects custom maxAgeMs", () => {
    recordRecentDocument({
      trackId: "fresh",
      type: "spec.md",
      content: "# Fresh",
    });

    // Should be available with default max age
    expect(getLastGeneratedDocument()).toBeDefined();
    // Should be available with reasonable max age
    expect(getLastGeneratedDocument({ maxAgeMs: 60000 })).toBeDefined();
  });
});

describe("getRecentDocuments", () => {
  beforeEach(() => {
    clearRecentDocuments();
  });

  test("returns empty array when no documents", () => {
    expect(getRecentDocuments()).toEqual([]);
  });

  test("returns all non-expired documents", () => {
    recordRecentDocument({ trackId: "a", type: "spec.md", content: "# A" });
    recordRecentDocument({ trackId: "b", type: "plan.md", content: "# B" });
    recordRecentDocument({ trackId: "c", type: "walkthrough", content: "# C" });

    const docs = getRecentDocuments();
    expect(docs).toHaveLength(3);
    expect(docs.map((d) => d.trackId)).toEqual(["c", "b", "a"]);
  });
});

describe("hasRecentDocument", () => {
  beforeEach(() => {
    clearRecentDocuments();
  });

  test("returns false when empty", () => {
    expect(hasRecentDocument()).toBe(false);
  });

  test("returns true after recording", () => {
    recordRecentDocument({ trackId: "t", type: "document", content: "# Doc" });
    expect(hasRecentDocument()).toBe(true);
  });
});

describe("clearRecentDocuments", () => {
  test("clears all documents", () => {
    recordRecentDocument({ trackId: "t1", type: "spec.md", content: "# A" });
    recordRecentDocument({ trackId: "t2", type: "plan.md", content: "# B" });

    expect(hasRecentDocument()).toBe(true);
    clearRecentDocuments();
    expect(hasRecentDocument()).toBe(false);
    expect(getRecentDocuments()).toEqual([]);
  });
});
