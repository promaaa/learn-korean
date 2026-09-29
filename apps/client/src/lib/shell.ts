import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export function hideWindow(): Promise<void> {
  return invoke("hide_window");
}

export interface Shown {
  /** Progress from another device was merged: the running session is stale. */
  synced: boolean;
}

/** Fires every time Super+Z (or `--toggle`) brings the window up. */
export function onShown(handler: (shown: Shown) => void): Promise<UnlistenFn> {
  return listen<Shown>("shell://shown", (event) => handler(event.payload));
}
