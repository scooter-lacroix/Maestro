/**
 * TrackLens Diff Viewer Component
 * Displays git diff with annotation support
 */

import React from "react";
import ReactDiffViewer from "react-diff-viewer-continued";
import type { DiffFile } from "../App";
import type { CodeAnnotation, CodeAnnotationType } from "@maestro/tracklens-ui";

interface DiffViewerProps {
  diffData: any;
  selectedFile: DiffFile | null;
  annotations: CodeAnnotation[];
  onAddAnnotation: (
    filePath: string,
    startLine: number,
    endLine: number,
    type: CodeAnnotationType,
    content: string
  ) => void;
}

export function DiffViewer({
  diffData,
  selectedFile,
  annotations,
  onAddAnnotation,
}: DiffViewerProps) {
  if (!selectedFile) {
    return (
      <div
        style={{
          padding: "20px",
          textAlign: "center",
          color: "var(--text-secondary)",
        }}
      >
        Select a file to view diff
      </div>
    );
  }

  const handleAddComment = (startLine: number, endLine: number) => {
    const content = prompt("Enter your comment:");
    if (content) {
      onAddAnnotation(selectedFile.path, startLine, endLine, "comment", content);
    }
  };

  const fileAnnotations = annotations.filter((a) => a.filePath === selectedFile.path);

  // Parse the patch to get old and new content
  const oldLines: string[] = [];
  const newLines: string[] = [];
  
  selectedFile.patch.split('\n').forEach(line => {
    if (line.startsWith('-') && !line.startsWith('---')) {
      oldLines.push(line.slice(1));
    } else if (line.startsWith('+') && !line.startsWith('+++')) {
      newLines.push(line.slice(1));
    } else if (!line.startsWith('@@') && !line.startsWith('+++') && !line.startsWith('---') && !line.startsWith('diff')) {
      oldLines.push(line.slice(1));
      newLines.push(line.slice(1));
    }
  });

  return (
    <div
      style={{
        flex: 1,
        overflow: "auto",
        padding: "16px",
      }}
    >
      <div
        style={{
          marginBottom: "16px",
          paddingBottom: "8px",
          borderBottom: "1px solid var(--border-color)",
        }}
      >
        <h3 style={{ margin: 0 }}>{selectedFile.path}</h3>
        <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
          {selectedFile.oldPath && selectedFile.oldPath !== selectedFile.path && (
            <span>
              renamed from {selectedFile.oldPath} •{" "}
            </span>
          )}
          <span>
            {selectedFile.additions} additions, {selectedFile.deletions} deletions
          </span>
        </div>
      </div>

      <ReactDiffViewer
        oldValue={oldLines.join('\n')}
        newValue={newLines.join('\n')}
        splitView={true}
      />

      {/* Annotations */}
      {fileAnnotations.map((annotation) => (
        <div
          key={annotation.id}
          style={{
            marginTop: "16px",
            padding: "12px",
            backgroundColor: "var(--bg-secondary)",
            borderRadius: "4px",
            borderLeft: `3px solid ${
              annotation.type === "comment" ? "#3b82f6" : "#ef4444"
            }`,
          }}
        >
          <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
            Line {annotation.lineStart}
            {annotation.author && ` • ${annotation.author}`}
          </div>
          <div style={{ marginTop: "4px" }}>{annotation.text}</div>
        </div>
      ))}
    </div>
  );
}
