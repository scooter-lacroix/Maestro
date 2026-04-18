/**
 * Tests for TrackLens keyword detection module
 *
 * Covers:
 * - Basic keyword detection (case-insensitive)
 * - False positive filtering: quotes, code spans, paths, questions, slash commands
 * - "review this" trigger
 * - Keyword replacement
 */

import { describe, test, expect } from "bun:test";
import {
  findTrackLensTriggerPositions,
  hasTrackLensKeyword,
  hasReviewTrigger,
  replaceTrackLensKeyword,
} from "../keyword";

describe("findTrackLensTriggerPositions", () => {
  // --- Basic detection ---
  test("detects lowercase 'tracklens'", () => {
    const result = findTrackLensTriggerPositions("use tracklens to review");
    expect(result).toHaveLength(1);
    expect(result[0].word).toBe("tracklens");
    expect(result[0].start).toBe(4);
  });

  test("detects uppercase 'TrackLens'", () => {
    const result = findTrackLensTriggerPositions("Open TrackLens please");
    expect(result).toHaveLength(1);
    expect(result[0].word).toBe("TrackLens");
  });

  test("detects 'TRACKLENS' (all caps)", () => {
    const result = findTrackLensTriggerPositions("I need TRACKLENS now");
    expect(result).toHaveLength(1);
  });

  test("returns empty for text without keyword", () => {
    expect(findTrackLensTriggerPositions("hello world")).toHaveLength(0);
  });

  test("detects multiple occurrences", () => {
    const result = findTrackLensTriggerPositions("tracklens is great, use tracklens!");
    expect(result).toHaveLength(2);
  });

  // --- Slash command filter ---
  test("skips slash commands starting with /", () => {
    expect(findTrackLensTriggerPositions("/tracklens review")).toHaveLength(0);
  });

  // --- Quoted range filtering ---
  test("skips keyword inside double quotes", () => {
    expect(findTrackLensTriggerPositions('type "tracklens" to start')).toHaveLength(0);
  });

  test("skips keyword inside single quotes", () => {
    expect(findTrackLensTriggerPositions("use 'tracklens' carefully")).toHaveLength(0);
  });

  test("skips keyword inside backticks (code span)", () => {
    expect(findTrackLensTriggerPositions("run `tracklens` in terminal")).toHaveLength(0);
  });

  test("skips keyword inside angle brackets (HTML)", () => {
    expect(findTrackLensTriggerPositions("click <tracklens> here")).toHaveLength(0);
  });

  test("skips keyword inside curly braces", () => {
    expect(findTrackLensTriggerPositions("config { tracklens: true }")).toHaveLength(0);
  });

  test("skips keyword inside square brackets", () => {
    expect(findTrackLensTriggerPositions("see [tracklens docs] for info")).toHaveLength(0);
  });

  test("detects keyword outside of quotes", () => {
    const result = findTrackLensTriggerPositions('run "npm start" then tracklens review');
    expect(result).toHaveLength(1);
  });

  // --- Path-like context filtering ---
  test("skips keyword preceded by / (file path)", () => {
    expect(findTrackLensTriggerPositions("see /tracklens/config")).toHaveLength(0);
  });

  test("skips keyword preceded by backslash", () => {
    expect(findTrackLensTriggerPositions("path\\tracklens\\file")).toHaveLength(0);
  });

  test("skips keyword preceded by dash (kebab-case)", () => {
    expect(findTrackLensTriggerPositions("use my-tracklens-plugin")).toHaveLength(0);
  });

  test("skips keyword followed by / (file path)", () => {
    expect(findTrackLensTriggerPositions("open tracklens/review")).toHaveLength(0);
  });

  test("skips keyword followed by backslash", () => {
    expect(findTrackLensTriggerPositions("open tracklens\\review")).toHaveLength(0);
  });

  test("skips keyword followed by dash", () => {
    expect(findTrackLensTriggerPositions("the tracklens-based tool")).toHaveLength(0);
  });

  // --- Question filter ---
  test("skips keyword followed by ? (question)", () => {
    expect(findTrackLensTriggerPositions("what is tracklens?")).toHaveLength(0);
  });

  // --- Property access filter ---
  test("skips keyword followed by .word (property access)", () => {
    expect(findTrackLensTriggerPositions("config.tracklens.enabled")).toHaveLength(0);
  });

  // --- Mixed/edge cases ---
  test("detects keyword in middle of sentence", () => {
    const result = findTrackLensTriggerPositions("Please use tracklens for this review");
    expect(result).toHaveLength(1);
  });

  test("detects keyword at start of text", () => {
    const result = findTrackLensTriggerPositions("tracklens review this file");
    expect(result).toHaveLength(1);
    expect(result[0].start).toBe(0);
  });

  test("detects keyword at end of text", () => {
    const result = findTrackLensTriggerPositions("please open tracklens");
    expect(result).toHaveLength(1);
  });

  test("handles possessive quote correctly (not filtered)", () => {
    // "tracklens's" — the `'s` is part of the word, not a delimiter start
    const result = findTrackLensTriggerPositions("use tracklens's features");
    expect(result).toHaveLength(1);
  });

  test("empty text returns empty", () => {
    expect(findTrackLensTriggerPositions("")).toHaveLength(0);
  });

  test("whitespace-only text returns empty", () => {
    expect(findTrackLensTriggerPositions("   ")).toHaveLength(0);
  });
});

