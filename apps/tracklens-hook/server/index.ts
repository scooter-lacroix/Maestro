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
import { fileURLToPath } from "url";
import { dirname, resolve } from "path";

// Load HTML at runtime (since compile-time imports don't work from dist/)
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = process.env.CLAUDE_PROJECT_DIR || resolve(__dirname, "..", "..", "..");

async function loadHtmlContent(paths: string[]): Promise<string> {
  for (const candidate of paths) {
    const file = Bun.file(candidate);
    if (await file.exists()) {
      return await file.text();
    }
  }

  throw new Error(
    `Could not load TrackLens HTML asset.\nChecked:\n${paths.join("\n")}\nCurrent directory: ${__dirname}\nProject root: ${projectRoot}\nEnsure packages are built: bun run build`
  );
}

// Load HTML content at startup (top-level await)
const planHtmlContent = await loadHtmlContent([
  resolve(__dirname, "index.html"),
  resolve(__dirname, "..", "dist", "index.html"),
  resolve(projectRoot, "packages", "tracklens-editor", "dist", "index.html"),
]);
const reviewHtmlContent = await loadHtmlContent([
  resolve(__dirname, "review.html"),
  resolve(__dirname, "..", "dist", "review.html"),
  resolve(projectRoot, "packages", "tracklens-review-editor", "dist", "index.html"),
]);
const annotateHtmlContent = reviewHtmlContent;

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

  try {
    const result = await server.waitForDecision();

    await Bun.sleep(1500);

    if (result.feedback) {
      console.log(result.feedback);
    }

    if (result.agentSwitch) {
      console.log(`\n[TrackLens] Switching to agent: ${result.agentSwitch}`);
      // Claude Code will handle the agent switch based on stdout
    }

    process.exit(result.feedback ? 0 : 1);
  } finally {
    server.stop();
  }
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

  try {
    const result = await server.waitForDecision();

    await Bun.sleep(1500);

    if (result.feedback) {
      console.log(result.feedback);
    }

    process.exit(result.feedback ? 0 : 1);
  } finally {
    server.stop();
  }
} else {
  // ============================================
  // PLAN REVIEW MODE (default, hook-invoked)
  // ============================================

  // Read hook event from stdin (using Bun.stdin.text() like original Plannotator)
  const eventJson = await Bun.stdin.text();

  let planContent = "";
  let permissionMode = "default";
  try {
    const event = JSON.parse(eventJson);
    planContent = event.tool_input?.plan || "";
    permissionMode = event.permission_mode || "default";
  } catch {
    console.error("Failed to parse hook event from stdin");
    process.exit(1);
  }

  if (!planContent) {
    console.error("No plan content in hook event");
    process.exit(1);
  }

  // Start the plan review server
  const server = await startTrackLensServer({
    plan: planContent,
    origin: "claude-code",
    autonomyMode: permissionMode,
    htmlContent: planHtmlContent,
  });

  // Wait for user decision (blocks until approve/deny)
  const result = await server.waitForDecision();

  // Give browser time to receive response and update UI
  await Bun.sleep(1500);

  // Cleanup
  server.stop();

  // Output JSON for PermissionRequest hook decision control (ORIGINAL PLANNOTATOR FORMAT)
  if (result.approved) {
    // Build updatedPermissions to preserve the current permission mode
    const updatedPermissions = [];
    if (result.autonomyMode) {
      updatedPermissions.push({
        type: "setMode",
        mode: result.autonomyMode,
        destination: "session",
      });
    }

    console.log(
      JSON.stringify({
        hookSpecificOutput: {
          hookEventName: "PermissionRequest",
          decision: {
            behavior: "allow",
            ...(updatedPermissions.length > 0 && { updatedPermissions }),
          },
        },
      })
    );
  } else {
    console.log(
      JSON.stringify({
        hookSpecificOutput: {
          hookEventName: "PermissionRequest",
          decision: {
            behavior: "deny",
            message: result.feedback || "Plan changes requested",
          },
        },
      })
    );
  }

  process.exit(0);
}
