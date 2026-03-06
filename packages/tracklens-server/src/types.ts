/**
 * TrackLens Server Types
 *
 * Centralized type definitions for all TrackLens server modules.
 */

import type { GitContext, DiffType, DiffResult } from "./git";
import type { RepoInfo } from "./repo";

// ============================================================================
// Common Types
// ============================================================================

/** Origin identifier for UI customization */
export type Origin = "opencode" | "claude-code";

/** Base decision result */
export interface BaseDecisionResult {
  feedback?: string;
  savedPath?: string;
  agentSwitch?: string;
  autonomyMode?: string;
  annotations?: unknown[];
}

/** Plan review decision result */
export interface PlanDecisionResult extends BaseDecisionResult {
  approved: boolean;
}

/** Review/annotate decision result (no approved field) */
export interface FeedbackDecisionResult extends BaseDecisionResult {
  feedback: string;
}

/** Common server result interface */
export interface ServerResult<T = BaseDecisionResult> {
  /** The port the server is running on */
  port: number;
  /** The full URL to access the server */
  url: string;
  /** Whether running in remote mode */
  isRemote: boolean;
  /** Wait for user decision */
  waitForDecision: () => Promise<T>;
  /** Stop the server */
  stop: () => void;
}

/** Common server options */
export interface CommonServerOptions {
  /** HTML content to serve for the UI */
  htmlContent: string;
  /** Origin identifier for UI customization */
  origin?: Origin;
}

// ============================================================================
// Plan Review Server Types
// ============================================================================

export interface PlanServerOptions extends CommonServerOptions {
  /** The plan markdown content */
  plan: string;
  /** Current autonomy mode to preserve (Claude Code only) */
  autonomyMode?: string;
}

export type PlanServerResult = ServerResult<PlanDecisionResult>;

// ============================================================================
// Review Server Types
// ============================================================================

export interface ReviewServerOptions extends CommonServerOptions {
  /** Raw git diff patch string */
  rawPatch: string;
  /** Git ref used for the diff (e.g., "HEAD", "main..HEAD", "--staged") */
  gitRef: string;
  /** Error message if git diff failed */
  error?: string;
  /** Current diff type being displayed */
  diffType?: DiffType;
  /** Git context with branch info and available diff options */
  gitContext?: GitContext;
}

export type ReviewServerResult = ServerResult<FeedbackDecisionResult & { annotations: unknown[] }>;

// ============================================================================
// Annotate Server Types
// ============================================================================

export interface AnnotateServerOptions extends CommonServerOptions {
  /** Markdown content of the file to annotate */
  markdown: string;
  /** Original file path (for display purposes) */
  filePath: string;
}

export type AnnotateServerResult = ServerResult<FeedbackDecisionResult & { annotations: unknown[] }>;

// ============================================================================
// API Response Types
// ============================================================================

export interface ApiResponse<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface PlanApiResponse {
  plan: string;
  origin: Origin;
  mode: "review" | "annotate";
  repoInfo: RepoInfo | null;
  autonomyMode?: string;
  filePath?: string;
}

export interface DiffApiResponse {
  rawPatch: string;
  gitRef: string;
  origin: Origin;
  diffType: DiffType;
  gitContext?: GitContext;
  repoInfo: RepoInfo | null;
  error?: string;
}

export interface FeedbackApiRequest {
  feedback: string;
  annotations: unknown[];
  agentSwitch?: string;
}

export interface DecisionApiRequest {
  approved: boolean;
  feedback?: string;
  customPath?: string;
  annotations?: string;
  agentSwitch?: string;
  autonomyMode?: string;
}

// ============================================================================
// Image Types
// ============================================================================

export interface ImageValidationResult {
  valid: boolean;
  resolved: string;
  error?: string;
}

export interface UploadValidationResult {
  valid: boolean;
  ext: string;
  error?: string;
}

export interface FileSanitizationResult {
  safe: boolean;
  sanitized: string;
  error?: string;
}

// ============================================================================
// Integration Types
// ============================================================================

export interface ObsidianConfig {
  vaultPath: string;
  folder: string;
  plan: string;
  filenameFormat?: string;
}

export interface BearConfig {
  plan: string;
}

export interface IntegrationResult {
  success: boolean;
  error?: string;
  path?: string;
}

// ============================================================================
// Vault Types
// ============================================================================

export interface VaultNode {
  name: string;
  path: string;
  type: "file" | "folder";
  children?: VaultNode[];
}

// Re-export git types for convenience
export type { GitContext, DiffType, DiffResult };
export type { RepoInfo };
