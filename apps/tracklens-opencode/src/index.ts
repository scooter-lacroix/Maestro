/**
 * TrackLens Plugin for OpenCode
 *
 * Provides a Claude Code-style planning experience with interactive plan review.
 * When the agent calls submit_plan, the TrackLens UI opens for the user to
 * annotate, approve, or request changes to the plan.
 *
 * REBRANDED: Plannotator → TrackLens
 * REMOVED: Sharing functionality (getSharingEnabled, getShareBaseUrl, writeRemoteShareLink)
 *
 * Environment variables:
 *   TRACKLENS_REMOTE - Set to "1" or "true" for remote mode (devcontainer, SSH)
 *   TRACKLENS_PORT   - Fixed port to use (default: random locally, 3750 for remote)
 *
 * @packageDocumentation
 */

import { type Plugin, tool } from "@opencode-ai/plugin";
import { startTrackLensServer } from "@maestro/tracklens-server";
import { startReviewServer } from "@maestro/tracklens-server/review";
import { startAnnotateServer } from "@maestro/tracklens-server/annotate";
import { getGitContext, runGitDiff } from "@maestro/tracklens-server/git";

// @ts-ignore - Bun import attribute for text
import reviewHtml from "./tracklens-review.html" with { type: "text" };
// @ts-ignore - Bun import attribute for text
import planHtml from "../tracklens.html" with { type: "text" };

const reviewHtmlContent = reviewHtml as unknown as string;
const planHtmlContent = planHtml as unknown as string;

