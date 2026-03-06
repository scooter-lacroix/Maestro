/**
 * TrackLens UI - Resize Handle Component
 *
 * Draggable handle for resizing panels.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import React from 'react';
import type { ResizeHandleProps as BaseProps } from '../hooks/useResizablePanel';

interface Props extends BaseProps {
  className?: string;
}

export const ResizeHandle: React.FC<Props> = ({
  isDragging,
  onMouseDown,
  onDoubleClick,
  className,
}) => (
  <div
    onMouseDown={onMouseDown}
    onDoubleClick={onDoubleClick}
    className={`w-1 cursor-col-resize flex-shrink-0 transition-colors ${
      isDragging ? 'bg-primary/50' : 'hover:bg-border'
    }${className ? ` ${className}` : ''}`}
  />
);
