import { useMemo } from 'react';
import { Memory } from '../types';
import './MemoryGraph.css';

interface MemoryGraphProps {
  memories: Memory[];
}

export const MemoryGraph: React.FC<MemoryGraphProps> = ({ memories }) => {
  const graphData = useMemo(() => {
    const categories = new Map<string, Memory[]>();
    const commands = new Map<string, Memory[]>();

    memories.forEach(memory => {
      if (!categories.has(memory.category)) {
        categories.set(memory.category, []);
      }
      categories.get(memory.category)!.push(memory);

      if (!commands.has(memory.command)) {
        commands.set(memory.command, []);
      }
      commands.get(memory.command)!.push(memory);
    });

    return { categories, commands };
  }, [memories]);

  return (
    <div className="memory-graph">
      <div className="graph-section">
        <h3 className="graph-title">Memories by Category</h3>
        <div className="graph-nodes">
          {Array.from(graphData.categories.entries()).map(([category, mems]) => (
            <div key={category} className="graph-node category-node">
              <div className="node-label">{category}</div>
              <div className="node-count">{mems.length}</div>
              {mems.slice(0, 3).map(mem => (
                <div key={mem.id} className="node-connection">
                  <div className="connection-line" />
                  <div className="connected-memory">
                    <span className="memory-command">{mem.command}</span>
                    <span className="memory-preview">{mem.content.slice(0, 30)}...</span>
                  </div>
                </div>
              ))}
            </div>
          ))}
        </div>
      </div>

      <div className="graph-section">
        <h3 className="graph-title">Memories by Command</h3>
        <div className="graph-commands">
          {Array.from(graphData.commands.entries())
            .sort((a, b) => b[1].length - a[1].length)
            .slice(0, 10)
            .map(([command, mems]) => (
              <div key={command} className="command-bar">
                <div className="command-label">{command}</div>
                <div className="bar-container">
                  <div
                    className="bar-fill"
                    style={{ width: `${(mems.length / memories.length) * 100}%` }}
                  />
                </div>
                <div className="command-count">{mems.length}</div>
              </div>
            ))}
        </div>
      </div>
    </div>
  );
};
