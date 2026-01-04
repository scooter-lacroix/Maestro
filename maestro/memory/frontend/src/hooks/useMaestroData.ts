import { useState, useEffect } from 'react';
import { apiClient } from '../utils/api';
import { Memory, Project, Track, StatsResponse } from '../types';

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
        const response = await apiClient.listTracks(projectId);
        setTracks(response.tracks);
        setError(null);
      } catch (err) {
        console.error('Error fetching tracks:', err);
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
