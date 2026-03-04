/**
 * TrackLens Walkthrough Remediation
 *
 * Handles the denial remediation loop for walkthrough reviews.
 * Converts user annotations into remediation tasks and manages
 * the regenerate/re-present workflow.
 *
 * @packageDocumentation
 */

// Define a local annotation type for walkthrough feedback
export interface WalkthroughAnnotation {
  id: string;
  blockId: string;
  type: "comment" | "concern" | "suggestion";
  text?: string;
  originalText: string;
  created_a: number;
  author?: string;
}
import type { GeneratedWalkthrough } from "./types.js";

/**
 * User decision from walkthrough review
 */
export interface WalkthroughReviewResult {
  /** Whether the walkthrough was approved */
  approved: boolean;
  /** User feedback (if denied) */
  feedback?: string;
  /** User annotations (if any) */
  annotations?: WalkthroughAnnotation[];
  /** Path to saved walkthrough */
  savedPath?: string;
}

/**
 * Remediation task created from user annotation
 */
export interface RemediationTask {
  /** Task description */
  description: string;
  /** Annotation that created this task */
  annotation: WalkthroughAnnotation;
  /** Priority (high/medium/low) */
  priority: "high" | "medium" | "low";
  /** Estimated complexity in hours */
  estimateHours: number;
}

/**
 * Process walkthrough review and create remediation tasks if needed
 *
 * @param reviewResult - Result from walkthrough review
 * @param walkthrough - The walkthrough that was reviewed
 * @returns Array of remediation tasks, or null if approved
 */
export function processWalkthroughReview(
  reviewResult: WalkthroughReviewResult,
  walkthrough: GeneratedWalkthrough
): RemediationTask[] | null {
  // If approved, no remediation needed
  if (reviewResult.approved) {
    return null;
  }

  const remediationTasks: RemediationTask[] = [];

  // Process annotations into remediation tasks
  if (reviewResult.annotations && reviewResult.annotations.length > 0) {
    for (const annotation of reviewResult.annotations) {
      const task = annotationToRemediationTask(annotation);
      if (task) {
        remediationTasks.push(task);
      }
    }
  }

  // If no annotations but feedback provided, create a general task
  if (remediationTasks.length === 0 && reviewResult.feedback) {
    remediationTasks.push({
      description: `Address walkthrough feedback: ${reviewResult.feedback}`,
      annotation: {
        id: "general-feedback",
        blockId: "walkthrough",
        type: "comment" as any,
        text: reviewResult.feedback,
        originalText: reviewResult.feedback,
        created_a: Date.now(),
      },
      priority: "medium",
      estimateHours: 1,
    });
  }

  return remediationTasks;
}

/**
 * Convert an annotation to a remediation task
 */
function annotationToRemediationTask(annotation: WalkthroughAnnotation): RemediationTask | null {
  // Skip global comments without actionable content
  if (annotation.type === "comment") {
    const text = annotation.text?.toLowerCase() || "";
    if (text.length < 10) {
      return null; // Too short to be actionable
    }
  }

  // Determine priority based on annotation type and content
  const priority = determinePriority(annotation);

  // Estimate complexity
  const estimateHours = estimateComplexity(annotation);

  // Generate task description
  const description = generateTaskDescription(annotation);

  return {
    description,
    annotation,
    priority,
    estimateHours,
  };
}

/**
 * Determine task priority from annotation
 */
function determinePriority(annotation: WalkthroughAnnotation): "high" | "medium" | "low" {
  const text = annotation.text?.toLowerCase() || "";

  // High priority indicators
  if (
    annotation.type === "concern" ||
    text.includes("critical") ||
    text.includes("urgent") ||
    text.includes("security") ||
    text.includes("bug") ||
    text.includes("fix")
  ) {
    return "high";
  }

  // Low priority indicators
  if (
    annotation.type === "comment" ||
    text.includes("consider") ||
    text.includes("maybe") ||
    text.includes("suggestion") ||
    text.includes("minor")
  ) {
    return "low";
  }

  return "medium";
}

/**
 * Estimate task complexity in hours
 */
function estimateComplexity(annotation: WalkthroughAnnotation): number {
  const text = annotation.text || "";

  // Base estimate by type
  let baseHours = 1;
  switch (annotation.type) {
    case "concern":
      baseHours = 0.5;
      break;
    case "suggestion":
      baseHours = 2;
      break;
    case "comment":
      baseHours = 1;
      break;
    default:
      baseHours = 1;
  }

  // Adjust based on text length
  const lengthMultiplier = Math.min(2, Math.max(0.5, text.length / 100));

  return Math.round(baseHours * lengthMultiplier * 10) / 10;
}

