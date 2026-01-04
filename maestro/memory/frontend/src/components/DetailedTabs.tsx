import { useState, useEffect } from 'react';
import { Memory, Project, Track, StatsResponse } from '../types';
import './DetailedTabs.css';

interface DetailedTabsProps {
  memories: Memory[];
  projects: Project[];
  tracks: Track[];
  stats: StatsResponse | null;
  onClose: () => void;
}

export const DetailedTabs: React.FC<DetailedTabsProps> = ({
  memories,
  projects,
  tracks,
  stats,
  onClose,
}) => {
  const [activeTab, setActiveTab] = useState<string>('memories');
  const [settings, setSettings] = useState({
    theme: localStorage.getItem('dashboard-theme') || 'dark',
    apiEndpoint: localStorage.getItem('api-endpoint') || '/api',
    refreshRate: parseInt(localStorage.getItem('refresh-rate') || '30'),
    dataDisplay: localStorage.getItem('dashboard-data-display') || 'detailed',
  });

  // Apply theme on mount and when changed
  useEffect(() => {
    document.body.className = `theme-${settings.theme}`;
  }, [settings.theme]);

  const updateSetting = (key: string, value: string | number) => {
    const newSettings = { ...settings, [key]: value };
    setSettings(newSettings);
    const storageKey = key === 'apiEndpoint' ? 'api-endpoint' :
      key === 'refreshRate' ? 'refresh-rate' :
        key === 'dataDisplay' ? 'data-display' : key;
    localStorage.setItem(`dashboard-${storageKey}`, String(value));

    // Apply theme immediately
    if (key === 'theme') {
      document.body.className = `theme-${value}`;
    }

    // Apply data display mode immediately
    if (key === 'dataDisplay') {
      document.body.setAttribute('data-display', String(value));
    }
  };

  // Initialize data display mode on mount
  useEffect(() => {
    document.body.setAttribute('data-display', settings.dataDisplay);
  }, []);

  const tabs = [
    { id: 'memories', label: 'Total Memories', icon: 'fa-database' },
    { id: 'projects', label: 'Projects', icon: 'fa-folder' },
    { id: 'tracks', label: 'Tracks', icon: 'fa-road' },
    { id: 'commands', label: 'Commands', icon: 'fa-terminal' },
    { id: 'activity', label: 'Recent Activity', icon: 'fa-clock' },
    { id: 'settings', label: 'Settings', icon: 'fa-cog' },
  ];

  return (
    <div className="detailed-tabs-overlay">
      <div className="detailed-tabs-container">
        {/* Header */}
        <div className="tabs-header">
          <h2 className="tabs-title">MAESTRO MEMORY DASHBOARD</h2>
          <button className="tabs-close-btn" onClick={onClose}>
            <i className="fas fa-times"></i>
          </button>
        </div>

        {/* Tab Navigation */}
        <div className="tabs-nav">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              className={`tab-btn ${activeTab === tab.id ? 'active' : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              <i className={`fas ${tab.icon}`}></i>
              {tab.label}
            </button>
          ))}
        </div>

        {/* Tab Content */}
        <div className={`tabs-content data-display-${settings.dataDisplay}`}>
          {activeTab === 'memories' && (
            <MemoriesTabContent memories={memories} stats={stats} />
          )}
          {activeTab === 'projects' && (
            <ProjectsTabContent projects={projects} />
          )}
          {activeTab === 'tracks' && (
            <TracksTabContent tracks={tracks} />
          )}
          {activeTab === 'commands' && (
            <CommandsTabContent stats={stats} />
          )}
          {activeTab === 'activity' && (
            <ActivityTabContent memories={memories} />
          )}
          {activeTab === 'settings' && (
            <SettingsTabContent settings={settings} updateSetting={updateSetting} onClose={onClose} />
          )}
        </div>
      </div>
    </div>
  );
};

// Memories Tab Content - Show ALL memories with full details
const MemoriesTabContent: React.FC<{ memories: Memory[]; stats: StatsResponse | null }> = ({
  memories,
  stats,
}) => (
  <div className="tab-content-inner">
    <div className="tab-stats-grid">
      <div className="stat-box">
        <div className="stat-value">{memories.length}</div>
        <div className="stat-label">Total Memories</div>
      </div>
      <div className="stat-box">
        <div className="stat-value">{Object.keys(stats?.memories_by_command || {}).length}</div>
        <div className="stat-label">Commands Used</div>
      </div>
      <div className="stat-box">
        <div className="stat-value">{new Set(memories.map(m => m.category)).size}</div>
        <div className="stat-label">Categories</div>
      </div>
    </div>

    <h3 className="tab-section-title">All Memories ({memories.length})</h3>
    <div className="memory-list-scroll">
      {memories.length === 0 ? (
        <div className="empty-state">No memories found</div>
      ) : (
        memories.map((memory) => (
          <div key={memory.id} className="memory-detail-item">
            <div className="memory-detail-header">
              <span className="memory-command-tag">{memory.command}</span>
              <span className="memory-category-tag">{memory.category}</span>
              <span className="memory-date-tag">
                {new Date(memory.created_at).toLocaleString()}
              </span>
            </div>
            <div className="memory-detail-content">{memory.content}</div>
            {memory.labels.length > 0 && (
              <div className="memory-detail-labels">
                {memory.labels.map((label, i) => (
                  <span key={i} className="memory-label-tag">
                    {label}
                  </span>
                ))}
              </div>
            )}
          </div>
        ))
      )}
    </div>

    <h3 className="tab-section-title">Command Breakdown</h3>
    <div className="command-breakdown-grid">
      {Object.entries(stats?.memories_by_command || {})
        .sort(([, a], [, b]) => (b as number) - (a as number))
        .map(([cmd, count]) => (
          <div key={cmd} className="command-breakdown-item">
            <span className="command-breakdown-name">{cmd}</span>
            <span className="command-breakdown-count">{count as number}</span>
            <div className="command-breakdown-bar">
              <div
                className="command-breakdown-fill"
                style={{ width: `${((count as number) / (stats?.total_memories || 1)) * 100}%` }}
              />
            </div>
          </div>
        ))}
    </div>
  </div>
);

// Projects Tab Content - Show ALL projects with full details
const ProjectsTabContent: React.FC<{ projects: Project[] }> = ({ projects }) => (
  <div className="tab-content-inner">
    <div className="tab-stats-grid">
      <div className="stat-box">
        <div className="stat-value">{projects.length}</div>
        <div className="stat-label">Total Projects</div>
      </div>
      <div className="stat-box">
        <div className="stat-value">{projects.filter(p => p.project_type === 'maestro').length}</div>
        <div className="stat-label">Maestro Projects</div>
      </div>
      <div className="stat-box">
        <div className="stat-value">{projects.filter(p => p.project_type === 'generic').length}</div>
        <div className="stat-label">Generic Projects</div>
      </div>
    </div>

    <h3 className="tab-section-title">All Projects ({projects.length})</h3>
    <div className="project-list-scroll">
      {projects.length === 0 ? (
        <div className="empty-state">No projects found</div>
      ) : (
        projects.map((project) => (
          <div key={project.id} className="project-detail-item">
            <div className="project-detail-header">
              <h4 className="project-detail-name">{project.project_name || project.project_path}</h4>
              <span className={`project-type-tag ${project.project_type}`}>
                {project.project_type}
              </span>
            </div>
            <div className="project-detail-path">{project.project_path}</div>
            {project.description && (
              <div className="project-detail-description">{project.description}</div>
            )}
            <div className="project-detail-meta">
              <span>Last Active: {new Date(project.last_active).toLocaleString()}</span>
              <span>Created: {new Date(project.created_at).toLocaleString()}</span>
            </div>
          </div>
        ))
      )}
    </div>
  </div>
);

// Tracks Tab Content - Show ALL tracks with complete details
const TracksTabContent: React.FC<{ tracks: Track[] }> = ({ tracks }) => (
  <div className="tab-content-inner">
    <div className="tab-stats-grid">
      <div className="stat-box">
        <div className="stat-value">{tracks.length}</div>
        <div className="stat-label">Total Tracks</div>
      </div>
      <div className="stat-box">
        <div className="stat-value">{tracks.filter(t => t.status === 'completed').length}</div>
        <div className="stat-label">Completed</div>
      </div>
      <div className="stat-box">
        <div className="stat-value">{tracks.filter(t => t.status === 'in_progress').length}</div>
        <div className="stat-label">In Progress</div>
      </div>
    </div>

    <h3 className="tab-section-title">All Tracks ({tracks.length})</h3>
    <div className="track-list-scroll">
      {tracks.length === 0 ? (
        <div className="empty-state">No tracks found</div>
      ) : (
        tracks.map((track) => (
          <div key={track.id} className={`track-detail-item status-${track.status}`}>
            <div className="track-detail-header">
              <h4 className="track-detail-title">{track.title}</h4>
              <span className={`track-status-tag status-${track.status}`}>
                {track.status.replace('_', ' ')}
              </span>
            </div>
            {track.description && (
              <div className="track-detail-description">{track.description}</div>
            )}
            <div className="track-detail-progress">
              <div className="progress-info">
                <span>Progress</span>
                <span>{track.completed_tasks}/{track.total_tasks} tasks ({track.total_tasks > 0 ? Math.round((track.completed_tasks / track.total_tasks) * 100) : 0}%)</span>
              </div>
              <div className="progress-bar-container">
                <div
                  className="progress-bar-fill"
                  style={{
                    width: `${track.total_tasks > 0 ? (track.completed_tasks / track.total_tasks) * 100 : 0}%`,
                  }}
                />
              </div>
            </div>
          </div>
        ))
      )}
    </div>
  </div>
);

// Commands Tab Content - Animated walkthrough of ALL commands
const CommandsTabContent: React.FC<{ stats: StatsResponse | null }> = ({ stats }) => {
  const commands = [
    {
      name: '/maestro:setup',
      description: 'Initialize Maestro in the current project directory. Creates the necessary directory structure and configuration files.',
      usage: '/maestro:setup',
      options: [],
      example: '/maestro:setup',
    },
    {
      name: '/maestro:newTrack',
      description: 'Create a new track with automatic planning. Generates a track plan with specific tasks and phases.',
      usage: '/maestro:newTrack <track description>',
      options: ['--phase <number>', '--priority <low|medium|high>'],
      example: '/maestro:newTrack "Implement user authentication with JWT"',
    },
    {
      name: '/maestro:implement',
      description: 'Execute tasks defined in a track. Implements the planned tasks systematically.',
      usage: '/maestro:implement [track ID]',
      options: ['--task <task ID>', '--phase <phase number>', '--continue'],
      example: '/maestro:implement maestro-auth_20250101',
    },
    {
      name: '/maestro:status',
      description: 'Display current track progress, task status, and completion metrics.',
      usage: '/maestro:status [track ID]',
      options: ['--verbose', '--tasks', '--phases'],
      example: '/maestro:status --verbose',
    },
    {
      name: 'maestro tui',
      description: 'Launch the Terminal User Interface for interactive project management.',
      usage: 'maestro tui',
      options: ['--project <path>', '--config <file>'],
      example: 'maestro tui',
    },
    {
      name: 'maestro memory serve',
      description: 'Start the web dashboard server for memory visualization and management.',
      usage: 'maestro memory serve [options]',
      options: ['--port <number>', '--host <address>', '--no-open'],
      example: 'maestro memory serve',
    },
    {
      name: 'maestro memory status',
      description: 'Display memory statistics including total memories, commands used, and storage info.',
      usage: 'maestro memory status',
      options: ['--json', '--detailed'],
      example: 'maestro memory status --detailed',
    },
    {
      name: 'maestro migrate:*',
      description: 'Database migration commands for creating, applying, and rolling back migrations.',
      usage: 'maestro migrate:create <name> | maestro migrate:up | maestro migrate:down',
      options: ['--steps <number>', '--force'],
      example: 'maestro migrate:create add_user_table',
    },
  ];

  return (
    <div className="tab-content-inner">
      <h3 className="tab-section-title">Maestro Command Reference</h3>
      <div className="commands-walkthrough">
        {commands.map((cmd, index) => (
          <div key={cmd.name} className="command-walkthrough-item" style={{ animationDelay: `${index * 0.1}s` }}>
            <div className="command-walkthrough-header">
              <h4 className="command-walkthrough-name">{cmd.name}</h4>
              <span className="command-walkthrough-index">{index + 1}</span>
            </div>
            <p className="command-walkthrough-description">{cmd.description}</p>
            <div className="command-walkthrough-usage">
              <span className="usage-label">Usage:</span>
              <code className="usage-code">{cmd.usage}</code>
            </div>
            {cmd.options.length > 0 && (
              <div className="command-walkthrough-options">
                <span className="options-label">Options:</span>
                <div className="options-list">
                  {cmd.options.map((opt, i) => (
                    <code key={i} className="option-code">{opt}</code>
                  ))}
                </div>
              </div>
            )}
            <div className="command-walkthrough-example">
              <span className="example-label">Example:</span>
              <code className="example-code">{cmd.example}</code>
            </div>
          </div>
        ))}
      </div>

      <h3 className="tab-section-title">Command Usage Statistics</h3>
      <div className="command-chart">
        {Object.entries(stats?.memories_by_command || {})
          .sort(([, a], [, b]) => (b as number) - (a as number))
          .map(([cmd, count]) => (
            <div key={cmd} className="command-chart-item">
              <div className="command-chart-label">{cmd}</div>
              <div className="command-chart-bar-container">
                <div
                  className="command-chart-bar"
                  style={{
                    width: `${((count as number) / (stats?.total_memories || 1)) * 100}%`,
                  }}
                >
                  <span className="command-chart-count">{count as number}</span>
                </div>
              </div>
            </div>
          ))}
      </div>
    </div>
  );
};

// Activity Tab Content - Clean, formatted timeline
const ActivityTabContent: React.FC<{ memories: Memory[] }> = ({ memories }) => {
  // Group memories by date
  const groupedMemories = memories.reduce((acc, memory) => {
    const date = new Date(memory.created_at).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'long',
      day: 'numeric'
    });
    if (!acc[date]) {
      acc[date] = [];
    }
    acc[date].push(memory);
    return acc;
  }, {} as Record<string, Memory[]>);

  const sortedDates = Object.keys(groupedMemories).sort((a, b) =>
    new Date(b).getTime() - new Date(a).getTime()
  );

  return (
    <div className="tab-content-inner">
      <h3 className="tab-section-title">Recent Activity Timeline</h3>
      <div className="activity-timeline-new">
        {sortedDates.length === 0 ? (
          <div className="empty-state">No activity found</div>
        ) : (
          sortedDates.map((date) => (
            <div key={date} className="timeline-date-group">
              <div className="timeline-date-header">
                <i className="fas fa-calendar-alt"></i>
                {date}
              </div>
              <div className="timeline-entries">
                {groupedMemories[date].map((memory) => (
                  <details key={memory.id} className="timeline-entry-details">
                    <summary className="timeline-entry-summary">
                      <div className="timeline-entry-main">
                        <span className={`timeline-command-icon ${memory.command.includes('maestro') ? 'maestro-cmd' : 'other-cmd'}`}>
                          <i className="fas fa-bolt"></i>
                        </span>
                        <div className="timeline-entry-info">
                          <div className="timeline-entry-command">{memory.command}</div>
                          <div className="timeline-entry-time">
                            {new Date(memory.created_at).toLocaleTimeString('en-US', {
                              hour: 'numeric',
                              minute: '2-digit',
                              second: '2-digit',
                              hour12: true
                            })}
                          </div>
                        </div>
                      </div>
                    </summary>
                    <div className="timeline-entry-details-content">
                      <div className="timeline-entry-content">{memory.content}</div>
                      <div className="timeline-entry-meta">
                        <span className="timeline-category">{memory.category}</span>
                        {memory.labels.length > 0 && (
                          <div className="timeline-labels">
                            {memory.labels.map((label, i) => (
                              <span key={i} className="timeline-label">{label}</span>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                  </details>
                ))}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

// Settings Tab Content - Functional settings
const SettingsTabContent: React.FC<{
  settings: { theme: string; apiEndpoint: string; refreshRate: number; dataDisplay: string };
  updateSetting: (key: string, value: string | number) => void;
  onClose: () => void;
}> = ({ settings, updateSetting }) => {
  const [showClearConfirm, setShowClearConfirm] = useState(false);

  const exportSettings = () => {
    const dataStr = JSON.stringify(settings, null, 2);
    const blob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'maestro-dashboard-settings.json';
    a.click();
  };

  const importSettings = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (file) {
        const reader = new FileReader();
        reader.onload = (e) => {
          try {
            const imported = JSON.parse(e.target?.result as string);
            Object.entries(imported).forEach(([key, value]) => {
              updateSetting(key, value as string | number);
            });
            alert('Settings imported successfully!');
          } catch {
            alert('Invalid settings file');
          }
        };
        reader.readAsText(file);
      }
    };
    input.click();
  };

  const clearData = () => {
    localStorage.clear();
    setShowClearConfirm(false);
    alert('All data cleared. Refreshing...');
    setTimeout(() => window.location.reload(), 500);
  };

  return (
    <div className="tab-content-inner">
      <h3 className="tab-section-title">Dashboard Settings</h3>

      <div className="settings-section">
        <h4 className="settings-subtitle">Appearance</h4>
        <div className="setting-item">
          <label className="setting-label">Theme</label>
          <select
            className="setting-select"
            value={settings.theme}
            onChange={(e) => updateSetting('theme', e.target.value)}
          >
            <option value="dark">Dark Brutalist</option>
            <option value="light">Light</option>
            <option value="dither">Dither Dark (Terminal)</option>
            <option value="cyberpunk">Cyberpunk Neon</option>
            <option value="midnight">Midnight Aurora</option>
            <option value="sepia">Sepia (Neutral)</option>
          </select>
        </div>
        <div className="setting-item">
          <label className="setting-label">Data Display</label>
          <select
            className="setting-select"
            value={settings.dataDisplay}
            onChange={(e) => updateSetting('dataDisplay', e.target.value)}
          >
            <option value="compact">Compact</option>
            <option value="detailed">Detailed</option>
            <option value="visual">Visual</option>
          </select>
        </div>
      </div>

      <div className="settings-section">
        <h4 className="settings-subtitle">API Configuration</h4>
        <div className="setting-item">
          <label className="setting-label">API Endpoint</label>
          <input
            type="text"
            className="setting-input"
            value={settings.apiEndpoint}
            onChange={(e) => updateSetting('apiEndpoint', e.target.value)}
          />
        </div>
      </div>

      <div className="settings-section">
        <h4 className="settings-subtitle">Data Preferences</h4>
        <div className="setting-item">
          <label className="setting-label">Refresh Rate (seconds)</label>
          <input
            type="number"
            className="setting-input"
            min="5"
            max="300"
            value={settings.refreshRate}
            onChange={(e) => updateSetting('refreshRate', parseInt(e.target.value))}
          />
        </div>
      </div>

      <div className="settings-section">
        <h4 className="settings-subtitle">Data Management</h4>
        <div className="settings-actions">
          <button className="settings-btn settings-btn-export" onClick={exportSettings}>
            <i className="fas fa-download"></i> Export Settings
          </button>
          <button className="settings-btn settings-btn-import" onClick={importSettings}>
            <i className="fas fa-upload"></i> Import Settings
          </button>
        </div>
        <button
          className="settings-btn settings-btn-clear"
          onClick={() => setShowClearConfirm(!showClearConfirm)}
        >
          <i className="fas fa-trash"></i> Clear All Data
        </button>
        {showClearConfirm && (
          <div className="clear-confirm">
            <p>Are you sure? This will delete all settings and cached data.</p>
            <div className="clear-confirm-actions">
              <button className="settings-btn settings-btn-danger" onClick={clearData}>
                Yes, Clear All
              </button>
              <button
                className="settings-btn settings-btn-cancel"
                onClick={() => setShowClearConfirm(false)}
              >
                Cancel
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