export const TrackLensPlugin: Plugin = async (ctx: any) => {
  // Config handler: Register submit_plan as primary-only tool (hidden from sub-agents)
  const config = async (opencodeConfig: any) => {
    const existingPrimaryTools = opencodeConfig.experimental?.primary_tools ?? [];
    if (!existingPrimaryTools.includes("submit_plan")) {
      opencodeConfig.experimental = {
        ...opencodeConfig.experimental,
        primary_tools: [...existingPrimaryTools, "submit_plan"],
      };
    }
  };

  // Inject planning instructions into system prompt
  const experimentalChatSystemTransform = async (input: any, output: any) => {
    // Skip for title generation requests
    const existingSystem = output.system.join("\n").toLowerCase();
    if (existingSystem.includes("title generator") || existingSystem.includes("generate a title")) {
      return;
    }

    try {
      // Fetch session messages to determine current agent
      const messagesResponse = await ctx.client.session.messages({
        path: { id: input.sessionID }
      });
      const messages = messagesResponse.data;

      // Find last user message (reverse iteration)
      let lastUserAgent: string | undefined;
      if (messages) {
        for (let i = messages.length - 1; i >= 0; i--) {
          const msg = messages[i];
          if (msg.info.role === "user") {
            // @ts-ignore - UserMessage has agent field
            lastUserAgent = msg.info.agent;
            break;
          }
        }
      }

      // Skip if agent detection fails (safer)
      if (!lastUserAgent) return;

      // Hardcoded exclusion: build agent
      if (lastUserAgent === "build") return;

      // Dynamic exclusion: check agent mode via API
      const agentsResponse = await ctx.client.app.agents({
        query: { directory: ctx.directory }
      });
      const agents = agentsResponse.data;
      const agent = agents?.find((a: { name: string }) => a.name === lastUserAgent);

      // Skip if agent is a sub-agent
      // @ts-ignore - Agent has mode field
      if (agent?.mode === "subagent") return;

    } catch {
      // Skip injection on any error (safer)
      return;
    }

    output.system.push(`
## Plan Submission

When you have completed your plan, you MUST call the \`submit_plan\` tool to submit it for user review.
The user will be able to:
- Review your plan visually in a dedicated UI
- Annotate specific sections with feedback
- Approve the plan to proceed with implementation
- Request changes with detailed feedback

If your plan is rejected, you will receive the user's annotated feedback. Revise your plan
based on their feedback and call submit_plan again.

Do NOT proceed with implementation until your plan is approved.
`);
  };

  // Event handler: Listen for /tracklens-review command
  const eventHandler = async ({ event }: { event: any }) => {
    // Check for command execution event
    const isCommandEvent =
      event.type === "command.executed" ||
      event.type === "tui.command.execute";

    // @ts-ignore - Event structure: event.properties.name for command.executed
    const commandName = event.properties?.name || event.command || event.payload?.name;

    if (isCommandEvent && commandName === "tracklens-review") {
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
        origin: "opencode",
        diffType: "uncommitted",
        gitContext,
        htmlContent: reviewHtmlContent,
      });

      // Wait for user decision
      const result = await server.waitForDecision();

      // Give browser time to receive response and update UI
      await Bun.sleep(1500);

      // Handle agent switch if requested
      if (result.agentSwitch) {
        // Switch to the specified agent
        const targetAgent = result.agentSwitch;
        console.log(`[TrackLens] Switching to agent: ${targetAgent}`);

        // Get agent list to find the agent ID
        try {
          const agentsResponse = await ctx.client.app.agents({
            query: { directory: ctx.directory }
          });
          const agents = agentsResponse.data;
          const targetAgentObj = agents?.find((a: { name: string }) => a.name === targetAgent);

          if (targetAgentObj) {
            // @ts-ignore - Agent has id field
            await ctx.client.agents.switch({
              agent_id: targetAgentObj.id,
            });
          }
        } catch (error) {
          console.error(`[TrackLens] Failed to switch agent: ${error}`);
        }
      }

      // Return feedback to agent
      if (result.feedback) {
        // Format as structured response
        return {
          content: `## Code Review Feedback\n\n${result.feedback}`,
          wait: true,
        };
      }
    }
  };

  // Register tools
  return {
    config,
    "experimental.chat.system.transform": experimentalChatSystemTransform,
    event: eventHandler,
    tools: {
      submit_plan: tool({
        description: `
          Submit your plan for user review in the TrackLens UI.

          The user will be able to:
          - Visually review your plan in a dedicated UI
          - Annotate specific sections with comments or suggestions
          - Approve the plan to proceed with implementation
          - Request changes with detailed feedback

          After submission, wait for the user's decision:
          - If approved: Your plan is accepted and you can proceed with implementation
          - If rejected: You will receive detailed feedback that must be addressed

          IMPORTANT: Do NOT proceed with implementation until the plan is approved.
        `,
        parameters: {
          plan: {
            type: "string",
            description: "The plan markdown content to submit for review",
          },
          agentSwitch: {
            type: "string",
            description: "Optional agent name to switch to after approval",
          },
          autonomyMode: {
            type: "string",
            description: "Optional autonomy mode to set after approval",
          },
        },
        execute: async (params: { plan?: string; agentSwitch?: string; autonomyMode?: string }) => {
          const { plan, agentSwitch, autonomyMode } = params;

          if (!plan) {
            return { content: "Error: Plan content is required" };
          }

          // Start TrackLens server with plan content
          const server = await startTrackLensServer({
            plan,
            origin: "opencode",
            htmlContent: planHtmlContent,
            autonomyMode,
          });

          // Wait for user decision
          const result = await server.waitForDecision();

          // Give browser time to receive response and update UI
          await Bun.sleep(1500);

          // Handle agent switch if requested
          if (result.agentSwitch) {
            const targetAgent = result.agentSwitch;
            console.log(`[TrackLens] Switching to agent: ${targetAgent}`);

            try {
              const agentsResponse = await ctx.client.app.agents({
                query: { directory: ctx.directory }
              });
              const agents = agentsResponse.data;
              const targetAgentObj = agents?.find((a: { name: string }) => a.name === targetAgent);

              if (targetAgentObj) {
                // @ts-ignore - Agent has id field
                await ctx.client.agents.switch({
                  agent_id: targetAgentObj.id,
                });
              }
            } catch (error) {
              console.error(`[TrackLens] Failed to switch agent: ${error}`);
            }
          }

          // Return feedback based on decision
          if (result.approved) {
            return {
              content: result.feedback
                ? `## Plan Approved\n\n${result.feedback}`
                : "Plan approved. You may proceed with implementation.",
            };
          } else {
            return {
              content: result.feedback
                ? `## Plan Rejected\n\n${result.feedback}`
                : "Plan rejected. Please revise based on user feedback.",
            };
          }
        },
      }),

      tracklens_review: tool({
        description: `
          Launch a code review in the TrackLens UI.

          Opens the TrackLens review interface showing git diffs, allowing the user
          to review changes visually and provide feedback.

          Supported diff types:
          - uncommitted: All uncommitted changes (default)
          - staged: Staged changes only
          - unstaged: Unstaged changes only
          - last-commit: The most recent commit
          - branch: Current branch vs default branch
        `,
        parameters: {
          diffType: {
            type: "string",
            description: "Git diff type: uncommitted, staged, unstaged, last-commit, branch",
            default: "uncommitted",
          },
        },
        execute: async (params: { diffType?: string }) => {
          const gitContext = await getGitContext();
          const diffType = (params.diffType || "uncommitted") as
            | "uncommitted"
            | "staged"
            | "unstaged"
            | "last-commit"
            | "branch";

          const { patch, label, error } = await runGitDiff(diffType, gitContext.defaultBranch);

          const server = await startReviewServer({
            rawPatch: patch,
            gitRef: label,
            error,
            origin: "opencode",
            diffType,
            gitContext,
            htmlContent: reviewHtmlContent,
          });

          const result = await server.waitForDecision();

          // Give browser time to receive response and update UI
          await Bun.sleep(1500);

          // Handle agent switch if requested
          if (result.agentSwitch) {
            const targetAgent = result.agentSwitch;
            console.log(`[TrackLens] Switching to agent: ${targetAgent}`);

            try {
              const agentsResponse = await ctx.client.app.agents({
                query: { directory: ctx.directory }
              });
              const agents = agentsResponse.data;
              const targetAgentObj = agents?.find((a: { name: string }) => a.name === targetAgent);

              if (targetAgentObj) {
                // @ts-ignore - Agent has id field
                await ctx.client.agents.switch({
                  agent_id: targetAgentObj.id,
                });
              }
            } catch (error) {
              console.error(`[TrackLens] Failed to switch agent: ${error}`);
            }
          }

          // Return feedback to agent
          if (result.feedback) {
            return {
              content: `## Code Review Feedback\n\n${result.feedback}`,
            };
          }

          return {
            content: "Review complete. No feedback provided.",
          };
        },
      }),

      tracklens_annotate: tool({
        description: `
          Launch annotation mode for a specific file in the TrackLens UI.

          Opens the TrackLens annotation interface showing a markdown file,
          allowing the user to annotate specific sections with comments or suggestions.

          Use this tool when you want the user to review or annotate a specific file,
          such as a spec document, plan, or any markdown content.
        `,
        parameters: {
          filePath: {
            type: "string",
            description: "Path to the file to annotate (relative to project root)",
          },
        },
        execute: async (params: { filePath?: string }) => {
          const { filePath } = params;

          if (!filePath) {
            return { content: "Error: filePath is required" };
          }

          // Read the file content
          let fileContent: string;
          try {
            fileContent = await Bun.file(filePath).text();
          } catch (error) {
            return {
              content: `Error: Failed to read file "${filePath}": ${error}`,
            };
          }

          // Start annotate server
          const server = await startAnnotateServer({
            markdown: fileContent,
            filePath,
            origin: "opencode",
            htmlContent: reviewHtmlContent, // Reuse review HTML for annotation mode
          });

          const result = await server.waitForDecision();

          // Give browser time to receive response and update UI
          await Bun.sleep(1500);

          // Return feedback to agent
          if (result.feedback) {
            return {
              content: `## Annotation Feedback\n\n${result.feedback}`,
            };
          }

          return {
            content: "Annotation complete. No feedback provided.",
          };
        },
      }),
    },
  };
};
