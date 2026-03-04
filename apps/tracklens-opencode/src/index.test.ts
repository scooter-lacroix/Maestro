/**
 * TrackLens OpenCode Plugin Tests
 *
 * Test suite for TrackLens plugin integration with OpenCode.
 * Verifies plugin structure, tool registration, and rebranding.
 *
 * Environment: Bun runtime
 */

import { describe, test, expect } from "bun:test";

describe("TrackLens OpenCode Plugin - Module Structure", () => {
  test("should have plugin source file", async () => {
    const exists = await Bun.file("./src/index.ts").exists();

    expect(exists).toBe(true);
  });

  test("should export TrackLensPlugin", async () => {
    const source = await Bun.file("./src/index.ts").text();

    // Should export TrackLensPlugin
    expect(source).toContain("export const TrackLensPlugin");
    expect(source).toMatch(/Plugin\s*=/);
  });
});

describe("TrackLens OpenCode Plugin - Rebranding", () => {
  test("should not import from Plannotator packages", async () => {
    const source = await Bun.file("./src/index.ts").text();

    // Should not import from @plannotator
    expect(source).not.toMatch(/import.*from.*@plannotator\//);
  });

  test("should use TrackLens server imports", async () => {
    const source = await Bun.file("./src/index.ts").text();

    // Should import from tracklens-server subpaths
    expect(source).toContain("@maestro/tracklens-server");
    expect(source).toContain("startReviewServer");
    expect(source).toContain("startAnnotateServer");
    expect(source).toContain("startTrackLensServer");
  });

  test("should have TrackLens branding in comments", async () => {
    const source = await Bun.file("./src/index.ts").text();

    // Should mention TrackLens
    expect(source).toMatch(/TrackLens Plugin for OpenCode/i);
  });
});

describe("TrackLens OpenCode Plugin - Removed Features", () => {
  test("should not contain sharing function implementations", async () => {
    const source = await Bun.file("./src/index.ts").text();

    // Should not contain actual sharing function calls (only allowed in comments)
    // Check that getSharingEnabled is not called as a function
    expect(source).not.toMatch(/getSharingEnabled\(/);
    expect(source).not.toMatch(/getShareBaseUrl\(/);
    expect(source).not.toMatch(/writeRemoteShareLink\(/);
  });

  test("should retain agentSwitch functionality", async () => {
    const source = await Bun.file("./src/index.ts").text();

    // Should keep agent switch feature
    expect(source).toMatch(/agentSwitch/);
    expect(source).toMatch(/result\.agentSwitch/);
  });
});

describe("TrackLens OpenCode Plugin - Tool Registration", () => {
  test("should export submit_plan tool", async () => {
    const source = await Bun.file("./src/index.ts").text();

    // Should register submit_plan tool
    expect(source).toMatch(/submit_plan/);
  });

  test("tool should have proper description", async () => {
    const source = await Bun.file("./src/index.ts").text();

    // Should mention TrackLens in tool description
    expect(source).toMatch(/TrackLens UI/i);
    expect(source).toMatch(/approve/i);
    expect(source).toMatch(/review/i);
  });
});

describe("TrackLens OpenCode Plugin - Event Handling", () => {
  test("should register event handler for commands", async () => {
    const source = await Bun.file("./src/index.ts").text();

    // Should have event handler
    expect(source).toMatch(/event:/);
    expect(source).toMatch(/tracklens-review/);
  });

  test("should handle command execution events", async () => {
    const source = await Bun.file("./src/index.ts").text();

    // Should check for command events
    expect(source).toMatch(/command\.executed/);
  });
});
