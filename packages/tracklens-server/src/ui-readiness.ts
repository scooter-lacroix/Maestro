/**
 * UI Readiness Monitor
 *
 * Provides client readiness monitoring and HTML bootstrap injection for TrackLens.
 */

export interface ClientReadyMonitor {
  /** Mark the client as ready */
  markClientReady: () => void;
  /** Wait for client to become ready, with timeout */
  waitForClientReady: () => Promise<boolean>;
}

/**
 * Create a monitor to track client UI readiness
 */
export function createClientReadyMonitor(): ClientReadyMonitor {
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

    waitForClientReady(): Promise<boolean> {
      // Set a timeout of 30 seconds for client readiness
      timeoutId = setTimeout(() => {
        if (!isReady) {
          resolveReady(false);
        }
      }, 30000);

      return readyPromise;
    },
  };
}

/**
 * Escape special characters to prevent XSS when injecting into HTML/JS
 */
function escapeForJs(str: string): string {
  return str
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/'/g, "\\'")
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t');
}

/**
 * Inject TrackLens bootstrap script into HTML content
 */
export function injectTrackLensBootstrap(
  htmlContent: string,
  authToken: string
): string {
  const escapedToken = escapeForJs(authToken);
  const bootstrapScript = `
<script>
  window.__TRACKLENS__ = {
    authToken: "${escapedToken}",
    clientReady: false,
    markReady: function() {
      fetch('/api/client-ready', {
        method: 'POST',
        headers: {
          'Authorization': 'Bearer ${escapedToken}',
          'Content-Type': 'application/json'
        }
      }).then(() => {
        window.__TRACKLENS__.clientReady = true;
      }).catch(err => {
        console.error('TrackLens: Failed to mark client ready:', err);
      });
    }
  };
  
  // Auto-mark ready when DOM is loaded
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', window.__TRACKLENS__.markReady);
  } else {
    window.__TRACKLENS__.markReady();
  }
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
