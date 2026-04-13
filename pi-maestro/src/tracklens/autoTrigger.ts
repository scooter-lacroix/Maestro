/**
 * TrackLens Auto-Trigger
 *
 * Detects TrackLens keywords in user messages and auto-invokes the
 * appropriate TrackLens tool when a recent document is available.
 *
 * Integration points:
 * 1. `before_send_message` event — intercepts user message before agent
 *    processing (requires runtime support; no-op if unavailable)
 * 2. `before_agent_start` context — injects keyword-trigger instructions
 *    so the LLM can self-invoke when it sees "tracklens" or "review this"
 *
 * @packageDocumentation
 */

import {
  hasTrackLensKeyword,
  hasReviewTrigger,
  replaceTrackLensKeyword,
} from "./keyword";
import {
  getLastGeneratedDocument,
  recordRecentDocument,
} from "./recentDoc";
import type { ExtensionAPI } from "../types";

/** Context injected into `before_agent_start` for LLM-driven auto-trigger */
const TRACKLENS_TRIGGER_CONTEXT = `
## TrackLens Auto-Trigger

When the user says "tracklens" or "review this" (at end of message), and a recent document has been generated:
- Call \`tracklens_review\` tool with the document content
- For walkthrough documents, call \`tracklens_walkthrough\` instead
- Include the original user message (minus the keyword) as context

If no recent document exists, ask the user what they'd like reviewed.`;

/**
 * Register TrackLens auto-trigger hooks with the extension API.
 *
 * Registers:
 * - `before_send_message` handler for message interception (future-proof)
 * - `before_agent_start` context for LLM-driven auto-trigger
 */
export function registerAutoTrigger(pi: ExtensionAPI): void {
  // Register before_send_message handler for direct message interception.
  // If the runtime supports this event, it will auto-invoke the TrackLens tool.
  // If not, this is a no-op and the LLM-driven path via before_agent_start
  // provides the auto-trigger capability.
  pi.on("before_send_message", async (event: BeforeSendMessageEvent) => {
    const userText = event.text ?? "";

    if (!hasTrackLensKeyword(userText) && !hasReviewTrigger(userText)) {
      return;
    }

    // Prevent double-trigger
    if (event.metadata?.tracklensAutoTriggered) {
      return;
    }

    const lastDoc = getLastGeneratedDocument({ maxAgeMs: 10 * 60 * 1000 });
    if (!lastDoc) {
      // No recent document — let message through to model
      return;
    }

    // Auto-trigger: invoke appropriate TrackLens tool
    event.preventDefault?.();

    const toolName =
      lastDoc.type === "walkthrough"
        ? "tracklens_walkthrough"
        : "tracklens_review";

    const userContext = replaceTrackLensKeyword(userText) || "Auto-triggered review";

    // Send message to agent to invoke the tool
    pi.sendMessage(
      {
        customType: "tracklens-auto-trigger",
        content: `The user requested a TrackLens review. Automatically invoking ${toolName} for the most recent ${lastDoc.type} document (track: ${lastDoc.trackId}).\n\nUser context: ${userContext}`,
        display: false,
      },
      { triggerTurn: true },
    );
  });

  // Register before_agent_start context for LLM-driven auto-trigger
  pi.on("before_agent_start", async () => {
    // Only add context if there's a recent document
    if (!getLastGeneratedDocument()) {
      return;
    }

    return {
      message: {
        customType: "tracklens-trigger-context",
        content: TRACKLENS_TRIGGER_CONTEXT,
        display: false,
      },
    };
  });
}

/**
 * Record a document for auto-trigger tracking.
 *
 * Call this after generating or accessing a document that could be
 * reviewed via TrackLens (spec, plan, walkthrough, etc.).
 */
export { recordRecentDocument };

/** Event shape for before_send_message (proposed runtime extension) */
interface BeforeSendMessageEvent {
  /** The user's message text */
  text?: string;
  /** Event metadata */
  metadata?: Record<string, unknown>;
  /** Prevent the message from being sent to the agent */
  preventDefault?: () => void;
}
