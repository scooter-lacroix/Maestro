/**
 * TrackLens Walkthrough Generator
 *
 * Generates comprehensive walkthrough documents for completed Maestro tracks.
 * Includes completed tasks, changed files with diffs/snippets, and spec summary.
 *
 * @packageDocumentation
 */

import { readFileSync, existsSync } from "fs";
import { resolve, join } from "path";
import { execSync } from "child_process";

import type {
  WalkthroughOptions,
  ChangedFile,
  CompletedTask,
  WalkthroughMetadata,
  GeneratedWalkthrough,
} from "./types.js";
import { FileChangeStatus } from "./types.js";

/**
 * Generate a comprehensive walkthrough for a completed track
 *
 * @param options - Walkthrough generation options
 * @returns Generated walkthrough content with metadata
 */
export function generateWalkthrough(options: WalkthroughOptions): GeneratedWalkthrough {
  const {
    trackId,
    root,
    trackDir,
    isSubtrack = false,
    parentTrackId,
    includeDiffs = true,
    includeSnippets = true,
    maxSnippetLines = 30,
  } = options;

  // Read track metadata
  const metadata = readTrackMetadata(trackDir, trackId);

  // Read spec and plan
  const specContent = readTrackSpec(trackDir);
  const planContent = readTrackPlan(trackDir);

  // Extract completed tasks from plan
  const completedTasks = extractCompletedTasks(planContent);

  // Get changed files via git
  const changedFiles = getTrackChangedFiles(root, trackDir, {
    includeDiffs,
    includeSnippets,
    maxSnippetLines,
  });

  // Build walkthrough metadata
  const walkthroughMetadata: WalkthroughMetadata = {
    trackId,
    description: metadata.description || trackId,
    type: metadata.type,
    status: metadata.status || "Completed",
    isSubtrack,
    parentTrackId,
    generatedAt: new Date().toISOString(),
  };

  // Generate markdown content
  const markdown = generateWalkthroughMarkdown(
    walkthroughMetadata,
    specContent,
    completedTasks,
    changedFiles
  );

  return {
    markdown,
    metadata: walkthroughMetadata,
    completedTasks,
    changedFiles,
  };
}

/**
 * Generate walkthrough markdown document
 */
function generateWalkthroughMarkdown(
  metadata: WalkthroughMetadata,
  specContent: string,
  completedTasks: CompletedTask[],
  changedFiles: ChangedFile[]
): string {
  let doc = `# Track Walkthrough: ${metadata.description}\n\n`;

  // Header section
  doc += `**Track ID:** \`${metadata.trackId}\`\n`;
  if (metadata.type) {
    doc += `**Type:** ${metadata.type}\n`;
  }
  doc += `**Status:** ${metadata.status}\n`;
  doc += `**Completed:** ${new Date(metadata.generatedAt).toLocaleDateString()}\n`;

  if (metadata.isSubtrack && metadata.parentTrackId) {
    doc += `**Parent Track:** \`${metadata.parentTrackId}\`\n`;
  }

  doc += `\n---\n\n`;

  // Specification summary section
  if (specContent.trim()) {
    doc += `## Specification Summary\n\n`;
    const specSummary = extractSpecSummary(specContent);
    doc += specSummary;
    doc += `\n`;
  }

  // Completed tasks section
  doc += `## Completed Tasks\n\n`;
  if (completedTasks.length === 0) {
    doc += `*No tasks found in plan.md*\n\n`;
  } else {
    let currentPhase = "";
    for (const task of completedTasks) {
      // Add phase header if changed
      if (task.phase && task.phase !== currentPhase) {
        if (currentPhase) doc += `\n`;
        doc += `### ${task.phase}\n\n`;
        currentPhase = task.phase;
      }
      const commitNote = task.commit ? ` (${task.commit.slice(0, 7)})` : "";
      doc += `- [x] ${task.description}${commitNote}\n`;
    }
    doc += `\n`;
  }

  // Files changed section
  doc += `## Files Changed\n\n`;

  if (changedFiles.length === 0) {
    doc += `*No changed files detected*\n\n`;
  } else {
    // Summary table
    doc += `| Status | File | Additions | Deletions |\n`;
    doc += `|--------|------|-----------|----------|\n`;

    for (const file of changedFiles) {
      const statusIcon = getStatusIcon(file.status);
      const relPath = file.path;
      const additions = file.additions || 0;
      const deletions = file.deletions || 0;
      doc += `| ${statusIcon} | \`${relPath}\` | +${additions} | -${deletions} |\n`;
    }

    doc += `\n`;

    // Detailed changes section
    doc += `## Detailed Changes\n\n`;

    for (const file of changedFiles) {
      doc += `### ${file.path}\n\n`;

      if (file.snippet) {
        doc += `#### Key Code\n\n`;
        doc += `\`\`\`${file.language}\n${file.snippet}\n\`\`\`\n\n`;
      }

      if (file.diff) {
        doc += `<details><summary>Full diff</summary>\n\n`;
        doc += `\`\`\`diff\n${file.diff}\n\`\`\`\n`;
        doc += `</details>\n\n`;
      }
    }
  }

  doc += `---\n\n`;
  doc += `> **Review this walkthrough.** Annotate any issues for remediation.\n`;

  return doc;
}

