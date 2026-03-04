/**
 * TrackLens Editor - Main App Component
 *
 * Main plan review interface with annotation support.
 * Properly ported from Plannotator without shortcuts.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import {
  parseMarkdownToBlocks,
  exportAnnotations,
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
  PermissionModeSetup,
  UIFeaturesSetup,
  useResizablePanel,
} from '@maestro/tracklens-ui';
import {
  getObsidianSettings,
  getEffectiveVaultPath,
} from '@maestro/tracklens-ui';
import { getBearSettings } from '@maestro/tracklens-ui';
import { getAgentSwitchSettings } from '@maestro/tracklens-ui';
import { needsUIFeaturesSetup } from '@maestro/tracklens-ui';
import { getEditorMode, saveEditorMode } from '@maestro/tracklens-ui';
import {
  AutonomyModeSetup as PermissionModeSetup,
  getAutonomyModeSettings as getPermissionModeSettings,
  needsAutonomyModeSetup as needsPermissionModeSetup,
} from '@maestro/tracklens-ui';
import type { Annotation, Block, EditorMode, ImageAttachment } from '@maestro/tracklens-ui';

// Initialize mermaid
const mermaid = require('mermaid');
mermaid.initialize({ startOnLoad: false, securityLevel: 'strict', theme: 'dark' });

export default function App() {
  const [markdown, setMarkdown] = useState('');
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [selectedAnnotationId, setSelectedAnnotationId] = useState<string | null>(null);
  const [globalImages, setGlobalImages] = useState<ImageAttachment[]>([]);
  
  const [mode, setMode] = useState<EditorMode>('selection');
  const [showExportModal, setShowExportModal] = useState(false);
  const [showPermissionSetup, setShowPermissionSetup] = useState(false);
  const [showUIFeaturesSetup, setShowUIFeaturesSetup] = useState(false);
  const [completionResult, setCompletionResult] = useState<'approved' | 'denied' | 'feedback' | null>(null);
  
  const viewerRef = useRef<ViewerHandle>(null);
  const leftPanel = useResizablePanel({ storageKey: 'tracklens-left-panel-width', defaultWidth: 320 });

  // Fetch plan on mount
  useEffect(() => {
    fetch('/api/plan')
      .then(res => res.json())
      .then((data: { plan: string }) => {
        setMarkdown(data.plan);
        setBlocks(parseMarkdownToBlocks(data.plan));
      })
      .catch(() => {
        // Demo data
        const demoMarkdown = `# Demo Plan\n\nThis is a demo plan for TrackLens.\n\n## Features\n\n- Annotation support\n- Code review\n- Export to Obsidian\n`;
        setMarkdown(demoMarkdown);
        setBlocks(parseMarkdownToBlocks(demoMarkdown));
      });
  }, []);

  // Check if setup is needed
  useEffect(() => {
    if (needsPermissionModeSetup()) {
      setShowPermissionSetup(true);
    } else if (needsUIFeaturesSetup()) {
      setShowUIFeaturesSetup(true);
    }

    const savedMode = getEditorMode();
    setMode(savedMode);
  }, []);

  const handleAddAnnotation = useCallback((ann: Annotation) => {
    setAnnotations(prev => [...prev, ann]);
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

  const handleApprove = async () => {
    const obsidianSettings = getObsidianSettings();
    const bearSettings = getBearSettings();
    const agentSettings = getAgentSwitchSettings();
    const permissionSettings = getPermissionModeSettings();

    // Export annotations for feedback
    const annotationsOutput = exportAnnotations(blocks, annotations);

    const body = {
      approved: true,
      feedback: annotationsOutput,
      obsidian: obsidianSettings.enabled ? {
        vaultPath: getEffectiveVaultPath(obsidianSettings),
        folder: obsidianSettings.folder,
        filenameFormat: obsidianSettings.filenameFormat,
      } : undefined,
      bear: bearSettings.enabled ? {} : undefined,
      agentSwitch: agentSettings.switchTo,
      autonomyMode: permissionSettings.mode,
      annotations: JSON.stringify(annotations),
    };

    try {
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
    }
  };

  const handleDeny = async () => {
    const annotationsOutput = exportAnnotations(blocks, annotations);

    try {
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
          annotations: JSON.stringify(annotations),
        }),
      });
      if (res.ok) {
        setCompletionResult('denied');
      }
    } catch (e) {
      console.error('Failed to deny:', e);
    }
  };

  const handleFeedback = async () => {
    const annotationsOutput = exportAnnotations(blocks, annotations);

    try {
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
          annotations: JSON.stringify(annotations),
        }),
      });
      if (res.ok) {
        setCompletionResult('feedback');
      }
    } catch (e) {
      console.error('Failed to send feedback:', e);
    }
  };

  const handleAddGlobalImage = useCallback((image: ImageAttachment) => {
    setGlobalImages(prev => [...prev, image]);
  }, []);

  const handleRemoveGlobalImage = useCallback((path: string) => {
    setGlobalImages(prev => prev.filter(img => img.path !== path));
  }, []);

  const annotationsOutput = exportAnnotations(blocks, annotations);
  const annotationCount = annotations.length;

  const completionTitles = {
    approved: 'Plan Approved',
    denied: 'Plan Denied',
    feedback: 'Feedback Sent',
  };

  const completionSubtitles = {
    approved: 'The plan has been approved and saved to your notes.',
    denied: 'The plan has been denied. You can provide feedback to help improve it.',
    feedback: 'Your feedback has been sent to help improve the plan.',
  };

  return (
    <ThemeProvider>
      <div className="h-screen flex flex-col bg-background text-foreground overflow-hidden">
        {/* Header */}
        <header className="flex items-center justify-between px-4 py-3 border-b border-border bg-card">
          <div className="flex items-center gap-3">
            <h1 className="text-lg font-semibold">TrackLens</h1>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowExportModal(true)}
              className="px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-lg hover:opacity-90 transition-opacity"
            >
              Export
            </button>
            <Settings onIdentityChange={() => {}} origin="claude-code" mode="plan" />
            <ModeToggle />
          </div>
        </header>

        {/* Main Content */}
        <div className="flex-1 flex overflow-hidden">
          {/* Left Panel - Annotation Panel */}
          <div
            className="border-r border-border overflow-hidden"
            style={{ width: leftPanel.width }}
          >
            <AnnotationPanel
              isOpen={true}
              annotations={annotations}
              blocks={blocks}
              onSelect={handleSelectAnnotation}
              onDelete={handleDeleteAnnotation}
              onEdit={handleEditAnnotation}
              selectedId={selectedAnnotationId}
              width={leftPanel.width}
            />
          </div>

          {/* Resizer */}
          <div {...leftPanel.handleProps} className="w-1 bg-border hover:bg-primary/50 cursor-col-resize" />

          {/* Viewer */}
          <div className="flex-1">
            <Viewer
              ref={viewerRef}
              blocks={blocks}
              markdown={markdown}
              frontmatter={null}
              annotations={annotations}
              onAddAnnotation={handleAddAnnotation}
              onSelectAnnotation={handleSelectAnnotation}
              selectedAnnotationId={selectedAnnotationId}
              mode={mode}
              onModeChange={(newMode) => {
                setMode(newMode);
                saveEditorMode(newMode);
              }}
              globalAttachments={globalImages}
              onAddGlobalAttachment={handleAddGlobalImage}
              onRemoveGlobalAttachment={handleRemoveGlobalImage}
              stickyActions={true}
              linkedDocInfo={null}
              showToc={false}
              onTocNavigate={undefined}
              onOpenLinkedDoc={undefined}
            />
          </div>
        </div>

        {/* Action Bar */}
        <div className="flex items-center justify-between px-4 py-3 border-t border-border bg-card">
          <div className="text-sm text-muted-foreground">
            {annotationCount} {annotationCount === 1 ? 'annotation' : 'annotations'}
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleFeedback}
              className="px-4 py-2 text-sm bg-muted text-foreground rounded-lg hover:bg-muted/80 transition-colors"
            >
              Feedback
            </button>
            <button
              onClick={handleDeny}
              className="px-4 py-2 text-sm bg-destructive/10 text-destructive rounded-lg hover:bg-destructive/20 transition-colors"
            >
              Deny
            </button>
            <button
              onClick={handleApprove}
              className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-lg hover:opacity-90 transition-opacity"
            >
              Approve
            </button>
          </div>
        </div>

        {/* Modals */}
        {showExportModal && (
          <ExportModal
            isOpen={showExportModal}
            onClose={() => setShowExportModal(false)}
            annotationsOutput={annotationsOutput}
            annotationCount={annotationCount}
            markdown={markdown}
            isApiMode={false}
            initialTab="annotations"
          />
        )}

        {showPermissionSetup && (
          <PermissionModeSetup
            isOpen={showPermissionSetup}
            onComplete={() => {
              setShowPermissionSetup(false);
              setShowUIFeaturesSetup(needsUIFeaturesSetup());
            }}
          />
        )}

        {showUIFeaturesSetup && (
          <UIFeaturesSetup
            isOpen={showUIFeaturesSetup}
            onComplete={() => setShowUIFeaturesSetup(false)}
          />
        )}

        {completionResult && (
          <CompletionOverlay
            submitted={completionResult}
            title={completionTitles[completionResult]}
            subtitle={completionSubtitles[completionResult]}
            agentLabel="Claude Code"
          />
        )}
      </div>
    </ThemeProvider>
  );
}
