/**
 * TrackLens Server - Consolidated Entry Point
 *
 * Provides a unified interface for starting TrackLens servers:
 * - Plan review server (startTrackLensServer / startTrackLensReviewServer)
 * - Code review server (startTrackLensReviewServer)
 * - Annotation server (startTrackLensAnnotateServer)
 *
 * @example
 * ```typescript
 * import { startTrackLensServer, startReviewServer } from "@maestro/tracklens-server/server";
 *
 * // Plan review
 * const planServer = await startTrackLensServer({
 *   plan: "# My Plan",
 *   origin: "claude-code",
 *   htmlContent: planHtml,
 * });
 *
 * // Code review
 * const reviewServer = await startReviewServer({
 *   rawPatch: gitDiffOutput,
 *   gitRef: "HEAD",
 *   origin: "opencode",
 *   htmlContent: reviewHtml,
 * });
 * ```
 */

// Re-export all server functions from their respective modules
export { startTrackLensServer } from "./index";
export type { ServerOptions, ServerResult } from "./index";
export { startReviewServer } from "./review";
export type { ReviewServerOptions, ReviewServerResult } from "./review";
export { startAnnotateServer } from "./annotate";
export type { AnnotateServerOptions, AnnotateServerResult } from "./annotate";

// Re-export types from types.ts
export type {
  Origin,
  BaseDecisionResult,
  PlanDecisionResult,
  FeedbackDecisionResult,
  CommonServerOptions,
  ApiResponse,
  PlanApiResponse,
  DiffApiResponse,
  FeedbackApiRequest,
  DecisionApiRequest,
  VaultNode,
} from "./types";

// Re-export utilities
export {
  corsHeaders,
  jsonResponse,
  errorResponse,
  generateAuthToken,
  validateAuthHeader,
  startServerWithRetry,
  sleep,
  createDeferred,
  getRequestOrigin,
  isApiRequest,
  log,
  injectScriptIntoHtml,
  injectAuthToken,
} from "./utils";

// Re-export git utilities
export {
  getGitContext,
  runGitDiff,
  type DiffType,
  type DiffOption,
  type GitContext,
  type DiffResult,
} from "./git";

// Re-export browser utilities
export { openBrowser } from "./browser";

// Re-export remote utilities
export { isRemoteSession, getServerPort } from "./remote";

// Re-export image utilities
export {
  validateImagePath,
  validateUploadExtension,
  sanitizeFileName,
  getSafeUploadPath,
  UPLOAD_DIR,
} from "./image";

// Re-export repo utilities
export { getRepoInfo, type RepoInfo } from "./repo";

// Re-export storage utilities
export {
  generateSlug,
  savePlan,
  saveAnnotations,
  saveFinalSnapshot,
  saveToHistory,
  getPlanVersion,
  getPlanVersionPath,
  getVersionCount,
  listVersions,
  listProjectPlans,
} from "./storage";

// Re-export integration utilities
export {
  detectObsidianVaults,
  saveToObsidian,
  saveToBear,
  type ObsidianConfig,
  type BearConfig,
  type IntegrationResult,
} from "./integrations";

// Re-export project utilities
export { detectProjectName, sanitizeTag, extractRepoName, extractDirName } from "./project";

// Re-export IDE utilities
export { openEditorDiff } from "./ide";
