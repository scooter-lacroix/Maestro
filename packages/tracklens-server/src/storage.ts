/**
 * TrackLens Storage
 *
 * Manages file storage for TrackLens documents.
 * REBRANDED: ~/.plannotator/ → ~/.maestro/tracklens/
 */

import {
  mkdirSync,
  writeFileSync,
  readFileSync,
  readdirSync,
  existsSync,
  statSync,
} from "fs";
import { join } from "path";
import { homedir } from "os";
import { sanitizeTag } from "./project";

/**
 * Get the plan directory (custom or default)
 */
function getPlanDir(customPath?: string | null): string {
  let planDir: string;

  if (customPath) {
    // Expand ~ to home directory
    planDir = customPath.startsWith("~")
      ? join(homedir(), customPath.slice(1))
      : customPath;
  } else {
    planDir = join(homedir(), ".maestro", "tracklens", "docs");
  }

  mkdirSync(planDir, { recursive: true });
  return planDir;
}

/**
 * Extract first heading from markdown
 */
function extractFirstHeading(markdown: string): string | null {
  const match = markdown.match(/^#\s+(.+)$/m);
  if (!match) return null;
  return match[1].trim();
}

/**
 * Generate slug for plan file
 */
export function generateSlug(plan: string): string {
  const date = new Date().toISOString().split("T")[0]; // YYYY-MM-DD

  const heading = extractFirstHeading(plan);
  const slug = heading ? sanitizeTag(heading) : null;

  return slug ? `${slug}-${date}` : `plan-${date}`;
}

/**
 * Save plan content to file
 */
function savePlan(
  slug: string,
  content: string,
  customPath?: string | null
): string {
  const planDir = getPlanDir(customPath);
  const filePath = join(planDir, `${slug}.md`);
  writeFileSync(filePath, content, "utf-8");
  return filePath;
}

/**
 * Save annotations to file
 */
function saveAnnotations(
  slug: string,
  annotationsContent: string,
  customPath?: string | null
): string {
  const planDir = getPlanDir(customPath);
  const filePath = join(planDir, `${slug}.annotations.md`);
  writeFileSync(filePath, annotationsContent, "utf-8");
  return filePath;
}

/**
 * Save final snapshot (approved/denied plan with annotations)
 */
function saveFinalSnapshot(
  slug: string,
  status: "approved" | "denied",
  plan: string,
  annotations: string,
  customPath?: string | null
): string {
  const planDir = getPlanDir(customPath);
  const filePath = join(planDir, `${slug}-${status}.md`);

  // Combine plan with annotations appended
  let content = plan;
  if (annotations && annotations !== "No changes detected.") {
    content += "\n\n---\n\n" + annotations;
  }

  writeFileSync(filePath, content, "utf-8");
  return filePath;
}

/**
 * Get history directory for version tracking
 */
function getHistoryDir(project: string, slug: string): string {
  const historyDir = join(
    homedir(),
    ".maestro",
    "tracklens",
    "history",
    project,
    slug
  );
  mkdirSync(historyDir, { recursive: true });
  return historyDir;
}

/**
 * Get next version number for history
 */
function getNextVersionNumber(historyDir: string): number {
  try {
    const entries = readdirSync(historyDir);
    let max = 0;
    for (const entry of entries) {
      const match = entry.match(/^(\d+)\.md$/);
      if (match) {
        const num = parseInt(match[1], 10);
        if (num > max) max = num;
      }
    }
    return max + 1;
  } catch {
    return 1;
  }
}

/**
 * Save plan to history with deduplication
 */
function saveToHistory(
  project: string,
  slug: string,
  plan: string
): { version: number; path: string; isNew: boolean } {
  const historyDir = getHistoryDir(project, slug);
  const nextVersion = getNextVersionNumber(historyDir);

  // Deduplicate: check if latest version has identical content
  if (nextVersion > 1) {
    const latestPath = join(
      historyDir,
      `${String(nextVersion - 1).padStart(3, "0")}.md`
    );
    try {
      const existing = readFileSync(latestPath, "utf-8");
      if (existing === plan) {
        return { version: nextVersion - 1, path: latestPath, isNew: false };
      }
    } catch {
      // Latest version not readable, continue
    }
  }

  // Save new version
  const fileName = `${String(nextVersion).padStart(3, "0")}.md`;
  const filePath = join(historyDir, fileName);
  writeFileSync(filePath, plan, "utf-8");

  return { version: nextVersion, path: filePath, isNew: true };
}

/**
 * Get specific plan version from history
 */
function getPlanVersion(
  project: string,
  slug: string,
  version: number
): string | null {
  const historyDir = join(
    homedir(),
    ".maestro",
    "tracklens",
    "history",
    project,
    slug
  );
  const fileName = `${String(version).padStart(3, "0")}.md`;
  const filePath = join(historyDir, fileName);

  try {
    return readFileSync(filePath, "utf-8");
  } catch {
    return null;
  }
}

/**
 * Get file path for specific version
 */
function getPlanVersionPath(
  project: string,
  slug: string,
  version: number
): string | null {
  const historyDir = join(
    homedir(),
    ".maestro",
    "tracklens",
    "history",
    project,
    slug
  );
  const fileName = `${String(version).padStart(3, "0")}.md`;
  const filePath = join(historyDir, fileName);
  return existsSync(filePath) ? filePath : null;
}

/**
 * Get version count for a plan
 */
function getVersionCount(project: string, slug: string): number {
  const historyDir = join(
    homedir(),
    ".maestro",
    "tracklens",
    "history",
    project,
    slug
  );
  try {
    const entries = readdirSync(historyDir);
    return entries.filter((e) => /^\d+\.md$/.test(e)).length;
  } catch {
    return 0;
  }
}

/**
 * List all versions of a plan with timestamps
 */
function listVersions(
  project: string,
  slug: string
): Array<{ version: number; timestamp: string }> {
  const historyDir = join(
    homedir(),
    ".maestro",
    "tracklens",
    "history",
    project,
    slug
  );
  try {
    const entries = readdirSync(historyDir);
    const versions: Array<{ version: number; timestamp: string }> = [];
    for (const entry of entries) {
      const match = entry.match(/^(\d+)\.md$/);
      if (match) {
        const version = parseInt(match[1], 10);
        const filePath = join(historyDir, entry);
        const stats = statSync(filePath);
        versions.push({
          version,
          timestamp: stats.mtime.toISOString(),
        });
      }
    }
    return versions.sort((a, b) => a.version - b.version);
  } catch {
    return [];
  }
}

/**
 * List all plans for a project with metadata
 */
function listProjectPlans(
  project: string
): Array<{ slug: string; versions: number; lastModified: string }> {
  const projectDir = join(
    homedir(),
    ".maestro",
    "tracklens",
    "history",
    project
  );
  try {
    const entries = readdirSync(projectDir, { withFileTypes: true });
    const plans: Array<{
      slug: string;
      versions: number;
      lastModified: string;
    }> = [];
    for (const entry of entries) {
      if (!entry.isDirectory()) continue;
      const slugDir = join(projectDir, entry.name);
      const files = readdirSync(slugDir);
      const versions = files.filter((f) => /^\d+\.md$/.test(f)).length;

      // Get last modified time from most recent file
      let lastModified = "";
      for (const file of files) {
        const filePath = join(slugDir, file);
        const stats = statSync(filePath);
        const time = stats.mtime.toISOString();
        if (!lastModified || time > lastModified) {
          lastModified = time;
        }
      }

      if (versions > 0) {
        plans.push({ slug: entry.name, versions, lastModified });
      }
    }
    return plans.sort(
      (a, b) => new Date(b.lastModified).getTime() - new Date(a.lastModified).getTime()
    );
  } catch {
    return [];
  }
}

export {
  getPlanDir,
  savePlan,
  saveAnnotations,
  saveFinalSnapshot,
  saveToHistory,
  getPlanVersion,
  getPlanVersionPath,
  getVersionCount,
  listVersions,
  listProjectPlans,
};
