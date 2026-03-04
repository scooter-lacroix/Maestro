/**
 * TrackLens Remote Detection
 *
 * Detects remote sessions and provides server port configuration.
 * REBRANDED: PLANNOTATOR_PORT → TRACKLENS_PORT, PLANNOTATOR_REMOTE → TRACKLENS_REMOTE
 */

const DEFAULT_REMOTE_PORT = 3750;

/**
 * Check if running in a remote session (SSH, etc.)
 */
export function isRemoteSession(): boolean {
  // New preferred env var
  const remote = process.env.TRACKLENS_REMOTE;
  if (remote === "1" || remote?.toLowerCase() === "true") {
    return true;
  }

  // Legacy: SSH_TTY/SSH_CONNECTION (deprecated, silent)
  if (process.env.SSH_TTY || process.env.SSH_CONNECTION) {
    return true;
  }

  return false;
}

/**
 * Get the server port from environment or default
 * Remote sessions use fixed port for port forwarding; local uses random
 */
export function getServerPort(): number {
  // Explicit port from environment takes precedence
  const envPort = process.env.TRACKLENS_PORT;
  if (envPort) {
    const parsed = parseInt(envPort, 10);
    if (!isNaN(parsed) && parsed > 0 && parsed < 65536) {
      return parsed;
    }
    console.error(
      `[TrackLens] Warning: Invalid TRACKLENS_PORT "${envPort}", using default`
    );
  }

  // Remote sessions use fixed port for port forwarding; local uses random
  return isRemoteSession() ? DEFAULT_REMOTE_PORT : 0;
}
