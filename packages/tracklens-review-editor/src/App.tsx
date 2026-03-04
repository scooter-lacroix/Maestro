/**
 * TrackLens Code Review Editor - Main App
 *
 * React app for git diff visualization and code annotation.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import { useState, useCallback } from "react";
import { DiffViewer } from "./components/DiffViewer";
import { ReviewPanel } from "./components/ReviewPanel";
import { FileTree } from "./components/FileTree";
import {
  Settings,
  ModeToggle,
  CompletionOverlay,
  ResizeHandle,
  useResizablePanel,
  getIdentity,
  ThemeProvider,
  useTheme,
  type CodeAnnotation,
  type CodeAnnotationType,
} from "@maestro/tracklens-ui";
import { DEMO_DIFF, type DiffFile, type DiffData } from "./demoData";

export type { DiffFile, DiffData };

export default function App() {
  const { theme, setTheme } = useTheme();
  const [diffData] = useState(DEMO_DIFF);
  const [selectedFile, setSelectedFile] = useState<DiffFile | null>(null);
  const [annotations, setAnnotations] = useState<CodeAnnotation[]>([]);
  const [completionResult, setCompletionResult] = useState<'approved' | 'denied' | 'feedback' | null>(null);

  const leftPanel = useResizablePanel({ storageKey: 'tracklens-review-left-panel-width', defaultWidth: 280 });

  const handleAddAnnotation = useCallback((
    filePath: string,
    startLine: number,
    endLine: number,
    type: CodeAnnotationType,
    content: string
  ) => {
    const newAnnotation: CodeAnnotation = {
      id: `ann-${Date.now()}`,
      type,
      filePath,
      lineStart: startLine,
      lineEnd: endLine,
      side: 'new',
      text: content,
      createdAt: Date.now(),
      author: getIdentity(),
    };
    setAnnotations(prev => [...prev, newAnnotation]);
  }, []);

  const handleDeleteAnnotation = useCallback((id: string) => {
    setAnnotations(prev => prev.filter(a => a.id !== id));
  }, []);

  const handleExport = () => {
    const data = JSON.stringify(annotations, null, 2);
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'annotations.json';
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleSubmit = () => setCompletionResult('approved');

  return (
    <ThemeProvider>
      <div className="h-screen flex flex-col bg-background text-foreground overflow-hidden">
        {/* Header */}
        <header className="flex items-center justify-between px-4 py-3 border-b border-border bg-card">
          <h1 className="text-lg font-semibold">TrackLens Code Review</h1>
          <div className="flex items-center gap-2">
            <Settings onIdentityChange={() => {}} origin="claude-code" mode="review" />
            <ModeToggle />
          </div>
        </header>

        {/* Main Content */}
        <div className="flex-1 flex overflow-hidden">
          {/* File Tree */}
          <div style={{ width: leftPanel.width }} className="border-r border-border overflow-y-auto">
            <FileTree
              files={diffData.files}
              selectedFile={selectedFile}
              onSelectFile={setSelectedFile}
            />
          </div>

          {/* Resizer */}
          <ResizeHandle {...leftPanel.handleProps} />

          {/* Diff Viewer */}
          <div className="flex-1">
            <DiffViewer
              diffData={diffData}
              selectedFile={selectedFile}
              annotations={annotations}
              onAddAnnotation={handleAddAnnotation}
            />
          </div>

          {/* Review Panel */}
          <div style={{ width: '320px' }} className="border-l border-border">
            <ReviewPanel
              annotations={annotations}
              onDeleteAnnotation={handleDeleteAnnotation}
              onExport={handleExport}
              onSubmit={handleSubmit}
            />
          </div>
        </div>

        {/* Completion Overlay */}
        {completionResult && (
          <CompletionOverlay
            submitted={completionResult}
            title={completionResult === 'approved' ? 'Review Approved' : completionResult === 'denied' ? 'Review Denied' : 'Feedback Sent'}
            subtitle={completionResult === 'approved' ? 'The review has been approved and submitted.' : completionResult === 'denied' ? 'The review has been denied.' : 'Your feedback has been sent.'}
            agentLabel="Maestro"
          />
        )}
      </div>
    </ThemeProvider>
  );
}
