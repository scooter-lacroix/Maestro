/**
 * TrackLens Review Server
 *
 * HTTP server for code review mode (git diff visualization).
 * REBRANDED: Removed share/paste routes
 */

import { mkdirSync, writeFileSync, existsSync } from "fs";
import { join } from "path";
import { randomUUID } from "crypto";
import { openBrowser } from "./browser";
import { getServerPort, isRemoteSession } from "./remote";
import { getRepoInfo } from "./repo";
import { validateImagePath, validateUploadExtension, UPLOAD_DIR, sanitizeFileName, getSafeUploadPath } from "./image";
import type { GitContext, DiffType, DiffResult } from "./git";
import { runGitDiff } from "./git";
import {
  createClientReadyMonitor,
  injectTrackLensBootstrap,
} from "./ui-readiness";

export interface ReviewServerOptions {
  /** Raw git diff patch string */
  rawPatch: string;
  /** Git ref used for the diff (e.g., "HEAD", "main..HEAD", "--staged") */
  gitRef: string;
  /** Error message if git diff failed */
  error?: string;
  /** HTML content to serve for the UI */
  htmlContent: string;
  /** Origin identifier for UI customization */
  origin?: "opencode" | "claude-code";
  /** Current diff type being displayed */
  diffType?: DiffType;
  /** Git context with branch info and available diff options */
  gitContext?: GitContext;
}

export interface ReviewServerResult {
  /** The port the server is running on */
  port: number;
  /** The full URL to access the server */
  url: string;
  /** Whether running in remote mode */
  isRemote: boolean;
  /** Wait for user feedback submission */
  waitForDecision: () => Promise<{
    feedback: string;
    annotations: unknown[];
    agentSwitch?: string;
  }>;
  /** Stop the server */
  stop: () => void;
}

/**
 * Start the TrackLens review server
 */
