/**
 * /maestro:configure command
 *
 * Configure maestro settings
 * Pi-mono specific: provider/model selection, workflow mode
 */

import type { ExtensionAPI } from "../types";
import * as path from "path";
import * as fs from "fs";

/** Maestro configuration */
interface MaestroConfig {
  defaultProvider: string;
  defaultModel: string;
  workflowMode: "sequential" | "parallel";
  criticalThinkEnabled: boolean;
  leIndexEnabled: boolean;
}

/** Config file path */
const CONFIG_PATH = path.join(
  process.env.HOME || "",
  ".claude",
  "maestro.local.md"
);

/**
 * Register /maestro:configure command
 */
export function registerConfigure(pi: ExtensionAPI, commandName: string) {
  pi.registerCommand(commandName, {
    description: "Configure maestro settings for pi-mono",
    handler: async (args, ctx) => {
      const setting = args.trim();

      if (!setting) {
        // Interactive configuration wizard
        await runConfigurationWizard(ctx);
      } else {
        // Quick set a specific setting
        await quickSetSetting(setting, ctx);
      }
    },
  });
}

/**
 * Run interactive configuration wizard
 */
async function runConfigurationWizard(ctx: any): Promise<void> {

  // Load existing config
  const config = loadConfig();

  // Question 1: Default provider
  const defaultProvider = await ctx.ui.select(
    "Default AI Provider",
    [
      "anthropic - Claude models (requires API key or OAuth)",
      "openai - GPT models (requires API key)",
      "openai-codex - Codex via ChatGPT (requires OAuth)",
      "google - Gemini models (requires API key)",
      "google-gemini-cli - Gemini CLI (free, OAuth)",
      "google-antigravity - Gemini 3 + Claude + GPT (free, OAuth)",
      "github-copilot - GPT-4o, Claude, Gemini (requires OAuth)",
      "amazon-bedrock - AWS Bedrock models",
      "mistral - Mistral AI models",
      "xai - Grok models",
      "groq - Fast inference",
      "openrouter - Multi-provider router",
    ]
  );

  if (defaultProvider) {
    const provider = defaultProvider.split(" - ")[0];
    config.defaultProvider = provider;
  }

  // Question 2: Workflow mode
  const workflowMode = await ctx.ui.select(
    "Workflow Mode",
    [
      "sequential - Execute tasks one at a time",
      "parallel - Execute tasks in parallel when possible",
    ]
  );

  if (workflowMode) {
    const mode = workflowMode.split(" - ")[0];
    config.workflowMode = mode as "sequential" | "parallel";
  }

  // Question 3: Critical Think
  const criticalThink = await ctx.ui.confirm(
    "Critical Think",
    "Enable Critical Think metacognitive analysis?"
  );

  config.criticalThinkEnabled = criticalThink ?? true;

  // Question 4: LeIndex integration
  const leIndex = await ctx.ui.confirm(
    "LeIndex Integration",
    "Enable LeIndex 5-phase code analysis? (Requires maestro CLI)"
  );

  config.leIndexEnabled = leIndex ?? true;

  // Save config
  saveConfig(config);

  ctx.ui.notify("Configuration saved", "success");
}

/**
 * Quick set a specific setting
 */
async function quickSetSetting(setting: string, ctx: any): Promise<void> {
  const [key, value] = setting.split("=");

  if (!key || !value) {
    ctx.ui.notify("Usage: /maestro:configure <key>=<value>", "error");
    return;
  }

  const config = loadConfig();

  switch (key.toLowerCase()) {
    case "provider":
      config.defaultProvider = value;
      break;
    case "model":
      config.defaultModel = value;
      break;
    case "workflow":
      if (value === "sequential" || value === "parallel") {
        config.workflowMode = value;
      } else {
        ctx.ui.notify('Invalid workflow mode. Use "sequential" or "parallel"', "error");
        return;
      }
      break;
    case "critical-think":
      config.criticalThinkEnabled = value === "true" || value === "1";
      break;
    case "leindex":
      config.leIndexEnabled = value === "true" || value === "1";
      break;
    default:
      ctx.ui.notify(`Unknown setting: ${key}`, "error");
      return;
  }

  saveConfig(config);
  ctx.ui.notify(`Set ${key} = ${value}`, "success");
}

/**
 * Load configuration from file
 */
function loadConfig(): MaestroConfig {
  const defaultConfig: MaestroConfig = {
    defaultProvider: "anthropic",
    defaultModel: "",
    workflowMode: "sequential",
    criticalThinkEnabled: true,
    leIndexEnabled: true,
  };

  if (!fs.existsSync(CONFIG_PATH)) {
    return defaultConfig;
  }

  try {
    const content = fs.readFileSync(CONFIG_PATH, "utf-8");
    // Parse frontmatter from markdown
    const frontmatterMatch = content.match(/^---\n([\s\S]+?)\n---/);
    if (frontmatterMatch) {
      const frontmatter = frontmatterMatch[1];
      const config: Partial<MaestroConfig> = {};

      for (const line of frontmatter.split("\n")) {
        const match = line.match(/^(\w+):\s*(.+)$/);
        if (match) {
          const key = match[1];
          const value = match[2];

          switch (key) {
            case "defaultProvider":
              config.defaultProvider = value;
              break;
            case "defaultModel":
              config.defaultModel = value;
              break;
            case "workflowMode":
              config.workflowMode = value as "sequential" | "parallel";
              break;
            case "criticalThinkEnabled":
              config.criticalThinkEnabled = value === "true";
              break;
            case "leIndexEnabled":
              config.leIndexEnabled = value === "true";
              break;
          }
        }
      }

      return { ...defaultConfig, ...config };
    }
  } catch {
    // Ignore parse errors, use defaults
  }

  return defaultConfig;
}

/**
 * Save configuration to file
 */
function saveConfig(config: MaestroConfig): void {
  const content = `---
defaultProvider: ${config.defaultProvider}
defaultModel: ${config.defaultModel}
workflowMode: ${config.workflowMode}
criticalThinkEnabled: ${config.criticalThinkEnabled}
leIndexEnabled: ${config.leIndexEnabled}
---

# Maestro Local Configuration

This file contains local maestro settings for pi-mono.

## Settings

- **defaultProvider**: AI provider to use (anthropic, openai, openai-codex, google, etc.)
- **defaultModel**: Specific model ID (leave empty for provider default)
- **workflowMode**: Task execution mode (sequential or parallel)
- **criticalThinkEnabled**: Enable Critical Think metacognitive analysis
- **leIndexEnabled**: Enable LeIndex 5-phase code analysis (requires maestro CLI)

## Note

This configuration is specific to pi-mono. For model and provider setup, see:
- pi-mono README: https://github.com/badlogic/pi-mono
- Run \`/login\` in pi to configure OAuth providers
- Set API keys in \`~/.pi/agent/auth.json\`
`;

  const configDir = path.dirname(CONFIG_PATH);
  fs.mkdirSync(configDir, { recursive: true });
  fs.writeFileSync(CONFIG_PATH, content, "utf-8");
}
