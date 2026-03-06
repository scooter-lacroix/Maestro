/**
 * /maestro:implement command
 *
 * CRITICAL COMMAND - Engages LLM to execute track plan using maestro workflow
 *
 * Architecture: LLM-ENGAGED WORKFLOW
 * - Injects maestro workflow instructions into LLM context
 * - LLM follows the workflow using available tools
 * - Agent delegation adapted for pi-mono subagents
 */

import type { ExtensionAPI } from "../types";
import {
  findMaestroProjectRoot,
  maestroProjectExists,
} from "../lib/project";
import {
  readTrackMetadata,
  parsePlan,
  calculateTrackProgress,
  updateTrackMetadata,
  listAllTracks,
} from "../lib/tracks";
import { isTrackLensEnabled } from "../tracklens/extension/command";
import * as fs from "fs";
import * as path from "path";

/** Store current track context for before_agent_start */
let currentImplementContext: {
  root: string;
  trackId?: string;
  description?: string;
} | null = null;

/**
 * Register /maestro:implement command
 */
export function registerImplement(pi: ExtensionAPI, commandName: string) {
  // Register the command
  pi.registerCommand(commandName, {
    description: "Execute track implementation using maestro workflow",
    handler: async (args, ctx) => {
      const trackId = args.trim();

      const root = findMaestroProjectRoot(process.cwd());
      if (!root) {
        ctx.ui.notify("Not in a maestro project", "error");
        return;
      }

      if (!maestroProjectExists(root)) {
        ctx.ui.notify("Maestro project incomplete. Run /maestro:setup first.", "error");
        return;
      }

      const templatesDir = path.join(root, "maestro/critical_think/templates");
      if (!fs.existsSync(templatesDir)) {
        ctx.ui.notify("Critical Think templates missing. Run /maestro:setup first.", "warning");
      }

      // Store context for before_agent_start event
      currentImplementContext = { root };

      if (!trackId) {
        // No track ID - will show list in workflow
        ctx.ui.notify("Finding next incomplete track...", "info");
      } else {
        // Track ID provided
        currentImplementContext.trackId = trackId;
        ctx.ui.notify(`Starting implementation: ${trackId}`, "info");
      }

      // The workflow will be injected via before_agent_start
      // Just trigger a turn to start the process
      pi.sendMessage(
        {
          customType: "maestro-implement-start",
          content: trackId
            ? `Starting implementation of track: ${trackId}`
            : `No track specified. Please help select the next incomplete track to implement.`,
          display: true,
        },
        { triggerTurn: true }
      );
    },
  });

  // Inject workflow instructions before agent starts
  pi.on("before_agent_start", async (event) => {
    // Only inject if we're in implement mode
    if (!currentImplementContext || !isInMaestroProject(currentImplementContext.root)) {
      return;
    }

    // Check if this message is related to maestro implement
    const isImplementRelated = event.messages.some((m: any) =>
      m.customType === "maestro-implement-start" ||
      m.customType === "maestro-implement-workflow"
    );

    if (!isImplementRelated) {
      return;
    }

    // Build the workflow instructions
    const workflow = buildMaestroWorkflow(currentImplementContext);

    return {
      message: {
        customType: "maestro-implement-workflow",
        content: workflow,
        display: false,
      },
    };
  });
}

function isInMaestroProject(root: string): boolean {
  return fs.existsSync(path.join(root, "maestro/tracks"));
}

