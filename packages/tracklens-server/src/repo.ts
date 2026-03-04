/**
 * TrackLens Repository Detection
 *
 * Detects git repository information for display in TrackLens UI.
 */

import { $ } from "bun";

export interface RepoInfo {
  /** Display string (e.g., "org/repo" or "my-project") */
  display: string;
  /** Current git branch (if in a git repo) */
  branch?: string;
}

/**
 * Parse remote URL to extract org/repo
 */
function parseRemoteUrl(url: string): string | null {
  if (!url) return null;

  // SSH format: git@github.com:org/repo.git
  const sshMatch = url.match(/:([^/]+\/[^/]+?)(?:\.git)?$/);
  if (sshMatch) return sshMatch[1];

  // HTTPS format: https://github.com/org/repo.git
  const httpsMatch = url.match(/\/([^/]+\/[^/]+?)(?:\.git)?$/);
  if (httpsMatch) return httpsMatch[1];

  return null;
}

/**
 * Get directory name from path
 */
function getDirName(path: string): string | null {
  if (!path) return null;
  const trimmed = path.trim().replace(/\/+$/, "");
  const parts = trimmed.split("/");
  return parts[parts.length - 1] || null;
}

/**
 * Get current git branch
 */
async function getCurrentBranch(): Promise<string | undefined> {
  try {
    const result = await $`git rev-parse --abbrev-ref HEAD`.quiet().nothrow();
    if (result.exitCode === 0) {
      const branch = result.stdout.toString().trim();
      return branch && branch !== "HEAD" ? branch : undefined;
    }
  } catch {
    // Not in a git repo
  }
  return undefined;
}

/**
 * Get repository information from git or current directory
 */
export async function getRepoInfo(): Promise<RepoInfo | null> {
  let branch: string | undefined;

  // Try git remote URL first
  try {
    const result = await $`git remote get-url origin`.quiet().nothrow();
    if (result.exitCode === 0) {
      const orgRepo = parseRemoteUrl(result.stdout.toString().trim());
      if (orgRepo) {
        branch = await getCurrentBranch();
        return { display: orgRepo, branch };
      }
    }
  } catch {
    // Git not available
  }

  // Fallback: git repo root directory name
  try {
    const result = await $`git rev-parse --show-toplevel`.quiet().nothrow();
    if (result.exitCode === 0) {
      const dirName = getDirName(result.stdout.toString());
      if (dirName) {
        branch = await getCurrentBranch();
        return { display: dirName, branch };
      }
    }
  } catch {
    // Git not available
  }

  // Final fallback: current directory name
  try {
    const cwd = process.cwd();
    const dirName = getDirName(cwd);
    if (dirName) {
      return { display: dirName };
    }
  } catch {
    // CWD not accessible
  }

  return null;
}
