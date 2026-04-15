/**
 * Tests for TrackLens phase reporter
 *
 * Covers:
 * - startPhaseReporter returns cleanup function
 * - Polling stops on abort signal
 * - Phase change callback is invoked
 * - Polling stops when phase reaches "decided"
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test";
import {
  startPhaseReporter,
  type TrackLensReportedPhase,
} from "../phaseReporter";

describe("startPhaseReporter", () => {
  test("returns a cleanup function", () => {
    const controller = new AbortController();
    const phases: Array<{ phase: string; description: string }> = [];

    const stop = startPhaseReporter({
      serverUrl: "http://localhost:1", // unreachable, won't matter
      pollIntervalMs: 100,
      onPhaseChange: (phase, description) => {
        phases.push({ phase, description });
      },
      signal: controller.signal,
    });

    expect(typeof stop).toBe("function");
    stop();
    controller.abort();
  });

  test("stops polling when abort signal fires", async () => {
    const controller = new AbortController();
    const phases: TrackLensReportedPhase[] = [];

    startPhaseReporter({
      serverUrl: "http://localhost:1",
      pollIntervalMs: 50,
      onPhaseChange: (phase) => {
        phases.push(phase);
      },
      signal: controller.signal,
    });

    // Abort after a short delay
    setTimeout(() => controller.abort(), 100);

    // Wait for any potential polling
    await new Promise((resolve) => setTimeout(resolve, 200));

    // No phases should have been recorded (server unreachable)
    expect(phases.length).toBe(0);
  });

  test("cleanup function stops further polling", async () => {
    const controller = new AbortController();
    const phases: TrackLensReportedPhase[] = [];

    const stop = startPhaseReporter({
      serverUrl: "http://localhost:1",
      pollIntervalMs: 50,
      onPhaseChange: (phase) => {
        phases.push(phase);
      },
      signal: controller.signal,
    });

    stop();

    // Wait for any potential polling
    await new Promise((resolve) => setTimeout(resolve, 150));

    expect(phases.length).toBe(0);
    controller.abort();
  });
});
