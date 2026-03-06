/**
 * TrackLens Shared Types
 *
 * Comprehensive type definitions for TrackLens - Maestro's integrated visual review,
 * annotation, and walkthrough system for track creation and completion workflows.
 *
 * Operating across Claude Code, OpenCode, Pi-mono, and Maestro CLI/Cockpit.
 *
 * @module tracklens-shared/types
 * @version 1.0.0
 */

// =============================================================================
// 1. CORE TYPES
// =============================================================================

/**
 * Editor interaction modes for the TrackLens annotation UI
 */
export type EditorMode = 'selection' | 'comment' | 'redline';

/**
 * Types of annotations that can be made on documents
 */
export enum AnnotationType {
  COMMENT = 'COMMENT',
  DELETION = 'DELETION',
  INSERTION = 'INSERTION',
  REPLACEMENT = 'REPLACEMENT',
  GLOBAL_COMMENT = 'GLOBAL_COMMENT',
}

/**
 * Markdown block types supported by the parser
 */
export type BlockType =
  | 'paragraph'
  | 'heading'
  | 'blockquote'
  | 'list-item'
  | 'code'
  | 'hr'
  | 'table';

// =============================================================================
// 2. METADATA TYPES
// =============================================================================

/**
 * Document frontmatter - flexible key-value storage
 */
export type Frontmatter = Record<string, any>;

/**
 * Image attachment metadata
 */
export interface ImageAttachment {
  /** File system path to the image */
  path: string;
  /** Human-readable name for the image */
  name: string;
}

/**
 * Web-highlighter metadata for cross-element text selections
 * @deprecated Use HighlightMeta instead
 */
export interface HighlighterMeta {
  /** Parent element tag name */
  parentTagName: string;
  /** Index of the parent element among siblings */
  parentIndex: number;
  /** Character offset within the text node */
  textOffset: number;
}

/**
 * Web-highlighter metadata for cross-element text selections
 * Matches Rust HighlightMeta struct naming
 */
export interface HighlightMeta {
  /** Parent element tag name */
  parentTagName: string;
  /** Index of the parent element among siblings */
  parentIndex: number;
  /** Character offset within the text node */
  textOffset: number;
}

// =============================================================================
// 3. ANNOTATION TYPES
// =============================================================================

/**
 * Individual annotation from the review UI
 * Maps to Rust Annotation struct in tracklens/types.rs
 */
export interface Annotation {
  /** Unique identifier for the annotation */
  id: string;
  /** Block ID this annotation is attached to (legacy - not used with web-highlighter) */
  blockId: string;
  /** Type of annotation */
  type: AnnotationType;
  /** Comment text (for COMMENT and GLOBAL_COMMENT types) */
  text?: string;
  /** The original text that was selected */
  originalText: string;
  /** Unix timestamp when annotation was created */
  createdAt: number;
  /** Author identifier for collaborative sharing */
  author?: string;
  /** Attached images with human-readable names */
  images?: ImageAttachment[];
  /** Web-highlighter start metadata for cross-element selections */
  startMeta?: HighlightMeta;
  /** Web-highlighter end metadata for cross-element selections */
  endMeta?: HighlightMeta;
  /** Legacy offset fields (deprecated, use startMeta/endMeta) */
  startOffset?: number;
  endOffset?: number;
}

/**
 * Markdown block representation
 * Maps to Rust Block struct in tracklens/types.rs
 */
export interface Block {
  /** Unique identifier for the block */
  id: string;
  /** Block type */
  type: BlockType;
  /** Plain text content */
  content: string;
  /** For headings (1-6) or list indentation level */
  level?: number;
  /** For code blocks (e.g., 'rust', 'typescript') */
  language?: string;
  /** For checkbox list items (true = checked, false = unchecked) */
  checked?: boolean;
  /** Sorting order */
  order: number;
  /** 1-based line number in source document */
  startLine: number;
}

// =============================================================================
// 4. CODE REVIEW TYPES
// =============================================================================

/**
 * Types of code review annotations
 */
export type CodeAnnotationType = 'comment' | 'suggestion' | 'concern';

