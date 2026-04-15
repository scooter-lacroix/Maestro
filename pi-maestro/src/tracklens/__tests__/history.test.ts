/**
 * Tests for TrackLens review history persistence
 *
 * Covers:
 * - Loading empty history
 * - Appending entries
 * - History cap (MAX_HISTORY_ENTRIES = 50)
 * - Formatting for agent context
 * - Round-trip (append then load)
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test";
import { mkdtempSync, rmSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import {
  loadReviewHistory,
  appendReviewEntry,
  formatHistoryForAgent,
  type ReviewHistoryEntry,
} from "../history";

describe("loadReviewHistory", () => {
  test("returns empty array for non-existent history file", () => {
    const tempDir = mkdtempSync(join(tmpdir(), "tracklens-test-"));
    try {
      const history = loadReviewHistory(tempDir);
      expect(history).toEqual([]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  test("returns empty array for corrupted JSON", () => {
    const tempDir = mkdtempSync(join(tmpdir(), "tracklens-test-"));
    try {
      const { writeFileSync } = require("fs");
      writeFileSync(join(tempDir, "review-history.json"), "not json");
      const history = loadReviewHistory(tempDir);
      expect(history).toEqual([]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});

describe("appendReviewEntry", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), "tracklens-test-"));
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  test("creates history file and appends entry", () => {
    const entry: ReviewHistoryEntry = {
      timestamp: new Date().toISOString(),
      documentType: "spec.md",
      approved: true,
      annotationCount: 0,
      reviewDurationMs: 5000,
      iteration: 0,
    };

    appendReviewEntry(tempDir, entry);

    const history = loadReviewHistory(tempDir);
    expect(history).toHaveLength(1);
    expect(history[0]!.documentType).toBe("spec.md");
    expect(history[0]!.approved).toBe(true);
  });

  test("appends multiple entries (newest first)", () => {
    const entry1: ReviewHistoryEntry = {
      timestamp: "2026-01-01T00:00:00.000Z",
      documentType: "spec.md",
      approved: false,
      annotationCount: 2,
      reviewDurationMs: 3000,
      iteration: 0,
    };

    const entry2: ReviewHistoryEntry = {
      timestamp: "2026-01-02T00:00:00.000Z",
      documentType: "plan.md",
      approved: true,
      annotationCount: 0,
      reviewDurationMs: 4000,
      iteration: 1,
    };

    appendReviewEntry(tempDir, entry1);
    appendReviewEntry(tempDir, entry2);

    const history = loadReviewHistory(tempDir);
    expect(history).toHaveLength(2);
    // Newest first
    expect(history[0]!.documentType).toBe("plan.md");
    expect(history[1]!.documentType).toBe("spec.md");
  });

  test("caps at 50 entries", () => {
    for (let i = 0; i < 55; i++) {
      appendReviewEntry(tempDir, {
        timestamp: new Date().toISOString(),
        documentType: "spec.md",
        approved: i % 2 === 0,
        annotationCount: i,
        reviewDurationMs: 1000 + i * 100,
        iteration: Math.floor(i / 5),
      });
    }

    const history = loadReviewHistory(tempDir);
    expect(history.length).toBe(50);
  });

  test("round-trips entry with all fields", () => {
    const entry: ReviewHistoryEntry = {
      timestamp: "2026-04-12T10:30:00.000Z",
      documentType: "walkthrough",
      approved: false,
      annotationCount: 3,
      feedback: "Fix the error handling section",
      editedContent: "# Updated walkthrough\n\nWith fixes",
      reviewDurationMs: 45000,
      iteration: 2,
    };

    appendReviewEntry(tempDir, entry);

    const history = loadReviewHistory(tempDir);
    expect(history).toHaveLength(1);
    expect(history[0]).toEqual(entry);
  });
});

describe("formatHistoryForAgent", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), "tracklens-test-"));
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  test("returns empty string for no history", () => {
    expect(formatHistoryForAgent(tempDir)).toBe("");
  });

  test("formats recent entries as markdown", () => {
    appendReviewEntry(tempDir, {
      timestamp: new Date().toISOString(),
      documentType: "spec.md",
      approved: true,
      annotationCount: 0,
      reviewDurationMs: 5000,
      iteration: 0,
    });

    const formatted = formatHistoryForAgent(tempDir);
    expect(formatted).toContain("## Review History");
    expect(formatted).toContain("Approved");
    expect(formatted).toContain("spec.md");
  });

  test("includes annotation count when > 0", () => {
    appendReviewEntry(tempDir, {
      timestamp: new Date().toISOString(),
      documentType: "plan.md",
      approved: false,
      annotationCount: 5,
      reviewDurationMs: 10000,
      iteration: 1,
      feedback: "Fix the error handling",
    });

    const formatted = formatHistoryForAgent(tempDir);
    expect(formatted).toContain("5 annotation(s)");
    expect(formatted).toContain("Fix the error handling");
    expect(formatted).toContain("Iteration 1");
  });

  test("respects count parameter", () => {
    for (let i = 0; i < 7; i++) {
      appendReviewEntry(tempDir, {
        timestamp: new Date().toISOString(),
        documentType: "spec.md",
        approved: true,
        annotationCount: i,
        reviewDurationMs: 1000,
        iteration: 0,
      });
    }

    const formatted = formatHistoryForAgent(tempDir, 3);
    // Should only contain entries 1, 2, 3 (numbered)
    const numberedEntries = formatted.match(/\*\*\d+\.\*\*/g);
    expect(numberedEntries).toHaveLength(3);
  });
});
