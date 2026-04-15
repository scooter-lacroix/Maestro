/**
 * Tests for TrackLens auto-trigger module
 *
 * Covers:
 * - registerAutoTrigger registers event handlers without error
 * - recordRecentDocument integration
 * - BeforeSendMessageEvent handling (proposed runtime extension)
 */

import { describe, test, expect, beforeEach } from "bun:test";
import {
  recordRecentDocument,
  getLastGeneratedDocument,
  getRecentDocuments,
  hasRecentDocument,
  clearRecentDocuments,
} from "../recentDoc";
import {
  hasTrackLensKeyword,
  hasReviewTrigger,
  replaceTrackLensKeyword,
} from "../keyword";

describe("recentDoc + keyword integration", () => {
  beforeEach(() => {
    clearRecentDocuments();
  });

  test("no recent document → keyword detection alone is not enough", () => {
    expect(hasTrackLensKeyword("tracklens")).toBe(true);
    expect(hasRecentDocument()).toBe(false);
    // Auto-trigger should NOT fire because no recent document
  });

  test("recent document + keyword → auto-trigger conditions met", () => {
    recordRecentDocument({
      trackId: "test-track",
      type: "spec.md",
      content: "# Test Spec",
    });

    expect(hasRecentDocument()).toBe(true);
    expect(hasTrackLensKeyword("please tracklens review")).toBe(true);
  });

  test("recent document + review trigger → auto-trigger conditions met", () => {
    recordRecentDocument({
      trackId: "test-track",
      type: "plan.md",
      content: "# Test Plan",
    });

    expect(hasRecentDocument()).toBe(true);
    expect(hasReviewTrigger("I finished the spec, review this")).toBe(true);
  });

  test("expired document is not considered recent", () => {
    recordRecentDocument({
      trackId: "old-track",
      type: "spec.md",
      content: "# Old Spec",
    });

    // Manually verify with a very short max age
    const doc = getLastGeneratedDocument({ maxAgeMs: 0 });
    expect(doc).toBeUndefined();
  });

  test("multiple documents → most recent is returned", () => {
    recordRecentDocument({
      trackId: "track-1",
      type: "spec.md",
      content: "# Spec 1",
    });

    recordRecentDocument({
      trackId: "track-2",
      type: "plan.md",
      content: "# Plan 2",
    });

    const doc = getLastGeneratedDocument();
    expect(doc).toBeDefined();
    expect(doc!.trackId).toBe("track-2");
    expect(doc!.type).toBe("plan.md");
  });

  test("keyword replacement preserves context for forwarded message", () => {
    const text = "please tracklens review the spec";
    const cleaned = replaceTrackLensKeyword(text);
    expect(cleaned).toBe("please  review the spec");
    expect(cleaned.length).toBeGreaterThan(0);
  });

  test("walkthrough type document is recorded correctly", () => {
    recordRecentDocument({
      trackId: "walkthrough-track",
      type: "walkthrough",
      content: "# Walkthrough",
    });

    const doc = getLastGeneratedDocument();
    expect(doc).toBeDefined();
    expect(doc!.type).toBe("walkthrough");
  });

  test("max entries cap at 5", () => {
    for (let i = 0; i < 7; i++) {
      recordRecentDocument({
        trackId: `track-${i}`,
        type: "document",
        content: `# Document ${i}`,
      });
    }

    const docs = getRecentDocuments();
    expect(docs.length).toBeLessThanOrEqual(5);
  });
});

describe("registerAutoTrigger", () => {
  test("registers without error on mock ExtensionAPI", async () => {
    const events: Array<{ event: string; handler: Function }> = [];
    const messages: Array<{ content: string }> = [];

    const mockPi = {
      registerCommand: () => {},
      registerTool: () => {},
      on: (event: string, handler: Function) => {
        events.push({ event, handler });
      },
      sendMessage: (msg: any) => {
        messages.push(msg);
      },
    };

    const { registerAutoTrigger } = await import("../autoTrigger");
    registerAutoTrigger(mockPi as any);

    // Should register at least 2 events
    expect(events.length).toBeGreaterThanOrEqual(2);

    const eventNames = events.map((e) => e.event);
    expect(eventNames).toContain("before_send_message");
    expect(eventNames).toContain("before_agent_start");
  });

  test("before_send_message handler does nothing without keyword", async () => {
    let sentEvent: any = null;
    const events: Array<{ event: string; handler: Function }> = [];

    const mockPi = {
      registerCommand: () => {},
      registerTool: () => {},
      on: (event: string, handler: Function) => {
        events.push({ event, handler });
      },
      sendMessage: (msg: any) => {},
    };

    const { registerAutoTrigger } = await import("../autoTrigger");
    registerAutoTrigger(mockPi as any);

    const handler = events.find((e) => e.event === "before_send_message")!.handler;

    let prevented = false;
    const event = {
      text: "hello world",
      metadata: {},
      preventDefault: () => { prevented = true; },
    };

    await handler(event);
    expect(prevented).toBe(false);
  });
});
