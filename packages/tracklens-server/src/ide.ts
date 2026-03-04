/**
 * TrackLens IDE Integration
 *
 * Opens external editors for diff viewing.
 */

/**
 * Open VS Code diff view comparing two files
 */
export async function openEditorDiff(
  oldPath: string,
  newPath: string
): Promise<{ ok: true } | { error: string }> {
  try {
    const proc = Bun.spawn(["code", "--diff", oldPath, newPath], {
      stdout: "ignore",
      stderr: "pipe",
    });
    const exitCode = await proc.exited;

    if (exitCode !== 0) {
      const stderr = await new Response(proc.stderr).text();
      if (stderr.includes("not found") || stderr.includes("ENOENT")) {
        return {
          error:
            "VS Code CLI not found. Install it with 'code --install-extension' from VS Code.",
        };
      }
      return { error: `Failed to open editor: ${stderr}` };
    }

    return { ok: true };
  } catch (error) {
    return {
      error:
        error instanceof Error
          ? error.message
          : "Unknown error opening editor",
    };
  }
}
