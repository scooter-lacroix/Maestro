/**
 * TrackLens Walkthrough Generator Tests
 *
 * @packageDocumentation
 */

import { describe, test, expect, beforeAll } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync, mkdirSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";

import {
  generateWalkthrough,
  extractCompletedTasks,
  extractSpecSummary,
  getStatusIcon,
} from "../generator.js";
import type { WalkthroughOptions, FileChangeStatus } from "../types.js";

describe("walkthrough generator", () => {
  let tempDir: string;

  beforeAll(() => {
    tempDir = mkdtempSync(join(tmpdir(), "walkthrough-test-"));
  });

  describe("extractCompletedTasks", () => {
    test("should extract tasks with commit hashes", () => {
      const planContent = `
## Phase 1 - Foundation
- [x] Create directory structure (a1b2c3d)
- [x] Initialize project (e4f5g6h)
- [ ] Complete documentation

## Phase 2 - Implementation
- [x] Write tests (i7j8k9l)
- [x] Implement features (a0b1c2d)
      `;

      const tasks = extractCompletedTasks(planContent);

      expect(tasks).toHaveLength(4);
      expect(tasks[0]).toEqual({
        description: "Create directory structure",
        phase: "Phase 1- Foundation",
        commit: "a1b2c3d",
      });
      expect(tasks[3]).toEqual({
        description: "Implement features",
        phase: "Phase 2- Implementation",
        commit: "a0b1c2d",
      });
    });

    test("should extract tasks without commits", () => {
      const planContent = `
## Phase 1
- [x] Task one
- [x] Task two
      `;

      const tasks = extractCompletedTasks(planContent);

      expect(tasks).toHaveLength(2);
      expect(tasks[0].description).toBe("Task one");
      expect(tasks[0].commit).toBeUndefined();
    });

    test("should handle empty plan", () => {
      const tasks = extractCompletedTasks("");
      expect(tasks).toHaveLength(0);
    });

    test("should clean task descriptions", () => {
      const planContent = `
- [x] Task: Create the initial setup (abc123)
- [x] Implement feature   Note: This is important  (def456)
      `;

      const tasks = extractCompletedTasks(planContent);

      expect(tasks).toHaveLength(2);
      expect(tasks[0].description).toBe("Create the initial setup");
      expect(tasks[1].description).toBe("Implement feature");
    });
  });

  describe("extractSpecSummary", () => {
    test("should extract overview section", () => {
      const specContent = `
## Overview
This is a test specification.
It has multiple lines.

## Goals
- Goal one
- Goal two
      `;

      const summary = extractSpecSummary(specContent);

      expect(summary).toContain("This is a test specification");
      expect(summary).toContain("It has multiple lines");
    });

    test("should extract goals", () => {
      const specContent = `
## Overview
Brief description.

## Goals
- Achieve goal one
- Achieve goal two
- Achieve goal three
      `;

      const summary = extractSpecSummary(specContent);

      expect(summary).toContain("**Goals:**");
      expect(summary).toContain("- Achieve goal one");
    });

    test("should handle empty spec", () => {
      const summary = extractSpecSummary("");
      expect(summary).toContain("spec.md");
    });

    test("should limit summary length", () => {
      const longContent = `
## Overview
Line 1
Line 2
Line 3
Line 4
Line 5
Line 6
Line 7
Line 8
Line 9
Line 10
Line 11
Line 12
      `;

      const summary = extractSpecSummary(longContent);
      const lines = summary.split("\n");

      // Should be less than 12 lines due to limiting
      expect(lines.length).toBeLessThan(12);
    });
  });

  describe("getStatusIcon", () => {
    test("should return correct icons", async () => {
      const types = await import("../types.js");
      const { FileChangeStatus } = types;

      expect(getStatusIcon(FileChangeStatus.Added)).toBe("➕");
      expect(getStatusIcon(FileChangeStatus.Modified)).toBe("✏️");
      expect(getStatusIcon(FileChangeStatus.Deleted)).toBe("🗑️");
      expect(getStatusIcon(FileChangeStatus.Renamed)).toBe("📝");
    });
  });

  describe("generateWalkthrough", () => {
    test("should generate walkthrough markdown", () => {
      // Create test track directory structure
      const trackId = "test-track";
      const trackDir = join(tempDir, trackId);
      mkdirSync(trackDir, { recursive: true });

      // Create spec.md
      writeFileSync(
        join(trackDir, "spec.md"),
        `
## Overview
Test specification
        `,
        "utf-8"
      );

      // Create plan.md
      writeFileSync(
        join(trackDir, "plan.md"),
        `
## Phase 1
- [x] Task one (abc123)
- [x] Task two (def456)
        `,
        "utf-8"
      );

      // Create metadata.json
      writeFileSync(
        join(trackDir, "metadata.json"),
        JSON.stringify({
          description: "Test Track",
          type: "feature",
          status: "Completed",
        }),
        "utf-8"
      );

      const options: WalkthroughOptions = {
        trackId,
        root: tempDir,
        trackDir,
        isSubtrack: false,
        includeDiffs: false,
        includeSnippets: false,
      };

      const walkthrough = generateWalkthrough(options);

      expect(walkthrough.markdown).toContain("# Track Walkthrough: Test Track");
      expect(walkthrough.markdown).toContain("**Track ID:** `test-track`");
      expect(walkthrough.markdown).toContain("**Type:** feature");
      expect(walkthrough.markdown).toContain("## Completed Tasks");
      expect(walkthrough.markdown).toContain("## Specification Summary");
      expect(walkthrough.completedTasks).toHaveLength(2);
      expect(walkthrough.metadata.description).toBe("Test Track");
      expect(walkthrough.metadata.isSubtrack).toBe(false);
    });

    test("should handle subtrack metadata", () => {
      const trackId = "subtrack-test";
      const trackDir = join(tempDir, trackId);
      mkdirSync(trackDir, { recursive: true });

      writeFileSync(
        join(trackDir, "metadata.json"),
        JSON.stringify({
          description: "Subtrack Test",
        }),
        "utf-8"
      );

      writeFileSync(join(trackDir, "spec.md"), "", "utf-8");
      writeFileSync(join(trackDir, "plan.md"), "", "utf-8");

      const options: WalkthroughOptions = {
        trackId,
        root: tempDir,
        trackDir,
        isSubtrack: true,
        parentTrackId: "parent-track",
      };

      const walkthrough = generateWalkthrough(options);

      expect(walkthrough.metadata.isSubtrack).toBe(true);
      expect(walkthrough.metadata.parentTrackId).toBe("parent-track");
      expect(walkthrough.markdown).toContain("**Parent Track:** `parent-track`");
    });

    test("should handle missing files gracefully", () => {
      const trackId = "missing-files";
      const trackDir = join(tempDir, trackId);
      mkdirSync(trackDir, { recursive: true });

      // No files created

      const options: WalkthroughOptions = {
        trackId,
        root: tempDir,
        trackDir,
      };

      const walkthrough = generateWalkthrough(options);

      // Should still generate walkthrough with minimal info
      expect(walkthrough.markdown).toContain(`# Track Walkthrough: ${trackId}`);
      expect(walkthrough.completedTasks).toHaveLength(0);
      expect(walkthrough.changedFiles).toHaveLength(0);
    });
  });
});
