/**
 * TrackLens Server - Main Entry Point
 *
 * HTTP server for plan review and annotation.
 * REBRANDED: startPlannotatorServer → startTrackLensServer
 * REBRANDED: Removed share/paste routes (not needed for TrackLens)
 */

import {
  mkdirSync,
  existsSync,
  readFileSync,
  writeFileSync,
  readdirSync,
} from "fs";
import { join, resolve, normalize, dirname } from "path";
import { fileURLToPath } from "url";
import { openBrowser } from "./browser";
import { getServerPort, isRemoteSession } from "./remote";
import { generateSlug, savePlan, saveAnnotations, saveFinalSnapshot } from "./storage";
import { saveToObsidian, saveToBear, detectObsidianVaults } from "./integrations";
import { getRepoInfo } from "./repo";
import { validateImagePath, validateUploadExtension, UPLOAD_DIR, sanitizeFileName, getSafeUploadPath } from "./image";
import { openEditorDiff } from "./ide";
import { detectProjectName, sanitizeTag } from "./project";
import {
  createClientReadyMonitor,
  injectTrackLensBootstrap,
} from "./ui-readiness";

// Resolve paths for asset serving
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = process.env.CLAUDE_PROJECT_DIR || resolve(__dirname, "..", "..", "..");

export interface ServerOptions {
  /** The plan markdown content */
  plan: string;
  /** Origin identifier (e.g., "claude-code", "opencode") */
  origin: string;
  /** HTML content to serve for the UI */
  htmlContent: string;
  /** Current autonomy mode to preserve (Claude Code only) */
  autonomyMode?: string;
}

export interface ServerResult {
  /** The port the server is running on */
  port: number;
  /** The full URL to access the server */
  url: string;
  /** Whether running in remote mode */
  isRemote: boolean;
  /** Wait for user decision (approve/deny) */
  waitForDecision: () => Promise<{
    approved: boolean;
    feedback?: string;
    savedPath?: string;
    annotations?: unknown[];
    agentSwitch?: string;
    autonomyMode?: string;
  }>;
  /** Stop the server */
  stop: () => void;
}

interface VaultNode {
  name: string;
  path: string; // relative path within vault
  type: "file" | "folder";
  children?: VaultNode[];
}

/**
 * Build file tree from relative paths
 */
function buildFileTree(relativePaths: string[]): VaultNode[] {
  const root: VaultNode[] = [];

  for (const filePath of relativePaths) {
    const parts = filePath.split("/");
    let current = root;
    let pathSoFar = "";

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      pathSoFar = pathSoFar ? `${pathSoFar}/${part}` : part;
      const isFile = i === parts.length - 1;

      let node = current.find(
        (n) => n.name === part && n.type === (isFile ? "file" : "folder")
      );

      if (!node) {
        node = {
          name: part,
          path: pathSoFar,
          type: isFile ? "file" : "folder",
          children: isFile ? undefined : [],
        };
        current.push(node);
      }

      if (!isFile && node.children) {
        current = node.children;
      }
    }
  }

  return root;
}

/**
 * Start the TrackLens server
 * REBRANDED: Renamed from startPlannotatorServer
 * REMOVED: sharingEnabled, shareBaseUrl, pasteApiUrl options
 */
