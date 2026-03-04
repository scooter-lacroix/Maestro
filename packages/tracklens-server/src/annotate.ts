/**
 * TrackLens Annotate Server
 *
 * HTTP server for markdown annotation mode.
 * REBRANDED: Removed share/paste routes
 */

import { mkdirSync, writeFileSync, existsSync } from "fs";
import { join } from "path";
import { randomUUID } from "crypto";
import { openBrowser } from "./browser";
import { getServerPort, isRemoteSession } from "./remote";
import { getRepoInfo } from "./repo";
import { validateImagePath, validateUploadExtension, UPLOAD_DIR, sanitizeFileName, getSafeUploadPath } from "./image";

export interface AnnotateServerOptions {
  /** Markdown content of the file to annotate */
  markdown: string;
  /** Original file path (for display purposes) */
  filePath: string;
  /** HTML content to serve for the UI */
  htmlContent: string;
  /** Origin identifier for UI customization */
  origin?: "opencode" | "claude-code";
}

export interface AnnotateServerResult {
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
  }>;
  /** Stop the server */
  stop: () => void;
}

/**
 * Start the TrackLens annotate server
 */
export async function startAnnotateServer(
  options: AnnotateServerOptions
): Promise<AnnotateServerResult> {
  const {
    markdown,
    filePath,
    htmlContent,
    origin,
  } = options;

  // Generate authentication token for decision endpoint
  const authToken = randomUUID();

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
      }) => void)
    | undefined;

  const decisionPromise = new Promise<{
    feedback: string;
    annotations: unknown[];
  }>((resolve) => {
    resolveDecision = resolve;
  });

  // Start server
  const server = Bun.serve({
    port: configuredPort,
    fetch: async (req) => {
      const url = new URL(req.url);

      // API: Get plan content (reuse /api/plan so the plan editor UI works)
      if (url.pathname === "/api/plan" && req.method === "GET") {
        return Response.json({
          plan: markdown,
          origin,
          mode: "annotate",
          filePath,
          repoInfo,
        });
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
          } = body as {
            feedback: string;
            annotations: unknown[];
          };

          // Resolve decision promise
          if (resolveDecision) {
            resolveDecision({ feedback, annotations });
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
      // Inject authentication token into HTML for client-side access
      const tokenScript = `<script>window.TRACKLENS_AUTH_TOKEN = "${authToken}";</script>`;
      const htmlWithToken = htmlContent.replace("<head>", `<head>${tokenScript}`);
      return new Response(htmlWithToken, {
        headers: { "Content-Type": "text/html" },
      });
    },
  });

  const port = server.port;
  const url = `http://localhost:${port}`;

  // Handle server ready
  async function handleAnnotateServerReady(
    url: string,
    isRemote: boolean,
    port: number
  ): Promise<void> {
    if (!isRemote) {
      await openBrowser(url);
    }
  }

  await handleAnnotateServerReady(url, isRemote, port);

  return {
    port,
    url,
    isRemote,
    waitForDecision: () => decisionPromise,
    stop: () => server.stop(),
  };
}
