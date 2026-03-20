/**
 * UI Readiness Monitor
 *
 * Provides client readiness monitoring and HTML bootstrap injection for TrackLens.
 */

export interface ClientReadyMonitor {
  /** Mark the client as ready */
  markClientReady: () => void;
  /** Wait for client to become ready, with timeout */
  waitForClientReady: (timeoutMs?: number) => Promise<boolean>;
}

/**
 * Create a monitor to track client UI readiness
 * @param defaultTimeoutMs - Default timeout in milliseconds (defaults to 30000)
 */
export function createClientReadyMonitor(defaultTimeoutMs: number = 30000): ClientReadyMonitor {
  let isReady = false;
  let resolveReady: (value: boolean) => void;
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  const readyPromise = new Promise<boolean>((resolve) => {
    resolveReady = resolve;
  });

  return {
    markClientReady(): void {
      if (!isReady) {
        isReady = true;
        if (timeoutId) {
          clearTimeout(timeoutId);
          timeoutId = null;
        }
        resolveReady(true);
      }
    },

    waitForClientReady(timeoutMs?: number): Promise<boolean> {
      // Use provided timeout or fall back to default
      const actualTimeout = timeoutMs ?? defaultTimeoutMs;

      timeoutId = setTimeout(() => {
        if (!isReady) {
          resolveReady(false);
        }
      }, actualTimeout);

      return readyPromise;
    },
  };
}

/**
 * Inject TrackLens bootstrap script into HTML content
 */
export function injectTrackLensBootstrap(
  htmlContent: string
): string {
  const bootstrapScript = `
<script>
  (function() {
    function markClientReady() {
      fetch('/api/client-ready', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        }
      }).then(response => {
        if (response.ok) {
          console.log('TrackLens: Client ready signal sent');
        } else {
          console.error('TrackLens: Failed to mark client ready, status:', response.status);
        }
      }).catch(err => {
        console.error('TrackLens: Failed to send client ready signal:', err);
      });
    }

    // Auto-mark ready when DOM is loaded
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', markClientReady);
    } else {
      // DOM is already ready, call immediately
      markClientReady();
    }
  })();
</script>`;

  // Inject before closing </head> tag, or before </body> if no head
  if (htmlContent.includes("</head>")) {
    return htmlContent.replace("</head>", `${bootstrapScript}</head>`);
  } else if (htmlContent.includes("</body>")) {
    return htmlContent.replace("</body>", `${bootstrapScript}</body>`);
  } else {
    // Fallback: append to end
    return htmlContent + bootstrapScript;
  }
}
