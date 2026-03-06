/**
 * TrackLens UI - Main Export
 *
 * REBRANDED: Plannotator → TrackLens
 */

export * from './components/ThemeProvider';
export * from './components/ModeToggle';
export * from './components/ModeSwitcher';
export * from './components/ConfirmDialog';
export * from './components/CompletionOverlay';
export * from './components/ResizeHandle';
export * from './components/AutonomyModeSetup';
export { PermissionModeSetup } from './components/PermissionModeSetup';
export * from './components/UIFeaturesSetup';
export * from './components/Settings';
export * from './components/TableOfContents';
export * from './components/MermaidBlock';
export * from './components/AnnotationPanel';
export * from './components/AnnotationSidebar';
export * from './components/ExportModal';
export * from './components/ImportModal';
export * from './components/Viewer';
export * from './components/AttachmentsButton';
export * from './components/ImageThumbnail';
export * from './components/Landing';
export * from './components/TaterSpritePullup';
export * from './components/TaterSpriteRunning';
export * from './components/TaterSpriteSitting';

// Plan Diff components
export * from './components/plan-diff/PlanCleanDiffView';
export * from './components/plan-diff/PlanDiffBadge';
export * from './components/plan-diff/PlanDiffMarketing';
export * from './components/plan-diff/PlanDiffModeSwitcher';
export * from './components/plan-diff/PlanDiffViewer';
export * from './components/plan-diff/PlanRawDiffView';
export * from './components/plan-diff/VSCodeIcon';

// Sidebar components
export * from './components/sidebar/SidebarContainer';
export * from './components/sidebar/SidebarTabs';
export * from './components/sidebar/VaultBrowser';
export * from './components/sidebar/VersionBrowser';

// Image Annotator components
export * from './components/ImageAnnotator';
export * from './components/ImageAnnotator/types';
export * from './components/ImageAnnotator/utils';

export * from './hooks/useResizablePanel';
export * from './hooks/useDismissOnOutsideAndEscape';
export * from './hooks/useAutoClose';
export * from './hooks/useAgents';
export * from './hooks/useSidebar';
export * from './hooks/useVaultBrowser';
export * from './hooks/useActiveSection';
export * from './hooks/usePlanDiff';
export * from './hooks/useLinkedDoc';

export * from './utils/storage';
export * from './utils/identity';
export * from './utils/obsidian';
export * from './utils/bear';
export * from './utils/agentSwitch';
export * from './utils/docSave';
export * from './utils/uiPreferences';
export * from './utils/autonomyMode';
export * from './utils/defaultNotesApp';
export * from './utils/editorMode';
export * from './utils/parser';
export * from './utils/annotationHelpers';

export * from './types';
