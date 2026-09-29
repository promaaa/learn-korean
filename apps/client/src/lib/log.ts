import { error, warn } from "@tauri-apps/plugin-log";

/** Forwards uncaught frontend errors to the Rust log (stdout + log file). */
export function forwardErrorsToLog(): void {
  window.addEventListener("error", (event) => {
    void error(`[ui] ${event.message} at ${event.filename}:${event.lineno}`);
  });
  window.addEventListener("unhandledrejection", (event) => {
    void error(`[ui] unhandled rejection: ${String(event.reason)}`);
  });
  const consoleWarn = console.warn.bind(console);
  console.warn = (...args: unknown[]) => {
    consoleWarn(...args);
    void warn(`[ui] ${args.map(String).join(" ")}`);
  };
  const consoleError = console.error.bind(console);
  console.error = (...args: unknown[]) => {
    consoleError(...args);
    void error(`[ui] ${args.map(String).join(" ")}`);
  };
}
