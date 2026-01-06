import { useMemo, useState, useRef, useEffect } from 'react';
import * as d3 from 'd3';
import { Memory, Project, Track } from '../types';
import './ComprehensiveGraphView.css';

interface ComprehensiveGraphViewProps {
  memories: Memory[];
  projects?: Project[];
  tracks?: Track[];
  onClose: () => void;
}

export const ComprehensiveGraphView: React.FC<ComprehensiveGraphViewProps> = ({
  memories,
  projects = [],
  tracks = [],
  onClose,
}) => {
  const [activeView, setActiveView] = useState<'network' | 'category' | 'timeline' | 'heatmap'>('network');
  const networkRef = useRef<HTMLDivElement>(null);
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);

  // Process data for different visualizations
  const graphData = useMemo(() => {
    const categories = new Map<string, Memory[]>();
    const commands = new Map<string, Memory[]>();
    const projectMems = new Map<string, Memory[]>();
    const timeline = new Map<string, Memory[]>();

    memories.forEach(memory => {
      if (!categories.has(memory.category)) {
        categories.set(memory.category, []);
      }
      categories.get(memory.category)!.push(memory);

      if (!commands.has(memory.command)) {
        commands.set(memory.command, []);
      }
      commands.get(memory.command)!.push(memory);

      const dateKey = new Date(memory.created_at).toISOString().split('T')[0];
      if (!timeline.has(dateKey)) {
        timeline.set(dateKey, []);
      }
      timeline.get(dateKey)!.push(memory);
    });

    // Create project-track relationships
    const projectTrackMap = new Map<number, Track[]>();
    tracks.forEach(track => {
      if (!projectTrackMap.has(track.project_id)) {
        projectTrackMap.set(track.project_id, []);
      }
      projectTrackMap.get(track.project_id)!.push(track);
    });

    return { categories, commands, projectMems, timeline, projectTrackMap };
  }, [memories, tracks]);

  // Network Graph with D3.js Force Simulation - Enhanced with Projects and Tracks
  useEffect(() => {
    if (activeView !== 'network' || !networkRef.current) return;

    // Clear previous visualization
    d3.select(networkRef.current).selectAll('*').remove();

    const width = 1200;
    const height = 700;
    const svg = d3.select(networkRef.current)
      .append('svg')
      .attr('width', width)
      .attr('height', height)
      .attr('viewBox', [0, 0, width, height])
      .style('background', 'rgba(0, 0, 0, 0.2)')
      .style('border-radius', '12px');

    // Create gradient definitions
    const defs = svg.append('defs');

    // Add beautiful gradients for different node types
    const gradients = [
      { id: 'gradient-project', colors: ['#667eea', '#764ba2'] },
      { id: 'gradient-track', colors: ['#f093fb', '#f5576c'] },
      { id: 'gradient-memory', colors: ['#4facfe', '#00f2fe'] },
      { id: 'gradient-category', colors: ['#43e97b', '#38f9d7'] },
    ];

    gradients.forEach(({ id, colors }) => {
      const gradient = defs.append('linearGradient')
        .attr('id', id)
        .attr('x1', '0%')
        .attr('y1', '0%')
        .attr('x2', '100%')
        .attr('y2', '100%');

      gradient.append('stop')
        .attr('offset', '0%')
        .attr('stop-color', colors[0]);

      gradient.append('stop')
        .attr('offset', '100%')
        .attr('stop-color', colors[1]);
    });

    // Add shadow filter
    const filter = defs.append('filter')
      .attr('id', 'shadow')
      .attr('x', '-50%')
      .attr('y', '-50%')
      .attr('width', '200%')
      .attr('height', '200%');

    filter.append('feDropShadow')
      .attr('dx', '0')
      .attr('dy', '2')
      .attr('stdDeviation', '3')
      .attr('flood-opacity', '0.3');

    // Create nodes and links with meaningful relationships
    const nodes: any[] = [];
    const links: any[] = [];

    // 1. Project nodes (largest, outer ring)
    projects.forEach((project) => {
      const trackCount = graphData.projectTrackMap.get(project.id)?.length || 0;
      nodes.push({
        id: `project-${project.id}`,
        type: 'project',
        label: project.project_name || project.project_path.split('/').pop(),
        fullPath: project.project_path,
        description: project.description,
        projectType: project.project_type,
        radius: 30 + Math.sqrt(trackCount) * 5,
        trackCount,
        group: 1,
      });
    });

    // 2. Track nodes (medium, connected to projects)
    let trackIndex = 0;
    tracks.forEach((track) => {
      nodes.push({
        id: `track-${track.id}`,
        type: 'track',
        label: track.title,
        description: track.description,
        status: track.status,
        progress: `${track.completed_tasks}/${track.total_tasks}`,
        radius: 20 + Math.sqrt(track.total_tasks) * 3,
        group: 2,
      });

      // Link track to its project
      links.push({
        source: `track-${track.id}`,
        target: `project-${track.project_id}`,
        value: 2,
      });

      trackIndex++;
    });

    // 3. Category nodes (organizing memories)
    let categoryIndex = 0;
    graphData.categories.forEach((mems, category) => {
      nodes.push({
        id: `category-${category}`,
        type: 'category',
        label: category,
        radius: 25 + Math.sqrt(mems.length) * 6,
        count: mems.length,
        group: 3 + categoryIndex,
      });
      categoryIndex++;
    });

    // 4. Memory nodes (smallest, connected to categories)
    memories.slice(0, 150).forEach((memory) => {
      nodes.push({
        id: `memory-${memory.id}`,
        type: 'memory',
        label: memory.command,
        content: memory.content.slice(0, 50) + '...',
        category: memory.category,
        radius: 8 + Math.random() * 4,
        group: 10 + memory.id % 5,
      });

      // Link memory to its category
      links.push({
        source: `memory-${memory.id}`,
        target: `category-${memory.category}`,
        value: 1,
      });
    });

    // Create force simulation with proper physics
    const simulation = d3.forceSimulation(nodes as any)
      .force('link', d3.forceLink(links as any)
        .id((d: any) => d.id)
        .distance((d: any) => {
          // Projects and tracks should be further apart
          if (d.source.type === 'project' || d.target.type === 'project') return 150;
          if (d.source.type === 'track' || d.target.type === 'track') return 120;
          if (d.source.type === 'category' || d.target.type === 'category') return 100;
          return 80;
        })
        .strength(0.4))
      .force('charge', d3.forceManyBody().strength((d: any) => {
        // Larger nodes have more repulsion
        if (d.type === 'project') return -800;
        if (d.type === 'track') return -600;
        if (d.type === 'category') return -500;
        return -300;
      }))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide().radius((d: any) => d.radius + 10))
      .force('x', d3.forceX(width / 2).strength(0.05))
      .force('y', d3.forceY(height / 2).strength(0.05));

    // Create zoomable group
    const g = svg.append('g');

    // Create links first (so they appear behind nodes)
    const link = g.append('g')
      .attr('class', 'links')
      .selectAll('line')
      .data(links)
      .join('line')
      .attr('stroke', (d: any) => {
        // Color links based on source type
        if (d.source.type === 'project') return 'rgba(102, 126, 234, 0.4)';
        if (d.source.type === 'track') return 'rgba(240, 147, 251, 0.4)';
        if (d.source.type === 'category') return 'rgba(67, 233, 123, 0.3)';
        return 'rgba(255, 255, 255, 0.15)';
      })
      .attr('stroke-width', (d: any) => Math.sqrt(d.value) * 1.5)
      .attr('stroke-opacity', 0.6);

    // Create nodes
    const node = g.append('g')
      .attr('class', 'nodes')
      .selectAll('g')
      .data(nodes)
      .join('g')
      .attr('class', 'node')
      .style('cursor', 'pointer')
      .call(d3.drag<SVGGElement, any>()
        .on('start', (event, d) => {
          if (!event.active) simulation.alphaTarget(0.3).restart();
          d.fx = d.x;
          d.fy = d.y;
        })
        .on('drag', (event, d) => {
          d.fx = event.x;
          d.fy = event.y;
        })
        .on('end', (event, d) => {
          if (!event.active) simulation.alphaTarget(0);
          d.fx = null;
          d.fy = null;
        }) as any);

    // Add circles to nodes
    node.append('circle')
      .attr('r', (d: any) => d.radius)
      .attr('fill', (d: any) => {
        if (d.type === 'project') return 'url(#gradient-project)';
        if (d.type === 'track') return 'url(#gradient-track)';
        if (d.type === 'category') return 'url(#gradient-category)';
        return 'url(#gradient-memory)';
      })
      .attr('stroke', 'rgba(255, 255, 255, 0.3)')
      .attr('stroke-width', 2)
      .style('filter', 'url(#shadow)')
      .on('mouseover', function(event, d) {
        d3.select(this)
          .transition()
          .duration(200)
          .attr('r', d.radius * 1.2)
          .attr('stroke', 'rgba(255, 255, 255, 0.9)');

        // Show tooltip with node details
        const tooltip = d3.select('body').append('div')
          .attr('class', 'graph-tooltip')
          .style('position', 'absolute')
          .style('background', 'rgba(0, 0, 0, 0.9)')
          .style('border', '1px solid rgba(255, 255, 255, 0.2)')
          .style('border-radius', '8px')
          .style('padding', '12px')
          .style('color', 'white')
          .style('font-family', 'Courier New, monospace')
          .style('font-size', '12px')
          .style('pointer-events', 'none')
          .style('z-index', '10000')
          .style('max-width', '300px');

        let tooltipContent = `<strong style="color: ${d.type === 'project' ? '#667eea' : d.type === 'track' ? '#f093fb' : d.type === 'category' ? '#43e97b' : '#4facfe'}">${d.label}</strong><br/>`;
        tooltipContent += `<span style="color: rgba(255, 255, 255, 0.6); font-size: 10px; text-transform: uppercase;">${d.type}</span><br/>`;

        if (d.type === 'project') {
          tooltipContent += `<br/>Path: ${d.fullPath}<br/>`;
          if (d.description) tooltipContent += `Description: ${d.description}<br/>`;
          tooltipContent += `Tracks: ${d.trackCount}`;
        } else if (d.type === 'track') {
          if (d.description) tooltipContent += `<br/>${d.description}<br/>`;
          tooltipContent += `Status: <span style="color: ${d.status === 'completed' ? '#4ade80' : d.status === 'in_progress' ? '#fbbf24' : '#94a3b8'}">${d.status}</span><br/>`;
          tooltipContent += `Progress: ${d.progress}`;
        } else if (d.type === 'category') {
          tooltipContent += `<br/>Memories: ${d.count}`;
        } else if (d.type === 'memory') {
          tooltipContent += `<br/>${d.content}<br/>`;
          tooltipContent += `Category: ${d.category}`;
        }

        tooltip.html(tooltipContent)
          .style('left', (event.pageX + 10) + 'px')
          .style('top', (event.pageY - 10) + 'px');

        // Store reference to remove on mouseout
        d3.select(this).datum({ ...d, tooltip });
      })
      .on('mouseout', function(_, d) {
        d3.select(this)
          .transition()
          .duration(200)
          .attr('r', d.radius)
          .attr('stroke', 'rgba(255, 255, 255, 0.3)');

        // Remove tooltip
        if (d.tooltip) {
          d.tooltip.remove();
        }
        d3.selectAll('.graph-tooltip').remove();
      })
      .on('click', (event, d) => {
        // Handle node click - could open modal or navigate
        console.log('Node clicked:', d);
        event.stopPropagation();
      });

    // Add labels to project and track nodes
    node.filter((d: any) => d.type === 'project' || d.type === 'track' || d.type === 'category')
      .append('text')
      .text((d: any) => {
        if (d.type === 'project') return d.label;
        if (d.type === 'track') return d.label;
        if (d.type === 'category') return `${d.label} (${d.count})`;
        return '';
      })
      .attr('text-anchor', 'middle')
      .attr('dy', (d: any) => d.radius + 16)
      .attr('fill', 'rgba(255, 255, 255, 0.85)')
      .attr('font-size', (d: any) => {
        if (d.type === 'project') return '11px';
        if (d.type === 'track') return '10px';
        return '9px';
      })
      .attr('font-weight', '600')
      .style('text-shadow', '0 2px 4px rgba(0,0,0,0.8)')
      .style('pointer-events', 'none');

    // Add count labels inside larger nodes
    node.filter((d: any) => d.type === 'category')
      .append('text')
      .text((d: any) => d.count)
      .attr('text-anchor', 'middle')
      .attr('dy', 5)
      .attr('fill', 'white')
      .attr('font-size', (d: any) => Math.min(d.radius / 2.5, 18) + 'px')
      .attr('font-weight', '700')
      .style('pointer-events', 'none');

    // Add zoom behavior
    svg.call(d3.zoom<HTMLElement, unknown>()
      .scaleExtent([0.1, 4])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      }) as any);

    // Update positions on tick
    simulation.on('tick', () => {
      link
        .attr('x1', (d: any) => d.source.x)
        .attr('y1', (d: any) => d.source.y)
        .attr('x2', (d: any) => d.target.x)
        .attr('y2', (d: any) => d.target.y);

      node
        .attr('transform', (d: any) => `translate(${d.x},${d.y})`);
    });

    // Cleanup function to prevent memory leaks
    return () => {
      simulation.stop();
      d3.select(networkRef.current).selectAll('*').remove();
      d3.selectAll('.graph-tooltip').remove();
    };
  }, [activeView, graphData, memories, projects, tracks]);

  // Calculate statistics for heatmap
  const heatmapData = useMemo(() => {
    const today = new Date();
    const data = [];
    for (let i = 364; i >= 0; i--) {
      const date = new Date(today);
      date.setDate(date.getDate() - i);
      const dateStr = date.toISOString().split('T')[0];
      const count = graphData.timeline.get(dateStr)?.length || 0;
      data.push({
        date: dateStr,
        count,
        level: count === 0 ? 0 : count < 3 ? 1 : count < 6 ? 2 : count < 10 ? 3 : 4,
      });
    }
    return data;
  }, [graphData]);

  // Timeline data sorted by date
  const timelineData = useMemo(() => {
    return Array.from(graphData.timeline.entries())
      .sort((a, b) => new Date(a[0]).getTime() - new Date(b[0]).getTime())
      .slice(-60);
  }, [graphData]);

  return (
    <div className="comprehensive-graph-view">
      <div className="graph-view-header">
        <h2 className="graph-view-title">MEMORY VISUALIZATION</h2>
        <button className="graph-view-close" onClick={onClose}>
          <i className="fas fa-times"></i>
        </button>
      </div>

      <div className="graph-view-tabs">
        <button
          className={`graph-tab ${activeView === 'network' ? 'active' : ''}`}
          onClick={() => setActiveView('network')}
        >
          <i className="fas fa-project-diagram"></i> Network Graph
        </button>
        <button
          className={`graph-tab ${activeView === 'category' ? 'active' : ''}`}
          onClick={() => setActiveView('category')}
        >
          <i className="fas fa-chart-pie"></i> Categories
        </button>
        <button
          className={`graph-tab ${activeView === 'timeline' ? 'active' : ''}`}
          onClick={() => setActiveView('timeline')}
        >
          <i className="fas fa-stream"></i> Timeline
        </button>
        <button
          className={`graph-tab ${activeView === 'heatmap' ? 'active' : ''}`}
          onClick={() => setActiveView('heatmap')}
        >
          <i className="fas fa-fire"></i> Heatmap
        </button>
      </div>

      <div className="graph-view-content">
        {activeView === 'network' && (
          <div className="network-graph-container">
            {/* Statistics Header */}
            <div className="network-stats-header">
              <div className="network-stat-box">
                <div className="network-stat-value">{projects.length}</div>
                <div className="network-stat-label">Projects</div>
              </div>
              <div className="network-stat-box">
                <div className="network-stat-value">{tracks.length}</div>
                <div className="network-stat-label">Tracks</div>
              </div>
              <div className="network-stat-box">
                <div className="network-stat-value">{memories.length}</div>
                <div className="network-stat-label">Memories</div>
              </div>
              <div className="network-stat-box">
                <div className="network-stat-value">{graphData.categories.size}</div>
                <div className="network-stat-label">Categories</div>
              </div>
            </div>

            {/* Control Panel */}
            <div className="network-controls">
              <div className="control-group">
                <label className="control-label">Filter</label>
                <select className="control-select">
                  <option>All Categories</option>
                  {Array.from(graphData.categories.keys()).map(cat => (
                    <option key={cat}>{cat}</option>
                  ))}
                </select>
              </div>
              <div className="control-group">
                <label className="control-label">Layout</label>
                <select className="control-select">
                  <option>Force Directed</option>
                  <option>Circular</option>
                  <option>Hierarchical</option>
                </select>
              </div>
              <div className="control-group">
                <button className="control-btn">
                  <i className="fas fa-search-plus"></i> Zoom In
                </button>
                <button className="control-btn">
                  <i className="fas fa-search-minus"></i> Zoom Out
                </button>
                <button className="control-btn">
                  <i className="fas fa-redo"></i> Reset
                </button>
              </div>
            </div>

            <div ref={networkRef} className="network-d3-container" />

            {/* Legend */}
            <div className="network-legend-full">
              <div className="legend-section">
                <h4 className="legend-title">Node Types</h4>
                <div className="legend-item">
                  <span className="legend-dot project-dot"></span>
                  <span>Projects ({projects.length})</span>
                </div>
                <div className="legend-item">
                  <span className="legend-dot track-dot"></span>
                  <span>Tracks ({tracks.length})</span>
                </div>
                <div className="legend-item">
                  <span className="legend-dot category-dot"></span>
                  <span>Categories ({graphData.categories.size})</span>
                </div>
                <div className="legend-item">
                  <span className="legend-dot memory-dot"></span>
                  <span>Memories ({memories.length})</span>
                </div>
              </div>
              <div className="legend-section">
                <h4 className="legend-title">Relationships</h4>
                <div className="legend-item">
                  <i className="fas fa-project-diagram"></i>
                  <span>Projects contain tracks</span>
                </div>
                <div className="legend-item">
                  <i className="fas fa-sitemap"></i>
                  <span>Memories grouped by category</span>
                </div>
              </div>
              <div className="legend-section">
                <h4 className="legend-title">Controls</h4>
                <div className="legend-item">
                  <i className="fas fa-mouse-pointer"></i>
                  <span>Drag nodes to reposition</span>
                </div>
                <div className="legend-item">
                  <i className="fas fa-search"></i>
                  <span>Scroll to zoom in/out</span>
                </div>
                <div className="legend-item">
                  <i className="fas fa-hand-pointer"></i>
                  <span>Click nodes for details</span>
                </div>
              </div>
            </div>
          </div>
        )}

        {activeView === 'category' && (
          <div className="category-chart-full">
            {/* Header with Statistics */}
            <div className="category-header">
              <div className="category-stat-box">
                <div className="category-stat-value">{graphData.categories.size}</div>
                <div className="category-stat-label">Categories</div>
              </div>
              <div className="category-stat-box">
                <div className="category-stat-value">{memories.length}</div>
                <div className="category-stat-label">Total Memories</div>
              </div>
              <div className="category-stat-box">
                <div className="category-stat-value">
                  {Math.round(memories.length / graphData.categories.size)}
                </div>
                <div className="category-stat-label">Avg per Category</div>
              </div>
            </div>

            {/* Main Content Area */}
            <div className="category-main-content">
              {/* Donut Chart */}
              <div className="donut-chart-wrapper-large">
                <svg viewBox="0 0 500 500" className="donut-chart-large">
                  {/* Background circle */}
                  <circle
                    cx="250"
                    cy="250"
                    r="180"
                    fill="none"
                    stroke="rgba(255, 255, 255, 0.05)"
                    strokeWidth="50"
                  />
                  {/* Donut segments */}
                  {Array.from(graphData.categories.entries()).map(([category, mems], index, arr) => {
                    const percentage = (mems.length / memories.length) * 100;
                    const startPercentage = arr.slice(0, index).reduce((sum, [, m]) => sum + (m.length / memories.length) * 100, 0);
                    const circumference = 2 * Math.PI * 180;
                    const dashArray = (percentage / 100) * circumference;
                    const dashOffset = -((startPercentage / 100) * circumference);
                    const hue = (index * 360) / graphData.categories.size;
                    const isSelected = selectedCategory === category;

                    return (
                      <g key={category}>
                        <circle
                          cx="250"
                          cy="250"
                          r="180"
                          fill="none"
                          stroke={`hsl(${hue}, 70%, 60%)`}
                          strokeWidth="50"
                          strokeDasharray={dashArray}
                          strokeDashoffset={dashOffset}
                          className={isSelected ? 'donut-segment selected' : 'donut-segment'}
                          style={{
                            transformOrigin: '250px 250px',
                            cursor: 'pointer',
                            filter: isSelected ? 'drop-shadow(0 0 15px rgba(255, 255, 255, 0.6))' : undefined,
                          }}
                          onClick={() => setSelectedCategory(isSelected ? null : category)}
                        />
                      </g>
                    );
                  })}
                  {/* Center text */}
                  <text x="250" y="240" textAnchor="middle" className="donut-label-large">
                    {selectedCategory || 'Total'}
                  </text>
                  <text x="250" y="280" textAnchor="middle" className="donut-value-large">
                    {selectedCategory
                      ? graphData.categories.get(selectedCategory)?.length || 0
                      : memories.length}
                  </text>
                </svg>
              </div>

              {/* Interactive Legend */}
              <div className="category-legend-full">
                <h3 className="legend-section-title">Category Breakdown</h3>
                {Array.from(graphData.categories.entries()).map(([category, mems], index) => {
                  const hue = (index * 360) / graphData.categories.size;
                  const percentage = ((mems.length / memories.length) * 100).toFixed(1);
                  return (
                    <div
                      key={category}
                      className={`category-legend-item-full ${selectedCategory === category ? 'selected' : ''}`}
                      onClick={() => setSelectedCategory(selectedCategory === category ? null : category)}
                    >
                      <span
                        className="legend-color-large"
                        style={{ background: `hsl(${hue}, 70%, 60%)` }}
                      ></span>
                      <div className="legend-info">
                        <span className="legend-label-full">{category}</span>
                        <span className="legend-stats">
                          <span className="legend-percentage">{percentage}%</span>
                          <span className="legend-value">{mems.length} memories</span>
                        </span>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Bottom Statistics Cards */}
            <div className="category-stats-cards">
              <div className="category-stat-card">
                <div className="stat-card-icon">
                  <i className="fas fa-layer-group"></i>
                </div>
                <div className="stat-card-content">
                  <div className="stat-card-value">{graphData.categories.size}</div>
                  <div className="stat-card-label">Categories</div>
                </div>
              </div>
              <div className="category-stat-card">
                <div className="stat-card-icon">
                  <i className="fas fa-database"></i>
                </div>
                <div className="stat-card-content">
                  <div className="stat-card-value">{memories.length}</div>
                  <div className="stat-card-label">Total Memories</div>
                </div>
              </div>
              <div className="category-stat-card">
                <div className="stat-card-icon">
                  <i className="fas fa-chart-line"></i>
                </div>
                <div className="stat-card-content">
                  <div className="stat-card-value">
                    {Math.max(...Array.from(graphData.categories.values()).map(m => m.length))}
                  </div>
                  <div className="stat-card-label">Largest Category</div>
                </div>
              </div>
            </div>
          </div>
        )}

        {activeView === 'timeline' && (
          <div className="timeline-chart-full">
            {/* Date Range Selector */}
            <div className="timeline-controls-full">
              <div className="timeline-stat-box">
                <div className="timeline-stat-value">{timelineData.length}</div>
                <div className="timeline-stat-label">Days Tracked</div>
              </div>
              <div className="timeline-stat-box">
                <div className="timeline-stat-value">
                  {Math.max(...timelineData.map(([, m]) => m.length))}
                </div>
                <div className="timeline-stat-label">Peak Day</div>
              </div>
              <div className="timeline-stat-box">
                <div className="timeline-stat-value">
                  {(memories.length / Math.max(timelineData.length, 1)).toFixed(1)}
                </div>
                <div className="timeline-stat-label">Avg/Day</div>
              </div>
              <div className="timeline-stat-box">
                <div className="timeline-stat-value">
                  {heatmapData.filter(d => d.count > 0).length}
                </div>
                <div className="timeline-stat-label">Active Days</div>
              </div>
            </div>

            {/* Wide Timeline Chart */}
            <div className="timeline-container-wide">
              {timelineData.map(([date, mems]) => {
                const maxCount = Math.max(...timelineData.map(([, m]) => m.length));
                const barHeight = (mems.length / maxCount) * 100;

                return (
                  <div key={date} className="timeline-item-wide">
                    <div className="timeline-date-wide">
                      {date}
                    </div>
                    <div className="timeline-bar-wrapper-wide">
                      <div
                        className="timeline-bar-wide"
                        style={{ height: `${Math.max(barHeight, 5)}%` }}
                        title={`${date}: ${mems.length} memories`}
                      >
                        <div className="timeline-bar-inner-wide"></div>
                      </div>
                    </div>
                    <div className="timeline-count-wide">{mems.length}</div>
                  </div>
                );
              })}
            </div>

            {/* Density Visualization */}
            <div className="timeline-density">
              <h3 className="density-title">Activity Density</h3>
              <div className="density-bars">
                {timelineData.slice(-10).map(([date, mems]) => {
                  const maxCount = Math.max(...timelineData.map(([, m]) => m.length));
                  const density = (mems.length / maxCount) * 100;
                  return (
                    <div key={date} className="density-bar-wrapper">
                      <div className="density-bar" style={{ height: `${density}%` }}></div>
                      <span className="density-label">{new Date(date).toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' })}</span>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        )}

        {activeView === 'heatmap' && (
          <div className="heatmap-chart-full">
            {/* Header Statistics */}
            <div className="heatmap-stats-header">
              <div className="heatmap-stat-box">
                <div className="heatmap-stat-value">{heatmapData.filter(d => d.count > 0).length}</div>
                <div className="heatmap-stat-label">Active Days</div>
              </div>
              <div className="heatmap-stat-box">
                <div className="heatmap-stat-value">{memories.length}</div>
                <div className="heatmap-stat-label">Total Memories</div>
              </div>
              <div className="heatmap-stat-box">
                <div className="heatmap-stat-value">
                  {(() => {
                    let streak = 0;
                    for (let i = heatmapData.length - 1; i >= 0; i--) {
                      if (heatmapData[i].count > 0) streak++;
                      else break;
                    }
                    return streak;
                  })()}
                </div>
                <div className="heatmap-stat-label">Current Streak</div>
              </div>
              <div className="heatmap-stat-box">
                <div className="heatmap-stat-value">
                  {Math.max(...heatmapData.map(d => d.count))}
                </div>
                <div className="heatmap-stat-label">Best Day</div>
              </div>
            </div>

            {/* Large Heatmap */}
            <div className="heatmap-container-large">
              <div className="heatmap-legend-large">
                <span>Less</span>
                {[0, 1, 2, 3, 4].map(level => (
                  <div key={level} className={`heatmap-legend-cell-large level-${level}`}></div>
                ))}
                <span>More</span>
              </div>
              <div className="heatmap-grid-large">
                {heatmapData.map((day, index) => (
                  <div
                    key={index}
                    className={`heatmap-cell-large level-${day.level}`}
                    title={`${day.date}: ${day.count} memories`}
                  ></div>
                ))}
              </div>
            </div>

            {/* Contribution Summary Cards */}
            <div className="heatmap-summary-cards">
              <div className="heatmap-summary-card">
                <div className="summary-card-icon">
                  <i className="fas fa-fire"></i>
                </div>
                <div className="summary-card-content">
                  <div className="summary-card-value">{heatmapData.filter(d => d.level >= 3).length}</div>
                  <div className="summary-card-label">High Activity Days</div>
                </div>
              </div>
              <div className="heatmap-summary-card">
                <div className="summary-card-icon">
                  <i className="fas fa-calendar-check"></i>
                </div>
                <div className="summary-card-content">
                  <div className="summary-card-value">{heatmapData.filter(d => d.count > 0).length}</div>
                  <div className="summary-card-label">Total Active Days</div>
                </div>
              </div>
              <div className="heatmap-summary-card">
                <div className="summary-card-icon">
                  <i className="fas fa-trophy"></i>
                </div>
                <div className="summary-card-content">
                  <div className="summary-card-value">{Math.max(...heatmapData.map(d => d.count))}</div>
                  <div className="summary-card-label">Peak Memories (Day)</div>
                </div>
              </div>
              <div className="heatmap-summary-card">
                <div className="summary-card-icon">
                  <i className="fas fa-chart-line"></i>
                </div>
                <div className="summary-card-content">
                  <div className="summary-card-value">
                    {(memories.length / 365).toFixed(1)}
                  </div>
                  <div className="summary-card-label">Avg/Day</div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
