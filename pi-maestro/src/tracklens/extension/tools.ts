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
 * - tracklens_code_review: Review git diffs in code-review mode
 *
 * @packageDocumentation
 */

import type { ExtensionAPI } from "../../types";
import { readFileSync, existsSync } from "fs";
import { resolve, isAbsolute } from "path";
import { execFileSync } from "child_process";
import { runRemediationLoop } from "../walkthrough/remediation";
import { recordRecentDocument } from "../recentDoc";
import { formatDenialForAgent } from "../feedback";
import { appendReviewEntry, formatHistoryForAgent } from "../history";

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
        markdown: {
          type: "string",
          description: "Markdown content to review (pass directly or read from file)",
        },
        documentType: {
          type: "string",
          enum: ["spec.md", "plan.md", "walkthrough", "document"],
          description: "Type of document being reviewed",
        },
        trackId: {
          type: "string",
          description: "Track ID for context (optional)",
        },
        mode: {
          type: "string",
          enum: ["review", "walkthrough"],
          description: "Review mode",
          default: "review",
        },
        filePath: {
          type: "string",
          description: "Alternative: Path to the markdown file to review (relative to project root)",
        },
        seedContent: {
          type: "string",
          description: "Optional seed content that opens in edit mode. Prefixes the document with an editable marker so the UI starts in CodeMirror edit mode. Use for 'refine this draft' workflows.",
        },
      },
      required: ["documentType"],
    },

    async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
      const { markdown: directMarkdown, documentType, trackId, mode = "review", filePath, seedContent } = params as {
        markdown?: string;
        documentType: "spec.md" | "plan.md" | "walkthrough" | "document";
        trackId?: string;
        mode?: "review" | "walkthrough";
        filePath?: string;
        seedContent?: string;
      };

      let markdown: string;

      // Get markdown content - either directly or from file
      if (directMarkdown) {
        markdown = directMarkdown;
      } else if (seedContent) {
        // Seed content: prefix with editable marker for UI to detect
        markdown = `<!-- tracklens:editable -->\n${seedContent}`;
      } else if (filePath) {
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
        markdown = readFileSync(absolutePath, "utf-8");
      } else {
        return {
          content: [
            {
              type: "text",
              text: `Error: Either 'markdown', 'filePath', or 'seedContent' must be provided`,
            },
          ],
          details: { approved: false },
        };
      }

      if (markdown.trim().length === 0) {
        return {
          content: [
            {
              type: "text",
              text: `Error: Markdown content is empty`,
            },
          ],
          details: { approved: false },
        };
      }

      // Record this document for auto-trigger tracking
      recordRecentDocument({
        trackId: trackId || "unknown",
        type: documentType,
        content: markdown,
        filePath,
      });

      // Import TrackLens server functions
      let startTrackLensServer: any;
      let htmlContent: string | null = null;

      try {
        // @ts-ignore - Dynamic import for TrackLens server
        const tracklensServer = await import("@maestro/tracklens-server");
        startTrackLensServer = tracklensServer.startTrackLensServer;

        // Try to load HTML content from apps/tracklens-opencode
        const { existsSync: exists, readFileSync: read } = await import("fs");
        const htmlPaths = [
          resolve(ctx.cwd, "apps/tracklens-opencode/tracklens.html"),
          resolve(ctx.cwd, "dist/tracklens-editor.html"),
        ];
        for (const htmlPath of htmlPaths) {
          if (exists(htmlPath)) {
            htmlContent = read(htmlPath, "utf-8");
            break;
          }
        }
      } catch (error) {
        // TrackLens server not available - return instructions for manual review
        return {
          content: [
            {
              type: "text",
              text: `# TrackLens Review Request

**Document Type:** ${documentType}
**Track ID:** ${trackId || "N/A"}
**Mode:** ${mode}

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

**Document Type:** ${documentType}
**Track ID:** ${trackId || "N/A"}
**Mode:** ${mode}

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
        // Validate trackId early to avoid discarding completed reviews later
        const validTrackId = trackId
          && !trackId.includes("..")
          && !isAbsolute(trackId)
          && !trackId.includes("/")
          && !trackId.includes("\\");

        const server = await startTrackLensServer({
          plan: markdown,
          origin: "pi-maestro",
          htmlContent,
        });

        let result: Awaited<ReturnType<typeof server.waitForDecision>>;
        try {
          result = await server.waitForDecision();
        } finally {
          server.stop();
        }

        // Persist review history if track directory is available
        if (validTrackId && ctx.cwd) {
          const { findMaestroProjectRoot } = await import("../../lib/project");
          const root = findMaestroProjectRoot(ctx.cwd);
          if (root) {
            const trackDir = resolve(root, "maestro/tracks", trackId);
            if (existsSync(trackDir)) {
              // Best-effort history persistence — never let logging fail the review
              try {
                appendReviewEntry(trackDir, {
                  timestamp: new Date().toISOString(),
                  documentType,
                  approved: result.approved,
                  annotationCount: result.annotations?.length ?? 0,
                  feedback: result.feedback,
                  editedContent: result.edited_content,
                  reviewDurationMs: result.review_duration_ms ?? 0,
                  iteration: result.iteration ?? 0,
                });
              } catch {
                // Intentionally swallowed — review result takes priority over history log
              }
            }
          }
        }

        // Format result for agent
        const resultText = result.approved
          ? (result.feedback || "Approved")
          : formatDenialForAgent(result, documentType);

        // Return the result
        return {
          content: [
            {
              type: "text",
              text: resultText,
            },
          ],
          details: {
            approved: result.approved,
            savedPath: result.savedPath,
            agentSwitch: result.agentSwitch,
            autonomyMode: result.autonomyMode,
            edited_content: result.edited_content,
          },
        };
      } catch (error) {
        // Server error - fallback to manual review
        return {
          content: [
            {
              type: "text",
              text: `# TrackLens Review Request

**Document Type:** ${documentType}
**Track ID:** ${trackId || "N/A"}
**Mode:** ${mode}

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
   * it in the TrackLens UI for user review with remediation loop support.
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
      - Handle denial with annotations and remediation loop

      The walkthrough will include:
      - Summary of the track's goals
      - List of tasks completed
      - Files changed with diffs
      - Key decisions made
      - Testing performed

      If the user denies with annotations, the tool will:
      - Create remediation tasks from annotations
      - Add tasks to plan.md
      - Regenerate walkthrough for re-review
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
        autoReview: {
          type: "boolean",
          description: "Whether to automatically start interactive review (default: true)",
        },
      },
      required: ["trackId"],
    },

    async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
      const { trackId, summary, autoReview = true } = params as {
        trackId: string;
        summary?: string;
        autoReview?: boolean;
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

      const trackDir = resolve(root, "maestro/tracks", trackId);

      // Import walkthrough generator
      const { generateWalkthrough, saveWalkthrough } = await import("../walkthrough");

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

      // Record walkthrough for auto-trigger tracking
      recordRecentDocument({
        trackId,
        type: "walkthrough",
        content: walkthrough.markdown,
      });

      // If autoReview is disabled, just return the walkthrough
      if (!autoReview) {
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
      }

      // Try to run interactive review with TrackLens server
      let startTrackLensServer: any;
      let htmlContent: string | null = null;

      try {
        // @ts-ignore - Dynamic import for TrackLens server
        const tracklensServer = await import("@maestro/tracklens-server");
        startTrackLensServer = tracklensServer.startTrackLensServer;

        // Try to load HTML content from apps/tracklens-opencode
        const { existsSync: exists, readFileSync: read } = await import("fs");
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
      } catch {
        // Server not available, return walkthrough for manual review
        return {
          content: [
            {
              type: "text",
              text: walkthrough.markdown,
            },
          ],
          details: {
            trackId,
            approved: false,
            savedPath,
            manualReview: true,
          },
        };
      }

      // Start TrackLens server with walkthrough
      try {
        const server = await startTrackLensServer({
          plan: walkthrough.markdown,
          origin: "pi-maestro",
          htmlContent,
          mode: "walkthrough",
        });

        let result: Awaited<ReturnType<typeof server.waitForDecision>>;
        try {
          result = await server.waitForDecision();
        } finally {
          server.stop();
        }

        // Handle approval/denial
        if (!result.approved && result.annotations && result.annotations.length > 0) {
          // User denied with annotations - run remediation loop
          const remediationResult = await runRemediationLoop({
            trackId,
            root,
            trackDir,
            maxIterations: 3,
            onReview: async (walkthroughMarkdown: string, iteration: number) => {
              // Re-present updated walkthrough for review
              const reviewServer = await startTrackLensServer({
                plan: walkthroughMarkdown,
                origin: "pi-maestro",
                htmlContent,
                mode: "walkthrough",
              });

              let reviewResult: Awaited<ReturnType<typeof reviewServer.waitForDecision>>;
              try {
                reviewResult = await reviewServer.waitForDecision();
              } finally {
                reviewServer.stop();
              }

              return {
                approved: reviewResult.approved,
                feedback: reviewResult.feedback,
                annotations: reviewResult.annotations,
                savedPath: reviewResult.savedPath,
              };
            },
          });

          if (!remediationResult.approved) {
            return {
              content: [
                {
                  type: "text",
                  text: `Walkthrough review failed after ${remediationResult.totalIterations} iterations.\n\nRemediation tasks:\n${remediationResult.remediationTasks?.map(t => `- ${t.description}`).join("\n") || "None"}`,
                },
              ],
              details: {
                trackId,
                approved: false,
                iterations: remediationResult.totalIterations,
                savedPath,
              },
            };
          }

          // Approved after remediation
          return {
            content: [
              {
                type: "text",
                text: `Walkthrough approved after ${remediationResult.totalIterations} iteration(s).\n\n${remediationResult.finalWalkthrough || ""}`,
              },
            ],
            details: {
              trackId,
              approved: true,
              iterations: remediationResult.totalIterations,
              savedPath,
            },
          };
        }

        // User approved or denied without annotations
        return {
          content: [
            {
              type: "text",
              text: result.feedback || (result.approved ? "Walkthrough approved" : "Walkthrough denied"),
            },
          ],
          details: {
            trackId,
            approved: result.approved,
            savedPath: result.savedPath,
            agentSwitch: result.agentSwitch,
            autonomyMode: result.autonomyMode,
          },
        };
      } catch (error) {
        // Server error - return walkthrough for manual review
        return {
          content: [
            {
              type: "text",
              text: `TrackLens server error: ${error}\n\nPlease review manually:\n\n${walkthrough.markdown}`,
            },
          ],
          details: {
            trackId,
            approved: false,
            savedPath,
            manualReview: true,
          },
        };
      }
    },
  });

  /**
   * TrackLens Code Review Tool
   *
   * Generates a git diff and presents it in TrackLens code-review mode.
   * Supports reviewing uncommitted changes, a specific ref, or selected files.
   */
  pi.registerTool({
    name: "tracklens_code_review",
    label: "TrackLens Code Review",
    description: `
      Generate and review a git diff in TrackLens code-review mode.

      Use this tool when you need visual review of code changes:
      - Uncommitted changes (default)
      - Changes at a specific git ref
      - Changes in specific files

      The user will see the diff in a side-by-side code review UI with
      annotation support. On denial, structured feedback is returned.
    `.trim(),
    parameters: {
      type: "object",
      properties: {
        gitRef: {
          type: "string",
          description: "Git ref to diff against (default: HEAD for uncommitted changes)",
          default: "HEAD",
        },
        files: {
          type: "array",
          items: { type: "string" },
          description: "Optional list of specific files to include in the diff",
        },
      },
    },

    async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
      const { gitRef = "HEAD", files } = params as {
        gitRef?: string;
        files?: string[];
      };

      // Validate gitRef: prevent flag injection (e.g., "--output=/etc/passwd")
      // Must not start with '-' and must match safe git ref pattern
      if (gitRef.startsWith("-")) {
        return {
          content: [
            {
              type: "text",
              text: "Error: gitRef cannot start with '-' (flag injection prevented)",
            },
          ],
          details: { approved: false },
        };
      }
      // Allow: HEAD, branch names, tags, commit SHAs, refs/* paths, and refspec suffixes (~^:)
      // Reject values that look like git flags
      if (!/^[a-zA-Z0-9_\/\-\.~^:]+$/.test(gitRef)) {
        return {
          content: [
            {
              type: "text",
              text: `Error: gitRef contains invalid characters. Expected branch name, tag, or commit SHA.`,
            },
          ],
          details: { approved: false },
        };
      }

      // Validate files array: prevent flag injection via file paths
      if (files && files.length > 0) {
        for (const file of files) {
          if (file.startsWith("-")) {
            return {
              content: [
                {
                  type: "text",
                  text: `Error: file path cannot start with '-' (flag injection prevented): ${file}`,
                },
              ],
              details: { approved: false },
            };
          }
        }
      }

      // Generate diff using execFileSync to prevent shell injection
      let diffContent: string;
      try {
        if (files && files.length > 0) {
          // Use args array to avoid shell injection
          diffContent = execFileSync("git", ["diff", "--no-ext-diff", "--no-textconv", gitRef, "--", ...files], {
            cwd: ctx.cwd,
            encoding: "utf-8",
            maxBuffer: 10 * 1024 * 1024, // 10MB buffer for large diffs
          });
        } else {
          // No files specified, diff everything
          diffContent = execFileSync("git", ["diff", "--no-ext-diff", "--no-textconv", gitRef], {
            cwd: ctx.cwd,
            encoding: "utf-8",
            maxBuffer: 10 * 1024 * 1024, // 10MB buffer for large diffs
          });
        }
      } catch (error: any) {
        // git diff returns exit code 1 on error, but may still produce output
        if (error.stdout && error.stdout.trim().length > 0) {
          diffContent = error.stdout;
        } else {
          return {
            content: [
              {
                type: "text",
                text: `Error generating diff: ${error.message}`,
              },
            ],
            details: { approved: false },
          };
        }
      }

      if (!diffContent.trim()) {
        return {
          content: [
            {
              type: "text",
              text: "No changes detected in the diff.",
            },
          ],
          details: { approved: true },
        };
      }

      // Import TrackLens server
      let startTrackLensServer: any;
      let htmlContent: string | null = null;

      try {
        // @ts-ignore - Dynamic import for TrackLens server
        const tracklensServer = await import("@maestro/tracklens-server");
        startTrackLensServer = tracklensServer.startTrackLensServer;

        const { existsSync: exists, readFileSync: read } = await import("fs");
        const htmlPaths = [
          resolve(ctx.cwd, "apps/tracklens-opencode/tracklens-review.html"),
          resolve(ctx.cwd, "apps/tracklens-opencode/tracklens.html"),
          resolve(ctx.cwd, "dist/tracklens-editor.html"),
        ];
        for (const htmlPath of htmlPaths) {
          if (exists(htmlPath)) {
            htmlContent = read(htmlPath, "utf-8");
            break;
          }
        }
      } catch {
        // Fallback: return diff as text
        return {
          content: [
            {
              type: "text",
              text: `# Code Review\n\nTrackLens server not available. Diff:\n\n\`\`\`diff\n${diffContent}\n\`\`\``,
            },
          ],
          details: { approved: false, manualReview: true },
        };
      }

      if (!htmlContent) {
        return {
          content: [
            {
              type: "text",
              text: `# Code Review\n\nTrackLens UI not built. Diff:\n\n\`\`\`diff\n${diffContent}\n\`\`\``,
            },
          ],
          details: { approved: false, manualReview: true },
        };
      }

      // Start TrackLens server in code-review mode
      const server = await startTrackLensServer({
        plan: diffContent,
        origin: "pi-maestro",
        htmlContent,
        mode: "code-review",
      });

      try {
        const result = await server.waitForDecision();

        return {
          content: [
            {
              type: "text",
              text: result.feedback || (result.approved ? "Code review approved" : "Changes requested"),
            },
          ],
          details: {
            approved: result.approved,
            annotations: result.annotations,
          },
        };
      } catch (error) {
        return {
          content: [
            {
              type: "text",
              text: `TrackLens server error: ${error}\n\nDiff:\n\n\`\`\`diff\n${diffContent}\n\`\`\``,
            },
          ],
          details: { approved: false, manualReview: true },
        };
      } finally {
        server.stop();
      }
    },
  });
}

