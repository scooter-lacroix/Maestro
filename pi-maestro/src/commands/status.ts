/**
 * /maestro:status command
 *
 * Show project progress
 * Reads tracks.md and all plan.md files to display progress summary
 */

import type { ExtensionAPI } from "../types";
import {
  findMaestroProjectRoot,
  parseTracksRegistry,
} from "../lib/project";
import {
  listAllTracks,
  readTrackMetadata,
  parsePlan,
  calculateTrackProgress,
} from "../lib/tracks";
import * as fs from "fs";
import * as path from "path";

/**
 * Register /maestro:status command
 */
export function registerStatus(pi: ExtensionAPI, commandName: string) {
  pi.registerCommand(commandName, {
    description: "Show maestro project progress and status",
    handler: async (args, ctx) => {
      const root = findMaestroProjectRoot(process.cwd());
      if (!root) {
        ctx.ui.notify("Not in a maestro project", "error");
        return;
      }

      // Read tracks registry
      const tracksContent = fs.readFileSync(path.join(root, "maestro/tracks.md"), "utf-8");
      const trackEntries = parseTracksRegistry(tracksContent);

      if (trackEntries.length === 0) {
        ctx.ui.notify("No tracks found. Use /maestro:newTrack to create a track.", "info");
        return;
      }

      // Build status report
      let statusReport = "# Maestro Project Status\n\n";

      // Summary statistics
      const totalTracks = trackEntries.length;
      const completedTracks = trackEntries.filter(t => t.status === "completed").length;
      const inProgressTracks = trackEntries.filter(t => t.status === "in_progress").length;
      const newTracks = trackEntries.filter(t => t.status === "new").length;

      statusReport += `## Summary\n\n`;
      statusReport += `- Total Tracks: ${totalTracks}\n`;
      statusReport += `- Completed: ${completedTracks}\n`;
      statusReport += `- In Progress: ${inProgressTracks}\n`;
      statusReport += `- New: ${newTracks}\n\n`;

      // Per-track details
      statusReport += `## Track Details\n\n`;

      const trackIds = listAllTracks(root);

      for (const trackId of trackIds) {
        const metadata = readTrackMetadata(root, trackId);
        const planContent = fs.readFileSync(
          path.join(root, "maestro/tracks", trackId, "plan.md"),
          "utf-8"
        );
        const phases = parsePlan(planContent);
        const progress = calculateTrackProgress(phases);

        const statusChar = metadata.status === "completed" ? "✓" :
                          metadata.status === "in_progress" ? "⟳" : "○";

        statusReport += `### ${statusChar} ${metadata.description}\n`;
        statusReport += `- **Track ID:** ${trackId}\n`;
        statusReport += `- **Type:** ${metadata.type}\n`;
        statusReport += `- **Status:** ${metadata.status}\n`;
        statusReport += `- **Progress:** ${progress}%\n`;

        // Show current phase if in progress
        if (metadata.status === "in_progress") {
          const currentPhase = phases.find(p => p.status === "in_progress");
          if (currentPhase) {
            const completedTasks = currentPhase.tasks.filter(t => t.status === "completed").length;
            const totalTasks = currentPhase.tasks.length;
            statusReport += `- **Current Phase:** ${currentPhase.name} (${completedTasks}/${totalTasks} tasks)\n`;
          }
        }

        statusReport += "\n";
      }

      // Display the status report as a widget
      ctx.ui.setWidget("maestro-status", statusReport.split("\n"));
    },
  });
}
