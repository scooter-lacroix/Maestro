/**
 * TrackLens Image Handling
 *
 * Validates image paths and uploads for annotation attachments.
 */

import { resolve, basename, normalize } from "path";

const ALLOWED_IMAGE_EXTENSIONS = new Set([
  "png",
  "jpg",
  "jpeg",
  "gif",
  "webp",
  "svg",
  "ico",
]);

export const UPLOAD_DIR = "/tmp/tracklens-uploads";

/**
 * Get file extension from path
 */
function getExtension(filePath: string): string {
  const lastDot = filePath.lastIndexOf(".");
  if (lastDot === -1) return "";
  return filePath.slice(lastDot + 1).toLowerCase();
}

/**
 * Check if file has an image extension
 */
function hasImageExtension(filePath: string): boolean {
  return ALLOWED_IMAGE_EXTENSIONS.has(getExtension(filePath));
}

/**
 * Sanitize filename to prevent path traversal attacks.
 * Extracts only the basename, removing any directory components.
 * Also validates the extension is allowed.
 */
export function sanitizeFileName(fileName: string): {
  safe: boolean;
  sanitized: string;
  error?: string;
} {
  // Remove any path components - only keep the filename
  const sanitized = basename(fileName);

  // Reject empty filenames
  if (!sanitized || sanitized === "." || sanitized === "..") {
    return {
      safe: false,
      sanitized: "",
      error: "Invalid filename",
    };
  }

  // Reject filenames starting with dot (hidden files)
  if (sanitized.startsWith(".")) {
    return {
      safe: false,
      sanitized: "",
      error: "Hidden files not allowed",
    };
  }

  // Validate extension
  if (!hasImageExtension(sanitized)) {
    return {
      safe: false,
      sanitized,
      error: "Invalid image extension",
    };
  }

  return { safe: true, sanitized };
}

/**
 * Get safe upload path - validates the path is within UPLOAD_DIR
 */
export function getSafeUploadPath(fileName: string): string {
  const { safe, sanitized } = sanitizeFileName(fileName);
  if (!safe) {
    throw new Error(`Invalid filename: ${fileName}`);
  }
  return `${UPLOAD_DIR}/${sanitized}`;
}

/**
 * Validate an image path for use in annotations
 */
export function validateImagePath(rawPath: string): {
  valid: boolean;
  resolved: string;
  error?: string;
} {
  const resolved = resolve(rawPath);

  if (!hasImageExtension(resolved)) {
    return {
      valid: false,
      resolved,
      error: "Path does not point to a supported image file",
    };
  }

  return { valid: true, resolved };
}

/**
 * Validate uploaded file extension
 */
export function validateUploadExtension(fileName: string): {
  valid: boolean;
  ext: string;
  error?: string;
} {
  const ext = getExtension(fileName) || "png";

  if (!ALLOWED_IMAGE_EXTENSIONS.has(ext)) {
    return {
      valid: false,
      ext,
      error: `File extension ".${ext}" is not a supported image type`,
    };
  }

  return { valid: true, ext };
}
