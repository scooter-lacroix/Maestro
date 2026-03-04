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

type Tab = 'annotations' | 'notes';
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
  const defaultTab = initialTab || 'annotations';
  const [activeTab, setActiveTab] = useState<Tab>(defaultTab);
  const [copied, setCopied] = useState(false);
  const [saveStatus, setSaveStatus] = useState<Record<SaveTarget, SaveStatus>>({ obsidian: 'idle', bear: 'idle' });
  const [saveErrors, setSaveErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (isOpen) {
      setActiveTab(initialTab || 'annotations');
      setSaveStatus({ obsidian: 'idle', bear: 'idle' });
      setSaveErrors({});
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
    if (!markdown) return;

    setSaveStatus(prev => ({ ...prev, [target]: 'saving' }));
    setSaveErrors(prev => { const next = { ...prev }; delete next[target]; return next; });

    try {
      let result: { success?: boolean; error?: string } | undefined;
      if (target === 'obsidian') {
        const res = await fetch('/api/obsidian', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            vaultPath: effectiveVaultPath,
            folder: obsidianSettings.folder || 'tracklens',
            filenameFormat: obsidianSettings.filenameFormat,
          }),
        });
        result = await res.json();
      } else if (target === 'bear') {
        const res = await fetch('/api/bear', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({}),
        });
        result = await res.json();
      }

      if (result?.success) {
        setSaveStatus(prev => ({ ...prev, [target]: 'success' }));
      } else {
        setSaveStatus(prev => ({ ...prev, [target]: 'error' }));
        setSaveErrors(prev => ({ ...prev, [target]: result?.error || 'Save failed' }));
      }
    } catch {
      setSaveStatus(prev => ({ ...prev, [target]: 'error' }));
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm p-4">
      <div className="bg-card border border-border rounded-xl w-full max-w-2xl shadow-2xl flex flex-col max-h-[80vh]">
        <div className="p-4 border-b border-border flex items-center justify-between">
          <h3 className="font-semibold text-sm">Export</h3>
          <button onClick={onClose} className="p-1.5 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground">
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="flex border-b border-border">
          <button onClick={() => setActiveTab('annotations')} className={`px-4 py-2 text-sm font-medium transition-colors ${activeTab === 'annotations' ? 'text-foreground border-b-2 border-primary' : 'text-muted-foreground hover:text-foreground'}`}>
            Annotations ({annotationCount})
          </button>
          {showNotesTab && (
            <button onClick={() => setActiveTab('notes')} className={`px-4 py-2 text-sm font-medium transition-colors ${activeTab === 'notes' ? 'text-foreground border-b-2 border-primary' : 'text-muted-foreground hover:text-foreground'}`}>
              Save to Notes
            </button>
          )}
        </div>

        <div className="p-4 overflow-y-auto flex-1">
          {activeTab === 'annotations' && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <p className="text-xs text-muted-foreground">
                  Copy annotations to clipboard or download as markdown
                </p>
                <div className="flex gap-2">
                  <button onClick={handleCopy} className="px-3 py-1.5 text-xs font-medium bg-primary text-primary-foreground rounded-md hover:opacity-90">
                    {copied ? 'Copied!' : 'Copy'}
                  </button>
                  <button onClick={handleDownload} className="px-3 py-1.5 text-xs font-medium bg-muted hover:bg-muted/80 rounded-md">
                    Download
                  </button>
                </div>
              </div>
              <pre className="bg-muted/50 p-3 rounded-lg text-xs overflow-x-auto max-h-60 overflow-y-auto">
                {annotationsOutput}
              </pre>
            </div>
          )}

          {activeTab === 'notes' && showNotesTab && (
            <div className="space-y-4">
              {isObsidianReady && (
                <div className="space-y-2">
                  <button
                    onClick={() => handleSaveToNotes('obsidian')}
                    disabled={saveStatus.obsidian === 'saving'}
                    className={`w-full px-3 py-2 text-xs font-medium rounded-md border border-border transition-colors ${
                      saveStatus.obsidian === 'saving' ? 'opacity-50' : 'hover:bg-muted'
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
                <div className="space-y-2">
                  <button
                    onClick={() => handleSaveToNotes('bear')}
                    disabled={saveStatus.bear === 'saving'}
                    className={`w-full px-3 py-2 text-xs font-medium rounded-md border border-border transition-colors ${
                      saveStatus.bear === 'saving' ? 'opacity-50' : 'hover:bg-muted'
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

        <div className="p-4 border-t border-border flex justify-end">
          <button onClick={onClose} className="px-3 py-1.5 text-xs font-medium bg-muted hover:bg-muted/80 rounded-md">
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
