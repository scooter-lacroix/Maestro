/**
 * Track creation and management operations
 *
 * Handles:
 * - Track ID generation (shortname_YYYYMMDD format)
 * - Track directory creation (spec.md, plan.md, metadata.json)
 * - Track registry updates (tracks.md)
 * - Plan parsing for task status
 */

import * as fs from "fs";
import * as path from "path";

export interface TrackMetadata {
  track_id: string;
  type: "feature" | "bug" | "chore" | "refactor" | "master";
  status: "new" | "in_progress" | "completed" | "cancelled";
  created_at: string;
  updated_at: string;
  description: string;
  maestro_project_id?: number;
  maestro_track_id?: number;
  subtracks?: string[];
}

export interface PlanTask {
  phase: string;
  task: string;
  status: "pending" | "in_progress" | "completed";
  line: number;
}

export interface PlanPhase {
  name: string;
  tasks: PlanTask[];
  status: "pending" | "in_progress" | "completed";
}

/** Generate unique track ID: shortname_YYYYMMDD */
export function generateTrackId(description: string): string {
  const date = new Date().toISOString().slice(0, 10).replace(/-/g, "");
  const shortname = description
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 30);
  return `${shortname}_${date}`;
}

/** Create track directory with spec.md, plan.md, metadata.json */
export function createTrack(
  root: string,
  trackId: string,
  spec: string,
  plan: string,
  metadata: TrackMetadata
): void {
  const trackDir = path.join(root, "maestro/tracks", trackId);
  fs.mkdirSync(trackDir, { recursive: true });

  fs.writeFileSync(path.join(trackDir, "spec.md"), spec, "utf-8");
  fs.writeFileSync(path.join(trackDir, "plan.md"), plan, "utf-8");
  fs.writeFileSync(
    path.join(trackDir, "metadata.json"),
    JSON.stringify(metadata, null, 2),
    "utf-8"
  );
}

/** Update track metadata (e.g., status change) */
export function updateTrackMetadata(root: string, trackId: string, updates: Partial<TrackMetadata>): void {
  const metadataPath = path.join(root, "maestro/tracks", trackId, "metadata.json");
  const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf-8")) as TrackMetadata;
  const updated = { ...metadata, ...updates, updated_at: new Date().toISOString() };
  fs.writeFileSync(metadataPath, JSON.stringify(updated, null, 2), "utf-8");
}

/** Read track metadata */
export function readTrackMetadata(root: string, trackId: string): TrackMetadata {
  const metadataPath = path.join(root, "maestro/tracks", trackId, "metadata.json");
  return JSON.parse(fs.readFileSync(metadataPath, "utf-8")) as TrackMetadata;
}

/** Add track to tracks.md registry */
export function addTrackToRegistry(
  root: string,
  trackId: string,
  description: string
): void {
  const tracksPath = path.join(root, "maestro/tracks.md");
  const tracksContent = fs.readFileSync(tracksPath, "utf-8");

  const newEntry = `

---

## [ ] Track: ${description}
*Link: [./maestro/tracks/${trackId}/](./maestro/tracks/${trackId}/)*
`;

  fs.appendFileSync(tracksPath, newEntry, "utf-8");
}

/** Parse plan.md to extract tasks and their status */
export function parsePlan(planContent: string): PlanPhase[] {
  const phases: PlanPhase[] = [];
  const lines = planContent.split("\n");
  let currentPhase: PlanPhase | null = null;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Match phase headers: ## Phase 1: Name
    const phaseMatch = line.match(/^##\s+Phase\s+\d+:\s*(.+)$/);
    if (phaseMatch) {
      if (currentPhase) {
        phases.push(currentPhase);
      }
      currentPhase = { name: phaseMatch[1].trim(), tasks: [], status: "pending" };
      continue;
    }

    // Match tasks: - [ ] Task: Description or - [x] Task: Description
    const taskMatch = line.match(/^-\s\[([ x])\]\s+Task:\s*(.+)$/);
    if (taskMatch && currentPhase) {
      const statusChar = taskMatch[1];
      let status: PlanTask["status"] = "pending";
      if (statusChar === "x") status = "completed";

      currentPhase.tasks.push({
        phase: currentPhase.name,
        task: taskMatch[2].trim(),
        status,
        line: i + 1,
      });
    }
  }

  if (currentPhase) {
    phases.push(currentPhase);
  }

  // Calculate phase status based on tasks
  for (const phase of phases) {
    if (phase.tasks.length === 0) {
      phase.status = "pending";
      continue;
    }

    const completedTasks = phase.tasks.filter(t => t.status === "completed").length;
    const inProgressTasks = phase.tasks.filter(t => t.status === "in_progress").length;

    if (completedTasks === phase.tasks.length) {
      phase.status = "completed";
    } else if (completedTasks > 0 || inProgressTasks > 0) {
      phase.status = "in_progress";
    } else {
      phase.status = "pending";
    }
  }

  return phases;
}

