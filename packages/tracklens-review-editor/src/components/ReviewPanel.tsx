/**
 * TrackLens Review Panel Component
 * Displays annotations and export/submit actions
 */

import React from "react";
import type { CodeAnnotation } from "@maestro/tracklens-ui";

interface ReviewPanelProps {
  annotations: CodeAnnotation[];
  onDeleteAnnotation: (id: string) => void;
  onExport: () => void;
  onSubmit: () => void;
}

export function ReviewPanel({
  annotations,
  onDeleteAnnotation,
  onExport,
  onSubmit,
}: ReviewPanelProps) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        padding: "16px",
        overflow: "hidden",
      }}
    >
      <h2 style={{ margin: "0 0 16px 0" }}>Review</h2>

      <div
        style={{
          flex: 1,
          overflow: "auto",
          marginBottom: "16px",
        }}
      >
        {annotations.length === 0 ? (
          <div
            style={{
              textAlign: "center",
              padding: "40px 20px",
              color: "var(--text-secondary)",
            }}
          >
            No annotations yet. Click on diff lines to add comments.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            {annotations.map((annotation) => (
              <div
                key={annotation.id}
                style={{
                  padding: "12px",
                  backgroundColor: "var(--bg-secondary)",
                  borderRadius: "4px",
                  borderLeft: `3px solid ${
                    annotation.type === "comment" ? "#3b82f6" : "#ef4444"
                  }`,
                }}
              >
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    marginBottom: "8px",
                  }}
                >
                  <span style={{ fontSize: "13px", fontWeight: 500 }}>
                    {annotation.filePath}:{annotation.lineStart}
                  </span>
                  <button
                    onClick={() => onDeleteAnnotation(annotation.id)}
                    style={{
                      background: "none",
                      border: "none",
                      color: "var(--text-secondary)",
                      cursor: "pointer",
                      padding: "4px 8px",
                    }}
                  >
                    ×
                  </button>
                </div>
                <div style={{ fontSize: "14px" }}>{annotation.text}</div>
                {annotation.author && (
                  <div
                    style={{
                      marginTop: "8px",
                      fontSize: "12px",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {annotation.author}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      <div
        style={{
          display: "flex",
          gap: "8px",
          paddingTop: "16px",
          borderTop: "1px solid var(--border-color)",
        }}
      >
        <button
          onClick={onExport}
          disabled={annotations.length === 0}
          style={{
            flex: 1,
            padding: "10px",
            backgroundColor: "var(--bg-secondary)",
            border: "1px solid var(--border-color)",
            borderRadius: "4px",
            cursor: annotations.length === 0 ? "not-allowed" : "pointer",
          }}
        >
          Export
        </button>
        <button
          onClick={onSubmit}
          style={{
            flex: 1,
            padding: "10px",
            backgroundColor: "#3b82f6",
            color: "white",
            border: "none",
            borderRadius: "4px",
            cursor: "pointer",
          }}
        >
          Submit
        </button>
      </div>
    </div>
  );
}
