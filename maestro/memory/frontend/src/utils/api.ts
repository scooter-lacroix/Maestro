import axios from 'axios';
import {
  Memory,
  MemoryListResponse,
  ProjectListResponse,
  TrackListResponse,
  SearchResponse,
  StatsResponse,
} from '../types';

// Use relative URL for production since frontend is served from same FastAPI app
// VITE_API_URL can be set for development (e.g., http://localhost:8000)
const API_BASE_URL = import.meta.env.VITE_API_URL || '';

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// API Functions - create a standalone object with typed methods
export const apiClient = {
  // Health Check
  healthCheck: async () => {
    const response = await api.get('/health');
    return response.data;
  },

  // Memories
  listMemories: async (params?: { project_id?: number; track_id?: number; limit?: number; offset?: number }) => {
    const response = await api.get<{ success: boolean; memories: Memory[]; total: number }>('/api/v1/memories', { params });
    return response.data;
  },

  getProjectContext: async (projectPath: string, limit = 10) => {
    const response = await api.get<MemoryListResponse>('/api/v1/context/project', {
      params: { project_path: projectPath, limit },
    });
    return response.data;
  },

  getTrackContext: async (trackId: string, limit = 20) => {
    const response = await api.get<MemoryListResponse>('/api/v1/context/track', {
      params: { track_id: trackId, limit },
    });
    return response.data;
  },

  searchMemories: async (query: string, projectPath?: string, limit = 5) => {
    const response = await api.get<SearchResponse>('/api/v1/search', {
      params: { query, project_path: projectPath, limit },
    });
    return response.data;
  },

  storeMemory: async (command: string, projectPath: string, context: Record<string, any>) => {
    const response = await api.post('/api/v1/store', {
      command,
      project_path: projectPath,
      context,
    });
    return response.data;
  },

  // Projects
  listProjects: async () => {
    const response = await api.get<ProjectListResponse>('/api/v1/projects');
    return response.data;
  },

  // Tracks
  listTracks: async (projectId?: number) => {
    const response = await api.get<TrackListResponse>('/api/v1/tracks', {
      params: { project_id: projectId },
    });
    return response.data;
  },

  // Statistics
  getStats: async () => {
    const response = await api.get<StatsResponse>('/api/v1/stats');
    return response.data;
  },
};

export default api;
