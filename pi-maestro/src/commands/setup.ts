/**
 * /maestro:setup command
 *
 * Initialize/refresh maestro project
 * Creates maestro/ directory with product.md, tech-stack.md, workflow.md, tracks.md
 * Copies Critical Think templates
 */

import type { ExtensionAPI } from "../types";
import {
  findMaestroProjectRoot,
  writeMaestroFile,
  maestroProjectExists,
} from "../lib/project";
import { initCriticalThinkTemplates } from "../lib/criticalThink";
import * as path from "path";
import * as fs from "fs";

/**
 * Register /maestro:setup command
 */
export function registerSetup(pi: ExtensionAPI, commandName: string) {
  pi.registerCommand(commandName, {
    description: "Initialize/refresh maestro project structure",
    handler: async (args, ctx) => {
      const root = process.cwd();

      // Check if maestro directory exists and what files are present
      const maestroDir = path.join(root, "maestro");
      const hasMaestroDir = fs.existsSync(maestroDir);

      if (!hasMaestroDir) {
        // No maestro directory - initialize new project
        await initializeMaestroProject(root, ctx);
        return;
      }

      // Check what files exist
      const files = {
        product: fs.existsSync(path.join(maestroDir, "product.md")),
        techStack: fs.existsSync(path.join(maestroDir, "tech-stack.md")),
        workflow: fs.existsSync(path.join(maestroDir, "workflow.md")),
        tracks: fs.existsSync(path.join(maestroDir, "tracks.md")),
        tracksDir: fs.existsSync(path.join(maestroDir, "tracks")),
        criticalThink: fs.existsSync(
          path.join(maestroDir, "critical_think", "templates"),
        ),
      };

      const hasAllCoreFiles =
        files.product && files.techStack && files.workflow && files.tracks;

      if (hasAllCoreFiles) {
        // All core files exist - refresh
        await refreshMaestroProject(root, files, ctx);
      } else {
        // Some files missing - initialize
        await initializeMaestroProject(root, ctx);
      }
    },
  });
}

/**
 * Initialize a new maestro project
 */
