/**
 * TrackLens CLI for Claude Code
 *
 * Supports three modes:
 *
 * 1. Plan Review (default, no args):
 *    - Spawned by ExitPlanMode hook
 *    - Reads hook event from stdin, extracts plan content
 *    - Serves UI, returns approve/deny decision to stdout
 *
 * 2. Code Review (`tracklens review`):
 *    - Triggered by /tracklens-review slash command
 *    - Runs git diff, opens review UI
 *    - Outputs feedback to stdout (captured by slash command)
 *
 * 3. Annotate (`tracklens annotate <file.md>`):
 *    - Triggered by /tracklens-annotate slash command
 *    - Opens any markdown file in the annotation UI
 *    - Outputs structured feedback to stdout
 *
 * REBRANDED: Plannotator → TrackLens
 * REMOVED: Paste service, share URL functionality
 */

import {
  startTrackLensServer,
} from "@maestro/tracklens-server";
import {
  startReviewServer,
} from "@maestro/tracklens-server/review";
import {
  startAnnotateServer,
} from "@maestro/tracklens-server/annotate";
import { getGitContext, runGitDiff } from "@maestro/tracklens-server/git";

// Embed the built HTML at compile time
// @ts-ignore - Bun import attribute for text
import planHtml from "../dist/index.html" with { type: "text" };
const planHtmlContent = planHtml as unknown as string;

// @ts-ignore - Bun import attribute for text
import reviewHtml from "../dist/review.html" with { type: "text" };
const reviewHtmlContent = reviewHtml as unknown as string;

// @ts-ignore - Bun import attribute for text
import annotateHtml from "../dist/annotate.html" with { type: "text" };
const annotateHtmlContent = annotateHtml as unknown as string;

// Check for subcommand
const args = process.argv.slice(2);

if (args[0] === "review") {
  // ============================================
  // CODE REVIEW MODE
  // ============================================

  // Get git context (branches, available diff options)
  const gitContext = await getGitContext();

  // Run git diff HEAD (uncommitted changes - default)
  const { patch: rawPatch, label: gitRef, error: diffError } = await runGitDiff(
    "uncommitted",
    gitContext.defaultBranch
  );

  // Start review server (even if empty - user can switch diff types)
  const server = await startReviewServer({
    rawPatch,
    gitRef,
    error: diffError,
    origin: "claude-code",
    diffType: "uncommitted",
    gitContext,
    htmlContent: reviewHtmlContent,
  });

  // Wait for user feedback
  const result = await server.waitForDecision();

  // Give browser time to receive response and update UI
  await Bun.sleep(1500);

  // Output feedback to stdout (captured by slash command)
  if (result.feedback) {
    console.log(result.feedback);
  }

  // Handle agent switch
  if (result.agentSwitch) {
    console.log(`\n[TrackLens] Switching to agent: ${result.agentSwitch}`);
    // Claude Code will handle the agent switch based on stdout
  }

  process.exit(result.feedback ? 0 : 1);
} else if (args[0] === "annotate") {
  // ============================================
  // ANNOTATE MODE
  // ============================================

  const filePath = args[1];

  if (!filePath) {
    console.error("Usage: tracklens annotate <file.md>");
    process.exit(1);
  }

  // Read file content
  const file = Bun.file(filePath);
  if (!(await file.exists())) {
    console.error(`File not found: ${filePath}`);
    process.exit(1);
  }

  const markdown = await file.text();

  // Start annotate server
  const server = await startAnnotateServer({
    markdown,
    filePath,
    origin: "claude-code",
    htmlContent: annotateHtmlContent,
  });

  // Wait for user feedback
  const result = await server.waitForDecision();

  // Give browser time to receive response and update UI
  await Bun.sleep(1500);

  // Output feedback to stdout
  if (result.feedback) {
    console.log(result.feedback);
  }

  process.exit(result.feedback ? 0 : 1);
} else {
  // ============================================
  // PLAN REVIEW MODE (default, hook-invoked)
  // ============================================

  // Read hook event from stdin
  const hookEventJson = await Bun.stdin.read();
  const hookEventText = new TextDecoder().decode(hookEventJson);
  const hookEvent = JSON.parse(hookEventText);

  // Extract plan content from the hook event
  // The event contains user_message which has the plan markdown
  const plan = hookEvent.user_message?.content || hookEvent.content || "";

  if (!plan) {
    console.error("No plan content found in hook event");
    process.exit(1);
  }

  // Get current autonomy mode if present
  const autonomyMode = hookEvent.autonomy_mode;

  // Start TrackLens server
  const server = await startTrackLensServer({
    plan,
    origin: "claude-code",
    htmlContent: planHtmlContent,
    autonomyMode,
  });

  // Wait for user decision
  const decision = await server.waitForDecision();

  // Give browser time to receive response and update UI
  await Bun.sleep(1500);

  // Output decision to stdout (captured by ExitPlanMode hook)
  if (!decision.approved) {
    console.log("\nPlan not approved.");
    if (decision.feedback) {
      console.log(`\nFeedback:\n${decision.feedback}`);
    }
    process.exit(1);
  }

  // Handle agent switch
  if (decision.agentSwitch) {
    console.log(`\n[TrackLens] Switching to agent: ${decision.agentSwitch}`);
  }

  // Handle autonomy mode change
  if (decision.autonomyMode && decision.autonomyMode !== autonomyMode) {
    console.log(`\n[TrackLens] Autonomy mode changed to: ${decision.autonomyMode}`);
  }

  process.exit(0);
}
