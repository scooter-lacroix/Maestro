/**
 * Tests for TrackLens structured feedback formatting
 *
 * Covers:
 * - Basic denial formatting
 * - Annotation grouping by severity
 * - Feedback and edited content sections
 * - Edge cases (no annotations, no feedback)
 */

import { describe, test, expect } from "bun:test";
import {
  formatDenialForAgent,
  type TrackLensAnnotation,
  type TrackLensDecisionResult,
} from "../feedback";

describe("formatDenialForAgent", () => {
  test("formats basic denial with no annotations", () => {
    const result: TrackLensDecisionResult = {
      approved: false,
      feedback: "Please revise the spec",
    };

    const output = formatDenialForAgent(result, "spec.md");
    expect(output).toContain("# TrackLens Review: Changes Requested");
    expect(output).toContain("**Document Type:** spec.md");
    expect(output).toContain("Please revise the spec");
    expect(output).toContain("**Action Required:**");
  });

  test("formats annotations grouped by severity", () => {
    const result: TrackLensDecisionResult = {
      approved: false,
      annotations: [
        {
          severity: "WARNING",
          comment: "Consider adding error handling",
          lineNumber: 42,
          selectionText: "fetch(url)",
        },
        {
          severity: "ERROR",
          comment: "Missing required field",
          lineNumber: 10,
          selectionText: "type: 'object'",
        },
        {
          severity: "INFO",
          comment: "Nice documentation style",
          lineNumber: 1,
        },
      ],
    };

    const output = formatDenialForAgent(result, "plan.md");

    // ERROR section should come before WARNING, which comes before INFO
    const errorIdx = output.indexOf("### X ERROR");
    const warningIdx = output.indexOf("### ! WARNING");
    const infoIdx = output.indexOf("### i INFO");

    expect(errorIdx).toBeGreaterThan(-1);
    expect(warningIdx).toBeGreaterThan(-1);
    expect(infoIdx).toBeGreaterThan(-1);
    expect(errorIdx).toBeLessThan(warningIdx);
    expect(warningIdx).toBeLessThan(infoIdx);

    expect(output).toContain("Missing required field");
    expect(output).toContain("Consider adding error handling");
    expect(output).toContain("Nice documentation style");
  });

  test("includes line numbers and selection text", () => {
    const result: TrackLensDecisionResult = {
      approved: false,
      annotations: [
        {
          severity: "ERROR",
          comment: "Bad code",
          lineNumber: 42,
          selectionText: "const x = null",
        },
      ],
    };

    const output = formatDenialForAgent(result, "spec.md");
    expect(output).toContain("Line 42");
    expect(output).toContain("> const x = null");
  });

  test("includes edited content section when present", () => {
    const result: TrackLensDecisionResult = {
      approved: false,
      editedContent: "# Edited Plan\n\nSome changes",
    };

    const output = formatDenialForAgent(result, "plan.md");
    expect(output).toContain("## Edited Content");
    expect(output).toContain("user edited the document");
  });

  test("handles annotation without severity (defaults to INFO)", () => {
    const result: TrackLensDecisionResult = {
      approved: false,
      annotations: [
        {
          comment: "Generic comment",
        } as TrackLensAnnotation,
      ],
    };

    const output = formatDenialForAgent(result, "spec.md");
    expect(output).toContain("### i INFO");
    expect(output).toContain("Generic comment");
  });

  test("handles empty annotations array", () => {
    const result: TrackLensDecisionResult = {
      approved: false,
      feedback: "Needs work",
      annotations: [],
    };

    const output = formatDenialForAgent(result, "spec.md");
    expect(output).toContain("Needs work");
    expect(output).not.toContain("## Annotations");
  });

  test("handles no feedback and no annotations", () => {
    const result: TrackLensDecisionResult = {
      approved: false,
    };

    const output = formatDenialForAgent(result, "spec.md");
    expect(output).toContain("# TrackLens Review: Changes Requested");
    expect(output).toContain("**Action Required:**");
  });

  test("counts annotations per severity group", () => {
    const result: TrackLensDecisionResult = {
      approved: false,
      annotations: [
        { severity: "ERROR", comment: "E1" },
        { severity: "ERROR", comment: "E2" },
        { severity: "WARNING", comment: "W1" },
        { severity: "INFO", comment: "I1" },
        { severity: "INFO", comment: "I2" },
        { severity: "INFO", comment: "I3" },
      ],
    };

    const output = formatDenialForAgent(result, "spec.md");
    expect(output).toContain("### X ERROR (2)");
    expect(output).toContain("### ! WARNING (1)");
    expect(output).toContain("### i INFO (3)");
  });
});
