/**
 * /maestro:orchestrate command
 *
 * Orchestrate master track
 * Manages sub-tracks, monitors progress, handles failures
 */

import type { ExtensionAPI } from "../types";
import {
  findMaestroProjectRoot,
  updateTrackStatus,
} from "../lib/project";
import {
  readTrackMetadata,
  updateTrackMetadata,
} from "../lib/tracks";
import {
  applyCriticalThinkForImplementation,
  applyCriticalThinkAfterAction,
} from "../lib/criticalThink";
import { isTrackLensEnabled } from "../tracklens/extension/command";
import * as fs from "fs";
import * as path from "path";

/**
 * Register /maestro:orchestrate command
 */
export function registerOrchestrate(pi: ExtensionAPI, commandName: string) {
  pi.registerCommand(commandName, {
    description: "Orchestrate a master track (manage sub-tracks)",
    handler: async (args, ctx) => {
      const trackId = args.trim();
      if (!trackId) {
        ctx.ui.notify("Usage: /maestro:orchestrate <master_track_id>", "error");
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

      const metadata = readTrackMetadata(root, trackId);

      if (metadata.type !== "master") {
        ctx.ui.notify(`Track is not a master track: ${trackId}`, "error");
        return;
      }

      // Update master track status
      updateTrackStatus(root, trackId, "in_progress");
      updateTrackMetadata(root, trackId, { status: "in_progress" });


      // Get sub-tracks
      const subtracks = metadata.subtracks || [];
      if (subtracks.length === 0) {
        ctx.ui.notify("No sub-tracks found in master track", "info");
        return;
      }

      // Orchestrate each sub-track
      let completedCount = 0;
      let failedCount = 0;

      for (const subtrackId of subtracks) {

        const result = await orchestrateSubtrack(root, subtrackId, ctx);

        if (result === "completed") {
          completedCount++;

          // TrackLens walkthrough for completed sub-track (if enabled)
          if (isTrackLensEnabled()) {
            ctx.ui.notify(`Requesting TrackLens walkthrough for ${subtrackId}...`, "info");
            // The implement workflow handles walkthrough; this is just a notification
          }
        } else if (result === "failed") {
          failedCount++;
        }
      }

      // Update master track status
      if (failedCount === 0) {
        updateTrackStatus(root, trackId, "completed");
        updateTrackMetadata(root, trackId, { status: "completed" });

        // TrackLens final walkthrough for master track
        if (isTrackLensEnabled()) {
          ctx.ui.notify("All sub-tracks complete. TrackLens walkthrough available.", "info");
        }

        ctx.ui.notify(`Master track completed: ${trackId}`, "info");
      } else {
        ctx.ui.notify(
          `Master track partial completion: ${completedCount}/${subtracks.length} sub-tracks done`,
          "warning"
        );
      }
    },
  });
}

/**
 * Orchestrate a single sub-track
 */
async function orchestrateSubtrack(
  root: string,
  subtrackId: string,
  ctx: any
): Promise<"completed" | "in_progress" | "failed"> {
  const subtrackDir = path.join(root, "maestro/tracks", subtrackId);
  if (!fs.existsSync(subtrackDir)) {
    return "failed";
  }

  const metadata = readTrackMetadata(root, subtrackId);

  // Apply Critical Think
  const criticalThinkPrompt = applyCriticalThinkForImplementation(
    `Orchestrating sub-track: ${subtrackId}`,
    `Master track orchestration`
  );

  if (criticalThinkPrompt) {
  }

  // Check sub-track status
  if (metadata.status === "completed") {
    return "completed";
  }

  // Launch sub-track implementation
  // Note: In actual implementation, this would delegate to /maestro:implement
  // or launch a background agent to execute the sub-track

  // Simulate sub-track execution
  // In real implementation, this would use pi-mono's subagent system
  // to execute /maestro:implement in a background context

  // Apply Critical Think after action
  const afterActionAnalysis = applyCriticalThinkAfterAction(
    `Orchestrated sub-track: ${subtrackId}`,
    "Sub-track launched",
    "Sub-track execution in progress"
  );

  if (afterActionAnalysis) {
  }

  return "in_progress";
}
