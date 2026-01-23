import { useState, useEffect } from 'react';
import { apiClient } from '../utils/api';
import { Memory, Project, Track, StatsResponse, CodeSearchResult, FileClaim, Handoff, ContinuityLedger, CoordinationSummary } from '../types';

export const useMemories = (params?: { project_id?: number; track_id?: number; limit?: number }) => {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchMemories = async () => {
      try {
        setLoading(true);
        const response = await apiClient.listMemories(params);
        setMemories(response.memories);
        setError(null);
      } catch (err) {
        console.error('Error fetching memories:', err);
        setError('Failed to fetch memories');
        setMemories([]);
      } finally {
        setLoading(false);
      }
    };

    fetchMemories();
  }, [JSON.stringify(params)]);

  return { memories, loading, error, refetch: () => { } };
};

export const useProjects = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchProjects = async () => {
      try {
        setLoading(true);
        const response = await apiClient.listProjects();
        setProjects(response.projects);
        setError(null);
      } catch (err) {
        console.error('Error fetching projects:', err);
        setError('Failed to fetch projects');
        setProjects([]);
      } finally {
        setLoading(false);
      }
    };

    fetchProjects();
  }, []);

  return { projects, loading, error };
};

export const useTracks = (projectId?: number) => {
  const [tracks, setTracks] = useState<Track[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchTracks = async () => {
      try {
        setLoading(true);
        console.log(`[useTracks] Fetching tracks for projectId: ${projectId}`);
        const response = await apiClient.listTracks(projectId);
        console.log(`[useTracks] Response:`, response);

        if (response.success && Array.isArray(response.tracks)) {
          setTracks(response.tracks);
          setError(null);
          console.log(`[useTracks] Loaded ${response.tracks.length} tracks`);
        } else {
          console.warn('[useTracks] Invalid response format:', response);
          setTracks([]);
          setError('Invalid response format');
        }
      } catch (err) {
        console.error('[useTracks] Error fetching tracks:', err);
        setError('Failed to fetch tracks');
        setTracks([]);
      } finally {
        setLoading(false);
      }
    };

    fetchTracks();
  }, [projectId]);

  return { tracks, loading, error };
};

export const useStats = () => {
  const [stats, setStats] = useState<StatsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchStats = async () => {
      try {
        setLoading(true);
        const response = await apiClient.getStats();
        setStats(response);
        setError(null);
      } catch (err) {
        console.error('Error fetching stats:', err);
        setError('Failed to fetch statistics');
        setStats(null);
      } finally {
        setLoading(false);
      }
    };

    fetchStats();
  }, []);

  return { stats, loading, error };
};

export const useSearch = () => {
  const [results, setResults] = useState<Memory[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const search = async (query: string, projectPath?: string) => {
    if (!query.trim()) {
      setResults([]);
      return;
    }

    try {
      setLoading(true);
      const response = await apiClient.searchMemories(query, projectPath, 10);
      setResults(response.results);
      setError(null);
    } catch (err) {
      console.error('Error searching memories:', err);
      setError('Failed to search memories');
      setResults([]);
    } finally {
      setLoading(false);
    }
  };

  return { results, loading, error, search };
};

export interface ScanResult {
  success: boolean;
  projects_found: number;
  tracks_found: number;
  projects: Array<{ path: string; name: string; type: string; id: number }>;
  tracks: Array<{ track_id: string; title: string; project_id: number }>;
  errors: string[];
}

export const useScan = () => {
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ScanResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const scan = async (baseDirs?: string[]) => {
    try {
      setLoading(true);
      setError(null);
      const response = await fetch('/api/v1/scan', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ base_dirs: baseDirs })
      });
      if (!response.ok) {
        throw new Error(`Scan failed: ${response.statusText}`);
      }
      const data: ScanResult = await response.json();
      setResult(data);
      return data;
    } catch (err) {
      console.error('Error scanning projects:', err);
      setError(err instanceof Error ? err.message : 'Scan failed');
      return null;
    } finally {
      setLoading(false);
    }
  };

  return { scan, loading, result, error };
};

