/**
 * TrackLens UI - Image Thumbnail Component
 *
 * Displays thumbnail for image attachments in annotations.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import React from 'react';

interface ImageThumbnailProps {
  path: string;
  onRemove: () => void;
}

export const ImageThumbnail: React.FC<ImageThumbnailProps> = ({ path, onRemove }) => {
  const [error, setError] = React.useState(false);

  return (
    <div className="relative inline-flex items-center gap-2 px-2 py-1 bg-muted/50 rounded border border-border/30">
      {!error ? (
        <img
          src={path}
          alt="Attachment"
          className="w-8 h-8 object-cover rounded"
          onError={() => setError(true)}
        />
      ) : (
        <div className="w-8 h-8 bg-muted rounded flex items-center justify-center text-[10px] text-muted-foreground">
          IMG
        </div>
      )}
      <span className="text-[10px] text-muted-foreground truncate max-w-[120px]" title={path}>
        {path.split('/').pop()}
      </span>
      <button
        onClick={onRemove}
        className="p-0.5 rounded hover:bg-muted text-muted-foreground hover:text-destructive transition-colors"
        title="Remove"
      >
        <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
  );
};