/** Update task status in plan.md */
export function updateTaskStatus(
  root: string,
  trackId: string,
  phaseName: string,
  taskDescription: string,
  status: "pending" | "in_progress" | "completed"
): void {
  const planPath = path.join(root, "maestro/tracks", trackId, "plan.md");
  let content = fs.readFileSync(planPath, "utf-8");
  const lines = content.split("\n");

  let inTargetPhase = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Check if we're entering the target phase
    const phaseMatch = line.match(/^##\s+Phase\s+\d+:\s*(.+)$/);
    if (phaseMatch) {
      inTargetPhase = phaseMatch[1].trim().toLowerCase() === phaseName.toLowerCase();
      continue;
    }

    // Update task status if in target phase
    if (inTargetPhase) {
      const taskMatch = line.match(/^-\s\[([ x])\]\s+Task:\s*(.+)$/);
      if (taskMatch) {
        const taskDesc = taskMatch[2].trim();
        if (taskDesc.toLowerCase() === taskDescription.toLowerCase()) {
          let statusChar = " ";
          if (status === "completed") statusChar = "x";
          lines[i] = line.replace(/^-\s\[.\]/, `- [${statusChar}]`);
          break;
        }
      }
    }
  }

  fs.writeFileSync(planPath, lines.join("\n"), "utf-8");
}

/** Calculate track progress percentage */
export function calculateTrackProgress(phases: PlanPhase[]): number {
  if (phases.length === 0) return 0;

  let totalTasks = 0;
  let completedTasks = 0;

  for (const phase of phases) {
    totalTasks += phase.tasks.length;
    completedTasks += phase.tasks.filter(t => t.status === "completed").length;
  }

  if (totalTasks === 0) return 0;
  return Math.round((completedTasks / totalTasks) * 100);
}

/** Update phase status in plan.md */
export function updatePhaseStatus(
  root: string,
  trackId: string,
  phaseName: string,
  status: "pending" | "in_progress" | "completed"
): void {
  const planPath = path.join(root, "maestro/tracks", trackId, "plan.md");
  const content = fs.readFileSync(planPath, "utf-8");
  const lines = content.split("\n");

  // Find and update the phase header
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const phaseMatch = line.match(/^##\s+Phase\s+\d+:\s*(.+)$/);
    if (phaseMatch) {
      const currentPhaseName = phaseMatch[1].trim();
      if (currentPhaseName.toLowerCase() === phaseName.toLowerCase()) {
        // Phase headers don't have status inline, status is derived from tasks
        // This function is called when all tasks in a phase are complete
        // The parsePlan function will calculate the correct status
        break;
      }
    }
  }

  // Status is calculated by parsePlan based on task completion
  // Just update the task statuses, parsePlan will handle phase status
  if (status === "completed") {
    // Mark all tasks in this phase as completed
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const phaseMatch = line.match(/^##\s+Phase\s+\d+:\s*(.+)$/);
      if (phaseMatch) {
        const currentPhaseName = phaseMatch[1].trim();
        const inTargetPhase = currentPhaseName.toLowerCase() === phaseName.toLowerCase();

        if (inTargetPhase) {
          const taskMatch = lines[i]?.match(/^-\s\[([ x])\]\s+Task:/);
          if (taskMatch && lines[i]) {
            lines[i] = lines[i].replace(/^-\s\[.\]/, `- [x]`);
          }
        }
      }
    }
    fs.writeFileSync(planPath, lines.join("\n"), "utf-8");
  }
}

/** List all tracks in project */
export function listAllTracks(root: string): string[] {
  const tracksDir = path.join(root, "maestro/tracks");
  if (!fs.existsSync(tracksDir)) return [];

  return fs.readdirSync(tracksDir, { withFileTypes: true })
    .filter(dirent => dirent.isDirectory())
    .map(dirent => dirent.name);
}