describe("hasTrackLensKeyword", () => {
  test("returns true for text with keyword", () => {
    expect(hasTrackLensKeyword("open tracklens")).toBe(true);
  });

  test("returns false for text without keyword", () => {
    expect(hasTrackLensKeyword("hello world")).toBe(false);
  });

  test("returns false for keyword in quotes", () => {
    expect(hasTrackLensKeyword('"tracklens"')).toBe(false);
  });

  test("returns false for slash command", () => {
    expect(hasTrackLensKeyword("/tracklens")).toBe(false);
  });

  test("returns false for question", () => {
    expect(hasTrackLensKeyword("what is tracklens?")).toBe(false);
  });
});

describe("hasReviewTrigger", () => {
  test("detects 'review this' at end of message", () => {
    expect(hasReviewTrigger("review this")).toBe(true);
  });

  test("detects 'review this' with trailing punctuation", () => {
    expect(hasReviewTrigger("review this.")).toBe(true);
    expect(hasReviewTrigger("review this!")).toBe(true);
    expect(hasReviewTrigger("review this,")).toBe(true);
  });

  test("detects 'Review This' (case insensitive)", () => {
    expect(hasReviewTrigger("Review This")).toBe(true);
  });

  test("detects 'review this' at end of longer message", () => {
    expect(hasReviewTrigger("I finished the spec, review this")).toBe(true);
  });

  test("does NOT match 'review this code' (followed by word)", () => {
    expect(hasReviewTrigger("review this code")).toBe(false);
  });

  test("does NOT match 'review this' in middle of sentence", () => {
    expect(hasReviewTrigger("review this and then commit")).toBe(false);
  });

  test("empty string returns false", () => {
    expect(hasReviewTrigger("")).toBe(false);
  });

  test("whitespace-only returns false", () => {
    expect(hasReviewTrigger("   ")).toBe(false);
  });
});

describe("replaceTrackLensKeyword", () => {
  test("removes first keyword occurrence", () => {
    expect(replaceTrackLensKeyword("open tracklens now")).toBe("open  now");
  });

  test("returns original text if no keyword", () => {
    expect(replaceTrackLensKeyword("hello world")).toBe("hello world");
  });

  test("returns empty string if only keyword", () => {
    expect(replaceTrackLensKeyword("tracklens")).toBe("");
  });

  test("returns empty for keyword with only whitespace", () => {
    expect(replaceTrackLensKeyword("  tracklens  ")).toBe("");
  });

  test("removes keyword from middle of sentence", () => {
    expect(replaceTrackLensKeyword("please tracklens review")).toBe("please  review");
  });

  test("does not remove quoted keyword", () => {
    // Keyword in quotes is filtered out, so no trigger found — returns original
    expect(replaceTrackLensKeyword('"tracklens" is great')).toBe('"tracklens" is great');
  });
});