export const useCodeSearch = () => {
  const [results, setResults] = useState<CodeSearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const searchCode = async (query: string, options?: {
    file_patterns?: string[];
    max_results?: number;
    context_lines?: number;
  }) => {
    if (!query.trim()) {
      setResults([]);
      return;
    }

    try {
      setLoading(true);
      const response = await apiClient.searchCode(query, options);
      setResults(response.results);
      setError(null);
    } catch (err) {
      console.error('Error searching code:', err);
      setError('Failed to search code');
      setResults([]);
    } finally {
      setLoading(false);
    }
  };

  return { results, loading, error, searchCode };
};

// Coordination hooks for Maestro v2
export const useCoordinationSummary = () => {
  const [summary, setSummary] = useState<CoordinationSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchSummary = async () => {
      try {
        setLoading(true);
        const response = await fetch('/api/v1/coordination/summary');
        const data = await response.json();
        setSummary(data);
        setError(null);
      } catch (err) {
        console.error('Error fetching coordination summary:', err);
        setError('Failed to fetch coordination summary');
        setSummary(null);
      } finally {
        setLoading(false);
      }
    };

    fetchSummary();
  }, []);

  return { summary, loading, error };
};

export const useFileClaims = (params?: {
  project_id?: number;
  track_id?: number;
  status?: string;
  limit?: number;
}) => {
  const [claims, setClaims] = useState<FileClaim[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchClaims = async () => {
      try {
        setLoading(true);
        const queryParams = new URLSearchParams();
        if (params?.project_id) queryParams.append('project_id', params.project_id.toString());
        if (params?.track_id) queryParams.append('track_id', params.track_id.toString());
        if (params?.status) queryParams.append('status', params.status);
        if (params?.limit) queryParams.append('limit', params.limit.toString());

        const response = await fetch(`/api/v1/coordination/file-claims?${queryParams}`);
        const data = await response.json();
        setClaims(data.claims || []);
        setError(null);
      } catch (err) {
        console.error('Error fetching file claims:', err);
        setError('Failed to fetch file claims');
        setClaims([]);
      } finally {
        setLoading(false);
      }
    };

    fetchClaims();
  }, [JSON.stringify(params)]);

  return { claims, loading, error };
};

export const useHandoffs = (params?: {
  project_id?: number;
  track_id?: number;
  status?: string;
  limit?: number;
}) => {
  const [handoffs, setHandoffs] = useState<Handoff[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchHandoffs = async () => {
      try {
        setLoading(true);
        const queryParams = new URLSearchParams();
        if (params?.project_id) queryParams.append('project_id', params.project_id.toString());
        if (params?.track_id) queryParams.append('track_id', params.track_id.toString());
        if (params?.status) queryParams.append('status', params.status);
        if (params?.limit) queryParams.append('limit', params.limit.toString());

        const response = await fetch(`/api/v1/coordination/handoffs?${queryParams}`);
        const data = await response.json();
        setHandoffs(data.handoffs || []);
        setError(null);
      } catch (err) {
        console.error('Error fetching handoffs:', err);
        setError('Failed to fetch handoffs');
        setHandoffs([]);
      } finally {
        setLoading(false);
      }
    };

    fetchHandoffs();
  }, [JSON.stringify(params)]);

  return { handoffs, loading, error };
};

export const useContinuityLedgers = (params?: {
  project_id?: number;
  track_id?: number;
  session_id?: string;
  limit?: number;
}) => {
  const [ledgers, setLedgers] = useState<ContinuityLedger[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchLedgers = async () => {
      try {
        setLoading(true);
        const queryParams = new URLSearchParams();
        if (params?.project_id) queryParams.append('project_id', params.project_id.toString());
        if (params?.track_id) queryParams.append('track_id', params.track_id.toString());
        if (params?.session_id) queryParams.append('session_id', params.session_id);
        if (params?.limit) queryParams.append('limit', params.limit.toString());

        const response = await fetch(`/api/v1/coordination/ledgers?${queryParams}`);
        const data = await response.json();
        setLedgers(data.ledgers || []);
        setError(null);
      } catch (err) {
        console.error('Error fetching ledgers:', err);
        setError('Failed to fetch ledgers');
        setLedgers([]);
      } finally {
        setLoading(false);
      }
    };

    fetchLedgers();
  }, [JSON.stringify(params)]);

  return { ledgers, loading, error };
};
