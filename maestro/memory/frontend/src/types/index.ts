// API Response Types
export interface Memory {
  id: number;
  content: string;
  category: string;
  labels: string[];
  created_at: string;
  command: string;
  metadata: Record<string, any>;
}

export interface Project {
  id: number;
  project_path: string;
  project_name: string;
  description?: string;
  project_type?: string;
  tech_stack?: Record<string, any>;
  created_at: string;
  last_active: string;
}

export interface Track {
  id: number;
  track_id: string;
  project_id: number;
  title: string;
  description?: string;
  status: 'new' | 'in_progress' | 'completed' | 'blocked';
  track_type?: string;
  phase_count: number;
  current_phase: number;
  total_tasks: number;
  completed_tasks: number;
  created_at: string;
  updated_at: string;
  started_at?: string;
  completed_at?: string;
}

// Zoekt Code Search Types
export interface LineMatch {
  line_number: number;
  line: string;
  before: string[];
  after: string[];
}

export interface CodeSearchResult {
  file_path: string;
  repository: string;
  line_matches: LineMatch[];
  score: number;
}

export interface CodeSearchResponse {
  success: boolean;
  query: string;
  results: CodeSearchResult[];
  total: number;
}

export interface ZoektHealthResponse {
  success: boolean;
  available: boolean;
  server_url: string;
  error?: string;
}

export interface MemoryListResponse {
  success: boolean;
  memories: Memory[];
  total: number;
  project_path?: string;
}

export interface ProjectListResponse {
  success: boolean;
  projects: Project[];
  total: number;
}

export interface TrackListResponse {
  success: boolean;
  tracks: Track[];
  total: number;
}

export interface SearchResponse {
  success: boolean;
  query: string;
  results: Memory[];
  total: number;
}

export interface StatsResponse {
  success: boolean;
  total_memories: number;
  total_projects: number;
  total_tracks: number;
  memories_by_command: Record<string, number>;
  memories_by_project: Record<string, number>;
}

export interface ErrorResponse {
  success: boolean;
  error: string;
  detail?: string;
}
