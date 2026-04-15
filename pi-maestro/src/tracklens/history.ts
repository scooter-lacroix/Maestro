/**
 * TrackLens Review History Persistence
 *
 * Persists review history entries to `maestro/tracks/<id>/review-history.json`
 * so agents can reference past review decisions and feedback patterns.
 *
 * @packageDocumentation
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "fs";
import { resolve } from "path";

/** A single review history entry */
export interface ReviewHistoryEntry {
  /** ISO timestamp of the review */
  timestamp: string;
  /** Document type reviewed */
  documentType: string;
  /** Whether the review was approved */
  approved: boolean;
  /** Number of annotations attached */
  annotationCount: number;
  /** General feedback text */
  feedback?: string;
  /** Edited content if the user made edits */
  editedContent?: string;
  /** How long the review took in milliseconds */
  reviewDurationMs: number;
  /** Review iteration (for multi-round reviews) */
  iteration: number;
}

/** Maximum number of history entries to keep */
const MAX_HISTORY_ENTRIES = 50;

/**
 * Load review history for a track.
 *
 * @param trackDir - Absolute path to the track directory
 * @returns Array of history entries (newest first)
 */
export function loadReviewHistory(trackDir: string): ReviewHistoryEntry[] {
  const historyPath = resolve(trackDir, "review-history.json");

  if (!existsSync(historyPath)) {
    return [];
  }

  try {
    const raw = readFileSync(historyPath, "utf-8");
    const entries = JSON.parse(raw) as ReviewHistoryEntry[];
    return Array.isArray(entries) ? entries : [];
  } catch {
    return [];
  }
}

/**
 * Append a review history entry.
 *
 * Automatically writes to `review-history.json` in the track directory.
 * Caps at MAX_HISTORY_ENTRIES by dropping oldest entries.
 *
 * @param trackDir - Absolute path to the track directory
 * @param entry - The review history entry to append
 */
export function appendReviewEntry(
  trackDir: string,
  entry: ReviewHistoryEntry,
): void {
  const history = loadReviewHistory(trackDir);

  // Add new entry at the front
  history.unshift(entry);

  // Cap at max entries
  const capped = history.slice(0, MAX_HISTORY_ENTRIES);

  // Ensure directory exists and write history file
  try {
    mkdirSync(trackDir, { recursive: true });
    const historyPath = resolve(trackDir, "review-history.json");
    writeFileSync(historyPath, JSON.stringify(capped, null, 2), "utf-8");
  } catch (error) {
    // History is non-critical; log warning but don't rethrow
    console.warn(`Failed to write review history for ${trackDir}:`, error);
  }
}

/**
 * Format the last N review history entries for agent context.
 *
 * @param trackDir - Absolute path to the track directory
 * @param count - Number of recent entries to include (default: 5)
 * @returns Formatted markdown string, or empty string if no history
 */
export function formatHistoryForAgent(
  trackDir: string,
  count: number = 5,
): string {
  const history = loadReviewHistory(trackDir);
  if (history.length === 0) return "";

  const recent = history.slice(0, count);
  const lines: string[] = [];

  lines.push("## Review History");
  lines.push("");

  for (let i = 0; i < recent.length; i++) {
    const entry = recent[i]!;
    const status = entry.approved ? "Approved" : "Changes Requested";
    const duration = (entry.reviewDurationMs / 1000).toFixed(1);
    const date = new Date(entry.timestamp).toLocaleString();

    lines.push(
      `**${i + 1}.** ${status} — ${entry.documentType} — ${duration}s — ${date}`,
    );

    if (entry.annotationCount > 0) {
      lines.push(`   ${entry.annotationCount} annotation(s)`);
    }

    if (entry.feedback) {
      // Truncate long feedback
      const truncated =
        entry.feedback.length > 200
          ? entry.feedback.slice(0, 200) + "..."
          : entry.feedback;
      lines.push(`   > ${truncated}`);
    }

    if (entry.iteration > 0) {
      lines.push(`   Iteration ${entry.iteration}`);
    }

    lines.push("");
  }

  return lines.join("\n");
}