function buildMaestroWorkflow(context: { root: string; trackId?: string }): string {
  const { root, trackId } = context;
  const trackLensEnabled = isTrackLensEnabled();

  return `# Maestro Implementation Protocol

You are executing a maestro track implementation. Follow this protocol precisely.

## CRITICAL: User Interaction Methods

**For interactive questions, use these methods based on your available tools:**

**Option A - If you have AskUserQuestion tool (Claude Code, some Codex versions):**
- Use AskUserQuestion with question, header, options, and multiSelect fields

**Option B - If you have ctx.ui methods (pi-mono extensions):**
- Use \`ctx.ui.select(title, options)\` for single-choice questions
- Use \`ctx.ui.confirm(title, message)\` for yes/no questions

**Option C - If no interactive tools available (Codex, fallback):**
- Ask ONE clear question at a time
- Present options as a numbered list: 1. Option A, 2. Option B, 3. Option C
- Wait for user response before proceeding

## CRITICAL: Agent Delegation (Pi-Mono Skills)

For task delegation, use pi-mono's skills instead of Task tool:

- **Trivial tasks (1-5 lines):** Use sonnet-specialist skill
- **Standard tasks (5-50 lines, single file):** Use opencode-scaffolder skill
- **Complex tasks (multiple files, >50 lines):** Use amp-code skill for implementation + codex-reviewer for design
- **ALL implementation:** Validate with codex-reviewer skill

**Agent mappings (from Claude Code maestro commands):**
- oracle → codex-reviewer
- macgyver → opencode-scaffolder
- michaelangello → gemini-analyzer
- luis → general-purpose
- hobbs → sonnet-specialist
- einstein → opus-specialist

## 1.0 SETUP CHECK

Verify maestro environment:
- \`maestro/tech-stack.md\` exists? → If missing, halt and run \`/maestro:setup\`
- \`maestro/workflow.md\` exists? → If missing, halt and run \`/maestro:setup\`
- \`maestro/product.md\` exists? → If missing, halt and run \`/maestro:setup\`

## 2.0 TRACK SELECTION

${trackId ? `
**Track provided: ${trackId}**

1. Verify track exists at \`maestro/tracks/${trackId}/\`
2. Load track context:
   - Read \`maestro/tracks/${trackId}/plan.md\`
   - Read \`maestro/tracks/${trackId}/spec.md\`
   - Read \`maestro/workflow.md\`
3. Proceed to implementation
` : `
**No track provided - auto-detect next incomplete track**

1. Read \`maestro/tracks.md\`
2. Parse by \`---\` separator to find track sections
3. Find first track with status \`[ ]\` or \`[~]\` (not \`[x]\`)
4. Confirm selection with user:
   - **If AskUserQuestion available:** Use tool to confirm track selection
   - **If ctx.ui methods available:** Use ctx.ui.confirm()
   - **Fallback:** Ask "Should I implement this track? (1) Yes, (2) No, select different track"
5. Extract track_id from link
6. Load track context:
   - Read \`maestro/tracks/<track_id>/plan.md\`
   - Read \`maestro/tracks/<track_id>/spec.md\`
   - Read \`maestro/workflow.md\`
`}

## 3.0 TRACK IMPLEMENTATION

1. **Update Status to In Progress:**
   - Find track heading in \`maestro/tracks.md\`
   - Change \`## [ ] Track: ...\` to \`## [~] Track: ...\`

2. **Execute Tasks from plan.md:**

   For each task in plan.md (iterate one by one):

   a. **CRITICAL THINK - BEFORE IMPLEMENTATION:**
      - Read \`maestro/critical_think/templates/criticalthink_implementation.md\`
      - Execute pre-implementation analysis (Steps 1-6)

   b. **CRITICAL THINK - BEFORE AGENT DELEGATION:**
      - Read \`maestro/critical_think/templates/criticalthink_agent_delegation.md\`
      - Execute pre-delegation analysis (Steps 1-6)

   c. **ASSESS TASK COMPLEXITY:**
      - Trivial (1-5 lines): Implement directly
      - Standard (5-50 lines): Use opencode-scaffolder skill
      - Complex (multi-file, >50 lines): Use amp-code + codex-reviewer
      - ALWAYS validate with codex-reviewer

   d. **FOR CODE ANALYSIS - USE MAESTRO CLI:**
      For understanding codebase before implementation:
      \`maestro leindex analyze --path /path/to/code\`

      This provides 5-phase analysis:
      - Phase 1: File Discovery & Structural Analysis
      - Phase 2: Symbol & Dependency Extraction
      - Phase 3: Control Flow & Data Flow Analysis
      - Phase 4: Pattern & Architecture Detection
      - Phase 5: Complete Codebase Graph

      LeIndex results help you understand:
      - Code structure and dependencies
      - Function call graphs
      - Data flow between components
      - Architecture patterns used

   e. **EXECUTE THE TASK:**
      - Follow \`maestro/workflow.md\` procedures
      - Use Read/Write/Edit tools
      - For complex tasks, invoke appropriate skill
      - Update task checkbox in plan.md

   f. **CRITICAL THINK - AFTER IMPLEMENTATION:**
      - Read \`maestro/critical_think/templates/criticalthink_after_action.md\`
      - Execute post-implementation validation (Steps 1-6)

   g. **CRITICAL THINK - AFTER AGENT DELEGATION:**
      - Read \`maestro/critical_think/templates/criticalthink_after_action.md\`
      - Execute post-agent validation (Steps 1-6)

   h. **BANK MEMORY: Task Completion** - Store task completion:
      - Task ID and title
      - Files modified/created
      - Commit hash
      - Use: \`maestro memory --store --category decision --content "Task completed: ..."\`

3. **Update Task Checkboxes:**
   - After completing each task, update plan.md
   - Change \`- [ ] Task: ...\` to \`- [x] Task: ...\`

${trackLensEnabled ? `
## 4.0 TRACKLENS WALKTHROUGH REVIEW