async function initializeMaestroProject(root: string, ctx: any): Promise<void> {
  // Detect brownfield vs greenfield
  const isBrownfield = detectBrownfield(root);

  // Create maestro directory structure
  const maestroDir = path.join(root, "maestro");
  const tracksDir = path.join(maestroDir, "tracks");
  const criticalThinkDir = path.join(maestroDir, "critical_think", "templates");

  fs.mkdirSync(tracksDir, { recursive: true });
  fs.mkdirSync(criticalThinkDir, { recursive: true });

  // Copy Critical Think templates
  const packageTemplatesDir = path.join(__dirname, "../../templates");
  initCriticalThinkTemplates(root, packageTemplatesDir);

  // Gather project information
  const productName = await ctx.ui.input(
    "Product Name",
    "What is the name of this project?",
  );

  if (!productName) {
    ctx.ui.notify("Setup cancelled - no product name provided", "warning");
    return;
  }

  const productDescription = await ctx.ui.input(
    "Product Description",
    "Briefly describe what this project does",
  );

  // Generate product.md
  const productMd = generateProductMd(
    productName,
    productDescription || "",
    isBrownfield,
  );
  writeMaestroFile(root, "product.md", productMd);

  // Generate tech-stack.md
  const techStackMd = await generateTechStackMd(root, ctx);
  writeMaestroFile(root, "tech-stack.md", techStackMd);

  // Generate workflow.md
  const workflowMd = generateWorkflowMd();
  writeMaestroFile(root, "workflow.md", workflowMd);

  // Generate tracks.md
  const tracksMd = generateTracksMd();
  writeMaestroFile(root, "tracks.md", tracksMd);

  // TrackLens review of generated setup docs
  try {
    const combinedMarkdown = [
      "# Maestro Setup Review\n",
      "---\n\n",
      "## product.md\n\n",
      productMd,
      "\n\n---\n\n",
      "## tech-stack.md\n\n",
      techStackMd,
      "\n\n---\n\n",
      "## workflow.md\n\n",
      workflowMd,
    ].join("");

    // @ts-ignore - Dynamic import for TrackLens server
    const tracklensServer = await import("@maestro/tracklens-server");
    const { existsSync: exists, readFileSync: read } = await import("fs");
    const { resolve } = await import("path");

    let htmlContent: string | null = null;
    const htmlPaths = [
      resolve(root, "apps/tracklens-opencode/tracklens.html"),
      resolve(root, "dist/tracklens-editor.html"),
    ];
    for (const htmlPath of htmlPaths) {
      if (exists(htmlPath)) {
        htmlContent = read(htmlPath, "utf-8");
        break;
      }
    }

    if (!htmlContent) {
      ctx.ui.notify("TrackLens review UI unavailable — setup docs not found. Setup aborted.", "error");
      return; // Fail closed — don't proceed without review
    }

    const server = await tracklensServer.startTrackLensServer({
        plan: combinedMarkdown,
        origin: "pi-maestro",
        htmlContent,
      });

      let result: { approved: boolean; feedback?: string; edited_content?: string; };
      try {
        result = await server.waitForDecision() as {
          approved: boolean; feedback?: string; edited_content?: string;
        };
      } finally {
        server.stop();
      }

      // Abort setup if review was denied - user rejected the generated docs
      if (!result.approved) {
        if (result.feedback) {
          ctx.ui.notify(`Setup review denied: ${result.feedback}`, "error");
        } else {
          ctx.ui.notify("Setup review was denied. Cancelling setup.", "error");
        }
        return; // Abort - do not show success toast
      }
      
      // Review approved - apply any user edits to the files
      if (result.edited_content) {
        // Parse edited content back into individual docs.
        // Expected format: each section separated by "\n---\n" (newline, 3 dashes, newline)
        const sections = result.edited_content.split(/\n\n---\n\n/);
        for (const section of sections) {
          const trimmed = section.trim();
          const docMatch = trimmed.match(/^## (product|tech-stack|workflow)\.md\n\n([\s\S]*)/);
          if (docMatch) {
            writeMaestroFile(root, `${docMatch[1]}.md`, docMatch[2].trim());
          }
        }
      }
  } catch (error) {
    // TrackLens server failed to start or crashed - abort setup
    ctx.ui.notify(`TrackLens review failed: ${error instanceof Error ? error.message : 'Unknown error'}. Setup aborted.`, "error");
    return; // Abort - do not show success toast
  }

  ctx.ui.notify("Maestro project initialized successfully", "success");
}

/**
 * Refresh an existing maestro project
 */
async function refreshMaestroProject(
  root: string,
  files: {
    product: boolean;
    techStack: boolean;
    workflow: boolean;
    tracks: boolean;
    tracksDir: boolean;
    criticalThink: boolean;
  },
  ctx: any,
): Promise<void> {
  // Show what's in place
  const status = [];
  if (files.product) status.push("✓ product.md");
  if (files.techStack) status.push("✓ tech-stack.md");
  if (files.workflow) status.push("✓ workflow.md");
  if (files.tracks) status.push("✓ tracks.md");
  if (files.tracksDir) status.push("✓ tracks/ directory");
  if (files.criticalThink) status.push("✓ Critical Think templates");

  // Ensure Critical Think templates are present
  const packageTemplatesDir = path.join(__dirname, "../../templates");
  initCriticalThinkTemplates(root, packageTemplatesDir);

  // Count tracks
  const tracksDir = path.join(root, "maestro/tracks");
  const trackCount = fs.existsSync(tracksDir)
    ? fs.readdirSync(tracksDir).length
    : 0;

  ctx.ui.notify(
    `Maestro project: ${status.join(", ")} | ${trackCount} tracks`,
    "info",
  );
}

/**
 * Detect if this is a brownfield project
 */
function detectBrownfield(root: string): boolean {
  const hasGit = fs.existsSync(path.join(root, ".git"));
  const hasPackageJson = fs.existsSync(path.join(root, "package.json"));
  const hasCargoToml = fs.existsSync(path.join(root, "Cargo.toml"));
  const hasSourceCode =
    fs.existsSync(path.join(root, "src")) ||
    fs.existsSync(path.join(root, "lib")) ||
    fs.existsSync(path.join(root, "app"));

  return hasGit || hasPackageJson || hasCargoToml || hasSourceCode;
}

/**
 * Generate product.md content
 */
function generateProductMd(
  name: string,
  description: string,
  isBrownfield: boolean,
): string {
  return `# Product: ${name}

## Description

${description || "A software project."}

## Project Type

${isBrownfield ? "Brownfield (existing codebase)" : "Greenfield (new project)"}

## Product Goals

- Goal 1
- Goal 2
- Goal 3

## Target Users

- User group 1
- User group 2

## Success Metrics

- Metric 1
- Metric 2
`;
}

/**
 * Generate tech-stack.md content
 */
async function generateTechStackMd(root: string, ctx: any): Promise<string> {
  // Detect common technologies
  const techs: string[] = [];

  if (fs.existsSync(path.join(root, "package.json"))) {
    const pkgJson = JSON.parse(
      fs.readFileSync(path.join(root, "package.json"), "utf-8"),
    );
    if (pkgJson.dependencies?.react) techs.push("React");
    if (pkgJson.dependencies?.vue) techs.push("Vue");
    if (pkgJson.dependencies?.next) techs.push("Next.js");
    if (pkgJson.dependencies?.typescript) techs.push("TypeScript");
    if (pkgJson.dependencies?.["@" + "mariozechner/pi-coding-agent"])
      techs.push("pi-mono");
  }

  if (fs.existsSync(path.join(root, "Cargo.toml"))) {
    techs.push("Rust");
  }

  return `# Technology Stack

## Languages

${techs.length > 0 ? techs.map((t) => `- ${t}`).join("\n") : "- To be determined"}

## Frameworks

- To be determined

## Tools

- Maestro (spec-driven development)
- TrackLens (visual review system)
- pi-mono (AI coding agent)

## Development Standards

- Code style: To be determined
- Testing framework: To be determined
- Documentation: Markdown in maestro/ directory
`;
}

/**
 * Generate workflow.md content
 */
function generateWorkflowMd(): string {
  return `# Development Workflow

## Track Development Process

1. **Planning Phase**
   - Use /maestro:newTrack to create a new track with spec.md and plan.md
   - **TrackLens Review**: Visual review of specification and implementation plan.
   - Review and approve artifacts in the TrackLens UI before proceeding.
   - Track is created in maestro/tracks/<track_id>/

2. **Implementation Phase**
   - Use /maestro:implement to execute the track plan
   - Tasks are completed sequentially or delegated to subagents
   - Progress is tracked in plan.md

3. **Review Phase**
   - Use /maestro:status to check overall progress
   - Review completed work against acceptance criteria
   - Make adjustments as needed

4. **Completion Phase**
   - **TrackLens Walkthrough**: Visual summary of completed work for final sign-off.
   - Mark track as completed when all tasks are done
   - Update tracks.md with completion status

## TrackLens Integration

This project uses TrackLens for visual verification:
- Visual specification review (\`spec.md\`)
- Visual implementation plan review (\`plan.md\`)
- Automated walkthrough generation after implementation

## Critical Think Integration

This project uses Critical Think templates for metacognitive analysis:
- Before asking user questions
- Before generating documentation
- Before implementation
- Before agent delegation
- After completing actions

## Agent Delegation Guidelines

- **Trivial (1-5 lines)**: Implement directly
- **Standard (5-50 lines)**: Use qwen-coder agent
- **Complex (multiple files, >50 lines)**: Use amp-code or rovo-dev agent
- **All implementation**: Use codex-reviewer for validation
`;
}

/**
 * Generate tracks.md content
 */
function generateTracksMd(): string {
  return `# Project Tracks

This file tracks all development tracks in this project.

---

*No tracks yet. Use /maestro:newTrack to create a track.*
`;
}
