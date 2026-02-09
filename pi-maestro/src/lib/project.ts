/**
 * Maestro project file I/O operations
 *
 * Handles reading and writing maestro project files like:
 * - product.md
 * - tech-stack.md
 * - workflow.md
 * - tracks.md
 */

import * as fs from "fs";
import * as path from "path";

/** Find maestro project root by looking for maestro/tracks/ directory */
export function findMaestroProjectRoot(startDir: string): string | null {
  let currentDir = path.resolve(startDir);
  while (currentDir !== "/" && currentDir !== ".") {
    const maestroDir = path.join(currentDir, "maestro");
    const tracksDir = path.join(currentDir, "maestro/tracks");
    if (fs.existsSync(maestroDir) && fs.existsSync(tracksDir)) {
      return currentDir;
    }
    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) break;
    currentDir = parentDir;
  }
  return null;
}

/** Read maestro project files */
export interface MaestroProject {
  root: string;
  product: string;
  techStack: string;
  workflow: string;
  tracks: string;
}

export function readMaestroProject(root: string): MaestroProject {
  return {
    root,
    product: fs.readFileSync(path.join(root, "maestro/product.md"), "utf-8"),
    techStack: fs.readFileSync(path.join(root, "maestro/tech-stack.md"), "utf-8"),
    workflow: fs.readFileSync(path.join(root, "maestro/workflow.md"), "utf-8"),
    tracks: fs.readFileSync(path.join(root, "maestro/tracks.md"), "utf-8"),
  };
}

/** Read a single maestro file */
export function readMaestroFile(root: string, relativePath: string): string {
  const fullPath = path.join(root, "maestro", relativePath);
  return fs.readFileSync(fullPath, "utf-8");
}

/** Write maestro project files */
export function writeMaestroFile(root: string, relativePath: string, content: string): void {
  const fullPath = path.join(root, "maestro", relativePath);
  fs.mkdirSync(path.dirname(fullPath), { recursive: true });
  fs.writeFileSync(fullPath, content, "utf-8");
}

/** Check if maestro project exists */
export function maestroProjectExists(root: string): boolean {
  const productPath = path.join(root, "maestro/product.md");
  const techStackPath = path.join(root, "maestro/tech-stack.md");
  const workflowPath = path.join(root, "maestro/workflow.md");
  const tracksPath = path.join(root, "maestro/tracks.md");
  return (
    fs.existsSync(productPath) &&
    fs.existsSync(techStackPath) &&
    fs.existsSync(workflowPath) &&
    fs.existsSync(tracksPath)
  );
}

/** Parse tracks.md to extract track list */
export interface TrackEntry {
  description: string;
  trackId: string;
  status: "new" | "in_progress" | "completed";
}

export function parseTracksRegistry(tracksContent: string): TrackEntry[] {
  const entries: TrackEntry[] = [];
  const lines = tracksContent.split("\n");

  for (const line of lines) {
    // Match: ## [ ] Track: Description or ## [~] Track: Description or ## [x] Track: Description
    const match = line.match(/^##\[([ ~x])\] Track:\s*(.+)$/);
    if (match) {
      const statusChar = match[1];
      let status: TrackEntry["status"] = "new";
      if (statusChar === "~") status = "in_progress";
      if (statusChar === "x") status = "completed";

      // Extract track ID from link on next line
      const description = match[2].trim();
      entries.push({ description, trackId: "", status });
    }
  }

  // Extract track IDs from links
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const linkMatch = line.match(/\[\.\/maestro\/tracks\/([^/]+)\//);
    if (linkMatch && entries[i - 1]) {
      entries[i - 1].trackId = linkMatch[1];
    }
  }

  return entries;
}

/** Update track status in tracks.md */
export function updateTrackStatus(root: string, trackId: string, status: TrackEntry["status"]): void {
  const tracksPath = path.join(root, "maestro/tracks.md");
  let content = fs.readFileSync(tracksPath, "utf-8");
  const lines = content.split("\n");

  let statusChar = " ";
  if (status === "in_progress") statusChar = "~";
  if (status === "completed") statusChar = "x";

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    // Check if this line contains the track ID link
    if (line.includes(`./maestro/tracks/${trackId}/`)) {
      // Update the previous line which contains the status marker
      if (i > 0) {
        lines[i - 1] = lines[i - 1].replace(/^##\[.?\]/, `##[${statusChar}]`);
      }
      break;
    }
  }

  fs.writeFileSync(tracksPath, lines.join("\n"), "utf-8");
}