/**
 * Side of a diff comparison
 * Maps to Rust DiffSide enum in tracklens/types.rs
 */
export type DiffSide = 'old' | 'new';

/**
 * Legacy side naming from @pierre/diffs integration
 */
export type LegacyDiffSide = 'deletions' | 'additions';

/**
 * Code review annotation (for diff review mode)
 * Maps to Rust CodeAnnotation struct in tracklens/types.rs
 */
export interface CodeAnnotation {
  /** Unique identifier */
  id: string;
  /** Type of code annotation */
  type: CodeAnnotationType;
  /** File path relative to repository root */
  filePath: string;
  /** Starting line number (1-based) */
  lineStart: number;
  /** Ending line number (1-based, inclusive) */
  lineEnd: number;
  /** Which side of the diff (maps to 'deletions' | 'additions' in @pierre/diffs) */
  side: DiffSide;
  /** Comment text */
  text?: string;
  /** Suggested replacement code */
  suggestedCode?: string;
  /** Original selected lines for suggestion diff */
  originalCode?: string;
  /** Unix timestamp when annotation was created */
  createdAt: number;
  /** Author identifier */
  author?: string;
}

/**
 * Metadata for diff annotations (for @pierre/diffs integration)
 */
export interface DiffAnnotationMetadata {
  /** Reference to parent annotation */
  annotationId: string;
  /** Type of annotation */
  type: CodeAnnotationType;
  /** Comment text */
  text?: string;
  /** Suggested code replacement */
  suggestedCode?: string;
  /** Original code before suggestion */
  originalCode?: string;
  /** Author identifier */
  author?: string;
}

/**
 * Selected line range in a diff view
 * Maps to @pierre/diffs integration
 */
export interface SelectedLineRange {
  /** Starting line number */
  start: number;
  /** Ending line number */
  end: number;
  /** Side where selection started */
  side: LegacyDiffSide;
  /** Side where selection ended (optional, for cross-side selections) */
  endSide?: LegacyDiffSide;
}

/**
 * Diff result structure
 */
export interface DiffResult {
  /** Original text */
  original: string;
  /** Modified text */
  modified: string;
  /** Formatted diff text */
  diffText: string;
}

// =============================================================================
// 5. SERVER / MODE TYPES
// =============================================================================

/**
 * TrackLens operating modes
 * Maps to Rust TrackLensMode enum in tracklens/types.rs
 */
export type TrackLensMode = 'review' | 'codeReview' | 'annotate' | 'walkthrough';

/**
 * Originating platform for the TrackLens session
 * Maps to Rust TrackLensOrigin enum in tracklens/types.rs
 */
export type TrackLensOrigin = 'claudeCode' | 'openCode' | 'piMono' | 'maestro';

/**
 * Maestro autonomy levels (merged from plannotator permission modes)
 * Maps to Rust AutonomyMode enum in tracklens/types.rs
 */
export type AutonomyMode = 'fullAuto' | 'semiAuto' | 'checkpoint';

/**
 * User decision returned after review
 * Maps to Rust TrackLensDecision struct in tracklens/types.rs
 */
export interface TrackLensDecision {
  /** Whether the document was approved */
  approved: boolean;
  /** Global feedback text */
  feedback?: string;
  /** Array of annotations made during review */
  annotations: Annotation[];
  /** Selected autonomy mode for subsequent operations */
  autonomyMode?: AutonomyMode;
  /** OpenCode agent routing identifier */
  agentSwitch?: string;
}

/**
 * Server startup options
 * Maps to Rust TrackLensServerOptions struct in tracklens/types.rs
 */
export interface TrackLensServerOptions {
  /** Markdown content to review */
  markdown: string;
  /** Document type label (e.g., "spec.md", "plan.md") */
  documentType: string;
  /** Track ID for context */
  trackId?: string;
  /** Operating mode */
  mode: TrackLensMode;
  /** Originating platform */
  origin: TrackLensOrigin;
  /** Pre-built HTML bundle content */
  htmlContent: string;
  /** Port to serve on (undefined = random available port) */
  port?: number;
  /** Whether to automatically open browser (defaults to true) */
  openBrowser?: boolean;
}

