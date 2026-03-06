/**
 * TrackLens Walkthrough Types
 *
 * Type definitions for the walkthrough generation system.
 *
 * @packageDocumentation
 */

/**
 * Walkthrough generation options
 */
export interface WalkthroughOptions {
  /** Track ID (e.g., "tracklens-fullport_20260304") */
  trackId: string;
  /** Maestro project root directory */
  root: string;
  /** Track directory (e.g., "maestro/tracks/tracklens-fullport_20260304") */
  trackDir: string;
  /** Whether this is a subtrack of a master track */
  isSubtrack?: boolean;
  /** Parent track ID if this is a subtrack */
  parentTrackId?: string;
  /** Include full git diffs */
  includeDiffs?: boolean;
  /** Include key code snippets */
  includeSnippets?: boolean;
  /** Maximum lines per snippet (default: 30) */
  maxSnippetLines?: number;
}

/**
 * Changed file entry for walkthrough
 */
export interface ChangedFile {
  /** File path relative to project root */
  path: string;
  /** Git status */
  status: FileChangeStatus;
  /** Programming language */
  language: string;
  /** Full git diff (optional) */
  diff?: string;
  /** Key code snippet (optional) */
  snippet?: string | undefined;
  /** Number of lines added */
  additions: number;
  /** Number of lines deleted */
  deletions: number;
}

/**
 * Git file change status
 */
export enum FileChangeStatus {
  Added = "added",
  Modified = "modified",
  Deleted = "deleted",
  Renamed = "renamed",
}

/**
 * Completed task entry
 */
export interface CompletedTask {
  /** Task description */
  description: string;
  /** Task phase (if available) */
  phase?: string;
  /** Commit hash (if available) */
  commit?: string;
}

/**
 * Walkthrough metadata
 */
export interface WalkthroughMetadata {
  /** Track ID */
  trackId: string;
  /** Track description */
  description: string;
  /** Track type (feature, bugfix, refactor, etc.) */
  type?: string;
  /** Track status */
  status: string;
  /** Whether this is a subtrack */
  isSubtrack: boolean;
  /** Parent track ID (if subtrack) */
  parentTrackId?: string;
  /** Generation timestamp */
  generatedAt: string;
}

/**
 * Generated walkthrough content
 */
export interface GeneratedWalkthrough {
  /** Walkthrough markdown content */
  markdown: string;
  /** Walkthrough metadata */
  metadata: WalkthroughMetadata;
  /** Completed tasks extracted from plan */
  completedTasks: CompletedTask[];
  /** Changed files with diffs/snippets */
  changedFiles: ChangedFile[];
}

/**
 * Storage format for compressed walkthroughs
 */
export interface StoredWalkthrough {
  /** Walkthrough metadata */
  metadata: WalkthroughMetadata;
  /** Compressed walkthrough data (base64url-encoded deflate) */
  compressed: string;
  /** Version of storage format */
  version: 1;
}
