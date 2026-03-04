/**
 * /maestro:newTrack command
 *
 * CRITICAL COMMAND - Engages LLM to create a new maestro track with spec.md and plan.md
 *
 * Architecture: LLM-ENGAGED WORKFLOW
 * - Injects maestro workflow instructions into LLM context
 * - LLM follows the step-by-step workflow using available tools
 */

import type { ExtensionAPI } from "../types";
import {
  findMaestroProjectRoot,
  readMaestroProject,
  maestroProjectExists,
} from "../lib/project";
import { initCriticalThinkTemplates } from "../lib/criticalThink";
import * as path from "path";
import * as fs from "fs";

/** Store current track context for before_agent_start */
let currentNewTrackContext: {
  root: string;
  description?: string;
  productName?: string;
  existingTracks: Array<{ id: string; status: string; description: string }>;
} | null = null;

/**
 * Register /maestro:newTrack command
 */
export function registerNewTrack(pi: ExtensionAPI, commandName: string) {
  // Register the command
  pi.registerCommand(commandName, {
    description: "Create a new maestro track with spec.md and plan.md",
    handler: async (args, ctx) => {
      const description = args.trim();

      const root = findMaestroProjectRoot(process.cwd());
      if (!root) {
        ctx.ui.notify("Not in a maestro project", "error");
        return;
      }

      if (!maestroProjectExists(root)) {
        ctx.ui.notify("Maestro project incomplete. Run /maestro:setup first.", "error");
        return;
      }

      const project = readMaestroProject(root);
      const packageTemplatesDir = path.join(__dirname, "../../templates");
      initCriticalThinkTemplates(root, packageTemplatesDir);

      const productName = project.product.split("\n")[0].replace("# Product: ", "").trim();

      // Get existing tracks
      const tracksDir = path.join(root, "maestro/tracks");
      const existingTracks: { id: string; status: string; description: string }[] = [];
      if (fs.existsSync(tracksDir)) {
        const trackIds = fs.readdirSync(tracksDir);
        for (const id of trackIds) {
          const metadataPath = path.join(tracksDir, id, "metadata.json");
          if (fs.existsSync(metadataPath)) {
            const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf-8"));
            existingTracks.push({ id, status: metadata.status, description: metadata.description });
          }
        }
      }

      // Store context for before_agent_start
      currentNewTrackContext = {
        root,
        description: description || undefined,
        productName,
        existingTracks,
      };

      // Trigger a turn to start the workflow
      pi.sendMessage(
        {
          customType: "maestro-newtrack-start",
          content: description
            ? `Creating new maestro track: "${description}"`
            : `No description provided. Please help create a new maestro track.`,
          display: true,
        },
        { triggerTurn: true }
      );
    },
  });

  // Inject workflow instructions before agent starts
  pi.on("before_agent_start", async (event) => {
    // Only inject if we're in newTrack mode
    if (!currentNewTrackContext || !isInMaestroProject(currentNewTrackContext.root)) {
      return;
    }

    // Check if this message is related to maestro newTrack
    const isRelated = event.messages.some((m: any) =>
      m.customType === "maestro-newtrack-start" ||
      m.customType === "maestro-newtrack-workflow"
    );

    if (!isRelated) {
      return;
    }

    // Build the workflow instructions
    const workflow = buildNewTrackWorkflow(currentNewTrackContext);

    // Clear context after first injection to avoid re-injecting
    currentNewTrackContext = null;

    return {
      message: {
        customType: "maestro-newtrack-workflow",
        content: workflow,
        display: false,
      },
    };
  });
}

function isInMaestroProject(root: string): boolean {
  return fs.existsSync(path.join(root, "maestro/tracks"));
}

