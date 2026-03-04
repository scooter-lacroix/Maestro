/**
 * TrackLens - Plan Diff Hook
 *
 * Manages version comparison for plan reviews.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import { useState, useCallback } from 'react';

export interface VersionInfo {
  hash: string;
  date: string;
  message: string;
}

export interface UsePlanDiffReturn {
  versions: VersionInfo[];
  selectedVersion: string | null;
  setSelectedVersion: (hash: string | null) => void;
  diffMode: 'side-by-side' | 'unified';
  setDiffMode: (mode: 'side-by-side' | 'unified') => void;
  isLoading: boolean;
}

export function usePlanDiff(): UsePlanDiffReturn {
  const [versions, setVersions] = useState<VersionInfo[]>([]);
  const [selectedVersion, setSelectedVersion] = useState<string | null>(null);
  const [diffMode, setDiffMode] = useState<'side-by-side' | 'unified'>('side-by-side');
  const [isLoading, setIsLoading] = useState(false);

  return {
    versions,
    selectedVersion,
    setSelectedVersion,
    diffMode,
    setDiffMode,
    isLoading,
  };
}
