/**
 * TrackLens Integrations
 *
 * Save TrackLens documents to external apps (Obsidian, Bear).
 * REBRANDED: Tag "plannotator" → "tracklens", source "plannotator" → "tracklens"
 */

import { mkdirSync, readFileSync, writeFileSync, existsSync } from "fs";
import { join } from "path";
import { sanitizeTag } from "./project";

export interface ObsidianConfig {
  vaultPath: string;
  folder: string;
  plan: string;
  filenameFormat?: string; // Custom format string, e.g. '{YYYY}-{MM}-{DD} - {title}'
}

export interface BearConfig {
  plan: string;
}

export interface IntegrationResult {
  success: boolean;
  error?: string;
  path?: string;
}

/**
 * Extract tags from plan markdown
 * REBRANDED: Default tag "plannotator" → "tracklens"
 */
async function extractTags(markdown: string): Promise<string[]> {
  const tags = new Set<string>(["tracklens"]);

  // Add project name tag (git repo name or directory fallback)
  const { detectProjectName } = await import("./project");
  const projectName = await detectProjectName();
  if (projectName) {
    tags.add(projectName);
  }

  const stopWords = new Set([
    "the",
    "and",
    "for",
    "with",
    "this",
    "that",
    "from",
    "into",
    "plan",
    "implementation",
    "overview",
    "phase",
    "step",
    "steps",
  ]);

  // Extract from first H1 title
  const h1Match = markdown.match(/^#\s+(?:Implementation\s+Plan:|Plan:)?\s*(.+)$/im);
  if (h1Match) {
    const titleWords = h1Match[1]
      .toLowerCase()
      .split(/[\s-]+/)
      .filter((w) => w && !stopWords.has(w) && w.length > 2);

    for (const word of titleWords.slice(0, 3)) {
      const sanitized = sanitizeTag(word);
      if (sanitized) tags.add(sanitized);
    }
  }

  return Array.from(tags);
}

/**
 * Generate frontmatter with tags
 * REBRANDED: source "plannotator" → "tracklens"
 */
function generateFrontmatter(tags: string[]): string {
  const now = new Date().toISOString();
  const tagList = tags.map((t) => t.toLowerCase()).join(", ");
  return `---
created: ${now}
source: tracklens
tags: [${tagList}]
---`;
}

/**
 * Extract title from plan markdown
 */
function extractTitle(markdown: string): string {
  const h1Match = markdown.match(/^#\s+(?:Implementation\s+Plan:|Plan:)?\s*(.+)$/im);
  if (h1Match) {
    // Clean up the title for use as filename
    return h1Match[1]
      .trim()
      .replace(/[<>:"/\\|?*]/g, "") // Remove invalid filename chars
      .replace(/\s+/g, " ") // Normalize whitespace
      .slice(0, 50); // Limit length
  }
  return "Plan";
}

/**
 * Generate filename with custom format support
 */
function generateFilename(markdown: string, format?: string): string {
  const title = extractTitle(markdown);
  const now = new Date();

  const months = [
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
  ];

  const hour24 = now.getHours();
  const hour12 = hour24 % 12 || 12;
  const ampm = hour24 >= 12 ? "pm" : "am";

  const vars: Record<string, string> = {
    title,
    YYYY: String(now.getFullYear()),
    MM: String(now.getMonth() + 1).padStart(2, "0"),
    DD: String(now.getDate()).padStart(2, "0"),
    Mon: months[now.getMonth()],
    HH: String(hour24).padStart(2, "0"),
    hh: String(hour12).padStart(2, "0"),
    mm: String(now.getMinutes()).padStart(2, "0"),
    ss: String(now.getSeconds()).padStart(2, "0"),
    ampm,
  };

  const formatString = format || "{YYYY}-{MM}-{DD} - {title}";
  let filename = formatString;

  for (const [key, value] of Object.entries(vars)) {
    filename = filename.replace(`{${key}}`, value);
  }

  // Sanitize filename
  return filename
    .replace(/[<>:"/\\|?*]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

/**
 * Detect Obsidian vaults from config
 */
export function detectObsidianVaults(): string[] {
  try {
    const home = process.env.HOME || process.env.USERPROFILE || "";
    let configPath: string;

    // Platform-specific config locations
    if (process.platform === "darwin") {
      configPath = join(home, "Library/Application Support/obsidian/obsidian.json");
    } else if (process.platform === "win32") {
      const appData = process.env.APPDATA || join(home, "AppData/Roaming");
      configPath = join(appData, "obsidian/obsidian.json");
    } else {
      configPath = join(home, ".config/obsidian/obsidian.json");
    }

    if (!existsSync(configPath)) {
      return [];
    }

    const config = JSON.parse(readFileSync(configPath, "utf-8"));
    const vaults: string[] = [];

    for (const key of Object.keys(config)) {
      const vault = config[key];
      if (vault && vault.path) {
        vaults.push(vault.path);
      }
    }

    return vaults;
  } catch {
    return [];
  }
}

/**
 * Save plan to Obsidian vault
 */
export async function saveToObsidian(
  config: ObsidianConfig
): Promise<IntegrationResult> {
  try {
    const { vaultPath, folder, plan } = config;

    // Normalize path (handle ~ on Unix, forward/back slashes)
    let normalizedVault = vaultPath.trim();

    // Expand ~ to home directory (Unix/macOS)
    if (normalizedVault.startsWith("~")) {
      const home = process.env.HOME || process.env.USERPROFILE || "";
      normalizedVault = join(home, normalizedVault.slice(1));
    }

    // Validate vault exists
    if (!existsSync(normalizedVault)) {
      return {
        success: false,
        error: `Vault not found: ${normalizedVault}`,
      };
    }

    // Create folder if needed
    const folderPath = join(normalizedVault, folder);
    mkdirSync(folderPath, { recursive: true });

    // Generate content with frontmatter
    const filename = generateFilename(plan, config.filenameFormat);
    const tags = await extractTags(plan);
    const frontmatter = generateFrontmatter(tags);

    const content = `${frontmatter}\n\n${plan}`;

    // Write file
    const filePath = join(folderPath, `${filename}.md`);
    writeFileSync(filePath, content, "utf-8");

    return { success: true, path: filePath };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

/**
 * Save plan to Bear app
 */
export async function saveToBear(
  config: BearConfig
): Promise<IntegrationResult> {
  try {
    const { plan } = config;

    // Extract title and tags
    const title = extractTitle(plan);
    const tags = await extractTags(plan);
    const hashtags = tags.map((t) => `#${t}`).join(" ");

    // Append hashtags to content
    const content = `${plan}\n\n${hashtags}`;

    // Build Bear URL
    const url = `bear://x-callback-url/create?title=${encodeURIComponent(
      title
    )}&text=${encodeURIComponent(content)}&mode=append`;

    // Open Bear URL
    // Note: This requires Bun.spawn or similar to open the URL
    // For now, return the URL for the caller to open
    console.log(`[TrackLens] Open this URL in Bear: ${url}`);

    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

export { extractTags, generateFrontmatter, extractTitle, generateFilename };
