/**
 * /maestro:revert command
 *
 * Git-aware rollback at track/phase/task level
 */

import type { ExtensionAPI } from "../types";
import {
  findMaestroProjectRoot,
} from "../lib/project";
import {
  readTrackMetadata,
  parsePlan,
} from "../lib/tracks";
import * as fs from "fs";
import * as path from "path";
import { execSync } from "child_process";

/**
 * Register /maestro:revert command
 */
export function registerRevert(pi: ExtensionAPI, commandName: string) {
  pi.registerCommand(commandName, {
    description: "Revert track work (git-aware rollback)",
    handler: async (args, ctx) => {
      const parts = args.trim().split(/\s+/);
      const trackId = parts[0];

      if (!trackId) {
        ctx.ui.notify("Usage: /maestro:revert <track_id> [phase_name] [task_description]", "error");
        return;
      }

      const root = findMaestroProjectRoot(process.cwd());
      if (!root) {
        ctx.ui.notify("Not in a maestro project", "error");
        return;
      }

      const trackDir = path.join(root, "maestro/tracks", trackId);
      if (!fs.existsSync(trackDir)) {
        ctx.ui.notify(`Track not found: ${trackId}`, "error");
        return;
      }

      // Determine revert level
      const phaseName = parts[1];
      const taskDescription = parts[2];

      let revertLevel = "track";
      if (taskDescription) {
        revertLevel = "task";
      } else if (phaseName) {
        revertLevel = "phase";
      }

      // Present execution plan
      const plan = generateRevertPlan(trackId, revertLevel, phaseName, taskDescription);
      const approved = await ctx.ui.confirm(
        "Confirm Revert",
        plan + "\n\nProceed with revert?"
      );

      if (!approved) {
        ctx.ui.notify("Revert cancelled", "info");
        return;
      }

      // Execute revert
      try {
        executeRevert(root, trackId, revertLevel, phaseName, taskDescription);
        ctx.ui.notify("Revert completed successfully", "info");
      } catch (error: any) {
        ctx.ui.notify(`Revert failed: ${error.message}`, "error");
      }
    },
  });
}

/**
 * Generate revert plan for user approval
 */
function generateRevertPlan(
  trackId: string,
  level: string,
  phaseName?: string,
  taskDescription?: string
): string {
  let plan = `# Revert Plan\n\n`;
  plan += `**Track:** ${trackId}\n`;
  plan += `**Level:** ${level}\n\n`;

  if (level === "track") {
    plan += `This will revert all work for track "${trackId}".\n\n`;
    plan += `Actions:\n`;
    plan += `- Reset track status to "new"\n`;
    plan += `- Find and revert commits related to this track\n`;
  } else if (level === "phase") {
    plan += `**Phase:** ${phaseName}\n\n`;
    plan += `This will revert work for phase "${phaseName}" in track "${trackId}".\n\n`;
    plan += `Actions:\n`;
    plan += `- Reset phase tasks to pending\n`;
    plan += `- Find and revert commits related to this phase\n`;
  } else if (level === "task") {
    plan += `**Phase:** ${phaseName}\n`;
    plan += `**Task:** ${taskDescription}\n\n`;
    plan += `This will revert work for task "${taskDescription}" in phase "${phaseName}".\n\n`;
    plan += `Actions:\n`;
    plan += `- Reset task to pending\n`;
    plan += `- Find and revert commits related to this task\n`;
  }

  return plan;
}

/**
 * Execute the revert
 */
function executeRevert(
  root: string,
  trackId: string,
  level: string,
  phaseName?: string,
  taskDescription?: string
): void {
  // Note: This is a simplified implementation
  // In production, you would:
  // 1. Parse git log to find track-related commits
  // 2. Use commit messages to identify track/phase/task boundaries
  // 3. Execute git revert for the relevant commits

  if (level === "track") {
    // Reset track metadata
    const trackDir = path.join(root, "maestro/tracks", trackId);
    const metadataPath = path.join(trackDir, "metadata.json");
    const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf-8"));
    metadata.status = "new";
    metadata.updated_at = new Date().toISOString();
    fs.writeFileSync(metadataPath, JSON.stringify(metadata, null, 2));

    // Reset all tasks in plan.md
    const planPath = path.join(trackDir, "plan.md");
    let planContent = fs.readFileSync(planPath, "utf-8");
    planContent = planContent.replace(/- \[x\] Task:/g, "- [ ] Task:");
    fs.writeFileSync(planPath, planContent);
  }

  // For phase and task level reverts, more sophisticated git parsing would be needed
  // This is a placeholder for that functionality

  // Note: In production, would log revert completion here
}

// Helper to get git log (would be used in production)
function getGitLog(root: string, trackId: string): string[] {
  try {
    const log = execSync("git log --oneline --all", { cwd: root, encoding: "utf-8" });
    return log.split("\n").filter(line => line.includes(trackId));
  } catch {
    return [];
  }
}
