/**
 * TrackLens Extension Tools for Pi-Maestro
 *
 * Registers TrackLens tools for integration with newTrack and implement workflows.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * Tools:
 * - tracklens_review: Review spec, plan, or walkthrough markdown
 * - tracklens_walkthrough: Generate and present walkthrough for completed track
 *
 * @packageDocumentation
 */

import type { ExtensionAPI } from "../../types";
import { readFileSync, existsSync } from "fs";
import { resolve } from "path";

/**
 * Register TrackLens tools with pi-maestro extension
 *
 * These tools integrate with newTrack and implement workflows to provide
 * visual review and walkthrough capabilities.
 */
export function registerTrackLensTools(pi: ExtensionAPI) {
  /**
   * TrackLens Review Tool
   *
   * Allows the agent to request TrackLens review for:
   * - Spec documents (after spec draft in newTrack)
   * - Plan documents (after plan draft in newTrack)
   * - Completed track walkthroughs (in implement)
   *
   * The tool opens a TrackLens UI for user review and annotation.
   */
  pi.registerTool({
    name: "tracklens_review",
    label: "TrackLens Review",
    description: `
      Request TrackLens visual review for a markdown document.

      Use this tool when you need user review and approval on:
      - Spec documents (spec.md)
      - Plan documents (plan.md)
      - Completed work walkthroughs

      The user will be able to:
      - Visually review the content in a dedicated UI
      - Annotate specific sections with comments or suggestions
      - Approve the content to proceed
      - Request changes with detailed feedback

      After the review, you will receive the user's decision and any feedback.
      If changes are requested, address the feedback and call this tool again.
    `.trim(),
    parameters: {
      type: "object",
      properties: {
        filePath: {
          type: "string",
          description: "Path to the markdown file to review (relative to project root)",
        },
        reviewType: {
          type: "string",
          enum: ["spec", "plan", "walkthrough"],
          description: "Type of review being requested",
        },
        summary: {
          type: "string",
          description: "Brief summary of what's being reviewed (for user context)",
        },
      },
      required: ["filePath", "reviewType"],
    },

    async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
      const { filePath, reviewType, summary } = params as {
        filePath: string;
        reviewType: "spec" | "plan" | "walkthrough";
        summary?: string;
      };

      // Resolve file path
      const absolutePath = resolve(ctx.cwd, filePath);

      // Check if file exists
      if (!existsSync(absolutePath)) {
        return {
          content: [
            {
              type: "text",
              text: `Error: File not found: ${absolutePath}`,
            },
          ],
          details: { approved: false },
        };
      }

      // Read markdown content
      const markdown = readFileSync(absolutePath, "utf-8");

      if (markdown.trim().length === 0) {
        return {
          content: [
            {
              type: "text",
              text: `Error: File is empty: ${absolutePath}`,
            },
          ],
          details: { approved: false },
        };
      }

      // Import TrackLens server functions
      let startTrackLensServer: any;
      let htmlContent: string | null = null;

      try {
        // @ts-ignore - Dynamic import for TrackLens server
        const tracklensServer = await import("@maestro/tracklens-server");
        startTrackLensServer = tracklensServer.startTrackLensServer;

        // Try to load HTML content from dist
        const { existsSync: exists, readFileSync: read } = await import("fs");
        const htmlPath = resolve(ctx.cwd, "dist/tracklens-editor.html");
        if (exists(htmlPath)) {
          htmlContent = read(htmlPath, "utf-8");
        }
      } catch (error) {
        // TrackLens server not available - return instructions for manual review
        return {
          content: [
            {
              type: "text",
              text: `# TrackLens Review Request

**Review Type:** ${reviewType}
**File:** ${filePath}
**Summary:** ${summary || "No summary provided"}

TrackLens UI is not available. Please review the file manually:

\`\`\`
${markdown}
\`\`\`

After review, provide your feedback or approval.`,
            },
          ],
          details: { approved: false, manualReview: true },
        };
      }

      // If no HTML content available, fallback to manual review
      if (!htmlContent) {
        return {
          content: [
            {
              type: "text",
              text: `# TrackLens Review Request

**Review Type:** ${reviewType}
**File:** ${filePath}
**Summary:** ${summary || "No summary provided"}

TrackLens UI HTML not built yet. Please review the file manually:

\`\`\`
${markdown}
\`\`\`

After review, provide your feedback or approval.`,
            },
          ],
          details: { approved: false, manualReview: true },
        };
      }

      // Start TrackLens server with the markdown content
      try {
        const server = await startTrackLensServer({
          plan: markdown,
          origin: "pi-maestro",
          htmlContent,
        });

        // Wait for user decision
        const result = await server.waitForDecision();

        // Stop the server
        server.stop();

        // Return the result
        return {
          content: [
            {
              type: "text",
              text: result.feedback || (result.approved ? "Approved" : "Changes requested"),
            },
          ],
          details: {
            approved: result.approved,
            savedPath: result.savedPath,
            agentSwitch: result.agentSwitch,
            autonomyMode: result.autonomyMode,
          },
        };
      } catch (error) {
        // Server error - fallback to manual review
        return {
          content: [
            {
              type: "text",
              text: `# TrackLens Review Request

**Review Type:** ${reviewType}
**File:** ${filePath}
**Summary:** ${summary || "No summary provided"}

TrackLens server error: ${error}. Please review manually:

\`\`\`
${markdown}
\`\`\`

After review, provide your feedback or approval.`,
            },
          ],
          details: { approved: false, manualReview: true },
        };
      }
    },
  });

  /**
   * TrackLens Walkthrough Tool
   *
   * Generates a walkthrough document for a completed track and presents
   * it in the TrackLens UI for user review.
   */
  pi.registerTool({
    name: "tracklens_walkthrough",
    label: "TrackLens Walkthrough",
    description: `
      Generate and present a TrackLens walkthrough for a completed track.

      Use this tool when a track is fully implemented and you want to:
      - Generate a comprehensive walkthrough of what was done
      - Present the changes in a visual, annotated format
      - Allow the user to review and approve the completed work

      The walkthrough will include:
      - Summary of the track's goals
      - List of tasks completed
      - Files changed with diffs
      - Key decisions made
      - Testing performed
    `.trim(),
    parameters: {
      type: "object",
      properties: {
        trackId: {
          type: "string",
          description: "Track ID to generate walkthrough for",
        },
        summary: {
          type: "string",
          description: "Brief summary of what was accomplished",
        },
      },
      required: ["trackId"],
    },

    async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
      const { trackId, summary } = params as {
        trackId: string;
        summary?: string;
      };

      // Find maestro project root
      const { findMaestroProjectRoot } = await import("../../lib/project");
      const root = findMaestroProjectRoot(ctx.cwd);

      if (!root) {
        return {
          content: [
            {
              type: "text",
              text: "Error: Not in a maestro project",
            },
          ],
        };
      }

      // Import walkthrough generator
      const { generateWalkthrough, saveWalkthrough } = await import("../walkthrough");

      const trackDir = resolve(root, "maestro/tracks", trackId);

      // Generate walkthrough
      const walkthrough = generateWalkthrough({
        trackId,
        root,
        trackDir,
        isSubtrack: false, // TODO: detect if subtrack
        includeDiffs: true,
        includeSnippets: true,
        maxSnippetLines: 30,
      });

      // Save walkthrough to storage
      const savedPath = await saveWalkthrough(trackId, walkthrough);

      // Return walkthrough markdown for review
      return {
        content: [
          {
            type: "text",
            text: walkthrough.markdown,
          },
        ],
        details: {
          trackId,
          approved: false, // Requires user approval
          savedPath,
          completedTasks: walkthrough.completedTasks.length,
          changedFiles: walkthrough.changedFiles.length,
        },
      };
    },
  });
}

