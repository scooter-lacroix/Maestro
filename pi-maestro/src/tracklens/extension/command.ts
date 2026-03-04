/**
 * TrackLens Command for Pi-Maestro
 *
 * Registers the /tracklens command for toggling TrackLens behavior.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * Command: /tracklens [on|off]
 * - Toggle TrackLens walkthrough reviews on/off
 * - Default: ON
 *
 * @packageDocumentation
 */

import type { ExtensionAPI } from "../../types";

/**
 * TrackLens state
 */
let trackLensEnabled = true;

/**
 * Register /tracklens command with pi-maestro extension
 *
 * The /tracklens command allows users to toggle TrackLens behavior:
 * - /tracklens - Show current status
 * - /tracklens on - Enable TrackLens walkthroughs
 * - /tracklens off - Disable TrackLens walkthroughs
 */
export function registerTrackLensCommand(pi: ExtensionAPI) {
  pi.registerCommand("tracklens", {
    description: "Toggle TrackLens walkthrough reviews (default: on)",
    handler: async (args, ctx) => {
      const arg = args.trim().toLowerCase();

      if (arg === "on") {
        trackLensEnabled = true;
        ctx.ui.notify("TrackLens: Walkthrough reviews ENABLED", "info");
        return;
      }

      if (arg === "off") {
        trackLensEnabled = false;
        ctx.ui.notify("TrackLens: Walkthrough reviews DISABLED", "warning");
        return;
      }

      // Show current status
      const status = trackLensEnabled ? "ENABLED" : "DISABLED";
      ctx.ui.notify(`TrackLens: Walkthrough reviews are ${status}`, "info");
    },
  });
}

/**
 * Check if TrackLens is enabled
 *
 * @returns true if TrackLens walkthroughs are enabled
 */
export function isTrackLensEnabled(): boolean {
  return trackLensEnabled;
}

/**
 * Set TrackLens enabled state
 *
 * @param enabled - Whether TrackLens should be enabled
 */
export function setTrackLensEnabled(enabled: boolean): void {
  trackLensEnabled = enabled;
}
