/**
 * TrackLens File Tree Component
 * Displays list of changed files in the diff
 */

import React from "react";
import type { DiffFile } from "../App";

interface FileTreeProps {
  files: DiffFile[];
  selectedFile: DiffFile | null;
  onSelectFile: (file: DiffFile) => void;
}

export function FileTree({ files, selectedFile, onSelectFile }: FileTreeProps) {
  return (
    <div
      style={{
        borderBottom: "1px solid var(--border-color)",
        maxHeight: "200px",
        overflowY: "auto",
      }}
    >
      {files.map((file, index) => (
        <div
          key={index}
          onClick={() => onSelectFile(file)}
          style={{
            padding: "8px 12px",
            cursor: "pointer",
            backgroundColor:
              selectedFile?.path === file.path ? "var(--bg-secondary)" : "transparent",
            borderBottom: "1px solid var(--border-color)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <span
            style={{
              fontSize: "13px",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {file.oldPath && file.oldPath !== file.path ? (
              <>
                <span style={{ textDecoration: "line-through", color: "var(--text-secondary)" }}>
                  {file.oldPath}
                </span>
                {" → "}
                {file.path}
              </>
            ) : (
              file.path
            )}
          </span>
          <span
            style={{
              fontSize: "11px",
              color: "var(--text-secondary)",
              marginLeft: "8px",
            }}
          >
            <span style={{ color: "#22c55e" }}>+{file.additions}</span>
            {" "}
            <span style={{ color: "#ef4444" }}>-{file.deletions}</span>
          </span>
        </div>
      ))}
    </div>
  );
}
