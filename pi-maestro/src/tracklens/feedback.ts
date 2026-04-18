/**
 * TrackLens Structured Feedback Formatting
 *
 * Formats denial feedback from TrackLens reviews into structured markdown
 * that agents can parse and act upon. Annotations are grouped by severity
 * (ERROR > WARNING > INFO) with clear formatting.
 *
 * @packageDocumentation
 */

/** Severity levels for review annotations */
export type AnnotationSeverity = "ERROR" | "WARNING" | "INFO";

/** A single annotation from a TrackLens review */
export interface TrackLensAnnotation {
  /** Severity level */
  severity?: AnnotationSeverity;
  /** Line number in the document */
  lineNumber?: number;
  /** Quoted text from the document */
  selectionText?: string;
  /** User comment */
  comment: string;
}

/** The result of a TrackLens review decision */
export interface TrackLensDecisionResult {
  /** Whether the user approved */
  approved: boolean;
  /** General feedback text */
  feedback?: string;
  /** Annotations from the review */
  annotations?: TrackLensAnnotation[];
  /** Edited content if the user made edits */
  editedContent?: string;
  /** File path where content was saved */
  savedPath?: string;
}

/**
 * Format a TrackLens denial for agent consumption.
 *
 * Output structure:
 * 1. Header with denial notice
 * 2. General feedback (if any)
 * 3. Annotations grouped by severity (ERROR > WARNING > INFO)
 * 4. Edited content note (if user edited)
 * 5. Footer with action instruction
 *
 * @param result - The TrackLens decision result
 * @param documentType - Type of document that was reviewed
 * @returns Formatted markdown string for the agent
 */
export function formatDenialForAgent(
  result: TrackLensDecisionResult,
  documentType: string,
): string {
  const sections: string[] = [];

  // Header
  sections.push(`# TrackLens Review: Changes Requested`);
  sections.push(`**Document Type:** ${documentType}`);
  sections.push("");

  // General feedback
  if (result.feedback && result.feedback.trim()) {
    sections.push("## Feedback");
    sections.push(result.feedback.trim());
    sections.push("");
  }

  // Annotations grouped by severity
  if (result.annotations && result.annotations.length > 0) {
    const grouped = groupBySeverity(result.annotations);

    sections.push("## Annotations");

    const severityOrder: AnnotationSeverity[] = ["ERROR", "WARNING", "INFO"];
    for (const severity of severityOrder) {
      const group = grouped.get(severity);
      if (!group || group.length === 0) continue;

      const icon = severityIcon(severity);
      sections.push(`### ${icon} ${severity} (${group.length})`);
      sections.push("");

      for (let i = 0; i < group.length; i++) {
        const ann = group[i]!;
        sections.push(`**${i + 1}.** ${ann.comment}`);

        if (ann.selectionText) {
          sections.push(`> ${ann.selectionText}`);
        }

        if (ann.lineNumber !== undefined) {
          sections.push(`*Line ${ann.lineNumber}*`);
        }

        sections.push("");
      }
    }
  }

  // Edited content note
  if (result.editedContent) {
    sections.push("## Edited Content");
    sections.push("The user edited the document during review. Their edited version is available.");
    sections.push("");
  }

  // Footer
  sections.push("---");
  sections.push(
    "**Action Required:** Address the feedback above and resubmit for review using `tracklens_review`.",
  );

  return sections.join("\n");
}

/** Group annotations by severity level */
function groupBySeverity(
  annotations: TrackLensAnnotation[],
): Map<AnnotationSeverity, TrackLensAnnotation[]> {
  const grouped = new Map<AnnotationSeverity, TrackLensAnnotation[]>();

  for (const ann of annotations) {
    const severity: AnnotationSeverity = ann.severity || "INFO";
    if (!grouped.has(severity)) {
      grouped.set(severity, []);
    }
    grouped.get(severity)!.push(ann);
  }

  // Sort within each group by line number
  for (const [, group] of grouped) {
    group.sort((a, b) => (a.lineNumber ?? 0) - (b.lineNumber ?? 0));
  }

  return grouped;
}

/** Get an icon for a severity level */
function severityIcon(severity: AnnotationSeverity): string {
  switch (severity) {
    case "ERROR":
      return "X";
    case "WARNING":
      return "!";
    case "INFO":
      return "i";
  }
}
