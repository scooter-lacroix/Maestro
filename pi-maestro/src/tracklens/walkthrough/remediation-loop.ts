/**
 * TrackLens Walkthrough Remediation Loop
 *
 * Manages the complete walkthrough review and remediation workflow.
 *
 * @packageDocumentation
 */

import { generateWalkthrough } from "./generator.js";
import { saveWalkthrough, saveFinalWalkthrough } from "./storage.js";
import {
  processWalkthroughReview,
  executeRemediationTasks,
  formatRemediationTasks,
  type WalkthroughReviewResult,
  type RemediationTask,
} from "./remediation.js";

export interface RemediationLoopOptions {
  trackId: string;
  root: string;
  trackDir: string;
  maxIterations?: number;
  onReview?: (walkthrough: string, iteration: number) => Promise<WalkthroughReviewResult>;
  onRemediation?: (tasks: RemediationTask[], iteration: number) => Promise<void>;
}

export interface RemediationLoopResult {
  approved: boolean;
  totalIterations: number;
  finalWalkthrough?: string;
  remediationTasks?: RemediationTask[];
}

/**
 * Run the complete walkthrough review and remediation loop
 *
 * @param options - Loop options
 * @returns Final result with approval status
 */
export async function runRemediationLoop(
  options: RemediationLoopOptions
): Promise<RemediationLoopResult> {
  const { trackId, root, trackDir, maxIterations = 5, onReview, onRemediation } = options;

  let iteration = 0;
  let walkthrough = generateWalkthrough({
    trackId,
    root,
    trackDir,
    includeDiffs: true,
    includeSnippets: true,
  });

  // Save initial walkthrough
  await saveWalkthrough(trackId, walkthrough);

  while (iteration < maxIterations) {
    iteration++;

    // Present walkthrough for review
    let reviewResult: WalkthroughReviewResult;

    if (onReview) {
      reviewResult = await onReview(walkthrough.markdown, iteration);
    } else {
      // Default: auto-approve (for testing)
      reviewResult = { approved: true };
    }

    // Check if approved
    if (reviewResult.approved) {
      // Save final walkthrough to track directory
      saveFinalWalkthrough(trackDir, walkthrough.markdown);

      return {
        approved: true,
        totalIterations: iteration,
        finalWalkthrough: walkthrough.markdown,
      };
    }

    // Process denial feedback
    const remediationTasks = processWalkthroughReview(reviewResult, walkthrough);

    if (!remediationTasks || remediationTasks.length === 0) {
      // No actionable feedback, re-present with note
      console.log("No actionable feedback provided. Re-presenting walkthrough...");
      continue;
    }

    // Execute remediation tasks
    if (onRemediation) {
      await onRemediation(remediationTasks, iteration);
    } else {
      await executeRemediationTasks(remediationTasks);
    }

    // Regenerate walkthrough
    walkthrough = generateWalkthrough({
      trackId,
      root,
      trackDir,
      includeDiffs: true,
      includeSnippets: true,
    });

    // Save updated walkthrough
    await saveWalkthrough(trackId, walkthrough);
  }

  // Max iterations reached
  return {
    approved: false,
    totalIterations: iteration,
  };
}

export { processWalkthroughReview, executeRemediationTasks, formatRemediationTasks };
export type { WalkthroughReviewResult, RemediationTask };
