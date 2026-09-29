import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export function hideWindow(): Promise<void> {
  return invoke("hide_window");
}

/** Fires every time Super+Z (or `--toggle`) brings the window up. */
export function onShown(handler: () => void): Promise<UnlistenFn> {
  return listen("shell://shown", handler);
}