/**
 * Generate task description from annotation
 */
function generateTaskDescription(annotation: WalkthroughAnnotation): string {
  const text = annotation.text || "No description provided";

  // Add context from annotation type
  let prefix = "";
  switch (annotation.type) {
    case "concern":
      prefix = "Fix: ";
      break;
    case "suggestion":
      prefix = "Add: ";
      break;
    case "comment":
      prefix = "Address: ";
      break;
    default:
      prefix = "Review: ";
  }

  // Truncate if too long
  const maxDescLength = 100;
  const trimmedText =
    text.length > maxDescLength ? text.substring(0, maxDescLength - 3) + "..." : text;

  return prefix + trimmedText;
}

/**
 * Format remediation tasks for display
 */
export function formatRemediationTasks(tasks: RemediationTask[]): string {
  if (tasks.length === 0) {
    return "No remediation tasks identified.";
  }

  let output = `# Remediation Tasks (${tasks.length})\n\n`;

  // Group by priority
  const high = tasks.filter((t) => t.priority === "high");
  const medium = tasks.filter((t) => t.priority === "medium");
  const low = tasks.filter((t) => t.priority === "low");

  if (high.length > 0) {
    output += `## High Priority\n`;
    for (const task of high) {
      output += `- [ ] ${task.description} (~${task.estimateHours}h)\n`;
    }
    output += `\n`;
  }

  if (medium.length > 0) {
    output += `## Medium Priority\n`;
    for (const task of medium) {
      output += `- [ ] ${task.description} (~${task.estimateHours}h)\n`;
    }
    output += `\n`;
  }

  if (low.length > 0) {
    output += `## Low Priority\n`;
    for (const task of low) {
      output += `- [ ] ${task.description} (~${task.estimateHours}h)\n`;
    }
  }

  const totalHours = tasks.reduce((sum, t) => sum + t.estimateHours, 0);
  output += `\n**Total estimated effort:** ~${totalHours}h\n`;

  return output;
}

/**
 * Execute remediation tasks
 *
 * Appends tasks to plan.md and marks them for execution.
 * Tasks can be executed manually or by an agent runner.
 *
 * @param tasks - Remediation tasks to execute
 * @param trackPath - Path to the track directory
 * @param ctx - Execution context (optional)
 * @returns Success status with task results
 */
export async function executeRemediationTasks(
  tasks: RemediationTask[],
  trackPath?: string,
  ctx?: any
): Promise<{ success: boolean; results: any[] }> {
  const results: any[] = [];

  for (const task of tasks) {
    // Generate task ID from annotation
    const taskId = task.annotation.id || `task-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

    // Append task to plan.md if track path is provided
    if (trackPath) {
      try {
        const { readFile, writeFile } = await import("fs/promises");
        const { join } = await import("path");
        const planPath = join(trackPath, "plan.md");

        // Read existing plan content
        let planContent = "";
        try {
          planContent = await readFile(planPath, "utf-8");
        } catch {
          // File doesn't exist, create with header
          planContent = "# Plan\n\n";
        }

        // Format the new task as a checklist item
        const priorityMarker = {
          high: "HIGH",
          medium: "MED",
          low: "LOW"
        }[task.priority];

        const newTaskEntry = `- [ ] **[${priorityMarker}]** ${task.description} (~${task.estimateHours}h)\n`;

        // Append to plan
        const updatedPlan = planContent.trimEnd() + "\n\n" + newTaskEntry;
        await writeFile(planPath, updatedPlan, "utf-8");

        results.push({
          taskId,
          status: "pending",
          description: task.description,
          priority: task.priority,
          estimateHours: task.estimateHours,
          appendedTo: planPath
        });
      } catch (error) {
        // If file operations fail, still mark task as pending
        results.push({
          taskId,
          status: "pending",
          description: task.description,
          error: error instanceof Error ? error.message : String(error)
        });
      }
    } else {
      // No track path provided, just log the task
      console.log(`[REMEDIATE] ${task.description} (${task.priority}, ~${task.estimateHours}h)`);
      results.push({
        taskId,
        status: "pending",
        description: task.description
      });
    }
  }

  return {
    success: true,
    results
  };
}