/**
 * Read track metadata from metadata.json or tracks.md
 */
function readTrackMetadata(trackDir: string, trackId: string): any {
  // First try metadata.json
  const metadataPath = join(trackDir, "metadata.json");
  if (existsSync(metadataPath)) {
    return JSON.parse(readFileSync(metadataPath, "utf-8"));
  }

  // Fallback: extract from tracks.md
  const tracksPath = join(trackDir, "..", "tracks.md");
  if (existsSync(tracksPath)) {
    const tracksContent = readFileSync(tracksPath, "utf-8");
    const trackSection = tracksContent.match(
      new RegExp(`##[^\\n]*${trackId}[^\\n]*\\n([\\s\\S]*?)(?=##|---|$)`)
    );
    if (trackSection) {
      // Parse description from heading
      const heading = trackSection[0].match(/##[^\n]*\n[^\n]*/);
      if (heading) {
        return {
          description: heading[0].replace(/##\s*/, "").trim(),
          status: "In Progress",
        };
      }
    }
  }

  // Minimal fallback
  return {
    description: trackId,
    status: "Completed",
  };
}

/**
 * Read track specification
 */
function readTrackSpec(trackDir: string): string {
  const specPath = join(trackDir, "spec.md");
  if (existsSync(specPath)) {
    return readFileSync(specPath, "utf-8");
  }
  return "";
}

/**
 * Read track implementation plan
 */
function readTrackPlan(trackDir: string): string {
  const planPath = join(trackDir, "plan.md");
  if (existsSync(planPath)) {
    return readFileSync(planPath, "utf-8");
  }
  return "";
}

/**
 * Extract completed tasks from plan.md
 */
export function extractCompletedTasks(planContent: string): CompletedTask[] {
  const tasks: CompletedTask[] = [];
  const lines = planContent.split("\n");

  let currentPhase = "";

  for (const line of lines) {
    // Detect phase headers
    const phaseMatch = line.match(/^##\s+Phase\s+(\d+)/i);
    if (phaseMatch) {
      const phaseTitle = line.replace(/^##\s+/, "").trim();
      currentPhase = `Phase ${phaseMatch[1]}${phaseTitle.slice(phaseTitle.indexOf(phaseMatch[1]) + phaseMatch[1].length).trim()}`;
      continue;
    }

    // Detect completed tasks
    const taskMatch = line.match(/^\s*-\s*\[x\]\s+(.+?)(?:\s+\(([a-f0-9]+)\))?$/);
    if (taskMatch) {
      const description = taskMatch[1].trim();
      const commit = taskMatch[2];

      // Clean up task description
      const cleanDescription = description
        .replace(/^Task:\s*/i, "")
        .replace(/\s*Note:.*$/i, "")
        .trim();

      tasks.push({
        description: cleanDescription,
        phase: currentPhase || undefined,
        commit: commit || undefined,
      });
    }
  }

  return tasks;
}

/**
 * Extract key information from spec.md for walkthrough
 */
export function extractSpecSummary(specContent: string): string {
  const lines = specContent.split("\n");
  let summary = "";
  let inOverview = false;
  let inGoals = false;

  for (const line of lines) {
    // Overview section
    if (line.match(/^##\s+Overview/i)) {
      inOverview = true;
      continue;
    }
    if (inOverview && line.match(/^##\s/)) {
      inOverview = false;
    }
    if (inOverview && line.trim()) {
      summary += line + "\n";
    }

    // Goals section (limit to 5 items)
    if (line.match(/^##\s+Goals/i) || line.match(/^##\s+Objectives/i)) {
      inGoals = true;
      summary += "\n**Goals:**\n";
      continue;
    }
    if (inGoals && line.match(/^##\s/)) {
      inGoals = false;
      continue;
    }
    if (inGoals && line.match(/^\s*-\s+/)) {
      summary += line + "\n";
    }

    // Only take first 10 lines of summary to keep it concise
    if (summary.split("\n").length > 10) {
      break;
    }
  }

  return summary.trim() || "*See spec.md for full specification*";
}

/**
 * Get changed files for a track using git
 */
function getTrackChangedFiles(
  root: string,
  trackDir: string,
  options: {
    includeDiffs?: boolean;
    includeSnippets?: boolean;
    maxSnippetLines?: number;
  } = {}
): ChangedFile[] {
  const { includeDiffs = true, includeSnippets = true, maxSnippetLines = 30 } = options;

  try {
    // Get list of changed files since track was created
    // We'll use git log to find the first commit for this track
    const trackName = trackDir.split("/").pop();
    const sinceCommit = findTrackStartCommit(root, trackName!);

    if (!sinceCommit) {
      // No commits found for this track
      return [];
    }

    // Get changed files
    const diffOutput = execSync(
      `git diff --name-status ${sinceCommit}^..HEAD`,
      { cwd: root, encoding: "utf-8" }
    );

    const files: ChangedFile[] = [];
    const fileEntries = diffOutput.trim().split("\n");

    for (const entry of fileEntries) {
      if (!entry) continue;

      const [status, ...pathParts] = entry.split("\t");
      const path = pathParts.join("\t"); // Handle files with tabs in names (rare)

      if (!path) continue;

      const fileStatus = parseFileStatus(status);
      const language = detectLanguage(path);

      const changedFile: ChangedFile = {
        path,
        status: fileStatus,
        language,
        additions: 0,
        deletions: 0,
      };

      // Get diff stats
      try {
        const stats = execSync(
          `git diff --numstat ${sinceCommit}^..HEAD -- "${path}"`,
          { cwd: root, encoding: "utf-8" }
        );
        const statsMatch = stats.trim().match(/^(\d+)\s+(\d+)\s+/);
        if (statsMatch) {
          changedFile.additions = parseInt(statsMatch[1], 10);
          changedFile.deletions = parseInt(statsMatch[2], 10);
        }
      } catch (e) {
        // Stats failed, continue without them
      }

      // Get full diff if requested
      if (includeDiffs) {
        try {
          const diff = execSync(
            `git diff ${sinceCommit}^..HEAD -- "${path}"`,
            { cwd: root, encoding: "utf-8" }
          );
          changedFile.diff = diff.trim();
        } catch (e) {
          // Diff failed
        }
      }

      // Extract snippet if requested
      if (includeSnippets && path.match(/\.(ts|tsx|js|jsx|rs|go|py)$/)) {
        try {
          changedFile.snippet = extractCodeSnippet(
            join(root, path),
            maxSnippetLines
          );
        } catch (e) {
          // Snippet extraction failed
        }
      }

      files.push(changedFile);
    }

    return files;
  } catch (e) {
    // Git command failed
    return [];
  }
}

/**
 * Find the first commit for a track
 */
function findTrackStartCommit(root: string, trackName: string): string | undefined {
  try {
    // Search for commits with track name in message
    const logOutput = execSync(
      `git log --all --grep="${trackName}" --format="%H" -n 1`,
      { cwd: root, encoding: "utf-8" }
    );

    const commit = logOutput.trim();
    return commit || undefined;
  } catch (e) {
    return undefined;
  }
}

/**
 * Parse git file status
 */
function parseFileStatus(status: string): FileChangeStatus {
  switch (status.trim()) {
    case "A":
      return FileChangeStatus.Added;
    case "M":
      return FileChangeStatus.Modified;
    case "D":
      return FileChangeStatus.Deleted;
    case "R":
      return FileChangeStatus.Renamed;
    default:
      return FileChangeStatus.Modified;
  }
}

/**
 * Detect programming language from file extension
 */
function detectLanguage(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase();

  const languageMap: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    rs: "rust",
    go: "go",
    py: "python",
    java: "java",
    cpp: "cpp",
    c: "c",
    cs: "csharp",
    php: "php",
    rb: "ruby",
    sh: "bash",
    md: "markdown",
    json: "json",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    xml: "xml",
    html: "html",
    css: "css",
    sql: "sql",
  };

  return languageMap[ext || ""] || "text";
}

/**
 * Extract key code snippet from a file
 */
function extractCodeSnippet(filePath: string, maxLines: number): string | undefined {
  if (!existsSync(filePath)) {
    return undefined;
  }

  try {
    const content = readFileSync(filePath, "utf-8");
    const lines = content.split("\n");

    // Extract the first meaningful section
    // Skip shebang, empty lines, and comments
    let startLine = 0;
    for (let i = 0; i < lines.length; i++) {
      const trimmed = lines[i].trim();
      if (trimmed && !trimmed.startsWith("#!") && !trimmed.startsWith("//") && !trimmed.startsWith("/*")) {
        startLine = i;
        break;
      }
    }

    // Extract up to maxLines, but stop at empty lines or comment blocks
    const snippetLines: string[] = [];
    for (let i = startLine; i < Math.min(lines.length, startLine + maxLines); i++) {
      const line = lines[i];
      snippetLines.push(line);

      // Stop at major breaks
      if (line.trim() === "" && snippetLines.length > 5) {
        break;
      }
    }

    // Trim leading/trailing empty lines
    while (snippetLines.length > 0 && snippetLines[0].trim() === "") {
      snippetLines.shift();
    }
    while (snippetLines.length > 0 && snippetLines[snippetLines.length - 1].trim() === "") {
      snippetLines.pop();
    }

    return snippetLines.join("\n") || undefined;
  } catch (e) {
    return undefined;
  }
}

/**
 * Get status icon for file status
 */
export function getStatusIcon(status: FileChangeStatus): string {
  switch (status) {
    case FileChangeStatus.Added:
      return "➕";
    case FileChangeStatus.Modified:
      return "✏️";
    case FileChangeStatus.Deleted:
      return "🗑️";
    case FileChangeStatus.Renamed:
      return "📝";
    default:
      return "📄";
  }
}
