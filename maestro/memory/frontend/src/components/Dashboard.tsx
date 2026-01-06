import { useState } from 'react';
import { GlitchCard, GlitchCardGrid } from './GlitchCard';
import { ImageEcho } from './ImageEcho';
import { MouseTrailer } from './MouseTrailer';
import { GlitchText } from './GlitchText';
import { DetailedTabs } from './DetailedTabs';
import { ComprehensiveGraphView } from './ComprehensiveGraphView';
import { CodeSearchResults } from './CodeSearchResults';
import { useMemories, useProjects, useTracks, useStats, useSearch, useScan, useCodeSearch } from '../hooks/useMaestroData';
import { MemoryDetailModal } from './MemoryDetailModal';
import { Memory } from '../types';
import './Dashboard.css';

export const Dashboard: React.FC = () => {
  const [selectedProject, setSelectedProject] = useState<number | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchMode, setSearchMode] = useState<'memory' | 'code'>('memory');
  const [showDetailedTabs, setShowDetailedTabs] = useState(false);
  const [showComprehensiveGraph, setShowComprehensiveGraph] = useState(false);
  const [selectedMemory, setSelectedMemory] = useState<Memory | null>(null);
  const { memories, loading: memoriesLoading } = useMemories({ limit: 20 });
  const { projects, loading: projectsLoading } = useProjects();
  const { tracks, loading: tracksLoading } = useTracks(selectedProject ?? undefined);
  const { stats } = useStats();
  const { results: searchResults, search } = useSearch();
  const { results: codeResults, searchCode, loading: codeSearchLoading } = useCodeSearch();
  const { scan, loading: scanLoading } = useScan();
  const [scanMessage, setScanMessage] = useState<string | null>(null);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchQuery.trim()) {
      if (searchMode === 'memory') {
        search(searchQuery);
      } else {
        searchCode(searchQuery);
      }
    }
  };

  return (
    <div className="dashboard">
      <MouseTrailer />

      {/* Detailed Tabs Overlay */}
      {showDetailedTabs && (
        <DetailedTabs
          memories={memories}
          projects={projects}
          tracks={tracks}
          stats={stats}
          onClose={() => setShowDetailedTabs(false)}
        />
      )}

      {/* Comprehensive Graph View Overlay */}
      {showComprehensiveGraph && (
        <ComprehensiveGraphView
          memories={memories}
          projects={projects}
          tracks={tracks}
          onClose={() => setShowComprehensiveGraph(false)}
        />
      )}

      {/* Header */}
      <header className="dashboard-header">
        <div className="header-content">
          <div className="header-left">
            <GlitchText text="MAESTRO" className="header-title" as="h1" />
            <p className="header-subtitle">Memory Dashboard v2.0</p>
          </div>

          {/* Search */}
          <form className="search-form" onSubmit={handleSearch}>
            <div className="search-mode-toggle">
              <button
                type="button"
                className={`mode-button ${searchMode === 'memory' ? 'active' : ''}`}
                onClick={() => setSearchMode('memory')}
              >
                <i className="fas fa-database" />
                Memories
              </button>
              <button
                type="button"
                className={`mode-button ${searchMode === 'code' ? 'active' : ''}`}
                onClick={() => setSearchMode('code')}
              >
                <i className="fas fa-code" />
                Code
              </button>
            </div>
            <input
              type="text"
              placeholder={searchMode === 'memory' ? 'Search memories...' : 'Search code with Zoekt...'}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="search-input"
            />
            <button
              type="submit"
              className="search-button"
              disabled={codeSearchLoading}
            >
              <i className={`fas ${codeSearchLoading ? 'fa-spinner fa-spin' : 'fa-search'}`} />
            </button>
          </form>

          {/* Scan Button */}
          <button
            className="scan-button"
            onClick={async () => {
              setScanMessage('Scanning...');
              const result = await scan();
              if (result) {
                setScanMessage(`Found ${result.projects_found} projects, ${result.tracks_found} tracks`);
                setTimeout(() => setScanMessage(null), 5000);
                // Trigger data refresh by reloading page
                window.location.reload();
              } else {
                setScanMessage('Scan failed');
                setTimeout(() => setScanMessage(null), 3000);
              }
            }}
            disabled={scanLoading}
          >
            <i className={`fas ${scanLoading ? 'fa-spinner fa-spin' : 'fa-sync-alt'}`} />
            {scanLoading ? 'Scanning...' : 'Scan Projects'}
          </button>

          {scanMessage && (
            <div className="scan-toast">{scanMessage}</div>
          )}
        </div>
      </header>

      {/* Main Content */}
      <main className="dashboard-main">
        {/* Stats Overview */}
        <section className="stats-section">
          <GlitchCardGrid>
            <GlitchCard
              icon="fa-database"
              title="Total Memories"
              description={`${stats?.total_memories || 0} stored`}
              onClick={() => setShowDetailedTabs(true)}
            />
            <GlitchCard
              icon="fa-folder"
              title="Projects"
              description={`${stats?.total_projects || 0} active`}
              onClick={() => setShowDetailedTabs(true)}
            />
            <GlitchCard
              icon="fa-road"
              title="Tracks"
              description={`${stats?.total_tracks || 0} in progress`}
              onClick={() => setShowDetailedTabs(true)}
            />
            <GlitchCard
              icon="fa-terminal"
              title="Commands"
              description={`${Object.keys(stats?.memories_by_command || {}).length} types`}
              onClick={() => setShowDetailedTabs(true)}
            />
            <GlitchCard
              icon="fa-clock"
              title="Recent Activity"
              description="Last 24 hours"
              onClick={() => setShowDetailedTabs(true)}
            />
            <GlitchCard
              icon="fa-cog"
              title="Settings"
              description="Configure dashboard"
              onClick={() => setShowDetailedTabs(true)}
            />
          </GlitchCardGrid>
        </section>

        {/* Two Column Layout */}
        <div className={`dashboard-columns ${selectedProject ? 'project-selected' : ''}`}>
          {/* Left Column - Projects & Tracks */}
          <div className="dashboard-left">
            <section className="projects-section">
              <GlitchText text="PROJECTS" className="section-title" as="h2" />

              {projectsLoading ? (
                <div className="loading-state">Loading projects...</div>
              ) : (
                <div className="project-list">
                  {projects.map((project) => (
                    <div
                      key={project.id}
                      className={`project-item ${selectedProject === project.id ? 'active' : ''}`}
                      onClick={() => setSelectedProject(project.id)}
                      onMouseMove={(e) => {
                        const rect = e.currentTarget.getBoundingClientRect();
                        const x = ((e.clientX - rect.left) / rect.width) * 100;
                        const y = ((e.clientY - rect.top) / rect.height) * 100;
                        e.currentTarget.style.setProperty('--mouse-x', `${x}%`);
                        e.currentTarget.style.setProperty('--mouse-y', `${y}%`);
                      }}
                    >
                      <div className="project-info">
                        <h3 className="project-name">{project.project_name || project.project_path}</h3>
                        <p className="project-path">{project.project_path}</p>
                        {project.description && (
                          <p className="project-description">{project.description}</p>
                        )}
                      </div>
                      <div className="project-meta">
                        <span className="project-type">{project.project_type || 'generic'}</span>
                        <span className="project-date">
                          {project.last_active ? new Date(project.last_active).toISOString().split('T')[0] : 'N/A'}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </section>

            {/* Tracks for selected project */}
            {selectedProject && (
              <section className="tracks-section">
                <GlitchText text="TRACKS" className="section-title" as="h2" />

                {tracksLoading ? (
                  <div className="loading-state">Loading tracks...</div>
                ) : tracks.length === 0 ? (
                  <div className="empty-state">No tracks found for this project</div>
                ) : (
                  <div className="track-list">
                    {tracks.map((track) => {
                      // Calculate progress safely
                      const hasTasks = track.total_tasks > 0;
                      const progressPercent = hasTasks
                        ? (track.completed_tasks / track.total_tasks) * 100
                        : 0;

                      return (
                        <div
                          key={track.id}
                          className={`track-item status-${track.status}`}
                          onClick={() => {
                            // Make tracks interactable - show track details
                            console.log('Track clicked:', track);
                            // TODO: Open track detail modal or navigate to track view
                          }}
                          onMouseMove={(e) => {
                            const rect = e.currentTarget.getBoundingClientRect();
                            const x = ((e.clientX - rect.left) / rect.width) * 100;
                            const y = ((e.clientY - rect.top) / rect.height) * 100;
                            e.currentTarget.style.setProperty('--mouse-x', `${x}%`);
                            e.currentTarget.style.setProperty('--mouse-y', `${y}%`);
                          }}
                          style={{ cursor: 'pointer' }}
                        >
                          <div className="track-header">
                            <h3 className="track-title">{track.title || 'Untitled Track'}</h3>
                            <span className={`track-status status-${track.status}`}>
                              {track.status.replace('_', ' ')}
                            </span>
                          </div>
                          {track.description && (
                            <p className="track-description">{track.description}</p>
                          )}
                          <div className="track-progress">
                            <div className="progress-bar">
                              <div
                                className="progress-fill"
                                style={{
                                  width: `${progressPercent}%`,
                                }}
                              />
                            </div>
                            <span className="progress-text">
                              {track.completed_tasks}/{track.total_tasks} tasks
                              {!hasTasks && ' (no tasks defined)'}
                            </span>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </section>
            )}
          </div>

          {/* Right Column - Memories (moves below tracks when project selected) */}
          <div className={`dashboard-right ${selectedProject ? 'below-tracks' : ''}`}>
            <section className="memories-section">
              <GlitchText text="RECENT MEMORIES" className="section-title" as="h2" />

              {/* Image Echo Effect card that opens graph view when clicked */}
              <div
                className="memories-echo-card"
                onClick={() => setShowComprehensiveGraph(true)}
              >
                <ImageEcho width={300} height={200} />
                <div className="echo-card-overlay">
                  <i className="fas fa-project-diagram" />
                  <span>Click to view Memory Graph</span>
                </div>
              </div>

              {memoriesLoading ? (
                <div className="loading-state">Loading memories...</div>
              ) : (
                <div className="memory-list">
                  {memories.map((memory) => (
                    <div
                      key={memory.id}
                      className="memory-item"
                      onClick={() => setSelectedMemory(memory)}
                      style={{ cursor: 'pointer' }}
                    >
                      <div className="memory-header">
                        <span className="memory-command">{memory.command}</span>
                        <span className="memory-category">{memory.category}</span>
                      </div>
                      <div className="memory-content">{memory.content}</div>
                      <div className="memory-footer">
                        <span className="memory-date">
                          {new Date(memory.created_at).toISOString().split('T')[0]}
                        </span>
                        {memory.labels.length > 0 && (
                          <div className="memory-labels">
                            {memory.labels.map((label, i) => (
                              <span key={i} className="memory-label">
                                {label}
                              </span>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </section>

            {/* Search Results */}
            {searchQuery && (searchResults.length > 0 || codeResults.length > 0) && (
              <section className="search-results-section">
                <GlitchText text="SEARCH RESULTS" className="section-title" as="h2" />

                {/* Memory Search Results */}
                {searchMode === 'memory' && searchResults.length > 0 && (
                  <div className="memory-search-results">
                    <div className="search-summary">
                      <div className="summary-info">
                        <h3>Memory Search Results</h3>
                        <p className="result-count">
                          Found <strong>{searchResults.length}</strong> result{searchResults.length !== 1 ? 's' : ''} for "<code>{searchQuery}</code>"
                        </p>
                      </div>
                    </div>
                    <div className="search-results">
                      {searchResults.map((memory) => (
                        <div key={memory.id} className="search-result-item">
                          <div className="result-header">
                            <span className="result-command">{memory.command}</span>
                            <span className="result-category">{memory.category}</span>
                          </div>
                          <div className="result-content">{memory.content}</div>
                          <div className="result-meta">
                            <span className="result-date">
                              {new Date(memory.created_at).toISOString().split('T')[0]}
                            </span>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Code Search Results with Progressive Disclosure */}
                {searchMode === 'code' && codeResults.length > 0 && (
                  <CodeSearchResults
                    results={codeResults}
                    query={searchQuery}
                    total={codeResults.length}
                  />
                )}
              </section>
            )}
          </div>
        </div>
      </main>

      {/* Memory Detail Modal */}
      {selectedMemory && (
        <MemoryDetailModal
          memory={selectedMemory}
          onClose={() => setSelectedMemory(null)}
        />
      )}

      {/* Footer */}
      <footer className="dashboard-footer">
        <p>Maestro Memory Dashboard v2.0 - Advanced Visual Interface</p>
      </footer>
    </div>
  );
};
