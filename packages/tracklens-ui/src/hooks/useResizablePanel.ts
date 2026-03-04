/**
 * TrackLens - Resizable Panel Hook
 *
 * Manages resizable panel width with persistence.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import { useState, useRef, useCallback, useEffect } from 'react';
import { storage } from '../utils/storage';

interface UseResizablePanelOptions {
  storageKey: string;
  defaultWidth?: number;
  minWidth?: number;
  maxWidth?: number;
  side?: 'left' | 'right';
}

export interface ResizeHandleProps {
  isDragging: boolean;
  onMouseDown: (e: React.MouseEvent) => void;
  onDoubleClick: () => void;
}

export function useResizablePanel({
  storageKey,
  defaultWidth = 288,
  minWidth = 200,
  maxWidth = 600,
  side = 'right',
}: UseResizablePanelOptions) {
  const [width, setWidth] = useState(() => {
    const saved = storage.getItem(storageKey);
    if (saved) {
      const n = Number(saved);
      if (!Number.isNaN(n) && n >= minWidth && n <= maxWidth) return n;
    }
    return defaultWidth;
  });

  const [isDragging, setIsDragging] = useState(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);
  const widthRef = useRef(width);

  const updateWidth = useCallback((value: number) => {
    widthRef.current = value;
    setWidth(value);
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    startXRef.current = e.clientX;
    startWidthRef.current = widthRef.current;
    setIsDragging(true);
  }, []);

  useEffect(() => {
    if (!isDragging) return;

    const onMove = (e: MouseEvent) => {
      const delta = side === 'right'
        ? startXRef.current - e.clientX
        : e.clientX - startXRef.current;
      updateWidth(Math.min(maxWidth, Math.max(minWidth, startWidthRef.current + delta)));
    };

    const onUp = () => {
      setIsDragging(false);
      storage.setItem(storageKey, String(widthRef.current));
    };

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
    return () => {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
    };
  }, [isDragging, minWidth, maxWidth, storageKey, side, updateWidth]);

  const resetWidth = useCallback(() => {
    updateWidth(defaultWidth);
    storage.setItem(storageKey, String(defaultWidth));
  }, [defaultWidth, storageKey, updateWidth]);

  return {
    width,
    isDragging,
    handleProps: { isDragging, onMouseDown: handleMouseDown, onDoubleClick: resetWidth } as ResizeHandleProps,
  };
}
