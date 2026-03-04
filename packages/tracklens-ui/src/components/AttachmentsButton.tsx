/**
 * TrackLens UI - Attachments Button Component
 *
 * Button and popover for managing image attachments.
 * Simplified version without annotator.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import React, { useState, useRef, useEffect } from 'react';
import type { ImageAttachment } from '../types';

export function deriveImageName(originalName: string, existingNames: string[]): string {
  const base = originalName.replace(/\.[^.]+$/, '');
  const generic = ['annotated', 'image', 'screenshot', 'paste', 'clipboard', 'untitled'];

  if (generic.includes(base.toLowerCase())) {
    let n = 1;
    while (existingNames.includes(`image-${n}`)) n++;
    return `image-${n}`;
  }

  let name = base.toLowerCase()
    .replace(/[_\s]+/g, '-')
    .replace(/[^a-z0-9-]/g, '')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');

  if (!name) {
    let n = 1;
    while (existingNames.includes(`image-${n}`)) n++;
    return `image-${n}`;
  }

  if (existingNames.includes(name)) {
    let n = 2;
    while (existingNames.includes(`${name}-${n}`)) n++;
    name = `${name}-${n}`;
  }

  return name;
}

interface AttachmentsButtonProps {
  images: ImageAttachment[];
  onAdd: (image: ImageAttachment) => void;
  onRemove: (path: string) => void;
  variant?: 'toolbar' | 'inline';
}

export const AttachmentsButton: React.FC<AttachmentsButtonProps> = ({
  images,
  onAdd,
  onRemove,
  variant = 'toolbar',
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [manualPath, setManualPath] = useState('');
  const [uploading, setUploading] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [position, setPosition] = useState({ top: 0, left: 0 });

  useEffect(() => {
    if (isOpen && buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect();
      setPosition({ top: rect.bottom + 8, left: Math.max(8, rect.left - 100) });
    }
  }, [isOpen]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) setIsOpen(false);
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen]);

  const handleFileSelect = async (file: File) => {
    setUploading(true);
    try {
      const formData = new FormData();
      formData.append('image', file);
      const res = await fetch('/api/upload-image', { method: 'POST', body: formData });
      const data = await res.json();
      if (data.url) {
        const name = deriveImageName(file.name, images.map(i => i.name));
        onAdd({ path: data.url, name });
      }
    } catch (err) {
      console.error('Upload failed:', err);
    } finally {
      setUploading(false);
    }
  };

  const handleFileInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      handleFileSelect(file);
    }
    e.target.value = '';
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const file = e.dataTransfer.files[0];
    if (file && file.type.startsWith('image/')) {
      handleFileSelect(file);
    }
  };

  const handleManualAdd = () => {
    const trimmed = manualPath.trim();
    if (trimmed) {
      const name = deriveImageName(trimmed.split('/').pop() || 'image', images.map(i => i.name));
      onAdd({ path: trimmed, name });
      setManualPath('');
    }
  };

  const handleClearAll = (e: React.MouseEvent) => {
    e.stopPropagation();
    images.forEach(img => onRemove(img.path));
  };

  function getImageSrc(path: string): string {
    return path.startsWith('/') ? path : `/${path}`;
  }

  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="group relative flex items-center gap-1.5 px-2 py-1.5 rounded-md text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
      >
        {images.length > 0 ? (
          <>
            <div className="relative flex items-center">
              {images.slice(0, 3).map((img, idx) => (
                <div key={img.path} className="relative w-5 h-5 rounded border border-background" style={{ marginLeft: idx > 0 ? '-6px' : 0, zIndex: 3 - idx }}>
                  <img src={getImageSrc(img.path)} alt={img.name} loading="lazy" className="w-5 h-5 rounded object-cover" />
                </div>
              ))}
              {images.length > 3 && (
                <div className="relative w-5 h-5 rounded bg-muted border border-background flex items-center justify-center text-[9px] font-medium" style={{ marginLeft: '-6px', zIndex: 0 }}>
                  +{images.length - 3}
                </div>
              )}
            </div>
            <button onClick={handleClearAll} className="absolute -top-1 -right-1 w-3.5 h-3.5 bg-destructive text-destructive-foreground rounded-full opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
              <svg className="w-2 h-2" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
            </button>
          </>
        ) : (
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 001.5-1.5V6a1.5 1.5 0 00-1.5-1.5H3.75A1.5 1.5 0 002.25 6v12a1.5 1.5 0 001.5 1.5zm10.5-11.25h.008v.008h-.008V8.25zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" />
          </svg>
        )}
        <span className={variant === 'inline' ? 'sr-only' : ''}>{images.length > 0 ? `${images.length}` : 'Images'}</span>
      </button>

      {isOpen && (
        <div className="fixed inset-0 z-50" onClick={() => setIsOpen(false)} />
      )}
      {isOpen && (
        <div className="fixed z-[100] w-72 bg-card border border-border rounded-xl shadow-2xl p-3" style={{ top: position.top, left: position.left }} onClick={(e) => e.stopPropagation()}>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="text-sm font-medium">Attachments</div>
              {images.length > 0 && <span className="text-[10px] text-muted-foreground">{images.length} image{images.length !== 1 ? 's' : ''}</span>}
            </div>
            <p className="text-[11px] text-muted-foreground -mt-1">Add images to include with your feedback</p>

            <div
              onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
              onDragLeave={() => setDragOver(false)}
              onDrop={handleDrop}
              onClick={() => fileInputRef.current?.click()}
              className={`flex flex-col items-center justify-center gap-2 px-3 py-4 border-2 border-dashed rounded-lg cursor-pointer transition-colors ${
                dragOver ? 'border-primary bg-primary/5' : 'border-border hover:border-muted-foreground'
              }`}
            >
              {uploading ? (
                <div className="flex items-center gap-2 text-muted-foreground">
                  <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth={4} />
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                  </svg>
                  <span className="text-xs">Uploading...</span>
                </div>
              ) : (
                <>
                  <svg className="w-6 h-6 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
                    <path strokeLinecap="round" strokeLinejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5" />
                  </svg>
                  <span className="text-xs text-muted-foreground">Drop image or click to browse</span>
                </>
              )}
            </div>
            <input ref={fileInputRef} type="file" accept="image/*" onChange={handleFileInputChange} className="hidden" />

            <div className="flex gap-2">
              <input
                type="text"
                value={manualPath}
                onChange={(e) => setManualPath(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && !e.nativeEvent.isComposing && handleManualAdd()}
                placeholder="Paste path or URL..."
                className="flex-1 px-2 py-1.5 text-xs bg-background border border-border rounded-md focus:outline-none focus:ring-1 focus:ring-primary"
              />
              <button type="button" onClick={handleManualAdd} disabled={!manualPath.trim()} className="px-2 py-1.5 text-xs font-medium bg-primary text-primary-foreground rounded-md hover:opacity-90 disabled:opacity-50">
                Add
              </button>
            </div>

            {images.length > 0 && (
              <div className="space-y-2">
                <div className="text-xs text-muted-foreground">Current</div>
                <div className="grid grid-cols-4 gap-2">
                  {images.map((img) => (
                    <div key={img.path} className="text-center">
                      <div className="relative w-12 h-12 mx-auto">
                        <img src={getImageSrc(img.path)} alt={img.name} className="w-12 h-12 rounded object-cover" />
                        <button
                          onClick={() => onRemove(img.path)}
                          className="absolute top-0 right-0 w-4 h-4 bg-destructive text-destructive-foreground rounded-full opacity-0 group-hover:opacity-100 flex items-center justify-center"
                          title="Remove"
                        >
                          <svg className="w-2.5 h-2.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}><path d="M6 18L18 6M6 6l12 12" /></svg>
                        </button>
                      </div>
                      <div className="text-[9px] text-muted-foreground truncate mt-0.5" title={img.name}>{img.name}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
}
