import { invoke } from "@tauri-apps/api/core";

export interface AppStatus {
  version: string;
  database: string | null;
  schemaVersion: number | null;
  error: string | null;
}

export function appStatus(): Promise<AppStatus> {
  return invoke("app_status");
}
