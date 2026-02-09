/**
 * Maestro CLI wrapper
 *
 * Executes maestro binary commands for TUI and LeIndex
 * (these are augmentation tools, not core workflow)
 */

import { exec } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);

/** CLI execution result */
export interface CliResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

/** Run maestro CLI command */
export async function runMaestroCli(
  args: string[],
  cwd: string,
  timeout: number = 300000
): Promise<CliResult> {
  const maestroPath = process.env.MAESTRO_PATH || "maestro";
  const cmd = `${maestroPath} ${args.join(" ")}`;

  try {
    const { stdout, stderr } = await execAsync(cmd, {
      cwd,
      timeout,
    });
    return { stdout, stderr, exitCode: 0 };
  } catch (error: any) {
    return {
      stdout: error.stdout || "",
      stderr: error.stderr || error.message,
      exitCode: error.code || 1,
    };
  }
}

/** Launch Maestro Cockpit TUI */
export async function launchMaestroTui(cwd: string): Promise<CliResult> {
  return runMaestroCli(["tui"], cwd);
}

/** Run Maestro LeIndex code analysis */
export async function runMaestroLeIndex(
  args: string[],
  cwd: string
): Promise<CliResult> {
  return runMaestroCli(["leindex", ...args], cwd);
}
