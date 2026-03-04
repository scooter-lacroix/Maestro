/**
 * TrackLens Project Detection
 *
 * Detects project name from git repo or current directory.
 * Used for tagging and organizing TrackLens documents.
 */

import { $ } from "bun";

/**
 * Sanitize a name for use as a tag
 * Converts to lowercase, replaces spaces/special chars with hyphens
 */
export function sanitizeTag(name: string): string | null {
  if (!name || typeof name !== "string") return null;

  const sanitized = name
    .toLowerCase()
    .trim()
    .replace(/[\s_]+/g, "-") // spaces/underscores -> hyphens
    .replace(/[^a-z0-9-]/g, "") // remove special chars
    .replace(/-+/g, "-") // collapse multiple hyphens
    .replace(/^-|-$/g, "") // trim leading/trailing hyphens
    .slice(0, 30); // max 30 chars

  return sanitized.length >= 2 ? sanitized : null;
}

/**
 * Extract repo name from git root path
 */
export function extractRepoName(gitRootPath: string): string | null {
  if (!gitRootPath || typeof gitRootPath !== "string") return null;

  const trimmed = gitRootPath.trim().replace(/\/+$/, ""); // remove trailing slashes
  const parts = trimmed.split("/");
  const name = parts[parts.length - 1];

  return sanitizeTag(name);
}

/**
 * Extract directory name from path
 * Skips generic names like home, users, tmp, etc.
 */
export function extractDirName(path: string): string | null {
  if (!path || typeof path !== "string") return null;

  const trimmed = path.trim().replace(/\/+$/, "");
  if (trimmed === "" || trimmed === "/") return null;

  const parts = trimmed.split("/");
  const name = parts[parts.length - 1];

  // Skip generic names
  const skipNames = new Set(["home", "users", "user", "root", "tmp", "var"]);
  if (skipNames.has(name.toLowerCase())) return null;

  return sanitizeTag(name);
}

/**
 * Detect project name from git repo or current directory
 */
export async function detectProjectName(): Promise<string | null> {
  // Try git repo name first
  try {
    const result = await $`git rev-parse --show-toplevel`.quiet().nothrow();
    if (result.exitCode === 0) {
      const repoName = extractRepoName(result.stdout.toString());
      if (repoName) return repoName;
    }
  } catch {
    // Git not available or not in a repo - continue to fallback
  }

  // Fallback to current directory name
  try {
    const cwd = process.cwd();
    const dirName = extractDirName(cwd);
    if (dirName) return dirName;
  } catch {
    // CWD not accessible
  }

  return null;
}
