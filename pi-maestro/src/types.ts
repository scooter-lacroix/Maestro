/**
 * Type definitions for pi-mono ExtensionAPI
 *
 * These types are compatible with pi-mono's extension system.
 * Based on /home/stan/pi-mono/packages/coding-agent/src/core/extensions/types.ts
 */

/** Extension UI context - provides methods for user interaction */
export interface ExtensionUIContext {
  /** Show a selector and return the user's choice */
  select(title: string, options: string[], opts?: any): Promise<string | undefined>;

  /** Show a confirmation dialog */
  confirm(title: string, message: string, opts?: any): Promise<boolean>;

  /** Show a text input dialog */
  input(title: string, placeholder?: string, opts?: any): Promise<string | undefined>;

  /** Show a notification to the user */
  notify(message: string, type?: "info" | "warning" | "error"): void;

  /** Set status text in the footer/status bar */
  setStatus(key: string, text: string | undefined): void;

  /** Set the working/loading message */
  setWorkingMessage(message?: string): void;

  /** Set a widget to display above or below the editor */
  setWidget(key: string, content: string[] | undefined, options?: any): void;

  /** Set a custom footer component */
  setFooter(factory: any | undefined): void;

  /** Set a custom header component */
  setHeader(factory: any | undefined): void;

  /** Set the terminal window/tab title */
  setTitle(title: string): void;

  /** Show a custom component with keyboard focus */
  custom<T>(factory: any, options?: any): Promise<T>;

  /** Set the text in the core input editor */
  setEditorText(text: string): void;

  /** Get the current text from the core input editor */
  getEditorText(): string;

  /** Show a multi-line editor for text editing */
  editor(title: string, prefill?: string): Promise<string | undefined>;

  /** Set a custom editor component */
  setEditorComponent(factory: any | undefined): void;

  /** Get the current theme */
  readonly theme: any;

  /** Get all available themes */
  getAllThemes(): { name: string; path: string | undefined }[];

  /** Load a theme by name */
  getTheme(name: string): any | undefined;
}

/** Session manager for branching support */
export interface SessionManager {
  getBranch(): Array<{ type: string; content: any }>;
}

/** Extension context passed to command handlers */
export interface ExtensionContext {
  ui: ExtensionUIContext;
  sessionManager: SessionManager;
}

/** Command handler function */
export type CommandHandler = (args: string, ctx: ExtensionContext) => Promise<void>;

/** Command registration options */
export interface CommandOptions {
  description: string;
  handler: CommandHandler;
}

/** Main ExtensionAPI interface */
export interface ExtensionAPI {
  /** Register a user-invoked command */
  registerCommand(name: string, options: CommandOptions): void;

  /** Register an LLM-callable tool */
  registerTool(options: {
    name: string;
    label: string;
    description: string;
    parameters: any;
    execute: (...args: any[]) => any;
  }): void;

  /** Register an event handler */
  on(event: string, handler: (...args: any[]) => any): void;

  /** Send a message to the agent session */
  sendMessage(message: { customType: string; content: string; display: boolean }, options?: { triggerTurn: boolean }): void;
}
