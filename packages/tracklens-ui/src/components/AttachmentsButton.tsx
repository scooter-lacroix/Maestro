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
      const windowHeight = window.innerHeight;
      const spaceBelow = windowHeight - rect.bottom;

      // If less than 350px below, open upwards
      if (spaceBelow < 350) {
        setPosition({
          top: rect.top - 12,
          left: Math.max(16, rect.left - 240)
        });
      } else {
        setPosition({
          top: rect.bottom + 12,
          left: Math.max(16, rect.left - 100)
        });
      }
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

  // Determine if popover should use translate-y because it's opening upwards
  const currentRect = buttonRef.current?.getBoundingClientRect();
  const isOpeningUpwards = currentRect ? (window.innerHeight - currentRect.bottom < 350) : false;

  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        onClick={(e) => {
          console.log('Images button clicked', { isOpen: !isOpen });
          e.preventDefault();
          e.stopPropagation();
          setIsOpen(!isOpen);
        }}
        className="group relative flex items-center gap-2 px-4 py-2.5 rounded-full bg-background shadow-neu-extruded hover:-translate-y-0.5 hover:shadow-neu-hover active:translate-y-[0.5px] active:shadow-neu-inset transition-all z-[100] cursor-pointer"
        title="Image Attachments"
      >
        <div className="relative flex items-center justify-center pointer-events-none">
          <svg className="w-5 h-5 text-muted-foreground group-hover:text-primary transition-colors" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 001.5-1.5V6a1.5 1.5 0 00-1.5-1.5H3.75A1.5 1.5 0 002.25 6v12a1.5 1.5 0 001.5 1.5zm10.5-11.25h.008v.008h-.008V8.25zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" />
          </svg>
          {images.length > 0 && (
            <span className="absolute -top-1.5 -right-1.5 flex h-4 w-4 items-center justify-center rounded-full bg-primary text-[8px] font-bold text-primary-foreground shadow-sm">
              {images.length}
            </span>
          )}
        </div>
        <span className="text-sm font-semibold text-muted-foreground group-hover:text-foreground transition-colors pointer-events-none">Images</span>
      </button>

      {isOpen && (
        <div className="fixed inset-0 z-[70] bg-black/10 backdrop-blur-[1px]" onClick={() => setIsOpen(false)} />
      )}
      {isOpen && (
        <div
          className={`fixed z-[100] w-80 bg-surface-glass shadow-neu-extruded rounded-[32px] p-6 lg:p-8 animate-in fade-in zoom-in duration-200 ${isOpeningUpwards ? 'origin-bottom -translate-y-full mb-2' : 'origin-top'}`}
          style={{
            top: position.top,
            left: position.left
          }}
          onClick={(e) => e.stopPropagation()}
        >
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
              className={`flex flex-col items-center justify-center gap-2 px-3 py-6 rounded-2xl cursor-pointer transition-all duration-300 ${dragOver ? 'bg-primary/5 shadow-neu-inset-deep' : 'bg-background shadow-neu-inset hover:shadow-neu-inset-deep border-none'
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
                className="flex-1 px-4 py-2 text-xs bg-background shadow-neu-inset rounded-xl focus:outline-none focus:ring-2 focus:ring-primary/50"
              />
              <button type="button" onClick={handleManualAdd} disabled={!manualPath.trim()} className="px-4 py-2 text-xs font-medium bg-background text-foreground rounded-xl shadow-neu-extruded hover:-translate-y-px hover:shadow-neu-hover active:translate-y-[0.5px] active:shadow-neu-inset transition-all disabled:opacity-50 disabled:-translate-y-0 disabled:shadow-neu-extruded">
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
