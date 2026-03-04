/**
 * Pi-Maestro Extension
 *
 * Maestro workflow commands for pi-mono
 * Provides spec-driven development workflows within pi-mono
 */

import type { ExtensionAPI } from "./types";
import * as fs from "fs";
import * as path from "path";

// Import command registration functions
import { registerSetup } from "./commands/setup";
import { registerNewTrack } from "./commands/newTrack";
import { registerImplement } from "./commands/implement";
import { registerOrchestrate } from "./commands/orchestrate";
import { registerStatus } from "./commands/status";
import { registerRevert } from "./commands/revert";
import { registerConfigure } from "./commands/configure";
import { registerTui } from "./commands/tui";
import { registerLeindex } from "./commands/leindex";
import { registerTrackLensTools } from "./tracklens/extension/tools";
import { registerTrackLensCommand } from "./tracklens/extension/command";

// Re-export for external use
export { registerSetup } from "./commands/setup";
export { registerNewTrack } from "./commands/newTrack";
export { registerImplement } from "./commands/implement";
export { registerOrchestrate } from "./commands/orchestrate";
export { registerStatus } from "./commands/status";
export { registerRevert } from "./commands/revert";
export { registerConfigure } from "./commands/configure";
export { registerTui } from "./commands/tui";
export { registerLeindex } from "./commands/leindex";

// Export libraries for external use
export * from "./lib/project";
export * from "./lib/tracks";
export * from "./lib/templates";
export * from "./lib/criticalThink";
export * from "./lib/cli";

/**
 * Check if current directory is in a maestro project
 */
function isInMaestroProject(): boolean {
  let cwd = process.cwd();
  while (cwd !== "/" && cwd !== ".") {
    const maestroDir = path.join(cwd, "maestro");
    const tracksDir = path.join(cwd, "maestro/tracks");
    if (fs.existsSync(maestroDir) && fs.existsSync(tracksDir)) {
      return true;
    }
    cwd = path.dirname(cwd);
  }
  return false;
}

/**
 * Extension entry point - registers all maestro commands
 */
export default function (pi: ExtensionAPI) {
  // Register TrackLens tools and command
  registerTrackLensTools(pi);
  registerTrackLensCommand(pi);

  // Core workflow commands (implement maestro workflows)
  registerSetup(pi, "maestro:setup");
  registerNewTrack(pi, "maestro:newTrack");  // CRITICAL COMMAND
  registerImplement(pi, "maestro:implement");
  registerOrchestrate(pi, "maestro:orchestrate");
  registerStatus(pi, "maestro:status");
  registerRevert(pi, "maestro:revert");
  registerConfigure(pi, "maestro:configure");

  // Augmentation commands (call maestro CLI)
  registerTui(pi, "maestro:tui");
  registerLeindex(pi, "maestro:leindex");

  // Inject context before agent starts so LLM knows about maestro commands
  pi.on("before_agent_start", async (event) => {
    // Only inject context if we're in a maestro project
    if (!isInMaestroProject()) {
      return;
    }

    return {
      message: {
        customType: "maestro-commands",
        content: `# Maestro Workflow Commands

You are in a maestro project. Use these slash commands for maestro workflows:

## Available Commands

- **/maestro:newTrack <description>** - Create a new track with spec.md and plan.md
  - Engages LLM in guided requirements gathering
  - Generates specification and implementation plan

- **/maestro:implement [track_id]** - Execute track implementation
  - Without track_id: Shows incomplete tracks, LLM helps select
  - With track_id: LLM executes next task in plan
  - LLM updates task checkboxes in plan.md as work progresses

- **/maestro:status** - Show all tracks and their progress

- **/maestro:setup** - Initialize/refresh maestro project structure

- **/maestro:orchestrate** - Orchestrate master track (manages sub-tracks)

- **/maestro:revert** - Revert track work (git-aware rollback)

- **/maestro:configure** - Configure maestro settings

- **/maestro:tui** - Launch Rust Cockpit TUI (uses maestro binary)

- **/maestro:leindex** - Code analysis (uses maestro binary)

## IMPORTANT: Slash Commands vs Binary

**ALWAYS use /maestro: slash commands for workflow operations.**

DO NOT run these bash commands for maestro workflows:
- \`maestro newTrack ...\` → Use **/maestro:newTrack** instead
- \`maestro implement ...\` → Use **/maestro:implement** instead
- \`maestro status\` → Use **/maestro:status** instead

The slash commands engage the LLM in guided workflows, while the binary is for:
- TUI: Use /maestro:tui
- LeIndex: Use /maestro:leindex
- Other CLI tools: Use maestro binary directly

## Workflow Example

\`\`\`
/maestro:newTrack Add user authentication
# LLM engages, asks questions, creates track

/maestro:implement add-auth_20250127
# LLM executes tasks one by one, updates plan.md
\`\`\`

## Track Structure

- maestro/tracks/<track_id>/spec.md - Requirements
- maestro/tracks/<track_id>/plan.md - Implementation phases/tasks
- maestro/tracks/<track_id>/metadata.json - Track metadata

Tasks in plan.md use checkboxes:
\`\`\`
## Phase 1: Analysis
- [ ] Task: Analyze requirements
- [x] Task: Review existing code
\`\`\`

LLM updates checkboxes with edit tool as work progresses.`,
        display: false,
      },
    };
  });
}
