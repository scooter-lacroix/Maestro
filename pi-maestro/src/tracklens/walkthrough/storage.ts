/**
 * TrackLens Walkthrough Storage
 *
 * Handles persistence and compression of walkthrough documents.
 *
 * @packageDocumentation
 */

import { writeFileSync, readFileSync, existsSync, mkdirSync } from "fs";
import { join } from "path";
import { compress, decompress } from "@maestro/tracklens-shared";

import type {
  StoredWalkthrough,
  GeneratedWalkthrough,
} from "./types.js";

/**
 * Storage directory for walkthroughs
 */
export const WALKTHROUGH_STORAGE_DIR = ".maestro/tracklens/walkthroughs";

/**
 * Save walkthrough with compression
 *
 * Compresses the walkthrough data and saves to storage directory.
 *
 * @param trackId - Track ID
 * @param walkthrough - Generated walkthrough to save
 * @returns Path to saved walkthrough file
 */
export async function saveWalkthrough(
  trackId: string,
  walkthrough: GeneratedWalkthrough
): Promise<string> {
  const storageDir = join(process.cwd(), WALKTHROUGH_STORAGE_DIR);

  // Ensure storage directory exists
  if (!existsSync(storageDir)) {
    mkdirSync(storageDir, { recursive: true });
  }

  // Prepare storage data
  const storedData: StoredWalkthrough = {
    metadata: walkthrough.metadata,
    compressed: await compress({
      markdown: walkthrough.markdown,
      completedTasks: walkthrough.completedTasks,
      changedFiles: walkthrough.changedFiles,
    }),
    version: 1,
  };

  // Save compressed walkthrough
  const walkthroughPath = join(storageDir, `${trackId}.json`);
  writeFileSync(walkthroughPath, JSON.stringify(storedData, null, 2), "utf-8");

  return walkthroughPath;
}

/**
 * Load walkthrough with decompression
 *
 * @param trackId - Track ID
 * @returns Loaded walkthrough or null if not found
 */
export async function loadWalkthrough(
  trackId: string
): Promise<GeneratedWalkthrough | null> {
  const walkthroughPath = join(process.cwd(), WALKTHROUGH_STORAGE_DIR, `${trackId}.json`);

  if (!existsSync(walkthroughPath)) {
    return null;
  }

  try {
    const storedData: StoredWalkthrough = JSON.parse(readFileSync(walkthroughPath, "utf-8"));

    // Decompress walkthrough data
    const decompressed = (await decompress(storedData.compressed)) as {
      markdown: string;
      completedTasks: any[];
      changedFiles: any[];
    };

    return {
      markdown: decompressed.markdown,
      metadata: storedData.metadata,
      completedTasks: decompressed.completedTasks,
      changedFiles: decompressed.changedFiles,
    };
  } catch (e) {
    // Failed to load or decompress
    return null;
  }
}

/**
 * Save final walkthrough markdown to track directory
 *
 * @param trackDir - Track directory path
 * @param markdown - Walkthrough markdown content
 * @returns Path to saved walkthrough file
 */
export function saveFinalWalkthrough(trackDir: string, markdown: string): string {
  const walkthroughPath = join(trackDir, "walkthrough-final.md");
  writeFileSync(walkthroughPath, markdown, "utf-8");
  return walkthroughPath;
}

/**
 * Check if walkthrough exists for a track
 *
 * @param trackId - Track ID
 * @returns True if walkthrough exists
 */
export function walkthroughExists(trackId: string): boolean {
  const walkthroughPath = join(process.cwd(), WALKTHROUGH_STORAGE_DIR, `${trackId}.json`);
  return existsSync(walkthroughPath);
}

/**
 * Delete walkthrough storage
 *
 * @param trackId - Track ID
 * @returns True if walkthrough was deleted
 */
export function deleteWalkthrough(trackId: string): boolean {
  const walkthroughPath = join(process.cwd(), WALKTHROUGH_STORAGE_DIR, `${trackId}.json`);

  if (existsSync(walkthroughPath)) {
    const fs = require("fs");
    fs.unlinkSync(walkthroughPath);
    return true;
  }

  return false;
}

/**
 * List all walkthroughs in storage
 *
 * @returns Array of track IDs with walkthroughs
 */
export function listWalkthroughs(): string[] {
  const storageDir = join(process.cwd(), WALKTHROUGH_STORAGE_DIR);

  if (!existsSync(storageDir)) {
    return [];
  }

  const fs = require("fs");
  const files = fs.readdirSync(storageDir);
  return files
    .filter((f: string) => f.endsWith(".json"))
    .map((f: string) => f.replace(".json", ""));
}
