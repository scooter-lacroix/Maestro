/**
 * TrackLens Editor - Main App Component
 *
 * Full-featured plan review interface with annotation support.
 * Enhanced to match Plannotator's feature set.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import {
  parseMarkdownToBlocks,
  exportAnnotations,
  exportLinkedDocAnnotations,
  extractFrontmatter,
} from '@maestro/tracklens-ui';
import {
  Viewer,
  type ViewerHandle,
  AnnotationPanel,
  ExportModal,
  Settings,
  CompletionOverlay,
  ThemeProvider,
  ModeToggle,
  UIFeaturesSetup,
  useResizablePanel,
  ResizeHandle,
  ConfirmDialog,
  TableOfContents,
  ModeSwitcher,
} from '@maestro/tracklens-ui';
import {
  getObsidianSettings,
  getEffectiveVaultPath,
  isObsidianConfigured,
} from '@maestro/tracklens-ui';
import { getBearSettings } from '@maestro/tracklens-ui';
import { getAgentSwitchSettings, getEffectiveAgentName } from '@maestro/tracklens-ui';
import { getDefaultNotesApp } from '@maestro/tracklens-ui';
import { needsUIFeaturesSetup, getUIPreferences, type UIPreferences } from '@maestro/tracklens-ui';
import { saveEditorMode } from '@maestro/tracklens-ui';
import {
  AutonomyModeSetup as PermissionModeSetup,
  getAutonomyModeSettings as getPermissionModeSettings,
  needsAutonomyModeSetup as needsPermissionModeSetup,
  type AutonomyMode as PermissionMode,
} from '@maestro/tracklens-ui';
import { useAgents } from '@maestro/tracklens-ui';
import { useSidebar } from '@maestro/tracklens-ui';
import { useActiveSection } from '@maestro/tracklens-ui';
import { useLinkedDoc } from '@maestro/tracklens-ui';
import { useVaultBrowser } from '@maestro/tracklens-ui';
import { isVaultBrowserEnabled } from '@maestro/tracklens-ui';

import type { Annotation, Block, EditorMode, ImageAttachment } from '@maestro/tracklens-ui';
import type { Frontmatter } from '@maestro/tracklens-ui';
import { MarkdownEditor } from './MarkdownEditor';

// Initialize mermaid (lazy - loaded by MermaidBlock when needed)

// Demo plan content
const DEMO_PLAN = `# Implementation Plan: Real-time Collaboration

## Overview
Add real-time collaboration features to the editor using WebSocket connections.

### Architecture

\`\`\`mermaid
flowchart LR
    subgraph Client["Client Browser"]
        UI[React UI] --> OT[OT Engine]
        OT <--> WS[WebSocket Client]
    end
\`\`\`

## Phase 1: Infrastructure

### WebSocket Server
Set up a WebSocket server to handle concurrent connections.

## Pre-launch Checklist

- [ ] Infrastructure ready
- [x] Security audit complete
- [x] Documentation updated
`;

export default function App() {
  // Core state
  const [markdown, setMarkdown] = useState('');
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [frontmatter, setFrontmatter] = useState<Frontmatter | null>(null);
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [selectedAnnotationId, setSelectedAnnotationId] = useState<string | null>(null);
  const [globalAttachments, setGlobalAttachments] = useState<ImageAttachment[]>([]);

  // Editor state
  const [mode, setMode] = useState<EditorMode>('selection');
  const [isPanelOpen, setIsPanelOpen] = useState(true);
  const [uiPrefs, setUiPrefs] = useState<UIPreferences>(() => getUIPreferences());

  // Edit mode state
  const [editMode, setEditMode] = useState(false);
  const [editedMarkdown, setEditedMarkdown] = useState('');

  // Modal states
  const [showExportModal, setShowExportModal] = useState(false);
  const [showImportModal, setShowImportModal] = useState(false);
  const [showPermissionSetup, setShowPermissionSetup] = useState(false);
  const [showUIFeaturesSetup, setShowUIFeaturesSetup] = useState(false);

  // Dialog states
  const [showFeedbackPrompt, setShowFeedbackPrompt] = useState(false);
  const [showClaudeCodeWarning, setShowClaudeCodeWarning] = useState(false);
  const [showAgentWarning, setShowAgentWarning] = useState(false);
  const [agentWarningMessage, setAgentWarningMessage] = useState('');

  // Completion state
  const [completionResult, setCompletionResult] = useState<'approved' | 'denied' | 'feedback' | null>(null);

  // API/Mode state
  const [isApiMode, setIsApiMode] = useState(false);
  const [origin, setOrigin] = useState<'claude-code' | 'opencode' | 'pi' | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [permissionMode, setPermissionMode] = useState<PermissionMode>('bypassPermissions');
  // Timeout dropdown state
  const [showTimeoutControls, setShowTimeoutControls] = useState(false);
  const [timeLeft, setTimeLeft] = useState<number | null>(1800);

  // Toast notification
  const [toast, setToast] = useState<{ type: 'success' | 'error'; message: string } | null>(null);

  // Phase indicator state
  const [phase, setPhase] = useState<string>('launching');

  // Export dropdown
  const [showExportDropdown, setShowExportDropdown] = useState(false);
  const [initialExportTab, setInitialExportTab] = useState<'export' | 'settings'>('export');

  // Refs
  const viewerRef = useRef<ViewerHandle>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Resizable panels
  const leftPanel = useResizablePanel({
    storageKey: 'tracklens-left-panel-width',
    defaultWidth: 320,
    minWidth: 240,
    maxWidth: 480,
  });
  const tocPanel = useResizablePanel({
    storageKey: 'tracklens-toc-width',
    defaultWidth: 240,
    minWidth: 160,
    maxWidth: 400,
    side: 'left',
  });
  const isResizing = leftPanel.isDragging || tocPanel.isDragging;

  // Sidebar (TOC + Vault)
  const sidebar = useSidebar(uiPrefs.tocEnabled);

  // Vault browser
  const vaultBrowser = useVaultBrowser();
  const showVaultTab = useMemo(() => isVaultBrowserEnabled(), [uiPrefs]);
  const vaultPath = useMemo(() => {
    if (!showVaultTab) return '';
    const settings = getObsidianSettings();
    return getEffectiveVaultPath(settings);
  }, [showVaultTab]);

  const obsidianFolder = useMemo(() => {
    if (!showVaultTab) return '';
    const settings = getObsidianSettings();
    return settings.folder;
  }, [showVaultTab]);

  // Track active section for TOC
  const headingIds = useMemo(() =>
    blocks.filter(b => b.type === 'heading').map(b => b.id),
    [blocks]
  );
  const activeSection = useActiveSection(headingIds);

  // Linked document navigation
  const sidebarForLinkedDoc = { open: (tab: string) => sidebar.open(tab as any) };
  const linkedDocHook = useLinkedDoc({
    markdown, annotations, selectedAnnotationId, globalAttachments,
    setMarkdown, setAnnotations, setSelectedAnnotationId, setGlobalAttachments,
    viewerRef, sidebar: sidebarForLinkedDoc,
  });

  // Agent validation (for OpenCode)
  const { getAgentWarning } = useAgents(origin);

  // Fetch plan on mount
  useEffect(() => {
    fetch('/api/plan')
      .then(res => {
        if (!res.ok) throw new Error('Not in API mode');
        return res.json();
      })
      .then((data: {
        plan: string;
        origin?: 'claude-code' | 'opencode' | 'pi';
        sharingEnabled?: boolean;
        repoInfo?: { display: string; branch?: string };
      }) => {
        if (data.plan) {
          // Detect editable marker for seed content auto-edit mode
          const editableMarker = '<!-- tracklens:editable -->';
          if (data.plan.startsWith(editableMarker)) {
            const stripped = data.plan.replace(editableMarker + '\n', '').replace(editableMarker, '');
            setMarkdown(stripped);
            setEditedMarkdown(stripped);
            setEditMode(true);
          } else {
            setMarkdown(data.plan);
          }
        }
        setIsApiMode(true);
        if (data.origin) {
          setOrigin(data.origin);
          if (data.origin === 'claude-code' && needsPermissionModeSetup()) {
            setShowPermissionSetup(true);
          } else if (needsUIFeaturesSetup()) {
            setShowUIFeaturesSetup(true);
          }
          setPermissionMode(getPermissionModeSettings().mode);
        }
      })
      .catch(() => {
        // Not in API mode - use demo content
        setMarkdown(DEMO_PLAN);
        setIsApiMode(false);
      })
  }, []);

  // Parse markdown when it changes
  useEffect(() => {
    const { frontmatter: fm } = extractFrontmatter(markdown);
    setFrontmatter(fm);
    setBlocks(parseMarkdownToBlocks(markdown));
  }, [markdown]);

  // Timer logic
  useEffect(() => {
    if (timeLeft === null) return;
    if (timeLeft <= 0) return;
    const timer = setInterval(() => {
      setTimeLeft(prev => (prev && prev > 0 ? prev - 1 : 0));
    }, 1000);
    return () => clearInterval(timer);
  }, [timeLeft]);

  // Poll server phase every 2 seconds
  useEffect(() => {
    if (!isApiMode) return;
    const poll = async () => {
      try {
        const res = await fetch('/api/phase');
        if (res.ok) {
          const data = await res.json();
          if (data.phase) setPhase(data.phase);
        }
      } catch { /* ignore */ }
    };
    poll();
    const interval = setInterval(poll, 2000);
    return () => clearInterval(interval);
  }, [isApiMode]);

  // Sync sidebar with preferences
  useEffect(() => {
    if (uiPrefs.tocEnabled) {
      sidebar.open('toc');
    } else {
      sidebar.close();
    }
  }, [uiPrefs.tocEnabled]);

  // Clear vault active file when disabled
  useEffect(() => {
    if (!showVaultTab) vaultBrowser.setActiveFile(null);
  }, [showVaultTab]);

  // Auto-fetch vault tree when vault tab opened
  useEffect(() => {
    if (sidebar.activeTab === 'vault' && showVaultTab && vaultPath &&
      vaultBrowser.tree.length === 0 && !vaultBrowser.isLoading) {
      vaultBrowser.fetchTree(vaultPath, obsidianFolder);
    }
  }, [sidebar.activeTab, showVaultTab, vaultPath, obsidianFolder, vaultBrowser]);

  // Global keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl+Shift+Enter - Force Deny (send feedback)
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && e.shiftKey) {
        const tag = (e.target as HTMLElement)?.tagName;
        if (tag === 'INPUT' || tag === 'TEXTAREA') return;

        if (showExportModal || showImportModal || showFeedbackPrompt ||
          showClaudeCodeWarning || showAgentWarning || showPermissionSetup ||
          showUIFeaturesSetup || isSubmitting || !isApiMode || linkedDocHook.isActive) {
          return;
        }

        e.preventDefault();
        handleDeny();
        return;
      }

      // Cmd/Ctrl+Enter - Approve/Deny
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        const tag = (e.target as HTMLElement)?.tagName;
        if (tag === 'INPUT' || tag === 'TEXTAREA') return;

        if (showExportModal || showImportModal || showFeedbackPrompt ||
          showClaudeCodeWarning || showAgentWarning || showPermissionSetup ||
          showUIFeaturesSetup || isSubmitting || !isApiMode || linkedDocHook.isActive) {
          return;
        }

        e.preventDefault();

        if (annotations.length === 0) {
          // Check agent for OpenCode
          if (origin === 'opencode') {
            const warning = getAgentWarning();
            if (warning) {
              setAgentWarningMessage(warning);
              setShowAgentWarning(true);
              return;
            }
          }
          handleApprove();
        } else {
          handleDeny();
        }
        return;
      }

      // Cmd/Ctrl+E - Toggle edit mode
      if (e.key === 'e' && (e.metaKey || e.ctrlKey)) {
        const tag = (e.target as HTMLElement)?.tagName;
        if (tag === 'INPUT' || tag === 'TEXTAREA') return;

        if (!isApiMode || isSubmitting) return;

        e.preventDefault();
        handleToggleEditMode();
        return;
      }

      // Cmd/Ctrl+S - Save to notes
      if (e.key === 's' && (e.metaKey || e.ctrlKey)) {
        const tag = (e.target as HTMLElement)?.tagName;
        if (tag === 'INPUT' || tag === 'TEXTAREA') return;

        if (showExportModal || showFeedbackPrompt || showClaudeCodeWarning ||
          showAgentWarning || showPermissionSetup || showUIFeaturesSetup ||
          isSubmitting || !isApiMode) {
          return;
        }

        e.preventDefault();
        handleSaveShortcut();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    showExportModal, showImportModal, showFeedbackPrompt, showClaudeCodeWarning,
    showAgentWarning, showPermissionSetup, showUIFeaturesSetup, isSubmitting,
    isApiMode, linkedDocHook.isActive, annotations.length, origin, getAgentWarning,
    markdown, annotations, globalAttachments, editMode,
  ]);

  // Close export dropdown on outside click
  useEffect(() => {
    if (!showExportDropdown) return;
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!target.closest('[data-export-dropdown]')) {
        setShowExportDropdown(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showExportDropdown]);

  // Handlers
  const handleAddAnnotation = useCallback((ann: Annotation) => {
    setAnnotations(prev => [...prev, ann]);
    setSelectedAnnotationId(ann.id);
    setIsPanelOpen(true);
  }, []);

  const handleSelectAnnotation = useCallback((id: string | null) => {
    setSelectedAnnotationId(id);
  }, []);

  const handleDeleteAnnotation = useCallback((id: string) => {
    setAnnotations(prev => prev.filter(a => a.id !== id));
    viewerRef.current?.removeHighlight(id);
    if (selectedAnnotationId === id) {
      setSelectedAnnotationId(null);
    }
  }, [selectedAnnotationId]);

  const handleEditAnnotation = useCallback((id: string, updates: Partial<Annotation>) => {
    setAnnotations(prev => prev.map(a => a.id === id ? { ...a, ...updates } : a));
  }, []);

  const handleIdentityChange = useCallback((oldIdentity: string, newIdentity: string) => {
    setAnnotations(prev => prev.map(ann =>
      ann.author === oldIdentity ? { ...ann, author: newIdentity } : ann
    ));
  }, []);

  const handleAddGlobalAttachment = useCallback((image: ImageAttachment) => {
    setGlobalAttachments(prev => [...prev, image]);
  }, []);

  const handleRemoveGlobalAttachment = useCallback((path: string) => {
    setGlobalAttachments(prev => prev.filter(img => img.path !== path));
  }, []);

  const handleModeChange = useCallback((newMode: EditorMode) => {
    setMode(newMode);
    saveEditorMode(newMode);
  }, []);

  const handleToggleEditMode = async () => {
    const newEditMode = !editMode;
    
    if (newEditMode) {
      // Entering edit mode - capture current markdown
      setEditedMarkdown(markdown);
      setEditMode(true);
      
      // Report phase change to server
      if (isApiMode) {
        try {
          const token = (window as any).TRACKLENS_AUTH_TOKEN;
          await fetch('/api/phase', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
            },
            body: JSON.stringify({ phase: 'editing' }),
          });
        } catch (e) {
          console.error('Failed to report phase:', e);
        }
      }
    } else {
      // Exiting edit mode - return to preview
      setEditMode(false);
      
      // Report phase change to server
      if (isApiMode) {
        try {
          const token = (window as any).TRACKLENS_AUTH_TOKEN;
          await fetch('/api/phase', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
            },
            body: JSON.stringify({ phase: 'reviewing' }),
          });
        } catch (e) {
          console.error('Failed to report phase:', e);
        }
      }
    }
  };

  const handleExtendTimeout = async (minutes: number = 30) => {
    try {
      const token = (window as any).TRACKLENS_AUTH_TOKEN;
      const res = await fetch('/api/extend-timeout', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
        },
        body: JSON.stringify({ minutes }),
      });

      if (res.ok) {
        setTimeLeft(prev => (prev || 0) + (minutes * 60));
        showToast('success', `Extended timeout by ${minutes} minutes`);
      }
    } catch (e) {
      showToast('error', 'Failed to extend timeout');
    }
  };

  const handleApprove = async () => {
    setIsSubmitting(true);
    try {
      const obsidianSettings = getObsidianSettings();
      const bearSettings = getBearSettings();
      const agentSwitchSettings = getAgentSwitchSettings();

      const body: {
        approved: boolean;
        obsidian?: object;
        bear?: object;
        agentSwitch?: string;
        permissionMode?: string;
        feedback?: string;
        annotations?: Annotation[];
        edited_content?: string;
      } = { approved: true };

      if (origin === 'claude-code') {
        body.permissionMode = permissionMode;
      }

      const effectiveAgent = getEffectiveAgentName(agentSwitchSettings);
      if (effectiveAgent) {
        body.agentSwitch = effectiveAgent;
      }

      const effectiveVaultPath = getEffectiveVaultPath(obsidianSettings);
      if (obsidianSettings.enabled && effectiveVaultPath) {
        body.obsidian = {
          vaultPath: effectiveVaultPath,
          folder: obsidianSettings.folder || 'tracklens',
          plan: markdown,
        };
      }

      if (bearSettings.enabled) {
        body.bear = { plan: markdown };
      }

      if (annotations.length > 0 || globalAttachments.length > 0) {
        body.feedback = annotationsOutput;
        body.annotations = annotations;
      }

      // Include edited content if edits exist and content has changed
      if (editedMarkdown && editedMarkdown !== markdown) {
        body.edited_content = editedMarkdown;
      }

      const token = (window as any).TRACKLENS_AUTH_TOKEN;
      const res = await fetch('/api/decision', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
        },
        body: JSON.stringify(body),
      });

      if (res.ok) {
        setCompletionResult('approved');
      }
    } catch (e) {
      console.error('Failed to approve:', e);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDeny = async () => {
    setIsSubmitting(true);
    try {
      const annotationsOutput = exportAnnotations(blocks, annotations, globalAttachments);

      const token = (window as any).TRACKLENS_AUTH_TOKEN;
      const res = await fetch('/api/decision', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
        },
        body: JSON.stringify({
          approved: false,
          feedback: annotationsOutput,
          annotations: annotations,
        }),
      });

      if (res.ok) {
        setCompletionResult('denied');
      }
    } catch (e) {
      console.error('Failed to deny:', e);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleFeedback = async () => {
    if (annotations.length === 0) {
      setShowFeedbackPrompt(true);
      return;
    }

    setIsSubmitting(true);
    try {
      const annotationsOutput = exportAnnotations(blocks, annotations, globalAttachments);

      const token = (window as any).TRACKLENS_AUTH_TOKEN;
      const res = await fetch('/api/decision', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
        },
        body: JSON.stringify({
          approved: false,
          feedback: annotationsOutput,
          annotations: annotations,
        }),
      });

      if (res.ok) {
        setCompletionResult('feedback');
      }
    } catch (e) {
      console.error('Failed to send feedback:', e);
    } finally {
      setIsSubmitting(false);
    }
  };

  // Export/Import handlers
  const handleDownloadAnnotations = () => {
    setShowExportDropdown(false);
    const blob = new Blob([annotationsOutput], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'annotations.md';
    a.click();
    URL.revokeObjectURL(url);
    showToast('success', 'Downloaded annotations');
  };

  const handleQuickSaveToNotes = async (target: 'obsidian' | 'bear') => {
    setShowExportDropdown(false);
    const body: { obsidian?: object; bear?: object } = {};

    if (target === 'obsidian') {
      const s = getObsidianSettings();
      const vaultPath = getEffectiveVaultPath(s);
      if (vaultPath) {
        body.obsidian = {
          vaultPath,
          folder: s.folder || 'tracklens',
          plan: markdown,
        };
      }
    }
    if (target === 'bear') {
      body.bear = { plan: markdown };
    }

    try {
      const res = await fetch('/api/save-notes', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      const result = data.results?.[target];
      if (result?.success) {
        showToast('success', `Saved to ${target === 'obsidian' ? 'Obsidian' : 'Bear'}`);
      } else {
        showToast('error', result?.error || 'Save failed');
      }
    } catch {
      showToast('error', 'Save failed');
    }
  };

  const handleSaveShortcut = () => {
    const defaultApp = getDefaultNotesApp();
    const obsOk = isObsidianConfigured();
    const bearOk = getBearSettings().enabled;

    if (defaultApp === 'download') {
      handleDownloadAnnotations();
    } else if (defaultApp === 'obsidian' && obsOk) {
      handleQuickSaveToNotes('obsidian');
    } else if (defaultApp === 'bear' && bearOk) {
      handleQuickSaveToNotes('bear');
    } else {
      setInitialExportTab('settings');
      setShowExportModal(true);
    }
  };

  const showToast = (type: 'success' | 'error', message: string) => {
    setToast({ type, message });
    setTimeout(() => setToast(null), 3000);
  };

  // Vault handlers
  const buildVaultDocUrl = useCallback(
    (vp: string) => (path: string) =>
      `/api/reference/obsidian/doc?vaultPath=${encodeURIComponent(vp)}&path=${encodeURIComponent(path)}`,
    []
  );

  const handleVaultFileSelect = useCallback((relativePath: string) => {
    linkedDocHook.openDoc(relativePath, buildVaultDocUrl(vaultPath));
    vaultBrowser.setActiveFile(relativePath);
  }, [vaultPath, linkedDocHook, vaultBrowser, buildVaultDocUrl]);

  const handleOpenLinkedDoc = useCallback((docPath: string) => {
    if (vaultBrowser.activeFile && vaultPath) {
      linkedDocHook.openDoc(docPath, buildVaultDocUrl(vaultPath));
    } else {
      linkedDocHook.openDoc(docPath);
    }
  }, [vaultBrowser.activeFile, vaultPath, linkedDocHook, buildVaultDocUrl]);

  const handleLinkedDocBack = useCallback(() => {
    linkedDocHook.back();
    vaultBrowser.setActiveFile(null);
  }, [linkedDocHook, vaultBrowser]);

  // Navigation handlers
  const handleTocNavigate = useCallback((blockId: string) => {
    const element = document.querySelector(`[data-block-id="${blockId}"]`);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }, []);

  // Computed values
  const annotationsOutput = useMemo(() => {
    const docAnnotations = linkedDocHook.isActive ? new Map() : new Map();
    const hasDocAnnotations = Array.from(docAnnotations.values()).some(
      (d: any) => d.annotations?.length > 0 || d.globalAttachments?.length > 0
    );
    const hasPlanAnnotations = annotations.length > 0 || globalAttachments.length > 0;

    if (!hasPlanAnnotations && !hasDocAnnotations) {
      return 'No changes detected.';
    }

    let output = hasPlanAnnotations
      ? exportAnnotations(blocks, annotations, globalAttachments)
      : '';

    if (hasDocAnnotations) {
      output += exportLinkedDocAnnotations(docAnnotations);
    }

    return output;
  }, [blocks, annotations, globalAttachments, linkedDocHook.isActive]);

  const annotationCount = annotations.length;

  const agentName = useMemo(() => {
    if (origin === 'opencode') return 'OpenCode';
    if (origin === 'claude-code') return 'Claude Code';
    if (origin === 'pi') return 'Pi';
    return 'Coding Agent';
  }, [origin]);

  const completionTitles = {
    approved: 'Plan Approved',
    denied: 'Feedback Sent',
    feedback: 'Feedback Sent',
  };

  const completionSubtitles = {
    approved: `${agentName} will proceed with the implementation.`,
    denied: `${agentName} will revise the plan based on your annotations.`,
    feedback: `${agentName} will revise the plan based on your annotations.`,
  };

  return (
    <ThemeProvider>
      <div className="h-screen flex flex-col bg-background text-foreground overflow-hidden">
        {/* Header */}
        <header className="h-12 flex items-center justify-between px-2 md:px-4 fabric-border-b bg-card/50 backdrop-blur-xl sticky top-0 z-20">
          <div className="flex items-center gap-2 md:gap-3">
            <button
              onClick={() => sidebar.isOpen ? sidebar.close() : sidebar.open()}
              className={`p-1.5 rounded-md transition-all ${sidebar.isOpen ? 'bg-primary/10 text-primary shadow-neu-inset-small' : 'text-muted-foreground hover:text-foreground hover:bg-muted shadow-neu-small'}`}
              title={sidebar.isOpen ? 'Close sidebar' : 'Open sidebar'}
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M4 6h16M4 12h16M4 18h7" />
              </svg>
            </button>
            {origin && (
              <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium hidden md:inline ${origin === 'claude-code'
                ? 'bg-orange-500/15 text-orange-400'
                : origin === 'pi'
                  ? 'bg-violet-500/15 text-violet-400'
                  : 'bg-zinc-500/20 text-zinc-400'
                }`}>
                {agentName}
              </span>
            )}
            {isApiMode && phase !== 'launching' && phase !== 'decided' && (
              <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium hidden md:inline ${
                phase === 'reviewing' ? 'bg-blue-500/15 text-blue-400'
                : phase === 'editing' ? 'bg-amber-500/15 text-amber-400'
                : phase === 'loading' ? 'bg-gray-500/15 text-gray-400'
                : 'bg-zinc-500/15 text-zinc-400'
              }`}>
                {phase.charAt(0).toUpperCase() + phase.slice(1)}
              </span>
            )}
          </div>

          <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">
            <a
              href="https://github.com/scooter-lacroix/Maestro.git"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-2 px-3 py-1.5 rounded-2xl bg-background border border-border/10 shadow-neu-extruded hover:-translate-y-0.5 hover:shadow-neu-hover active:translate-y-[0.5px] active:shadow-neu-inset transition-all animate-brand-pulse"
            >
              <h1 className="text-sm font-bold font-display tracking-tight text-foreground/90 group-hover:text-primary transition-colors">
                TrackLens
              </h1>
            </a>
          </div>
          <div className="flex items-center gap-1 md:gap-2">
            {isApiMode && !linkedDocHook.isActive && (
              <>
                <button
                  onClick={handleToggleEditMode}
                  disabled={isSubmitting}
                  className={`p-1.5 md:px-2.5 md:py-1 rounded-md text-xs font-medium transition-all ${isSubmitting
                    ? 'opacity-50 cursor-not-allowed bg-muted text-muted-foreground'
                    : editMode
                      ? 'bg-primary/10 text-primary hover:bg-primary/20 border border-primary/30'
                      : 'bg-muted hover:bg-muted/80'
                    }`}
                  title={editMode ? 'Switch to preview mode' : 'Switch to edit mode'}
                >
                  <span className="hidden md:inline">{editMode ? 'Preview' : 'Edit'}</span>
                  <svg className="w-4 h-4 md:hidden" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                    {editMode ? (
                      <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                    ) : (
                      <path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                    )}
                  </svg>
                </button>

                <button
                  onClick={handleFeedback}
                  disabled={isSubmitting || editMode}
                  className={`p-1.5 md:px-2.5 md:py-1 rounded-md text-xs font-medium transition-all ${isSubmitting || editMode
                    ? 'opacity-50 cursor-not-allowed bg-muted text-muted-foreground'
                    : 'bg-accent/15 text-accent hover:bg-accent/25 border border-accent/30'
                    }`}
                  title="Send Feedback (Cmd+Enter)"
                >
                  <span className="hidden md:inline">{isSubmitting ? 'Sending...' : 'Feedback'}</span>
                  <svg className="w-4 h-4 md:hidden" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                    <path strokeLinecap="round" strokeLinejoin="round" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                  </svg>
                </button>

                <div className="relative group/approve">
                  <button
                    onClick={() => {
                      if (origin === 'claude-code' && annotations.length > 0) {
                        setShowClaudeCodeWarning(true);
                        return;
                      }
                      if (origin === 'opencode') {
                        const warning = getAgentWarning();
                        if (warning) {
                          setAgentWarningMessage(warning);
                          setShowAgentWarning(true);
                          return;
                        }
                      }
                      handleApprove();
                    }}
                    disabled={isSubmitting || editMode}
                    className={`px-2 py-1 md:px-2.5 rounded-md text-xs font-medium transition-all ${isSubmitting || editMode
                      ? 'opacity-50 cursor-not-allowed bg-muted text-muted-foreground'
                      : 'bg-success text-success-foreground hover:opacity-90'
                      }`}
                    title="Approve (Cmd+Enter)"
                  >
                    <span className="md:hidden">{isSubmitting ? '...' : 'OK'}</span>
                    <span className="hidden md:inline">{isSubmitting ? 'Approving...' : 'Approve'}</span>
                  </button>
                  {origin === 'claude-code' && annotations.length > 0 && (
                    <div className="absolute top-full right-0 mt-2 px-3 py-2 bg-popover border border-border rounded-lg shadow-xl text-xs text-foreground w-56 text-center opacity-0 invisible group-hover/approve:opacity-100 group-hover/approve:visible transition-all pointer-events-none z-50">
                      <div className="absolute bottom-full right-4 border-4 border-transparent border-b-border" />
                      <div className="absolute bottom-full right-4 mt-px border-4 border-transparent border-b-popover" />
                      {agentName} doesn't support feedback on approval. Your annotations won't be seen.
                    </div>
                  )}
                </div>

                <div className="w-px h-5 bg-border/50 mx-1 hidden md:block" />
              </>
            )}

            {timeLeft !== null && (
              <div className="relative">
                <button
                  onClick={() => setShowTimeoutControls(!showTimeoutControls)}
                  className={`flex items-center gap-1 px-2 py-1 rounded-md text-xs font-medium transition-all ${timeLeft < 300
                    ? 'bg-red-500/20 text-red-500 border border-red-500/30'
                    : 'bg-yellow-500/10 text-yellow-600 border border-yellow-500/20'
                    } hover:scale-105 active:scale-95`}
                  title="Click to extend timeout"
                >
                  <svg className={`w-3.5 h-3.5 ${timeLeft < 300 ? 'animate-pulse' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                    <path strokeLinecap="round" strokeLinejoin="round" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span>{Math.floor(timeLeft / 60)}:{(timeLeft % 60).toString().padStart(2, '0')}</span>
                </button>

                {showTimeoutControls && (
                  <div className="absolute top-full right-0 mt-2 p-2 bg-popover border border-border rounded-lg shadow-xl z-50 min-w-[140px]">
                    <div className="text-xs text-muted-foreground mb-2 px-1">Extend timeout:</div>
                    {[3, 5, 10, 15].map(minutes => (
                      <button
                        key={minutes}
                        onClick={() => {
                          handleExtendTimeout(minutes);
                          setShowTimeoutControls(false);
                        }}
                        className="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted transition-colors flex items-center gap-2"
                      >
                        <span className="font-medium">+{minutes} min</span>
                      </button>
                    ))}
                    <button
                      onClick={() => setShowTimeoutControls(false)}
                      className="w-full text-center px-2 py-1 text-xs text-muted-foreground hover:text-foreground mt-1 border-t border-border/50"
                    >
                      Cancel
                    </button>
                  </div>
                )}
              </div>
            )}

            <ModeToggle />

            {!linkedDocHook.isActive && (
              <Settings
                onIdentityChange={handleIdentityChange}
                origin={origin}
                mode="plan"
                onUIPreferencesChange={setUiPrefs}
              />
            )}

            <button
              onClick={() => setIsPanelOpen(!isPanelOpen)}
              className={`p-1.5 rounded-md transition-all ml-1 ${isPanelOpen
                ? 'bg-primary/10 text-primary shadow-neu-inset-small'
                : 'text-muted-foreground hover:text-foreground hover:bg-muted shadow-neu-small'
                }`}
              title={isPanelOpen ? 'Close annotations' : 'Open annotations'}
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
              </svg>
            </button>

            {/* Export Dropdown */}
            <div className="relative flex" data-export-dropdown>
              <button
                onClick={() => { setShowExportModal(true); setInitialExportTab('export'); }}
                className="p-1.5 md:px-2.5 md:py-1 rounded-l-md text-xs font-medium bg-muted hover:bg-muted/80 transition-colors"
                title="Export"
              >
                <svg className="w-4 h-4 md:hidden" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
                </svg>
                <span className="hidden md:inline">Export</span>
              </button>
              <button
                onClick={() => setShowExportDropdown(prev => !prev)}
                className="px-1 md:px-1.5 rounded-r-md text-xs bg-muted hover:bg-muted/80 border-l border-border/50 transition-colors flex items-center"
                title="Quick save options"
              >
                <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                </svg>
              </button>

              {showExportDropdown && (
                <div className="absolute top-full right-0 mt-1 w-48 bg-popover border border-border rounded-lg shadow-xl z-50 py-1">
                  <button
                    onClick={handleDownloadAnnotations}
                    className="w-full text-left px-3 py-1.5 text-xs hover:bg-muted transition-colors flex items-center gap-2"
                  >
                    <svg className="w-3.5 h-3.5 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                      <path strokeLinecap="round" strokeLinejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                    </svg>
                    Download Annotations
                  </button>
                  {isApiMode && isObsidianConfigured() && (
                    <button
                      onClick={() => handleQuickSaveToNotes('obsidian')}
                      className="w-full text-left px-3 py-1.5 text-xs hover:bg-muted transition-colors flex items-center gap-2"
                    >
                      <svg className="w-3.5 h-3.5 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                        <path strokeLinecap="round" strokeLinejoin="round" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
                      </svg>
                      Save to Obsidian
                    </button>
                  )}
                  {isApiMode && getBearSettings().enabled && (
                    <button
                      onClick={() => handleQuickSaveToNotes('bear')}
                      className="w-full text-left px-3 py-1.5 text-xs hover:bg-muted transition-colors flex items-center gap-2"
                    >
                      <svg className="w-3.5 h-3.5 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                        <path strokeLinecap="round" strokeLinejoin="round" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
                      </svg>
                      Save to Bear
                    </button>
                  )}
                </div>
              )}
            </div>
          </div>
        </header>

        {/* Linked document error banner */}
        {
          linkedDocHook.error && (
            <div className="bg-destructive/10 border-b border-destructive/20 px-4 py-2 flex items-center gap-2 flex-shrink-0">
              <span className="text-xs text-destructive">{linkedDocHook.error}</span>
            </div>
          )
        }

        <div className={`flex-1 flex overflow-hidden bg-complex relative ${isResizing ? 'select-none' : ''}`}>
          {/* Left Sidebar: TOC / Vault */}
          {sidebar.isOpen && (
            <>
              <div
                className="fabric-border-r overflow-hidden flex flex-col bg-card/30"
                style={{ width: tocPanel.width }}
              >
                {/* Tab Switcher */}
                <div className="flex border-b border-border">
                  <button
                    onClick={() => sidebar.toggleTab('toc')}
                    className={`flex-1 px-3 py-2 text-xs font-medium transition-colors ${sidebar.activeTab === 'toc'
                      ? 'bg-primary/10 text-primary border-b-2 border-primary'
                      : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
                      }`}
                  >
                    Contents
                  </button>
                  {showVaultTab && (
                    <button
                      onClick={() => sidebar.toggleTab('vault')}
                      className={`flex-1 px-3 py-2 text-xs font-medium transition-colors ${sidebar.activeTab === 'vault'
                        ? 'bg-primary/10 text-primary border-b-2 border-primary'
                        : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
                        }`}
                    >
                      Vault
                    </button>
                  )}
                </div>

                {/* Tab Content */}
                <div className="flex-1 overflow-y-auto">
                  {sidebar.activeTab === 'toc' && (
                    <TableOfContents
                      blocks={blocks}
                      annotations={annotations}
                      activeId={activeSection}
                      onNavigate={handleTocNavigate}
                    />
                  )}
                  {sidebar.activeTab === 'vault' && showVaultTab && (
                    <div className="p-3">
                      {vaultBrowser.isLoading ? (
                        <div className="text-xs text-muted-foreground text-center py-4">Loading...</div>
                      ) : vaultBrowser.error ? (
                        <div className="text-xs text-destructive text-center py-4">{vaultBrowser.error}</div>
                      ) : (
                        <div className="space-y-1">
                          {vaultBrowser.tree.map((node) => (
                            <VaultNodeComponent
                              key={node.path}
                              node={node}
                              activeFile={vaultBrowser.activeFile}
                              onSelect={handleVaultFileSelect}
                            />
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                </div>
              </div>
              <ResizeHandle {...tocPanel.handleProps} className="hidden lg:block" />
            </>
          )}

          <main ref={containerRef} className="flex-1 min-w-0 overflow-y-auto relative">
            {/* Mode Switcher - Sticky Placement with Sliding Animation */}
            {!editMode && (
              <div
                className="sticky top-6 z-30 flex justify-end px-8 pointer-events-none transition-all duration-300 ease-in-out"
                style={{
                  right: isPanelOpen ? `${leftPanel.width + 32}px` : '32px'
                }}
              >
                <div className="pointer-events-auto">
                  <ModeSwitcher mode={mode} onChange={handleModeChange} />
                </div>
              </div>
            )}

            <div className="min-h-full flex flex-col items-center px-4 py-1 md:px-10 md:py-4 xl:px-16">
              {editMode ? (
                <div className="w-full max-w-4xl py-4">
                  <MarkdownEditor
                    value={editedMarkdown}
                    onChange={setEditedMarkdown}
                  />
                </div>
              ) : (
                <Viewer
                  key={linkedDocHook.isActive ? `doc:${linkedDocHook.filepath}` : 'plan'}
                  ref={viewerRef}
                  blocks={blocks}
                  markdown={markdown}
                  frontmatter={frontmatter}
                  annotations={annotations}
                  onAddAnnotation={handleAddAnnotation}
                  onSelectAnnotation={handleSelectAnnotation}
                  selectedAnnotationId={selectedAnnotationId}
                  mode={mode}
                  onModeChange={handleModeChange}
                  globalAttachments={globalAttachments}
                  onAddGlobalAttachment={handleAddGlobalAttachment}
                  onRemoveGlobalAttachment={handleRemoveGlobalAttachment}
                  stickyActions={uiPrefs.stickyActionsEnabled}
                  onOpenLinkedDoc={handleOpenLinkedDoc}
                  linkedDocInfo={linkedDocHook.isActive ? {
                    filepath: linkedDocHook.filepath!,
                    onBack: handleLinkedDocBack
                  } : null}
                  showToc={false}
                />
              )}
            </div>
          </main>

          {/* Resize Handle */}
          {isPanelOpen && !editMode && <ResizeHandle {...leftPanel.handleProps} />}

          {/* Annotation Panel */}
          {!editMode && (
            <div className="fabric-border-l flex flex-col bg-card/30" style={{ width: leftPanel.width }}>
              <AnnotationPanel
                isOpen={isPanelOpen}
                annotations={annotations}
                blocks={blocks}
                onSelect={handleSelectAnnotation}
                onDelete={handleDeleteAnnotation}
                onEdit={handleEditAnnotation}
                selectedId={selectedAnnotationId}
              />
            </div>
          )}
        </div>

        {/* Export Modal */}
        <ExportModal
          isOpen={showExportModal}
          onClose={() => { setShowExportModal(false); setInitialExportTab('export'); }}
          annotationsOutput={annotationsOutput}
          annotationCount={annotationCount}
          markdown={markdown}
          isApiMode={isApiMode}
          initialTab={initialExportTab}
        />

        {/* Import Modal */}
        {
          showImportModal && (
            <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
              <div className="bg-card border border-border rounded-lg shadow-xl p-6 w-full max-w-md">
                <h2 className="text-lg font-semibold mb-4">Import Review</h2>
                <p className="text-sm text-muted-foreground mb-4">
                  Paste a share URL to import a review.
                </p>
                <input
                  type="text"
                  placeholder="https://tracklens.dev/share/..."
                  className="w-full px-3 py-2 bg-background border border-border rounded-md text-sm mb-4"
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      setShowImportModal(false);
                    }
                  }}
                />
                <div className="flex justify-end gap-2">
                  <button
                    onClick={() => setShowImportModal(false)}
                    className="px-4 py-2 text-sm bg-muted rounded-md hover:bg-muted/80"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={() => setShowImportModal(false)}
                    className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:opacity-90"
                  >
                    Import
                  </button>
                </div>
              </div>
            </div>
          )
        }

        {/* Feedback prompt dialog */}
        <ConfirmDialog
          isOpen={showFeedbackPrompt}
          onClose={() => setShowFeedbackPrompt(false)}
          title="Add Annotations First"
          message={`To provide feedback, select text in the plan and add annotations. ${agentName} will use your annotations to revise the plan.`}
          variant="info"
        />

        {/* Claude Code annotation confirmation dialog */}
        <ConfirmDialog
          isOpen={showClaudeCodeWarning}
          onClose={() => setShowClaudeCodeWarning(false)}
          onConfirm={() => {
            setShowClaudeCodeWarning(false);
            handleApprove();
          }}
          title="Confirm Approval with Annotations"
          message={`You have ${annotations.length} annotation${annotations.length !== 1 ? 's' : ''} that will be sent with your approval.`}
          confirmText="Approve"
          cancelText="Cancel"
          variant="info"
          showCancel
        />

        {/* OpenCode agent not found warning dialog */}
        <ConfirmDialog
          isOpen={showAgentWarning}
          onClose={() => setShowAgentWarning(false)}
          onConfirm={() => {
            setShowAgentWarning(false);
            handleApprove();
          }}
          title="Agent Not Found"
          message={agentWarningMessage}
          subMessage="You can change the agent in Settings, or approve anyway and OpenCode will use the default agent."
          confirmText="Approve Anyway"
          cancelText="Cancel"
          variant="warning"
          showCancel
        />

        {/* Permission Mode Setup */}
        {
          showPermissionSetup && (
            <PermissionModeSetup
              isOpen={showPermissionSetup}
              onComplete={(mode) => {
                setPermissionMode(mode);
                setShowPermissionSetup(false);
                if (needsUIFeaturesSetup()) {
                  setShowUIFeaturesSetup(true);
                }
              }}
            />
          )
        }

        {/* UI Features Setup */}
        {
          showUIFeaturesSetup && (
            <UIFeaturesSetup
              isOpen={showUIFeaturesSetup}
              onComplete={(prefs) => {
                setUiPrefs(prefs);
                setShowUIFeaturesSetup(false);
              }}
            />
          )
        }

        {/* Completion overlay */}
        {
          completionResult && (
            <CompletionOverlay
              submitted={completionResult}
              title={completionTitles[completionResult]}
              subtitle={completionSubtitles[completionResult]}
              agentLabel={agentName}
            />
          )
        }

        {/* Toast notification */}
        {
          toast && (
            <div className={`fixed top-16 right-4 z-50 px-3 py-2 rounded-lg text-xs font-medium shadow-lg transition-opacity ${toast.type === 'success'
              ? 'bg-success/15 text-success border border-success/30'
              : 'bg-destructive/15 text-destructive border border-destructive/30'
              }`}>
              {toast.message}
            </div>
          )
        }
      </div >
    </ThemeProvider >
  );
}

// Vault browser node component
interface VaultNodeProps {
  node: {
    name: string;
    path: string;
    type: 'file' | 'folder';
    children?: VaultNodeProps['node'][];
  };
  activeFile: string | null;
  onSelect: (path: string) => void;
  depth?: number;
}

function VaultNodeComponent({ node, activeFile, onSelect, depth = 0 }: VaultNodeProps) {
  const [expanded, setExpanded] = useState(true);
  const isActive = activeFile === node.path;
  const isFolder = node.type === 'folder';

  return (
    <div>
      <button
        onClick={() => isFolder ? setExpanded(!expanded) : onSelect(node.path)}
        className={`w-full text-left px-2 py-1.5 rounded text-xs flex items-center gap-1.5 transition-colors ${isActive
          ? 'bg-primary/10 text-primary'
          : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
          }`}
        style={{ paddingLeft: `${8 + depth * 12}px` }}
      >
        {isFolder && (
          <svg
            className={`w-3 h-3 transition-transform ${expanded ? '' : '-rotate-90'}`}
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
          </svg>
        )}
        {!isFolder && (
          <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
        )}
        <span className="truncate">{node.name}</span>
      </button>
      {isFolder && expanded && node.children?.map((child) => (
        <VaultNodeComponent
          key={child.path}
          node={child}
          activeFile={activeFile}
          onSelect={onSelect}
          depth={depth + 1}
        />
      ))}
    </div>
  );
}