/**
 * Server state exposed via API
 * Maps to Rust server state response
 */
export interface TrackLensServerState {
  markdown: string;
  documentType: string;
  trackId?: string;
  mode: TrackLensMode;
}

// =============================================================================
// 6. INTEGRATION TYPES
// =============================================================================

/**
 * Obsidian vault configuration
 */
export interface ObsidianConfig {
  /** Path to Obsidian vault directory */
  vaultPath: string;
  /** Vault name */
  vaultName: string;
  /** Whether to use Obsidian URI scheme links */
  useUriLinks: boolean;
}

/**
 * Bear notes app configuration
 */
export interface BearConfig {
  /** Bear API token */
  apiToken?: string;
  /** Default tag to apply to imported notes */
  defaultTag?: string;
  /** Whether to encrypt note contents */
  encrypt?: boolean;
  /** Whether Bear integration is enabled */
  enabled: boolean;
  /** Tags to apply automatically */
  tags: string[];
  /** Whether to auto-export notes */
  autoExport: boolean;
}

/**
 * Agent switching configuration for OpenCode integration
 */
export interface AgentSwitchSettings {
  /** Whether agent switching is enabled */
  enabled: boolean;
  /** Default agent to route feedback to */
  defaultAgent: string;
  /** Agents available for switching */
  availableAgents: string[];
  /** Agent mapping by annotation type (optional extended config) */
  agentMap?: Record<CodeAnnotationType | AnnotationType, string>;
}

/**
 * UI preference settings
 */
export interface UIPreferences {
  /** Theme preference */
  theme: 'light' | 'dark' | 'system';
  /** Font size - can be specific pixels or named size */
  fontSize: 'small' | 'medium' | 'large' | number;
  /** Line height multiplier */
  lineHeight?: number;
  /** Editor width in characters */
  editorWidth?: number;
  /** Whether to show line numbers */
  showLineNumbers: boolean;
  /** Whether to enable spell checking */
  spellCheck?: boolean;
  /** Whether to enable vim keybindings */
  vimMode?: boolean;
  /** Default sidebar visibility */
  sidebarCollapsed: boolean;
  /** Whether to auto-save annotations */
  autoSave?: boolean;
  /** Whether to auto-open browser on server start */
  autoOpenBrowser: boolean;
}

// =============================================================================
// 7. WALKTHROUGH TYPES
// =============================================================================

/**
 * Git file change status for walkthrough
 * Maps to Rust FileChangeStatus enum in tracklens/types.rs
 */
export type FileChangeStatus = 'added' | 'modified' | 'deleted' | 'renamed';

/**
 * A changed file entry for walkthrough
 * Maps to Rust ChangedFile struct in tracklens/types.rs
 */
export interface ChangedFile {
  /** File path relative to repository root */
  path: string;
  /** Change status */
  status: FileChangeStatus;
  /** Programming language for syntax highlighting */
  language: string;
  /** Full git diff for the file */
  diff?: string;
  /** Key code snippet (first N lines) */
  snippet?: string;
  /** Number of lines added */
  additions: number;
  /** Number of lines deleted */
  deletions: number;
}

/**
 * Walkthrough generation configuration
 * Maps to Rust WalkthroughConfig struct in tracklens/types.rs
 */
export interface WalkthroughConfig {
  /** Track ID to generate walkthrough for */
  trackId: string;
  /** Maestro project root path */
  root: string;
  /** Track directory path */
  trackDir: string;
  /** Whether this is a subtrack */
  isSubtrack: boolean;
  /** Parent track ID (if subtrack) */
  parentTrackId?: string;
  /** Include full git diffs in output */
  includeDiffs: boolean;
  /** Include key code snippets in output */
  includeSnippets: boolean;
  /** Maximum lines per snippet (default: 30) */
  maxSnippetLines: number;
}

// =============================================================================
// 8. VAULT / FILE SYSTEM TYPES
// =============================================================================

/**
 * Vault node for file browser (Obsidian-style)
 */
