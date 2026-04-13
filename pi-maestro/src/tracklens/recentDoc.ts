/**
 * Recent Document Tracker for TrackLens Auto-Trigger
 *
 * Tracks recently generated/accessed documents so that keyword detection
 * can auto-invoke the appropriate TrackLens tool when the user types
 * "tracklens" or "review this".
 *
 * Documents are tracked with a timestamp and auto-expire after 10 minutes.
 *
 * @packageDocumentation
 */

/** A recently generated document that can be auto-reviewed */
export interface RecentDocument {
  /** Track ID this document belongs to */
  trackId: string;
  /** Document type */
  type: "spec.md" | "plan.md" | "walkthrough" | "document";
  /** Markdown content of the document */
  content: string;
  /** Timestamp when the document was recorded (Date.now()) */
  timestamp: number;
  /** File path if the document was loaded from a file */
  filePath?: string;
}

/** Maximum age for a recent document before it's considered stale (10 minutes) */
const MAX_AGE_MS = 10 * 60 * 1000;

/** Maximum number of recent documents to track */
const MAX_ENTRIES = 5;

/** In-memory store of recent documents */
let recentDocuments: RecentDocument[] = [];

/**
 * Record a recently generated/accessed document.
 * Automatically prunes expired entries.
 */
export function recordRecentDocument(doc: Omit<RecentDocument, "timestamp">): void {
  const entry: RecentDocument = {
    ...doc,
    timestamp: Date.now(),
  };

  // Add to front of list
  recentDocuments.unshift(entry);

  // Prune: remove expired entries and cap at max
  pruneExpired();
  if (recentDocuments.length > MAX_ENTRIES) {
    recentDocuments = recentDocuments.slice(0, MAX_ENTRIES);
  }
}

/**
 * Get the most recent document that hasn't expired.
 * Returns undefined if no recent documents exist.
 */
export function getLastGeneratedDocument(options?: {
  maxAgeMs?: number;
}): RecentDocument | undefined {
  const maxAge = options?.maxAgeMs ?? MAX_AGE_MS;
  pruneExpired();
  const now = Date.now();
  return recentDocuments.find((doc) => now - doc.timestamp < maxAge);
}

/**
 * Get all recent documents that haven't expired.
 */
export function getRecentDocuments(options?: {
  maxAgeMs?: number;
}): RecentDocument[] {
  const maxAge = options?.maxAgeMs ?? MAX_AGE_MS;
  const now = Date.now();
  return recentDocuments.filter((doc) => now - doc.timestamp < maxAge);
}

/**
 * Check if there's a recent document available for auto-trigger.
 */
export function hasRecentDocument(): boolean {
  return getLastGeneratedDocument() !== undefined;
}

/**
 * Clear all tracked recent documents.
 */
export function clearRecentDocuments(): void {
  recentDocuments = [];
}

/** Remove documents that have exceeded the max age */
function pruneExpired(): void {
  const now = Date.now();
  recentDocuments = recentDocuments.filter(
    (doc) => now - doc.timestamp < MAX_AGE_MS,
  );
}