function buildNewTrackWorkflow(context: {
  root: string;
  description?: string;
  productName?: string;
  existingTracks: Array<{ id: string; status: string; description: string }>;
}): string {
  const { description, productName, existingTracks } = context;

  const trackList = existingTracks.length > 0
    ? "\\n\\nExisting Tracks:\\n" + existingTracks.map(t => {
        const statusIcon = t.status === "completed" ? "✓" : t.status === "in_progress" ? "⟳" : "○";
        return `  ${statusIcon} ${t.description} (${t.id})`;
      }).join("\\n")
    : "";

  const jsonTemplate = `{
  "track_id": "<track_id>",
  "type": "feature",
  "status": "new",
  "created_at": "<timestamp>",
  "updated_at": "<timestamp>",
  "description": "<description>"
}`;

  const tracksMdTemplate = `
---

## [ ] Track: <description>
*Link: [./maestro/tracks/<track_id>/](./maestro/tracks/<track_id>/)*
`;

  return `# Maestro New Track Creation Protocol

You are creating a new maestro track. Follow this protocol precisely.

## CRITICAL: User Interaction Methods

**For interactive questions, use these methods based on your available tools:**

**Option A - If you have AskUserQuestion tool (Claude Code, some Codex versions):**
Use AskUserQuestion with: question, header, options (label, description), multiSelect (boolean)

**Option B - If you have ctx.ui methods (pi-mono extensions):**
- Use ctx.ui.select(title, options) for single-choice questions
- Use ctx.ui.confirm(title, message) for yes/no questions
- Use ctx.ui.input(title, placeholder) for text input

**Option C - If no interactive tools available (Codex, fallback):**
- Ask ONE clear question at a time
- Present options as a numbered list: 1. Option A, 2. Option B, 3. Option C
- Wait for user response before proceeding
- Do NOT present all questions at once

**CRITICAL:** Always ask questions sequentially (one at a time), never all at once.

## 1.0 SETUP CHECK

Verify maestro environment:
- maestro/tech-stack.md exists? → If missing, halt and run /maestro:setup
- maestro/workflow.md exists? → If missing, halt and run /maestro:setup
- maestro/product.md exists? → If missing, halt and run /maestro:setup

Product: ${productName || "Unknown"}${trackList}

## 2.0 GET TRACK DESCRIPTION

${description ? `
**Description provided: "${description}"

Proceed to specification generation.
` : `
**No description provided**

Ask the user for a brief description of the track (feature, bug fix, chore, etc.)

> "Please provide a brief description of the track (feature, bug fix, chore, etc.) you wish to start."
`}

## 3.0 INTERACTIVE SPECIFICATION GENERATION (spec.md)

1. State your goal:
   "I'll now guide you through a series of questions to build a comprehensive specification (spec.md) for this track."

2. Questioning Phase:
   - Ask 3-5 relevant questions (sequential, one by one)
   - For features: users, goals, constraints, acceptance criteria
   - For bugs: reproduction steps, scope
   - For chores: specific scope, success criteria

   **CRITICAL THINK - BEFORE EACH QUESTION:**
   - Read maestro/critical_think/templates/criticalthink_question.md
   - Execute 6-step analysis
   - If confidence < 7/10, consider making reasonable assumption instead

   **CRITICAL THINK - AFTER EACH ANSWER:**
   - Read maestro/critical_think/templates/criticalthink_after_action.md
   - Execute validation
   - Confirm understanding or ask follow-up

3. **CRITICAL THINK - BEFORE SPEC GENERATION:**
   - Read maestro/critical_think/templates/criticalthink_docs.md
   - Execute pre-documentation analysis

4. Draft spec.md with:
   - Overview
   - Functional Requirements
   - Non-Functional Requirements
   - Acceptance Criteria
   - Out of Scope

5. **CRITICAL THINK - AFTER SPEC GENERATION:**
   - Read maestro/critical_think/templates/criticalthink_after_action.md
   - Execute post-documentation validation

6. **TRACKLENS REVIEW CHECKPOINT (3.6)**
   Call the tracklens_review tool to request visual review:
   \`\`\`
   tracklens_review with:
   - filePath: "maestro/tracks/<track_id>/spec.md"
   - reviewType: "spec"
   - summary: "Specification for <track description>"
   \`\`\`

   Wait for user decision in TrackLens UI:
   - **If approved:** Proceed to plan generation
   - **If denied with feedback:** Address the feedback, revise spec.md, and call tracklens_review again
   - **If TrackLens unavailable:** Fall back to manual confirmation:
     - **If AskUserQuestion available:** Use tool with options: "Approve", "Suggest Changes"
     - **If ctx.ui methods available:** Use ctx.ui.confirm() or ctx.ui.select()
     - **Fallback:** Ask: "Does the spec.md look good? (1) Approve - proceed to plan, (2) Suggest changes - tell me what to modify"

## 4.0 INTERACTIVE PLAN GENERATION (plan.md)

1. State your goal:
   "Now I will create an implementation plan (plan.md) based on the specification."

2. **CRITICAL THINK - BEFORE PLAN GENERATION:**
   - Read maestro/critical_think/templates/criticalthink_docs.md
   - Execute pre-plan analysis

3. Generate plan.md with:
   - Phase 1: Analysis
   - Phase 2: Implementation
   - Phase 3: Validation
   - Each phase has tasks with - [ ] Task: ... format

4. **CRITICAL THINK - AFTER PLAN GENERATION:**
   - Read maestro/critical_think/templates/criticalthink_after_action.md
   - Execute post-plan validation

5. **TRACKLENS REVIEW CHECKPOINT (4.5)**
   Call the tracklens_review tool to request visual review:
   \`\`\`
   tracklens_review with:
   - filePath: "maestro/tracks/<track_id>/plan.md"
   - reviewType: "plan"
   - summary: "Implementation plan for <track description>"
   \`\`\`

   Wait for user decision in TrackLens UI:
   - **If approved:** Proceed to create track artifacts
   - **If denied with feedback:** Address the feedback, revise plan.md, and call tracklens_review again
   - **If TrackLens unavailable:** Fall back to manual confirmation:
     - **If AskUserQuestion available:** Use tool with options: "Approve", "Suggest Changes"
     - **If ctx.ui methods available:** Use ctx.ui.confirm() or ctx.ui.select()
     - **Fallback:** Ask: "Does the plan.md look good? (1) Approve, (2) Suggest changes"

## 5.0 CREATE TRACK ARTIFACTS

1. Generate Track ID: Format: shortname_YYYYMMDD
   - Shortname: lowercase, hyphens for spaces, max 30 chars
   - Example: add-auth_20250127

2. Check for existing names:
   - List existing track directories in maestro/tracks/
   - If shortname matches, halt and suggest different name

3. Create directory: maestro/tracks/<track_id>/

4. Create metadata.json with this format:
${jsonTemplate}

5. Write files:
   - Write spec.md to maestro/tracks/<track_id>/spec.md
   - Write plan.md to maestro/tracks/<track_id>/plan.md

6. Update tracks.md:
   Append to end of maestro/tracks.md:
${tracksMdTemplate}

7. **BANK MEMORY: Track Creation** - Store track creation memory:
   - Track ID and title
   - Track type and description
   - Creation timestamp
   - Use: \`maestro memory --store --category context --content "Track created: ..."\`

8. **TRACKLENS CONSOLIDATED REVIEW CHECKPOINT (5.7)**
   Call the tracklens_review tool for final consolidated review:
   \`\`\`
   tracklens_review with:
   - filePath: "maestro/tracks/<track_id>/spec.md"
   - reviewType: "spec"
   - summary: "Final review: Spec and Plan for <track description> (consolidated)"
   \`\`\`

   Present both spec.md and plan.md for final user approval:
   - **If approved:** Track is ready for implementation
   - **If denied with feedback:** Address the feedback, update files, and request review again
   - **If TrackLens unavailable:** Skip consolidated review (spec and plan already approved individually)

9. Announce completion:
   "New track '<track_id>' has been created and approved. You can now start implementation by running /maestro:implement <track_id>."

---
Begin track creation now.`;
}
