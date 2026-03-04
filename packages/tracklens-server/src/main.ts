/**
 * TrackLens Server - Main Entry Point
 *
 * Exports all public APIs for the TrackLens server package.
 */

// Main server functions
export {
  startTrackLensServer,
  type ServerOptions,
  type ServerResult,
} from "./index";

export {
  startReviewServer,
  type ReviewServerOptions,
  type ReviewServerResult,
} from "./review";

export {
  startAnnotateServer,
  type AnnotateServerOptions,
  type AnnotateServerResult,
} from "./annotate";

// Utilities
export { isRemoteSession, getServerPort } from "./remote";
export { openBrowser } from "./browser";
export { generateSlug } from "./storage";
export { detectObsidianVaults, saveToObsidian, saveToBear } from "./integrations";
export { getRepoInfo, type RepoInfo } from "./repo";
export { detectProjectName, sanitizeTag } from "./project";

// Git utilities
export {
  getGitContext,
  runGitDiff,
  type DiffType,
  type DiffOption,
  type GitContext,
  type DiffResult,
} from "./git";

// Image handling
export {
  validateImagePath,
  validateUploadExtension,
  UPLOAD_DIR,
} from "./image";

// IDE integration
export { openEditorDiff } from "./ide";