When all tasks in plan.md are complete, request TrackLens walkthrough review:

1. **CALL TRACKLENS_WALKTHROUGH TOOL:**
   \`\`\`
   tracklens_walkthrough with:
   - trackId: "<track_id>"
   - summary: "Brief summary of what was accomplished"
   \`\`\`

2. **WAIT FOR USER DECISION:**
   - **If approved:** Proceed to finalize track (step 5.0)
   - **If denied with annotations:** Parse annotations into remediation tasks, execute fixes, regenerate walkthrough, and call tracklens_walkthrough again
   - **If TrackLens unavailable:** Fall back to manual completion (skip to step 5.0)

3. **REMEDIATION LOOP:**
   - If user denies with annotations:
     a. Parse each annotation into a remediation task
     b. Add tasks to plan.md with - [ ] format
     c. Execute remediation tasks
     d. Regenerate walkthrough markdown
     e. Call tracklens_walkthrough again
     f. Loop until approved or max 3 iterations

4. **MINIMAL TEXT FALLBACK:**
   If TrackLens UI is unavailable and manual review is needed:
   - Generate text-based walkthrough summary
   - Present as markdown in chat
   - Ask for approval: "Does this walkthrough look complete? (1) Approve, (2) Request changes"

## 5.0 FINALIZE TRACK

After walkthrough approval:` : `
## 4.0 FINALIZE TRACK

After completing all tasks:`}

1. Update track status to complete:
   - Change \`## [~] Track: ...\` to \`## [x] Track: ...\`

2. **SAVE WALKTHROUGH-FINAL.MD:**
   - Save the approved walkthrough to \`maestro/tracks/<track_id>/walkthrough-final.md\`
   - Include: summary, completed tasks, files changed, key decisions

3. **BANK MEMORY: Track Completion** - Store track completion summary:
   - Track ID and title
   - Total tasks completed
   - Final completion timestamp
   - Brief summary of changes made
   - Use the maestro memory CLI: \`maestro memory --store --category decision --content "Track completed: ..."\`

4. Announce completion

## 6.0 IMPORTANT NOTES

- **Workflow.md is single source of truth** for task lifecycle
- **Validate ALL tool calls** - halt on failure
- **Use LeIndex CLI** for codebase analysis before implementation
- **Update checkboxes** in plan.md as work progresses
- **Follow Critical Think templates** for quality assurance

## 7.0 TRACKLENS INTEGRATION

**TrackLens Walkthrough Status:** ${trackLensEnabled ? "ENABLED" : "DISABLED"}
${trackLensEnabled ? `
- All completed tracks require walkthrough review
- User can approve, deny with feedback, or request changes
- Review/denial loop continues until user approves

**Toggle TrackLens Behavior:**
- To disable walkthrough reviews: Use \`/tracklens off\` command
- To re-enable walkthrough reviews: Use \`/tracklens on\` command
` : `
- Walkthrough reviews are currently DISABLED
- To re-enable walkthrough reviews: Use \`/tracklens on\` command
`}

## 6.0 TOOL MAPPING

**File Operations:**
- **Read** → Read tool
- **Write** → Write tool
- **Edit** → Edit tool
- **Bash** → Bash tool (for maestro leindex CLI)

**User Interaction:**
- **AskUserQuestion** → If available (Claude Code, some Codex versions)
- **ctx.ui.select() / ctx.ui.confirm()** → If available (pi-mono extensions)
- **Fallback:** Ask numbered questions and wait for response

**Agent Delegation:**
- **Task** → NOT AVAILABLE in pi-mono
- Use pi-mono skills instead: sonnet-specialist, opencode-scaffolder, amp-code, codex-reviewer, gemini-analyzer, general-purpose, opus-specialist

---
Track context stored in extension. Begin implementation now.`;
}
