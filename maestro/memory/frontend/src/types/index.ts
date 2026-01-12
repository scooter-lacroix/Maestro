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

// Coordination Types for Maestro v2
export interface FileClaim {
  id: number;
  claim_id: string;
  agent_id: string;
  session_id?: string;
  file_patterns: string[];
  status: 'active' | 'released' | 'expired' | 'revoked';
  is_exclusive: boolean;
  reason?: string;
  task_description?: string;
  created_at: string;
  expires_at: string;
  released_at?: string;
  project_id?: number;
  track_id?: number;
}

export interface Handoff {
  id: number;
  handoff_id: string;
  title: string;
  from_session_id: string;
  to_session_id?: string;
  from_agent_id: string;
  to_agent_id?: string;
  status: 'pending' | 'in_progress' | 'resumed' | 'abandoned' | 'completed';
  context_yaml: string;
  context_data?: Record<string, any>;
  project_path?: string;
  summary?: string;
  tags?: string[];
  created_at: string;
  resumed_at?: string;
  completed_at?: string;
  project_id?: number;
  track_id?: number;
}

export interface ContinuityLedger {
  id: number;
  ledger_id: string;
  session_id: string;
  agent_id: string;
  entry_type: 'decision' | 'action' | 'outcome' | 'observation' | 'question' | 'answer';
  title: string;
  content: string;
  metadata?: Record<string, any>;
  parent_entry_id?: number;
  created_at: string;
  sequence_number: number;
  project_id?: number;
  track_id?: number;
}

export interface CoordinationSummary {
  success: boolean;
  summary: {
    active_file_claims: number;
    pending_handoffs: number;
    recent_ledger_entries: number;
  };
}

export interface FileClaimsResponse {
  success: boolean;
  claims: FileClaim[];
  total: number;
}

export interface HandoffsResponse {
  success: boolean;
  handoffs: Handoff[];
  total: number;
}

export interface LedgersResponse {
  success: boolean;
  ledgers: ContinuityLedger[];
  total: number;
}
