/**
 * /maestro:tui command
 *
 * Launch Rust Cockpit TUI
 * Calls maestro binary TUI command
 */

import type { ExtensionAPI } from "../types";
import { launchMaestroTui } from "../lib/cli";

/**
 * Register /maestro:tui command
 */
export function registerTui(pi: ExtensionAPI, commandName: string) {
  pi.registerCommand(commandName, {
    description: "Launch Maestro Cockpit TUI (Rust terminal UI)",
    handler: async (args, ctx) => {
      const root = process.cwd();


      const result = await launchMaestroTui(root);

      if (result.exitCode === 0) {
      } else {
        ctx.ui.notify(`TUI exited with error: ${result.stderr}`, "error");
      }
    },
  });
}
