/**
 * TrackLens Browser Opening
 *
 * Opens the browser to the TrackLens server URL.
 * REBRANDED: PLANNOTATOR_BROWSER → MAESTRO_BROWSER
 */

import os from "node:os";
import { $ } from "bun";

/**
 * Check if running under WSL
 */
async function isWSL(): Promise<boolean> {
  if (process.platform !== "linux") {
    return false;
  }

  if (os.release().toLowerCase().includes("microsoft")) {
    return true;
  }

  // Fallback: check /proc/version for WSL signature (if available)
  try {
    const file = Bun.file("/proc/version");
    if (await file.exists()) {
      const content = await file.text();
      return (
        content.toLowerCase().includes("wsl") ||
        content.toLowerCase().includes("microsoft")
      );
    }
  } catch {
    // File not readable
  }

  return false;
}

/**
 * Open browser to the specified URL
 * Returns true if successful, false otherwise
 */
export async function openBrowser(url: string): Promise<boolean> {
  try {
    const browser = process.env.MAESTRO_BROWSER || process.env.BROWSER;
    const platform = process.platform;
    const wsl = await isWSL();

    if (browser) {
      const maestroBrowser = process.env.MAESTRO_BROWSER;
      if (maestroBrowser && platform === "darwin") {
        await $`open -a ${maestroBrowser} ${url}`.quiet();
      } else if ((platform === "win32" || wsl) && maestroBrowser) {
        // Windows or WSL: use 'start' with custom browser
        if (wsl) {
          await $`cmd.exe /c start ${maestroBrowser} ${url}`.quiet();
        } else {
          await $`start ${maestroBrowser} ${url}`.quiet();
        }
      } else if (platform === "linux" && maestroBrowser) {
        await $`${maestroBrowser} ${url}`.quiet();
      } else {
        // Fallback for unsupported platform with custom browser
        console.error(
          `[TrackLens] Custom browser not supported on this platform`
        );
        return false;
      }
    } else {
      // Default browser for each platform
      if (platform === "darwin") {
        await $`open ${url}`.quiet();
      } else if (platform === "win32" || wsl) {
        // Windows or WSL: use 'start' command
        if (wsl) {
          await $`cmd.exe /c start ${url}`.quiet();
        } else {
          await $`start ${url}`.quiet();
        }
      } else if (platform === "linux") {
        // Try common Linux browsers
        const linuxBrowsers = ["xdg-open", "sensible-browser", "firefox", "chromium"];
        let opened = false;
        for (const b of linuxBrowsers) {
          try {
            await $`${b} ${url}`.quiet();
            opened = true;
            break;
          } catch {
            continue;
          }
        }
        if (!opened) {
          console.error(
            `[TrackLens] No suitable browser found on Linux`
          );
          return false;
        }
      } else {
        console.error(
          `[TrackLens] Browser opening not supported on platform: ${platform}`
        );
        return false;
      }
    }

    return true;
  } catch (error) {
    console.error(`[TrackLens] Failed to open browser: ${error}`);
    return false;
  }
}
