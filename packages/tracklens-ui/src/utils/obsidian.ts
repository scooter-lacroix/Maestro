/**
 * TrackLens - Obsidian Integration Utility
 *
 * Manages settings for auto-saving reviews to Obsidian vaults.
 * Settings are stored in localStorage.
 *
 * REBRANDED: Plannotator → TrackLens
 * Updated: Default folder 'plannotator' → 'tracklens'
 * Updated: Tags include 'tracklens' instead of 'plannotator'
 *
 * @packageDocumentation
 */

import { storage } from './storage';

// Storage keys
const STORAGE_KEY_ENABLED = 'tracklens-obsidian-enabled';
const STORAGE_KEY_VAULT = 'tracklens-obsidian-vault';
const STORAGE_KEY_FOLDER = 'tracklens-obsidian-folder';
const STORAGE_KEY_CUSTOM_PATH = 'tracklens-obsidian-custom-path';
const STORAGE_KEY_FILENAME_FORMAT = 'tracklens-obsidian-filename-format';
const STORAGE_KEY_VAULT_BROWSER = 'tracklens-obsidian-vault-browser';

// Sentinel value for custom path selection
export const CUSTOM_PATH_SENTINEL = '__custom__';

// Default folder name in the vault
const DEFAULT_FOLDER = 'tracklens';

// Default filename format — matches the original hardcoded behavior
export const DEFAULT_FILENAME_FORMAT = '{title} - {Mon} {D}, {YYYY} {h}-{mm}{ampm}';

/**
 * Obsidian integration settings
 */
export interface ObsidianSettings {
  enabled: boolean;
  vaultPath: string;      // Selected vault path OR '__custom__' sentinel
  folder: string;
  customPath?: string;    // User-entered path when vaultPath === '__custom__'
  filenameFormat?: string; // Custom filename format (e.g. '{YYYY}-{MM}-{DD} - {title}')
  vaultBrowserEnabled: boolean; // Show vault file browser in sidebar
}

/**
 * Get current Obsidian settings from storage
 */
export function getObsidianSettings(): ObsidianSettings {
  return {
    enabled: storage.getItem(STORAGE_KEY_ENABLED) === 'true',
    vaultPath: storage.getItem(STORAGE_KEY_VAULT) || '',
    folder: storage.getItem(STORAGE_KEY_FOLDER) || DEFAULT_FOLDER,
    customPath: storage.getItem(STORAGE_KEY_CUSTOM_PATH) || undefined,
    filenameFormat: storage.getItem(STORAGE_KEY_FILENAME_FORMAT) || undefined,
    vaultBrowserEnabled: storage.getItem(STORAGE_KEY_VAULT_BROWSER) === 'true',
  };
}

/**
 * Save Obsidian settings to storage
 */
export function saveObsidianSettings(settings: ObsidianSettings): void {
  storage.setItem(STORAGE_KEY_ENABLED, String(settings.enabled));
  storage.setItem(STORAGE_KEY_VAULT, settings.vaultPath);
  storage.setItem(STORAGE_KEY_FOLDER, settings.folder);
  storage.setItem(STORAGE_KEY_CUSTOM_PATH, settings.customPath || '');
  storage.setItem(STORAGE_KEY_FILENAME_FORMAT, settings.filenameFormat || '');
  storage.setItem(STORAGE_KEY_VAULT_BROWSER, String(settings.vaultBrowserEnabled));
}

/**
 * Get the effective vault path, resolving custom path if selected
 */
export function getEffectiveVaultPath(settings: ObsidianSettings): string {
  if (settings.vaultPath === CUSTOM_PATH_SENTINEL) {
    return settings.customPath || '';
  }
  return settings.vaultPath;
}

/**
 * Check if Obsidian integration is properly configured
 */
export function isObsidianConfigured(): boolean {
  const settings = getObsidianSettings();
  const effectivePath = getEffectiveVaultPath(settings);
  return settings.enabled && effectivePath.trim().length > 0;
}

/**
 * Check if the vault browser sidebar tab should be shown
 */
export function isVaultBrowserEnabled(): boolean {
  const settings = getObsidianSettings();
  const effectivePath = getEffectiveVaultPath(settings);
  return settings.enabled && settings.vaultBrowserEnabled && effectivePath.trim().length > 0;
}

/**
 * Extract tags from markdown content using simple heuristics
 *
 * Extracts:
 * - Words from the first H1 title (excluding common words)
 * - Code fence languages (```typescript, ```sql, etc.)
 * - Always includes "tracklens" and "review" as base tags
 *
 * @param markdown - The markdown content to extract tags from
 * @returns Array of lowercase tag strings (max 6)
 */
export function extractTags(markdown: string): string[] {
  const tags = new Set<string>(['tracklens', 'review']);

  // Common words to exclude from title extraction
  const stopWords = new Set([
    'the', 'and', 'for', 'with', 'this', 'that', 'from', 'into',
    'review', 'implementation', 'overview', 'phase', 'step', 'steps',
  ]);

  // 1. Extract from first H1 title
  // Matches: "# Title" or "# Implementation Plan: Title" or "# Plan: Title"
  const h1Match = markdown.match(/^#\s+(?:Implementation\s+Plan:|Plan:)?\s*(.+)$/im);
  if (h1Match) {
    const titleWords = h1Match[1]
      .toLowerCase()
      .replace(/[^\w\s-]/g, ' ')  // Remove special chars except hyphens
      .split(/\s+/)
      .filter(word => word.length > 2 && !stopWords.has(word));

    // Add first 3 meaningful words from title
    titleWords.slice(0, 3).forEach(word => tags.add(word));
  }

  // 2. Extract code fence languages
  // Matches: ```typescript, ```sql, ```rust, etc.
  const langMatches = markdown.matchAll(/```(\w+)/g);
  const seenLangs = new Set<string>();

  for (const [, lang] of langMatches) {
    const normalizedLang = lang.toLowerCase();
    // Skip generic/config languages and duplicates
    if (!seenLangs.has(normalizedLang) &&
        !['json', 'yaml', 'yml', 'text', 'txt', 'markdown', 'md'].includes(normalizedLang)) {
      seenLangs.add(normalizedLang);
      tags.add(normalizedLang);
    }
  }

  // Return max 6 tags
  return Array.from(tags).slice(0, 6);
}

/**
 * Generate YAML frontmatter for an Obsidian note
 *
 * @param tags - Array of tags to include
 * @returns Frontmatter string including opening and closing ---
 */
export function generateFrontmatter(tags: string[]): string {
  const now = new Date().toISOString();
  const tagList = tags.map(t => t.toLowerCase()).join(', ');

  return `---
created: ${now}
source: tracklens
tags: [${tagList}]
---`;
}

/**
 * Generate a filename for the review note
 * Format: YYYY-MM-DD-HHmm.md (e.g., 2026-01-02-1430.md)
 *
 * @returns Filename string
 */
export function generateFilename(): string {
  const now = new Date();
  const timestamp = now.toISOString()
    .slice(0, 16)           // "2026-01-02T14:30"
    .replace('T', '-')      // "2026-01-02-14:30"
    .replace(/:/g, '');     // "2026-01-02-1430"

  return `${timestamp}.md`;
}

/**
 * Prepare the full note content with frontmatter
 *
 * @param markdown - The review markdown content
 * @returns Full note content with frontmatter prepended
 */
export function prepareNoteContent(markdown: string): string {
  const tags = extractTags(markdown);
  const frontmatter = generateFrontmatter(tags);

  return `${frontmatter}\n\n${markdown}`;
}
