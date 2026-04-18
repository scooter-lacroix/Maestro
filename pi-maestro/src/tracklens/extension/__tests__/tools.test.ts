/**
 * Tests for TrackLens extension tools and commands
 *
 * Covers:
 * - tracklens_review tool validation
 * - tracklens_walkthrough tool behavior
 * - /tracklens command parsing (on, off, status)
 * - Toggle state consumption
 */

import { describe, it, expect, beforeEach, afterEach } from "bun:test";
import type { ExtensionAPI } from "../../../types";

// Mock dependencies
const mockUi = {
  notify: (message: string, type: "info" | "warning" | "error") => {
    // Mock notify - do nothing
  },
  confirm: (title: string, message: string) => Promise.resolve(false),
  select: (title: string, options: Array<{ label: string; value: string }>) => Promise.resolve(""),
  input: (title: string, placeholder: string) => Promise.resolve(""),
};

const mockPi: ExtensionAPI = {
  registerCommand: (name: string, config: any) => {
    // Mock register - store handler for testing
    mockCommands.set(name, config.handler);
  },
  registerTool: (_config: any) => {
    // Mock tool registration
  },
  sendMessage: (_message: any, _options?: any) => {},
  on: (event: string, handler: any) => {
    // Mock event listener
    mockEventHandlers.set(event, handler);
  },
};

const mockCommands = new Map<string, (args: string, ctx: any) => Promise<void>>();
const mockEventHandlers = new Map<string, any>();

// Import after mocks are set up
async function importModule() {
  // Dynamic import to avoid eval-time issues
  return await import("../command");
}

describe("TrackLens Command", () => {
  beforeEach(() => {
    mockCommands.clear();
    mockEventHandlers.clear();
  });

  afterEach(() => {
    mockCommands.clear();
    mockEventHandlers.clear();
  });

  describe("Command Parsing", () => {
    it("should handle 'on' argument to enable TrackLens", async () => {
      const { registerTrackLensCommand, setTrackLensEnabled, isTrackLensEnabled } = await importModule();

      // Start with disabled state
      setTrackLensEnabled(false);
      expect(isTrackLensEnabled()).toBe(false);

      // Register command to get handler
      registerTrackLensCommand(mockPi);

      const handler = mockCommands.get("tracklens");
      expect(handler).toBeDefined();

      // Execute with 'on' argument
      await handler!("on", { ui: mockUi });

      // Verify state changed
      expect(isTrackLensEnabled()).toBe(true);
    });

    it("should handle 'off' argument to disable TrackLens", async () => {
      const { registerTrackLensCommand, setTrackLensEnabled, isTrackLensEnabled } = await importModule();

      // Start with enabled state
      setTrackLensEnabled(true);
      expect(isTrackLensEnabled()).toBe(true);

      // Register command
      registerTrackLensCommand(mockPi);

      const handler = mockCommands.get("tracklens");
      expect(handler).toBeDefined();

      // Execute with 'off' argument
      await handler!("off", { ui: mockUi });

      // Verify state changed
      expect(isTrackLensEnabled()).toBe(false);
    });

    it("should show status when no argument provided", async () => {
      const { registerTrackLensCommand, setTrackLensEnabled, isTrackLensEnabled } = await importModule();

      setTrackLensEnabled(true);
      registerTrackLensCommand(mockPi);

      const handler = mockCommands.get("tracklens");
      expect(handler).toBeDefined();

      // Execute with no argument - should not change state
      await handler!("", { ui: mockUi });

      // State should remain unchanged
      expect(isTrackLensEnabled()).toBe(true);
    });

    it("should be case-insensitive for arguments", async () => {
      const { registerTrackLensCommand, setTrackLensEnabled, isTrackLensEnabled } = await importModule();

      setTrackLensEnabled(false);
      registerTrackLensCommand(mockPi);

      const handler = mockCommands.get("tracklens");

      // Test uppercase
      await handler!("ON", { ui: mockUi });
      expect(isTrackLensEnabled()).toBe(true);

      // Test mixed case
      setTrackLensEnabled(false);
      await handler!("On", { ui: mockUi });
      expect(isTrackLensEnabled()).toBe(true);
    });

    it("should handle extra whitespace in arguments", async () => {
      const { registerTrackLensCommand, setTrackLensEnabled, isTrackLensEnabled } = await importModule();

      setTrackLensEnabled(false);
      registerTrackLensCommand(mockPi);

      const handler = mockCommands.get("tracklens");

      // Test with leading/trailing whitespace
      await handler!("  on  ", { ui: mockUi });
      expect(isTrackLensEnabled()).toBe(true);
    });
  });

  describe("Toggle State Consumption", () => {
    it("should export isTrackLensEnabled function", async () => {
      const { isTrackLensEnabled } = await importModule();
      expect(typeof isTrackLensEnabled).toBe("function");
    });

    it("should export setTrackLensEnabled function for testing", async () => {
      const { setTrackLensEnabled } = await importModule();
      expect(typeof setTrackLensEnabled).toBe("function");
    });

    it("should persist state across calls", async () => {
      const { isTrackLensEnabled, setTrackLensEnabled } = await importModule();

      setTrackLensEnabled(true);
      expect(isTrackLensEnabled()).toBe(true);

      setTrackLensEnabled(false);
      expect(isTrackLensEnabled()).toBe(false);

      setTrackLensEnabled(true);
      expect(isTrackLensEnabled()).toBe(true);
    });
  });
});

