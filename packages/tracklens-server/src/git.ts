/**
 * TrackLens Git Integration
 *
 * Provides git diff functionality for code review mode.
 */

import { $ } from "bun";

export type DiffType =
  | "uncommitted"
  | "staged"
  | "unstaged"
  | "last-commit"
  | "branch";

export interface DiffOption {
  id: DiffType | "separator";
  label: string;
}

export interface GitContext {
  currentBranch: string;
  defaultBranch: string;
  diffOptions: DiffOption[];
}

export interface DiffResult {
  patch: string;
  label: string;
  error?: string;
}

/**
 * Get current git branch
 */
async function getCurrentBranch(): Promise<string> {
  try {
    const result = await $`git rev-parse --abbrev-ref HEAD`.quiet();
    return result.text().trim();
  } catch {
    return "HEAD"; // Detached HEAD state
  }
}

/**
 * Get default branch (main or master)
 */
async function getDefaultBranch(): Promise<string> {
  // Try origin's HEAD first (most reliable for repos with remotes)
  try {
    const result =
      await $`git symbolic-ref refs/remotes/origin/HEAD`.quiet();
    const ref = result.text().trim();
    return ref.replace("refs/remotes/origin/", "");
  } catch {
    // No remote or no HEAD set - check local branches
  }

  // Fallback: check if main exists locally
  try {
    await $`git show-ref --verify refs/heads/main`.quiet();
    return "main";
  } catch {
    // main doesn't exist
  }

  // Fallback to master
  try {
    await $`git show-ref --verify refs/heads/master`.quiet();
    return "master";
  } catch {
    // Neither exists, return main as default
    return "main";
  }
}

/**
 * Get git context (branches, available diff options)
 */
export async function getGitContext(): Promise<GitContext> {
  const [currentBranch, defaultBranch] = await Promise.all([
    getCurrentBranch(),
    getDefaultBranch(),
  ]);

  const diffOptions: DiffOption[] = [
    { id: "uncommitted", label: "Uncommitted changes" },
    { id: "last-commit", label: "Last commit" },
  ];

  // Only show branch diff if not on default branch
  if (currentBranch !== defaultBranch) {
    diffOptions.push({ id: "branch", label: `vs ${defaultBranch}` });
  }

  return { currentBranch, defaultBranch, diffOptions };
}

/**
 * Run git diff for the specified type
 */
export async function runGitDiff(
  diffType: DiffType,
  defaultBranch: string = "main"
): Promise<DiffResult> {
  let patch: string;
  let label: string;

  try {
    switch (diffType) {
      case "uncommitted":
        patch = (await $`git diff HEAD --src-prefix=a/ --dst-prefix=b/`.quiet()).text();
        label = "Uncommitted changes";
        break;

      case "staged":
        patch = (await $`git diff --staged --src-prefix=a/ --dst-prefix=b/`.quiet()).text();
        label = "Staged changes";
        break;

      case "unstaged":
        patch = (await $`git diff --src-prefix=a/ --dst-prefix=b/`.quiet()).text();
        label = "Unstaged changes";
        break;

      case "last-commit":
        patch = (await $`git diff HEAD^ HEAD --src-prefix=a/ --dst-prefix=b/`.quiet()).text();
        label = "Last commit";
        break;

      case "branch":
        patch = (await $`git diff ${defaultBranch}...HEAD --src-prefix=a/ --dst-prefix=b/`.quiet()).text();
        label = `vs ${defaultBranch}`;
        break;

      default:
        return {
          patch: "",
          label: "Unknown diff type",
          error: `Unknown diff type: ${diffType}`,
        };
    }

    return { patch, label };
  } catch (error) {
    return {
      patch: "",
      label: "Diff failed",
      error: error instanceof Error ? error.message : String(error),
    };
  }
}
