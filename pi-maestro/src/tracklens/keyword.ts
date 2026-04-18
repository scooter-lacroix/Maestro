/**
 * TrackLens Keyword Detection
 *
 * Context-aware keyword detection for auto-triggering TrackLens review.
 * Ported from Ultraplan's findKeywordTriggerPositions with TrackLens-specific
 * keywords and filtering.
 *
 * Keywords detected:
 * - "tracklens" — triggers review of most recent document
 * - "review this" — triggers review at end of message
 *
 * False positive filters:
 * - Inside quoted ranges ("...", '...', `<...>`, {...}, [...], `...`)
 * - Path-like contexts (/tracklens/, \tracklens\, -tracklens-)
 * - Question suffix (tracklens?)
 * - Slash commands (starting with /)
 *
 * @packageDocumentation
 */

/** A detected keyword trigger position in user text */
export interface TriggerPosition {
  /** The matched word */
  word: string;
  /** Start index in the source text */
  start: number;
  /** End index (exclusive) in the source text */
  end: number;
}

/** Mapping of opening delimiters to their closing counterparts */
const OPEN_TO_CLOSE: Record<string, string> = {
  "`": "`",
  '"': '"',
  "<": ">",
  "{": "}",
  "[": "]",
  "(": ")",
  "'": "'",
};

/** Check if a character is a word character (letter, number, underscore) */
function isWord(ch: string | undefined): boolean {
  return !!ch && /[\p{L}\p{N}_]/u.test(ch);
}

/**
 * Find all positions of a keyword in text, skipping occurrences inside
 * delimiters, path-like contexts, or question contexts.
 *
 * Filtering rules:
 * 1. Skip if text starts with `/` (slash command)
 * 2. Track quoted/delimited ranges and skip matches inside them
 * 3. Skip matches preceded by `/`, `\`, or `-` (path-like)
 * 4. Skip matches followed by `/`, `\`, `-`, or `?` (path-like or question)
 * 5. Skip matches followed by `.` + word char (property access like `tracklens.config`)
 */
function findKeywordTriggerPositions(
  text: string,
  keyword: string,
): TriggerPosition[] {
  const escaped = keyword.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const re = new RegExp(escaped, "i");
  if (!re.test(text)) return [];
  if (text.trimStart().startsWith("/")) return []; // Slash command — don't trigger

  // Build list of quoted/delimited ranges to exclude
  const quotedRanges: Array<{ start: number; end: number }> = [];
  let openQuote: string | null = null;
  let openAt = 0;
  let nestingDepth = 0; // Track nesting depth for nestable delimiters: [ { (

  for (let i = 0; i < text.length; i++) {
    const ch = text[i]!;
    if (openQuote) {
      // Handle nesting for bracket-like delimiters: [ { (
      if (openQuote === ch && (ch === "[" || ch === "{" || ch === "(")) {
        nestingDepth++;
        continue;
      }
      if (ch !== OPEN_TO_CLOSE[openQuote]) continue;
      // For nestable delimiters, only close when depth is fully unwound
      if ((openQuote === "[" || openQuote === "{" || openQuote === "(") && nestingDepth > 0) {
        nestingDepth--;
        continue;
      }
      // Single quotes: only close if not followed by a word char (possessive)
      if (openQuote === "'" && isWord(text[i + 1])) continue;
      quotedRanges.push({ start: openAt, end: i + 1 });
      openQuote = null;
      nestingDepth = 0;
    } else if (
      // HTML tag opening: `<` followed by letter or `/`
      (ch === "<" && i + 1 < text.length && /[a-zA-Z/]/.test(text[i + 1]!)) ||
      // Single quote: only open if not preceded by a word char
      (ch === "'" && !isWord(text[i - 1])) ||
      // Other delimiter characters
      (ch !== "<" && ch !== "'" && ch in OPEN_TO_CLOSE)
    ) {
      openQuote = ch;
      openAt = i;
      nestingDepth = 0;
    }
  }

  // If delimiter was never closed, treat the tail as excluded
  if (openQuote !== null) {
    quotedRanges.push({ start: openAt, end: text.length });
  }

  // Find keyword matches, excluding false positives
  const positions: TriggerPosition[] = [];
  const wordRe = new RegExp(`\\b${keyword}\\b`, "gi");
  const matches = text.matchAll(wordRe);

  for (const match of matches) {
    if (match.index === undefined) continue;
    const start = match.index;
    const end = start + match[0].length;

    // Skip if inside a quoted/delimited range
    if (quotedRanges.some((r) => start >= r.start && start < r.end)) continue;

    // Path-like context: preceded by separator
    const before = text[start - 1];
    if (before === "/" || before === "\\" || before === "-") continue;

    // Path-like or question context: followed by separator or `?`
    const after = text[end];
    if (after === "/" || after === "\\" || after === "-" || after === "?")
      continue;

    // Property access: `tracklens.config`
    if (after === "." && isWord(text[end + 1])) continue;

    positions.push({ word: match[0], start, end });
  }

  return positions;
}

/**
 * Find all TrackLens keyword trigger positions in user text.
 * Matches the word "tracklens" (case-insensitive) with context filtering.
 */
export function findTrackLensTriggerPositions(
  text: string,
): TriggerPosition[] {
  return findKeywordTriggerPositions(text, "tracklens");
}

/**
 * Check if user text contains a triggerable "tracklens" keyword.
 * Returns true if at least one non-filtered match exists.
 */
export function hasTrackLensKeyword(text: string): boolean {
  return findTrackLensTriggerPositions(text).length > 0;
}

/**
 * Check if text contains a "review this" trigger.
 * Only matches the bare phrase at the end of a message (optionally followed
 * by punctuation/whitespace). This avoids false triggers from phrases like
 * "review this code" which would be too aggressive.
 */
export function hasReviewTrigger(text: string): boolean {
  return /\breview\s+this\b[\s.,!]*$/i.test(text.trim());
}

/**
 * Replace the first triggerable "tracklens" keyword from text so the
 * forwarded prompt stays grammatical. Returns the trimmed result with
 * the keyword removed. Returns empty string if removing the keyword
 * would leave nothing.
 */
export function replaceTrackLensKeyword(text: string): string {
  const [trigger] = findTrackLensTriggerPositions(text);
  if (!trigger) return text;
  const before = text.slice(0, trigger.start);
  const after = text.slice(trigger.end);
  if (!(before + after).trim()) return "";
  return (before + after).trim();
}
