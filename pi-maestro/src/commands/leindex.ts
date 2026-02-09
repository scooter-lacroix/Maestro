/**
 * /maestro:leindex command
 *
 * Run Maestro LeIndex code analysis
 * Calls maestro binary LeIndex command
 */

import type { ExtensionAPI } from "../types";
import { runMaestroLeIndex } from "../lib/cli";

/**
 * Register /maestro:leindex command
 */
export function registerLeindex(pi: ExtensionAPI, commandName: string) {
  pi.registerCommand(commandName, {
    description: "Run Maestro LeIndex code analysis and search",
    handler: async (args, ctx) => {
      const root = process.cwd();


      // Parse arguments
      const leindexArgs = args ? args.split(/\s+/) : [];

      const result = await runMaestroLeIndex(leindexArgs, root);

      if (result.exitCode === 0) {
      } else {
        ctx.ui.notify(`LeIndex error: ${result.stderr}`, "error");
      }
    },
  });
}