export interface VaultNode {
  /** Node name (file or folder name) */
  name: string;
  /** Relative path within vault */
  path: string;
  /** Node type */
  type: 'file' | 'folder';
  /** Child nodes (for folders) */
  children?: VaultNode[];
}

// =============================================================================
// 9. API REQUEST/RESPONSE TYPES
// =============================================================================

/**
 * Initial state injected into the HTML page
 */
export interface TrackLensInitState {
  /** Markdown content to display */
  markdown: string;
  /** Document type label */
  documentType: string;
  /** Track ID for context */
  trackId?: string;
  /** Operating mode */
  mode: TrackLensMode;
  /** Originating platform */
  origin: TrackLensOrigin;
}

/**
 * API response for server state
 */
export interface TrackLensStateResponse {
  markdown: string;
  documentType: string;
  trackId?: string;
  mode: TrackLensMode;
}

/**
 * POST /api/approve request body
 */
export interface ApproveRequest {
  /** Optional global feedback */
  feedback?: string;
  /** Annotations made during review */
  annotations: Annotation[];
  /** Selected autonomy mode */
  autonomyMode?: AutonomyMode;
  /** Agent switch target (OpenCode) */
  agentSwitch?: string;
}

/**
 * POST /api/deny request body
 */
export interface DenyRequest {
  /** Feedback explaining denial */
  feedback?: string;
  /** Annotations with change requests */
  annotations: Annotation[];
  /** Agent switch target (OpenCode) */
  agentSwitch?: string;
}

// =============================================================================
// 10. HOOK EVENT TYPES (Claude Code Integration)
// =============================================================================

/**
 * Hook event structure for Claude Code integration
 * Matches Rust hook event processing
 */
export interface ClaudeCodeHookEvent {
  /** Tool input containing plan content */
  tool_input?: {
    plan?: string;
  };
  /** Current permission mode */
  permission_mode?: string;
}

/**
 * Hook output structure for Claude Code integration
 */
export interface ClaudeCodeHookOutput {
  hookSpecificOutput: {
    hookEventName: 'PermissionRequest';
    decision: {
      behavior: 'allow' | 'deny';
      message?: string;
      updatedPermissions?: Array<{
        type: 'setMode';
        mode: string;
        destination: 'session';
      }>;
    };
  };
}

/**
 * @deprecated Use ClaudeCodeHookEvent instead
 */
export interface TrackLensHookEvent {
  tool_input?: {
    plan?: string;
  };
  permission_mode?: string;
}

/**
 * @deprecated Use ClaudeCodeHookOutput instead
 */
export interface TrackLensHookDecision {
  hookSpecificOutput: {
    hookEventName: 'PermissionRequest';
    decision: {
      behavior: 'allow' | 'deny';
      message?: string;
      updatedPermissions?: Array<{
        type: 'setMode';
        mode: string;
        destination: 'session';
      }>;
    };
  };
}

// =============================================================================
// 11. TOOL PARAMETER TYPES (Pi-Mono Integration)
// =============================================================================

/**
 * Tool parameter structure for tracklens_review tool (Pi-mono)
 * Matches Rust tool parameter definitions
 */
export interface TrackLensReviewParams {
  /** Markdown content to review */
  markdown: string;
  /** Document type label */
  documentType: string;
  /** Track ID for context */
  trackId?: string;
  /** Operating mode */
  mode: 'review' | 'walkthrough';
}

/**
 * Parameters for tracklens_walkthrough tool
 */
export interface TrackLensWalkthroughParams {
  /** Track ID to generate walkthrough for */
  trackId: string;
  /** Whether to include full git diffs */
  includeDiffs?: boolean;
  /** Whether to include code snippets */
  includeSnippets?: boolean;
}

// =============================================================================
// 12. COCKPIT TUI TYPES
// =============================================================================

/**
 * TrackLens review status for the Cockpit TUI
 * Maps to Rust ReviewStatus struct in cockpit/src/tracklens/pane.rs
 */
export interface ReviewStatus {
  /** Track ID being reviewed */
  trackId: string;
  /** Document type label */
  documentType: string;
  /** Operating mode */
  mode: TrackLensMode;
  /** Server URL */
  serverUrl: string;
  /** When the review started (ISO 8601 timestamp) */
  startedAt: string;
}

