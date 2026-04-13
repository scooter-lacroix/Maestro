/**
 * TrackLens Phase Reporter
 *
 * Reports TrackLens server phase changes back to the agent context,
 * so the agent knows what the user is doing (reviewing, editing, etc.).
 *
 * Polls `GET /api/phase` at a configurable interval and fires a callback
 * when the phase changes.
 *
 * @packageDocumentation
 */

/** TrackLens phase values reported by the server */
export type TrackLensReportedPhase =
  | "launching"
  | "loading"
  | "reviewing"
  | "editing"
  | "decided";

/** Human-readable descriptions for each phase */
const PHASE_DESCRIPTIONS: Record<TrackLensReportedPhase, string> = {
  launching: "TrackLens is starting up",
  loading: "Loading review content",
  reviewing: "User is reviewing the document",
  editing: "User is editing the document",
  decided: "User has made a decision",
};

/** Options for starting a phase reporter */
export interface PhaseReporterOptions {
  /** Base URL of the TrackLens server */
  serverUrl: string;
  /** Poll interval in milliseconds (default: 3000) */
  pollIntervalMs?: number;
  /** Callback fired when the phase changes */
  onPhaseChange: (phase: TrackLensReportedPhase, description: string) => void;
  /** AbortSignal for cleanup */
  signal: AbortSignal;
}

/**
 * Start polling the TrackLens server for phase changes.
 *
 * Returns immediately and runs polling in the background.
 * Polling stops when:
 * - The AbortSignal is aborted
 * - The server becomes unreachable
 * - The phase reaches "decided"
 *
 * @returns A cleanup function that stops polling
 */
export function startPhaseReporter(options: PhaseReporterOptions): () => void {
  const {
    serverUrl,
    pollIntervalMs = 3000,
    onPhaseChange,
    signal,
  } = options;

  let lastPhase: TrackLensReportedPhase | null = null;
  let stopped = false;
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  const stop = () => {
    stopped = true;
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }
  };

  const poll = async () => {
    if (stopped || signal.aborted) return;

    try {
      const response = await fetch(`${serverUrl}/api/phase`);
      if (!response.ok) {
        // Server might not have phase endpoint yet; retry
        scheduleNext();
        return;
      }

      const data = (await response.json()) as { phase: TrackLensReportedPhase };
      const phase = data.phase;

      if (phase !== lastPhase) {
        lastPhase = phase;
        const description = PHASE_DESCRIPTIONS[phase] || `Unknown phase: ${phase}`;
        onPhaseChange(phase, description);
      }

      // Stop polling if decided
      if (phase === "decided") {
        stop();
        return;
      }
    } catch {
      // Network error — server might be restarting or shutting down
      if (stopped || signal.aborted) return;
    }

    scheduleNext();
  };

  const scheduleNext = () => {
    if (stopped || signal.aborted) return;
    timeoutId = setTimeout(poll, pollIntervalMs);
  };

  // Listen for abort
  signal.addEventListener("abort", stop, { once: true });

  // Start polling immediately
  poll();

  return stop;
}