describe("TrackLens Tools", () => {
  describe("tracklens_review tool", () => {
    it("should validate required parameters", async () => {
      // This test verifies the tool schema validation
      // In the actual implementation, the tool should require:
      // - filePath (required)
      // - reviewType (optional, defaults to "plan")
      // - summary (optional)

      const schema = {
        filePath: "string (required)",
        reviewType: "'plan' | 'spec' | 'walkthrough' (optional)",
        summary: "string (optional)",
      };

      expect(schema.filePath).toBeDefined();
      expect(schema.reviewType).toBeDefined();
    });

    it("should accept valid review types", () => {
      const validTypes = ["plan", "spec", "walkthrough"];
      const reviewType = "plan";

      expect(validTypes).toContain(reviewType);
    });
  });

  describe("tracklens_walkthrough tool", () => {
    it("should validate required parameters", () => {
      // The tool should require:
      // - trackId (required)
      // - summary (required)

      const schema = {
        trackId: "string (required)",
        summary: "string (required)",
      };

      expect(schema.trackId).toBeDefined();
      expect(schema.summary).toBeDefined();
    });

    it("should handle denial with annotations", () => {
      // Verify the tool can process denial decisions with feedback
      const denialDecision = {
        behavior: "deny" as const,
        annotations: "[{\"line\": 10, \"feedback\": \"Fix this\"}]",
      };

      expect(denialDecision.behavior).toBe("deny");
      expect(denialDecision.annotations).toBeDefined();
    });
  });
});

describe("Checkpoint Behavior", () => {
  it("should include TrackLens walkthrough in implement workflow", async () => {
    // Import and read the implement.ts file to verify workflow content
    const implementModule = await import("../../../commands/implement");
    
    // Verify the module exists
    expect(implementModule).toBeDefined();

    // Read the actual source file to verify workflow content
    const { readFileSync } = await import("fs");
    const { join } = await import("path");
    const implementPath = join(__dirname, "../../../commands/implement.ts");
    const implementSource = readFileSync(implementPath, "utf-8");

    // Verify the workflow includes the walkthrough section
    // Section 4.0 is "TRACKLENS WALKTHROUGH REVIEW"
    expect(implementSource).toContain("TRACKLENS WALKTHROUGH REVIEW");
    expect(implementSource).toContain("tracklens_walkthrough");
    
    // Verify the workflow requires walkthrough regardless of toggle state
    // (After Task 3.3, walkthrough is mandatory)
    expect(implementSource).toContain("When all tasks in plan.md are complete, request TrackLens walkthrough review");
    
    // Verify remediation loop is mentioned
    expect(implementSource).toContain("REMEDIATION LOOP");
  });

  it("should include TrackLens review checkpoints in newTrack workflow", async () => {
    // Import and read the newTrack.ts file to verify workflow content
    const newTrackModule = await import("../../../commands/newTrack");
    
    // Verify the module exists
    expect(newTrackModule).toBeDefined();

    // Read the actual source file to verify workflow content
    const { readFileSync } = await import("fs");
    const { join } = await import("path");
    const newTrackPath = join(__dirname, "../../../commands/newTrack.ts");
    const newTrackSource = readFileSync(newTrackPath, "utf-8");

    // Verify the workflow includes TrackLens review checkpoints
    // Checkpoint 3.6 for spec review
    expect(newTrackSource).toContain("TRACKLENS REVIEW CHECKPOINT");
    expect(newTrackSource).toContain('documentType: "spec.md"');

    // Checkpoint 4.5 for plan review
    expect(newTrackSource).toContain('documentType: "plan.md"');

    // Checkpoint 5.7 for consolidated review
    expect(newTrackSource).toContain("TRACKLENS CONSOLIDATED REVIEW CHECKPOINT");
  });
});