export async function startTrackLensServer(
  options: ServerOptions
): Promise<ServerResult> {
  const { plan, origin, htmlContent, autonomyMode } = options;

  const isRemote = isRemoteSession();
  const configuredPort = getServerPort();

  // Generate slug for potential saving (actual save happens on decision)
  const slug = generateSlug(plan);

  // Detect repo info (cached for this session)
  const repoInfo = await getRepoInfo();

  // Ensure upload directory exists
  mkdirSync(UPLOAD_DIR, { recursive: true });

  // Decision promise
  let resolveDecision:
    | ((result: {
        approved: boolean;
        feedback?: string;
        savedPath?: string;
        annotations?: unknown[];
        agentSwitch?: string;
        autonomyMode?: string;
      }) => void)
    | undefined;

  const decisionPromise = new Promise<{
    approved: boolean;
    feedback?: string;
    savedPath?: string;
    annotations?: unknown[];
    agentSwitch?: string;
    autonomyMode?: string;
  }>((resolve) => {
    resolveDecision = resolve;
  });
  const clientReady = createClientReadyMonitor();

  // Mutable state for auto-close
  let shouldAutoClose = false;

  // Start server
  const server = Bun.serve({
    port: configuredPort,
    fetch: async (req) => {
      const url = new URL(req.url);

      // API: Get plan content
      if (url.pathname === "/api/plan" && req.method === "GET") {
        return Response.json({
          plan,
          origin,
          mode: "review",
          repoInfo,
          ...(autonomyMode && { autonomyMode }),
        });
      }

      // API: Save plan (user clicked save)
      if (url.pathname === "/api/save" && req.method === "POST") {
        try {
          const body = await req.json();
          const { customPath } = body as { customPath?: string };

          const savedPath = savePlan(slug, plan, customPath);

          return Response.json({ success: true, path: savedPath });
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
          const { vaultPath, folder, filenameFormat } = body as {
            vaultPath: string;
            folder: string;
            filenameFormat?: string;
          };

          const result = await saveToObsidian({
            vaultPath,
            folder,
            content: plan,
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
          const result = await saveToBear({ content: plan });
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
        const vaults = detectObsidianVaults();
        return Response.json({ vaults });
      }

      // API: Detect project name
      if (url.pathname === "/api/project" && req.method === "GET") {
        const projectName = await detectProjectName();
        return Response.json({ projectName });
      }

      if (url.pathname === "/api/client-ready" && req.method === "POST") {
        clientReady.markClientReady();
        return Response.json({ ready: true });
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

      // API: Get vault file tree
      if (url.pathname === "/api/vault-tree" && req.method === "POST") {
        try {
          const body = await req.json();
          const { vaultPath, folder } = body as {
            vaultPath: string;
            folder: string;
          };

          // Normalize path
          let normalizedVault = vaultPath.trim();
          if (normalizedVault.startsWith("~")) {
            const home =
              process.env.HOME || process.env.USERPROFILE || "";
            normalizedVault = join(home, normalizedVault.slice(1));
          }

          // Path traversal sanitization: verify resolved path stays within vault root
          const resolvedVault = resolve(normalizedVault);
          const resolved = resolve(resolvedVault, normalize(folder));
          if (!resolved.startsWith(resolve(resolvedVault))) {
            return Response.json({
              success: false,
              error: "Invalid folder path",
            }, { status: 400 });
          }

          const folderPath = join(normalizedVault, folder);

          if (!existsSync(folderPath)) {
            return Response.json(
              { success: false, error: "Folder not found" },
              { status: 404 }
            );
          }

          // Collect all markdown files with depth/file limits
          const files: string[] = [];
          const MAX_DEPTH = 10;
          const MAX_FILES = 5000;

          const collectFiles = (dir: string, base = "", depth = 0) => {
            // Check depth limit
            if (depth > MAX_DEPTH || files.length >= MAX_FILES) {
              return;
            }

            const items = readdirSync(dir, { withFileTypes: true });

            for (const item of items) {
              // Check file count limit before processing each item
              if (files.length >= MAX_FILES) {
                return;
              }

              const itemPath = join(dir, item.name);
              const relativePath = base ? `${base}/${item.name}` : item.name;

              if (item.isDirectory()) {
                collectFiles(itemPath, relativePath, depth + 1);
              } else if (item.name.endsWith(".md")) {
                files.push(relativePath);
              }
            }
          };

          collectFiles(folderPath);

          const tree = buildFileTree(files);
          return Response.json({ success: true, tree });
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

      // API: Open editor diff
      if (url.pathname === "/api/open-diff" && req.method === "POST") {
        try {
          const body = await req.json();
          const { oldPath, newPath } = body as {
            oldPath: string;
            newPath: string;
          };

          const result = await openEditorDiff(oldPath, newPath);
          return Response.json(result);
        } catch (error) {
          return Response.json(
            {
              error: error instanceof Error ? error.message : String(error),
            },
            { status: 500 }
          );
        }
      }

      // API: Approve plan (ORIGINAL PLANNOTATOR ENDPOINT)
      if (url.pathname === "/api/approve" && req.method === "POST") {
        try {
          const body = await req.json().catch(() => ({}));
          const {
            feedback,
            agentSwitch,
            autonomyMode: requestedPermissionMode,
          } = body as {
            feedback?: string;
            agentSwitch?: string;
            permissionMode?: string;
            autonomyMode?: string;
          };

          // Resolve decision promise with approval
          if (resolveDecision) {
            resolveDecision({
              approved: true,
              feedback,
              agentSwitch,
              autonomyMode: requestedPermissionMode || autonomyMode,
            });
          }

          shouldAutoClose = true;
          return Response.json({ ok: true });
        } catch (error) {
          return Response.json(
            { error: error instanceof Error ? error.message : String(error) },
            { status: 500 }
          );
        }
      }

      // API: Deny with feedback (ORIGINAL PLANNOTATOR ENDPOINT)
      if (url.pathname === "/api/deny" && req.method === "POST") {
        try {
          const body = await req.json().catch(() => ({}));
          let feedback = "Plan rejected by user";
          if (body.feedback) {
            feedback = body.feedback;
          }

          // Resolve decision promise with denial
          if (resolveDecision) {
            resolveDecision({ approved: false, feedback });
          }

          shouldAutoClose = true;
          return Response.json({ ok: true });
        } catch (error) {
          return Response.json(
            { error: error instanceof Error ? error.message : String(error) },
            { status: 500 }
          );
        }
      }

      // API: Submit decision (approve/deny) - COMPATIBILITY ENDPOINT
      if (url.pathname === "/api/decision" && req.method === "POST") {
        try {
          const body = await req.json();
          const {
            approved,
            feedback,
            customPath,
            annotations,
            agentSwitch,
            autonomyMode: newAutonomyMode,
          } = body as {
            approved: boolean;
            feedback?: string;
            customPath?: string;
            annotations?: unknown[] | string;
            agentSwitch?: string;
            autonomyMode?: string;
          };

          // Save final snapshot
          let savedPath: string | undefined;
          const annotationSnapshot =
            typeof annotations === "string"
              ? annotations
              : annotations
                ? JSON.stringify(annotations, null, 2)
                : "";
          if (feedback) {
            savedPath = saveFinalSnapshot(
              slug,
              approved ? "approved" : "denied",
              plan,
              annotationSnapshot,
              customPath
            );
          }

          // Resolve decision promise
          if (resolveDecision) {
            resolveDecision({
              approved,
              feedback,
              savedPath,
              annotations:
                typeof annotations === "string"
                  ? annotations
                    ? [{ id: "tracklens-feedback", text: annotations }]
                    : undefined
                  : annotations,
              agentSwitch,
              autonomyMode: newAutonomyMode,
            });
          }

          shouldAutoClose = true;

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

      // Serve static assets (JS, CSS files)
      if (url.pathname.startsWith("/assets/")) {
        const assetPath = url.pathname.slice(1); // Remove leading /
        const assetCandidates = [
          resolve(__dirname, "..", assetPath),
          resolve(__dirname, assetPath),
          resolve(projectRoot, "apps", "tracklens-hook", "dist", assetPath),
          resolve(projectRoot, "packages", "tracklens-editor", "dist", assetPath),
          resolve(projectRoot, "packages", "tracklens-review-editor", "dist", assetPath),
        ];

        for (const candidate of assetCandidates) {
          if (existsSync(candidate)) {
            const file = Bun.file(candidate);
            const ext = candidate.split(".").pop()?.toLowerCase();
            const contentType =
              ext === "js" ? "application/javascript" :
              ext === "css" ? "text/css" :
              ext === "svg" ? "image/svg+xml" :
              "application/octet-stream";
            return new Response(file, {
              headers: { "Content-Type": contentType },
            });
          }
        }
        return new Response("Asset not found", { status: 404 });
      }

      // Serve HTML
      const htmlWithBootstrap = injectTrackLensBootstrap(htmlContent);
      return new Response(htmlWithBootstrap, {
        headers: { "Content-Type": "text/html" },
      });
    },
  });

  const port = server.port;
  const url = `http://localhost:${port}`;

  // Handle server ready
  async function handleServerReady(
    url: string,
    isRemote: boolean,
    port: number
  ): Promise<void> {
    if (!isRemote) {
      const opened = await openBrowser(url);
      if (!opened) {
        throw new Error(`TrackLens could not open a browser for ${url}`);
      }
    }
  }

  await handleServerReady(url, isRemote, port);
  const ready = await clientReady.waitForClientReady();
  if (!ready) {
    server.stop();
    throw new Error(
      "TrackLens UI never reported readiness. Review aborted instead of assuming approval."
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