/**
 * Review history entry for Cockpit TUI
 * Maps to Rust ReviewHistoryEntry struct in cockpit/src/tracklens/pane.rs
 */
export interface ReviewHistoryEntry {
  /** Track ID */
  trackId: string;
  /** Document type label */
  documentType: string;
  /** Whether the review was approved */
  approved: boolean;
  /** When the review completed (ISO 8601 timestamp) */
  timestamp: string;
  /** Number of annotations made */
  annotationCount: number;
}

/**
 * TrackLens pane state for Cockpit TUI
 * Maps to Rust TrackLensPane struct in cockpit/src/tracklens/pane.rs
 */
export interface TrackLensPaneState {
  /** Whether TrackLens is currently active */
  active: boolean;
  /** Current review status (if active) */
  currentReview?: ReviewStatus;
  /** History of completed reviews */
  history: ReviewHistoryEntry[];
}

// =============================================================================
// 13. DOCUMENT TYPES
// =============================================================================

/**
 * Parsed document structure
 */
export interface ParsedDocument {
  frontmatter: Frontmatter;
  blocks: Block[];
  annotations: Annotation[];
  rawContent: string;
}

// =============================================================================
// 14. UTILITY TYPES
// =============================================================================

/**
 * Git context for code review mode
 */
export interface GitContext {
  /** Default branch name */
  defaultBranch: string;
  /** Current branch name */
  currentBranch: string;
  /** Whether there are uncommitted changes */
  hasUncommittedChanges: boolean;
  /** Number of commits ahead of default branch */
  commitsAhead: number;
  /** Number of commits behind default branch */
  commitsBehind: number;
}

// =============================================================================
// 15. CONSTANT ARRAYS
// =============================================================================

/** All TrackLens modes */
export const TRACKLENS_MODES: TrackLensMode[] = [
  'review',
  'codeReview',
  'annotate',
  'walkthrough'
];

/** All annotation types */
export const ANNOTATION_TYPES: AnnotationType[] = [
  AnnotationType.COMMENT,
  AnnotationType.DELETION,
  AnnotationType.INSERTION,
  AnnotationType.REPLACEMENT,
  AnnotationType.GLOBAL_COMMENT,
];

/** All file change statuses */
export const FILE_CHANGE_STATUSES: FileChangeStatus[] = [
  'added',
  'modified',
  'deleted',
  'renamed',
];

/** All block types */
export const BLOCK_TYPES: BlockType[] = [
  'paragraph',
  'heading',
  'blockquote',
  'list-item',
  'code',
  'hr',
  'table',
];

/** All editor modes */
export const EDITOR_MODES: EditorMode[] = [
  'selection',
  'comment',
  'redline',
];

/** All code annotation types */
export const CODE_ANNOTATION_TYPES: CodeAnnotationType[] = [
  'comment',
  'suggestion',
  'concern',
];

/** Status icon mapping for walkthroughs */
export const STATUS_ICONS: Record<FileChangeStatus, string> = {
  added: '🆕',
  modified: '✏️',
  deleted: '🗑️',
  renamed: '📝',
};

// =============================================================================
// 16. LEGACY COMPATIBILITY EXPORTS
// =============================================================================

/** @deprecated Use AnnotationType instead */
export type PlannotatorAnnotationType = AnnotationType;

/** @deprecated Use TrackLensMode instead */
export type PlannotatorMode = TrackLensMode;

/** @deprecated Use EditorMode instead */
export type PlannotatorEditorMode = EditorMode;

/** @deprecated Use TrackLensDecision instead */
export type PlannotatorDecision = TrackLensDecision;

/** @deprecated Use Annotation instead */
export type PlannotatorAnnotation = Annotation;

// Re-export with TrackLens prefix for explicit naming
export { AnnotationType as TrackLensAnnotationType };
export { Annotation as TrackLensAnnotation };
export { Block as TrackLensBlock };
export { CodeAnnotation as TrackLensCodeAnnotation };
export { TrackLensDecision as TrackLensUserDecision };
