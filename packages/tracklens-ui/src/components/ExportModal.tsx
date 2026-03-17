/**
 * TrackLens UI - Export Modal Component
 *
 * Export modal with Annotations and Notes tabs.
 * Removed: Share tab, TaterSprite.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import React, { useState, useEffect } from 'react';
import { getObsidianSettings, getEffectiveVaultPath } from '../utils/obsidian';
import { getBearSettings } from '../utils/bear';

interface ExportModalProps {
  isOpen: boolean;
  onClose: () => void;
  annotationsOutput: string;
  annotationCount: number;
  markdown?: string;
  isApiMode?: boolean;
  initialTab?: Tab;
}

export type Tab = 'export' | 'settings';
type ExportSource = 'document' | 'annotations' | 'both';
type SaveTarget = 'obsidian' | 'bear';
type SaveStatus = 'idle' | 'saving' | 'success' | 'error';

export const ExportModal: React.FC<ExportModalProps> = ({
  isOpen,
  onClose,
  annotationsOutput,
  annotationCount,
  markdown,
  isApiMode = false,
  initialTab,
}) => {
  const [activeTab, setActiveTab] = useState<Tab>('export');
  const [exportSource, setExportSource] = useState<ExportSource>('document');
  const [copied, setCopied] = useState(false);
  const [saveStatus, setSaveStatus] = useState<Record<SaveTarget, SaveStatus>>({ obsidian: 'idle', bear: 'idle' });
  const [saveErrors, setSaveErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (isOpen) {
      setSaveStatus({ obsidian: 'idle', bear: 'idle' });
      setSaveErrors({});
      if (initialTab) {
        setActiveTab(initialTab);
      }
    }
  }, [isOpen, initialTab]);

  if (!isOpen) return null;

  const showNotesTab = isApiMode && !!markdown;
  const obsidianSettings = getObsidianSettings();
  const bearSettings = getBearSettings();
  const effectiveVaultPath = getEffectiveVaultPath(obsidianSettings);
  const isObsidianReady = obsidianSettings.enabled && effectiveVaultPath.trim().length > 0;
  const isBearReady = bearSettings.enabled;

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(annotationsOutput);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      console.error('Failed to copy:', e);
    }
  };

  const handleDownload = () => {
    const blob = new Blob([annotationsOutput], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'annotations.md';
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleSaveToNotes = async (target: SaveTarget) => {
    let contentToExport = "";
    if (exportSource === 'document') {
      contentToExport = markdown || "";
    } else if (exportSource === 'annotations') {
      contentToExport = annotationsOutput;
    } else {
      contentToExport = `# Document\n\n${markdown || ""}\n\n---\n\n# Feedback\n\n${annotationsOutput}`;
    }

    if (!contentToExport) return;

    setSaveStatus(prev => ({ ...prev, [target]: 'saving' }));
    setSaveErrors(prev => { const next = { ...prev }; delete next[target]; return next; });

    try {
      type ApiResponse = { success: boolean; error?: string };
      let result: ApiResponse | undefined;

      if (target === 'obsidian') {
        const res = await fetch('/api/obsidian', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            content: contentToExport,
            vaultPath: effectiveVaultPath,
            folder: obsidianSettings.folder || 'tracklens',
            filenameFormat: obsidianSettings.filenameFormat,
          }),
        });
        result = await res.json() as ApiResponse;
      } else if (target === 'bear') {
        const res = await fetch('/api/bear', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            content: contentToExport,
          }),
        });
        result = await res.json() as ApiResponse;
      }

      if (result?.success) {
        setSaveStatus(prev => ({ ...prev, [target]: 'success' }));
      } else {
        setSaveStatus(prev => ({ ...prev, [target]: 'error' }));
        setSaveErrors(prev => ({ ...prev, [target]: result?.error || 'Save failed' }));
      }
    } catch (error: unknown) {
      setSaveStatus(prev => ({ ...prev, [target]: 'error' }));
      console.error('Save failed:', error);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-xl p-4">
      <div className="bg-background rounded-[32px] w-full max-w-2xl shadow-neu-extruded flex flex-col max-h-[80vh] border border-border/10 overflow-hidden">
        <div className="p-6 border-b border-border/50 flex flex-col gap-6">
          <div className="flex items-center justify-between">
            <h3 className="font-bold font-display text-lg">Export</h3>
            <button onClick={onClose} className="p-2 rounded-xl bg-background shadow-neu-extruded-small text-muted-foreground hover:text-foreground active:shadow-neu-inset transition-all">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <div className="flex gap-4">
            <button onClick={() => setActiveTab('export')} className={`px-4 py-2 text-sm font-medium rounded-xl transition-all ${activeTab === 'export' ? 'bg-background shadow-neu-inset text-primary' : 'text-muted-foreground hover:text-foreground'}`}>
              Export
            </button>
            <button onClick={() => setActiveTab('settings')} className={`px-4 py-2 text-sm font-medium rounded-xl transition-all ${activeTab === 'settings' ? 'bg-background shadow-neu-inset text-primary' : 'text-muted-foreground hover:text-foreground'}`}>
              Destinations
            </button>
          </div>
        </div>

        <div className="p-6 overflow-y-auto flex-1">
          {activeTab === 'export' && (
            <div className="space-y-8">
              {/* Content Selection */}
              <div className="space-y-3">
                <label className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Select Content</label>
                <div className="grid grid-cols-3 gap-3">
                  <button
                    onClick={() => setExportSource('document')}
                    className={`p-3 rounded-2xl border transition-all text-center flex flex-col items-center gap-2 ${exportSource === 'document' ? 'bg-primary/5 border-primary shadow-neu-inset text-primary' : 'bg-background border-border/10 shadow-neu-extruded hover:shadow-neu-hover text-muted-foreground'}`}
                  >
                    <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                      <path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                    </svg>
                    <span className="text-xs font-medium">Active Doc</span>
                  </button>
                  <button
                    onClick={() => setExportSource('annotations')}
                    className={`p-3 rounded-2xl border transition-all text-center flex flex-col items-center gap-2 ${exportSource === 'annotations' ? 'bg-primary/5 border-primary shadow-neu-inset text-primary' : 'bg-background border-border/10 shadow-neu-extruded hover:shadow-neu-hover text-muted-foreground'}`}
                  >
                    <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                      <path strokeLinecap="round" strokeLinejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
                    </svg>
                    <span className="text-xs font-medium">Feedback ({annotationCount})</span>
                  </button>
                  <button
                    onClick={() => setExportSource('both')}
                    className={`p-3 rounded-2xl border transition-all text-center flex flex-col items-center gap-2 ${exportSource === 'both' ? 'bg-primary/5 border-primary shadow-neu-inset text-primary' : 'bg-background border-border/10 shadow-neu-extruded hover:shadow-neu-hover text-muted-foreground'}`}
                  >
                    <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                      <path strokeLinecap="round" strokeLinejoin="round" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                    </svg>
                    <span className="text-xs font-medium">Merge Both</span>
                  </button>
                </div>
              </div>

              {/* Destination Actions */}
              <div className="space-y-4">
                <label className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Choose Destination</label>
                <div className="grid grid-cols-2 gap-4">
                  <button
                    onClick={() => handleSaveToNotes('obsidian')}
                    disabled={!isObsidianReady || saveStatus.obsidian === 'saving'}
                    className={`p-4 rounded-[24px] shadow-neu-extruded hover:shadow-neu-hover active:shadow-neu-inset transition-all flex flex-col items-center gap-3 border border-border/5 ${!isObsidianReady ? 'opacity-40 grayscale pointer-events-none' : ''}`}
                  >
                    <div className="w-12 h-12 rounded-full bg-background shadow-neu-inset flex items-center justify-center">
                      <svg className="w-6 h-6 text-primary" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M9.75 3L2 12l7.75 9L11.5 13.5 9.75 3zM14.25 3l7.75 9-7.75 9-1.75-7.5L14.25 3z" />
                      </svg>
                    </div>
                    <div>
                      <div className="text-sm font-bold">Obsidian</div>
                      <div className="text-[10px] text-muted-foreground uppercase tracking-widest">{saveStatus.obsidian === 'idle' ? 'Vault Sync' : saveStatus.obsidian}</div>
                    </div>
                  </button>

                  <button
                    onClick={() => handleSaveToNotes('bear')}
                    disabled={!isBearReady || saveStatus.bear === 'saving'}
                    className={`p-4 rounded-[24px] shadow-neu-extruded hover:shadow-neu-hover active:shadow-neu-inset transition-all flex flex-col items-center gap-3 border border-border/5 ${!isBearReady ? 'opacity-40 grayscale pointer-events-none' : ''}`}
                  >
                    <div className="w-12 h-12 rounded-full bg-background shadow-neu-inset flex items-center justify-center">
                      <svg className="w-6 h-6 text-red-500" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M12 2C6.47 2 2 6.47 2 12s4.47 10 10 10 10-4.47 10-10S17.53 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z" />
                      </svg>
                    </div>
                    <div>
                      <div className="text-sm font-bold">Bear Notes</div>
                      <div className="text-[10px] text-muted-foreground uppercase tracking-widest">{saveStatus.bear === 'idle' ? 'X-Callback' : saveStatus.bear}</div>
                    </div>
                  </button>
                </div>
              </div>

              {/* Quick Actions */}
              <div className="pt-4 border-t border-border/20 flex items-center justify-between">
                <span className="text-xs text-muted-foreground font-medium">Raw Output</span>
                <div className="flex gap-2">
                  <button onClick={handleCopy} className="px-4 py-2 text-xs font-bold rounded-xl shadow-neu-extruded hover:shadow-neu-hover active:shadow-neu-inset transition-all">
                    {copied ? 'Copied!' : 'Copy to Clipboard'}
                  </button>
                  <button onClick={handleDownload} className="px-4 py-2 text-xs font-bold rounded-xl shadow-neu-extruded hover:shadow-neu-hover active:shadow-neu-inset transition-all">
                    Download .md
                  </button>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'settings' && (
            <div className="space-y-4">
              {isObsidianReady && (
                <div className="space-y-3">
                  <button
                    onClick={() => handleSaveToNotes('obsidian')}
                    disabled={saveStatus.obsidian === 'saving'}
                    className={`w-full px-5 py-3 text-sm font-medium rounded-2xl shadow-neu-extruded hover:-translate-y-px hover:shadow-neu-hover active:translate-y-[0.5px] active:shadow-neu-inset transition-all duration-300 ${saveStatus.obsidian === 'saving' ? 'opacity-50' : ''
                      }`}
                  >
                    {saveStatus.obsidian === 'saving' ? 'Saving to Obsidian...' :
                      saveStatus.obsidian === 'success' ? '✓ Saved to Obsidian' :
                        saveStatus.obsidian === 'error' ? '✗ Save failed' :
                          'Save to Obsidian'}
                  </button>
                  {saveErrors.obsidian && <p className="text-[10px] text-destructive">{saveErrors.obsidian}</p>}
                </div>
              )}

              {isBearReady && (
                <div className="space-y-3">
                  <button
                    onClick={() => handleSaveToNotes('bear')}
                    disabled={saveStatus.bear === 'saving'}
                    className={`w-full px-5 py-3 text-sm font-medium rounded-2xl shadow-neu-extruded hover:-translate-y-px hover:shadow-neu-hover active:translate-y-[0.5px] active:shadow-neu-inset transition-all duration-300 ${saveStatus.bear === 'saving' ? 'opacity-50' : ''
                      }`}
                  >
                    {saveStatus.bear === 'saving' ? 'Saving to Bear...' :
                      saveStatus.bear === 'success' ? '✓ Saved to Bear' :
                        saveStatus.bear === 'error' ? '✗ Save failed' :
                          'Save to Bear'}
                  </button>
                  {saveErrors.bear && <p className="text-[10px] text-destructive">{saveErrors.bear}</p>}
                </div>
              )}

              {!isObsidianReady && !isBearReady && (
                <p className="text-xs text-muted-foreground">
                  Configure Obsidian or Bear in Settings to save notes directly.
                </p>
              )}
            </div>
          )}
        </div>

        <div className="p-6 flex justify-end">
          <button onClick={onClose} className="px-5 py-2.5 text-sm font-medium bg-background text-foreground rounded-xl shadow-neu-extruded hover:-translate-y-px hover:shadow-neu-hover active:translate-y-[0.5px] active:shadow-neu-inset transition-all duration-300">
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
