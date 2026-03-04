/**
 * TrackLens Walkthrough Storage Tests
 *
 * @packageDocumentation
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test";
import { mkdtempSync, rmSync, readFileSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";

import {
  saveWalkthrough,
  loadWalkthrough,
  saveFinalWalkthrough,
  walkthroughExists,
  deleteWalkthrough,
  listWalkthroughs,
} from "../storage.js";
import type { GeneratedWalkthrough } from "../types.js";

describe("walkthrough storage", () => {
  let tempDir: string;
  let originalCwd: string;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), "walkthrough-storage-test-"));
    originalCwd = process.cwd();
    process.chdir(tempDir);
  });

  afterEach(() => {
    process.chdir(originalCwd);
    rmSync(tempDir, { recursive: true, force: true });
  });

  describe("saveWalkthrough and loadWalkthrough", () => {
    test("should save and load walkthrough", async () => {
      const walkthrough: GeneratedWalkthrough = {
        markdown: "# Test Walkthrough\n\nThis is a test.",
        metadata: {
          trackId: "test-track",
          description: "Test Track",
          status: "Completed",
          isSubtrack: false,
          generatedAt: new Date().toISOString(),
        },
        completedTasks: [
          {
            description: "Task one",
            phase: "Phase 1",
            commit: "abc123",
          },
        ],
        changedFiles: [],
      };

      const savedPath = await saveWalkthrough("test-track", walkthrough);

      expect(savedPath).toContain("test-track.json");
      expect(savedPath).toContain(".maestro/tracklens/walkthroughs");

      const loaded = await loadWalkthrough("test-track");

      expect(loaded).not.toBeNull();
      expect(loaded?.markdown).toBe(walkthrough.markdown);
      expect(loaded?.metadata.trackId).toBe("test-track");
      expect(loaded?.completedTasks).toHaveLength(1);
      expect(loaded?.completedTasks[0].description).toBe("Task one");
    });

    test("should return null for non-existent walkthrough", async () => {
      const loaded = await loadWalkthrough("non-existent");
      expect(loaded).toBeNull();
    });

    test("should compress walkthrough data", async () => {
      const largeWalkthrough: GeneratedWalkthrough = {
        markdown: "A".repeat(10000), // Large content to test compression
        metadata: {
          trackId: "large-track",
          description: "Large Track",
          status: "Completed",
          isSubtrack: false,
          generatedAt: new Date().toISOString(),
        },
        completedTasks: Array(100).fill(null).map((_, i) => ({
          description: `Task ${i}`,
          phase: `Phase ${Math.floor(i / 10) + 1}`,
        })),
        changedFiles: [],
      };

      await saveWalkthrough("large-track", largeWalkthrough);

      // Read the saved file and check it's compressed (base64url encoded)
      const savedPath = join(tempDir, ".maestro/tracklens/walkthroughs/large-track.json");
      const savedContent = JSON.parse(readFileSync(savedPath, "utf-8"));

      expect(savedContent.compressed).toBeDefined();
      expect(savedContent.compressed.length).toBeGreaterThan(0);
      // Base64url encoded data should not contain + or / or =
      expect(savedContent.compressed).not.toContain("+");
      expect(savedContent.compressed).not.toContain("/");
      expect(savedContent.compressed).not.toContain("=");
    });
  });

  describe("saveFinalWalkthrough", () => {
    test("should save final markdown to track directory", () => {
      const markdown = "# Final Walkthrough\n\nCompleted successfully.";
      const trackDir = join(tempDir, "maestro/tracks/test-track");

      // Create directory first
      const { mkdirSync } = require("fs");
      mkdirSync(trackDir, { recursive: true });

      const savedPath = saveFinalWalkthrough(trackDir, markdown);

      expect(savedPath).toContain("walkthrough-final.md");
      expect(savedPath).toContain("test-track");

      const content = readFileSync(savedPath, "utf-8");
      expect(content).toBe(markdown);
    });
  });

  describe("walkthroughExists", () => {
    test("should check existence correctly", async () => {
      const walkthrough: GeneratedWalkthrough = {
        markdown: "# Test",
        metadata: {
          trackId: "exists-test",
          description: "Exists Test",
          status: "Completed",
          isSubtrack: false,
          generatedAt: new Date().toISOString(),
        },
        completedTasks: [],
        changedFiles: [],
      };

      expect(await walkthroughExists("exists-test")).toBe(false);

      await saveWalkthrough("exists-test", walkthrough);

      expect(await walkthroughExists("exists-test")).toBe(true);
    });
  });

  describe("deleteWalkthrough", () => {
    test("should delete walkthrough", async () => {
      const walkthrough: GeneratedWalkthrough = {
        markdown: "# Test",
        metadata: {
          trackId: "delete-test",
          description: "Delete Test",
          status: "Completed",
          isSubtrack: false,
          generatedAt: new Date().toISOString(),
        },
        completedTasks: [],
        changedFiles: [],
      };

      await saveWalkthrough("delete-test", walkthrough);

      expect(await walkthroughExists("delete-test")).toBe(true);

      const deleted = deleteWalkthrough("delete-test");

      expect(deleted).toBe(true);
      expect(await walkthroughExists("delete-test")).toBe(false);
    });

    test("should return false for non-existent walkthrough", () => {
      const deleted = deleteWalkthrough("non-existent");
      expect(deleted).toBe(false);
    });
  });

  describe("listWalkthroughs", () => {
    test("should list all walkthroughs", async () => {
      const walkthrough1: GeneratedWalkthrough = {
        markdown: "# Test 1",
        metadata: {
          trackId: "track-1",
          description: "Track 1",
          status: "Completed",
          isSubtrack: false,
          generatedAt: new Date().toISOString(),
        },
        completedTasks: [],
        changedFiles: [],
      };

      const walkthrough2: GeneratedWalkthrough = {
        markdown: "# Test 2",
        metadata: {
          trackId: "track-2",
          description: "Track 2",
          status: "Completed",
          isSubtrack: false,
          generatedAt: new Date().toISOString(),
        },
        completedTasks: [],
        changedFiles: [],
      };

      expect(listWalkthroughs()).toHaveLength(0);

      await saveWalkthrough("track-1", walkthrough1);
      await saveWalkthrough("track-2", walkthrough2);

      const walkthroughs = listWalkthroughs();

      expect(walkthroughs).toHaveLength(2);
      expect(walkthroughs).toContain("track-1");
      expect(walkthroughs).toContain("track-2");
    });

    test("should return empty array when no walkthroughs exist", () => {
      const walkthroughs = listWalkthroughs();
      expect(walkthroughs).toEqual([]);
    });
  });
});
