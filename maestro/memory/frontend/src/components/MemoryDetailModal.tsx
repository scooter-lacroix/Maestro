import React from 'react';
import { Memory } from '../types';
import './MemoryDetailModal.css';

interface MemoryDetailModalProps {
    memory: Memory;
    onClose: () => void;
}

export const MemoryDetailModal: React.FC<MemoryDetailModalProps> = ({ memory, onClose }) => {
    return (
        <div className="memory-modal-overlay" onClick={onClose}>
            <div className="memory-modal-container" onClick={(e) => e.stopPropagation()}>
                <div className="memory-modal-header">
                    <div className="memory-modal-meta">
                        <span className="memory-command-badge">{memory.command}</span>
                        <span className="memory-category-badge">{memory.category}</span>
                    </div>
                    <button className="memory-modal-close" onClick={onClose}>
                        <i className="fas fa-times" />
                    </button>
                </div>

                <div className="memory-modal-content">
                    <div className="memory-timestamp">
                        Created: {new Date(memory.created_at).toLocaleString()}
                    </div>

                    <div className="memory-body">
                        {memory.content}
                    </div>

                    {memory.labels && memory.labels.length > 0 && (
                        <div className="memory-labels-section">
                            <h4>Labels:</h4>
                            <div className="memory-labels-list">
                                {memory.labels.map((label, idx) => (
                                    <span key={idx} className="memory-label-item">{label}</span>
                                ))}
                            </div>
                        </div>
                    )}

                    {memory.metadata && Object.keys(memory.metadata).length > 0 && (
                        <div className="memory-metadata-section">
                            <h4>Metadata:</h4>
                            <pre className="memory-metadata-pre">
                                {JSON.stringify(memory.metadata, null, 2)}
                            </pre>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
};