export async function startReviewServer(
  options: ReviewServerOptions
): Promise<ReviewServerResult> {
  const { htmlContent, origin, gitContext, rawPatch, gitRef, error } = options;

  // Generate authentication token for decision endpoint
  const authToken = randomUUID();

  // Mutable state for diff switching
  let currentPatch = options.rawPatch;
  let currentGitRef = options.gitRef;
  let currentDiffType: DiffType = options.diffType || "uncommitted";
  let currentError = options.error;

  const isRemote = isRemoteSession();
  const configuredPort = getServerPort();

  // Detect repo info (cached for this session)
  const repoInfo = await getRepoInfo();

  // Ensure upload directory exists
  mkdirSync(UPLOAD_DIR, { recursive: true });

  // Decision promise
  let resolveDecision:
    | ((result: {
      feedback: string;
      annotations: unknown[];
      agentSwitch?: string;
    }) => void)
    | undefined;

  const decisionPromise = new Promise<{
    feedback: string;
    annotations: unknown[];
    agentSwitch?: string;
  }>((resolve) => {
    resolveDecision = resolve;
  });
  const clientReady = createClientReadyMonitor();

  // Start server
  const server = Bun.serve({
    port: configuredPort,
    fetch: async (req) => {
      const url = new URL(req.url);

      // API: Get diff content
      if (url.pathname === "/api/diff" && req.method === "GET") {
        return Response.json({
          rawPatch: currentPatch,
          gitRef: currentGitRef,
          origin,
          diffType: currentDiffType,
          gitContext,
          repoInfo,
          ...(currentError && { error: currentError }),
        });
      }

      if (url.pathname === "/api/client-ready" && req.method === "POST") {
        const authHeader = req.headers.get("authorization");
        if (authHeader !== `Bearer ${authToken}`) {
          return Response.json({ error: "Unauthorized" }, { status: 401 });
        }

        clientReady.markClientReady();
        return Response.json({ ready: true });
      }

      // API: Switch diff type
      if (url.pathname === "/api/switch-diff" && req.method === "POST") {
        try {
          const body = await req.json();
          const { diffType } = body as { diffType: DiffType };

          if (gitContext) {
            const result = await runGitDiff(diffType, gitContext.defaultBranch);
            currentPatch = result.patch;
            currentDiffType = diffType;
            currentGitRef = diffType;
            currentError = result.error;

            return Response.json({
              success: true,
              rawPatch: currentPatch,
              gitRef: currentGitRef,
              diffType: currentDiffType,
              error: currentError,
            });
          }

          return Response.json(
            { success: false, error: "No git context available" },
            { status: 400 }
          );
        } catch (error) {
          return Response.json(
            {
              success: false,
              error: error instanceof Error ? error.message : String(error),
            },
            { status: 500 }
          );
        }
      }

      // API: Save to Obsidian
      if (url.pathname === "/api/obsidian" && req.method === "POST") {
        try {
          const body = await req.json();
          const { vaultPath, folder, filenameFormat, content } = body as {
            vaultPath: string;
            folder: string;
            filenameFormat?: string;
            content?: string;
          };

          const { saveToObsidian: save } = await import("./integrations");
          const result = await save({
            vaultPath,
            folder,
            content: content || currentPatch,
            filenameFormat,
          });

          return Response.json(result);
        } catch (error) {
          return Response.json(
            {
              success: false,
              error: error instanceof Error ? error.message : String(error),
            },
            { status: 500 }
          );
        }
      }

      // API: Save to Bear
      if (url.pathname === "/api/bear" && req.method === "POST") {
        try {
          const body = await req.json().catch(() => ({}));
          const { content } = body as { content?: string };
          const { saveToBear: save } = await import("./integrations");
          const result = await save({ content: content || currentPatch });
          return Response.json(result);
        } catch (error) {
          return Response.json(
            {
              success: false,
              error: error instanceof Error ? error.message : String(error),
            },
            { status: 500 }
          );
        }
      }

      // API: List Obsidian vaults
      if (url.pathname === "/api/vaults" && req.method === "GET") {
        const { detectObsidianVaults } = await import("./integrations");
        const vaults = detectObsidianVaults();
        return Response.json({ vaults });
      }
      // API: Validate image path
      if (url.pathname === "/api/validate-image" && req.method === "POST") {
        try {
          const body = await req.json();
          const { imagePath } = body as { imagePath: string };

          const result = validateImagePath(imagePath);
          return Response.json(result);
        } catch (error) {
          return Response.json(
            {
              valid: false,
              resolved: "",
              error: error instanceof Error ? error.message : String(error),
            },
            { status: 400 }
          );
        }
      }

      // API: Upload image
      if (url.pathname === "/api/upload-image" && req.method === "POST") {
        try {
          const formData = await req.formData();
          const file = formData.get("image") as File;

          if (!file) {
            return Response.json(
              { success: false, error: "No image file provided" },
              { status: 400 }
            );
          }

          // Validate extension
          const validation = validateUploadExtension(file.name);
          if (!validation.valid) {
            return Response.json(
              { success: false, error: validation.error },
              { status: 400 }
            );
          }

          // Save file
          const fileName = `${Date.now()}.${validation.ext}`;
          const filePath = join(UPLOAD_DIR, fileName);
          const buffer = await file.arrayBuffer();
          writeFileSync(filePath, Buffer.from(buffer));

          return Response.json({
            success: true,
            url: `/api/images/${fileName}`,
          });
        } catch (error) {
          return Response.json(
            {
              success: false,
              error: error instanceof Error ? error.message : String(error),
            },
            { status: 500 }
          );
        }
      }

      // API: Serve uploaded images
      if (url.pathname.startsWith("/api/images/") && req.method === "GET") {
        const fileName = url.pathname.replace("/api/images/", "");
        const { safe, sanitized, error } = sanitizeFileName(fileName);

        if (!safe) {
          return Response.json(
            { success: false, error: error || "Invalid filename" },
            { status: 400 }
          );
        }

        const filePath = getSafeUploadPath(sanitized);

        if (existsSync(filePath)) {
          const file = Bun.file(filePath);
          return new Response(file);
        }

        return Response.json(
          { success: false, error: "Image not found" },
          { status: 404 }
        );
      }

      // API: Submit decision
      if (url.pathname === "/api/decision" && req.method === "POST") {
        // Validate authentication token
        const authHeader = req.headers.get("authorization");
        if (authHeader !== `Bearer ${authToken}`) {
          return Response.json({ error: "Unauthorized" }, { status: 401 });
        }

        try {
          const body = await req.json();
          const {
            feedback,
            annotations,
            agentSwitch,
          } = body as {
            feedback: string;
            annotations: unknown[];
            agentSwitch?: string;
          };

          // Resolve decision promise
          if (resolveDecision) {
            resolveDecision({ feedback, annotations, agentSwitch });
          }

          return Response.json({ success: true });
        } catch (error) {
          return Response.json(
            {
              success: false,
              error: error instanceof Error ? error.message : String(error),
            },
            { status: 500 }
          );
        }
      }

      // Serve HTML
      const htmlWithToken = injectTrackLensBootstrap(htmlContent, authToken);
      return new Response(htmlWithToken, {
        headers: { "Content-Type": "text/html" },
      });
    },
  });

  const port = server.port;
  const url = `http://localhost:${port}`;

  // Handle server ready
  async function handleReviewServerReady(
    url: string,
    isRemote: boolean,
    port: number
  ): Promise<void> {
    if (!isRemote) {
      const opened = await openBrowser(url);
      if (!opened) {
        throw new Error(`TrackLens review browser open failed for ${url}`);
      }
    }
  }

  await handleReviewServerReady(url, isRemote, port);
  const ready = await clientReady.waitForClientReady();
  if (!ready) {
    server.stop();
    throw new Error(
      "TrackLens review UI never became ready. Aborting instead of waiting indefinitely."
    );
  }

  return {
    port,
    url,
    isRemote,
    waitForDecision: () => decisionPromise,
    stop: () => server.stop(),
  };
}
