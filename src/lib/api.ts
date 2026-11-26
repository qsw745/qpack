import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  ArchiveInfo,
  CompressFormat,
  CompressOptions,
  CompressResult,
  EngineStatus,
  ExtractConflictInfo,
  ExtractOptions,
  ExtractResult,
  PendingLaunchPayload,
  SystemIntegrationStatus,
} from "../types/archive";

export const api = {
  takePendingLaunchPaths: () =>
    invoke<PendingLaunchPayload>("take_pending_launch_paths"),

  checkExtractConflict: (options: ExtractOptions) =>
    invoke<ExtractConflictInfo>("check_extract_conflict", { options }),

  openOutputDirectory: (path: string) =>
    invoke<void>("open_output_directory", { path }),

  getAppSettings: () => invoke<AppSettings>("get_app_settings"),

  updateAppSettings: (newSettings: AppSettings) =>
    invoke<AppSettings>("update_app_settings", { newSettings }),

  getSystemIntegrationStatus: () =>
    invoke<SystemIntegrationStatus>("get_system_integration_status"),

  installSystemContextMenu: () =>
    invoke<SystemIntegrationStatus>("install_system_context_menu"),

  uninstallSystemContextMenu: () =>
    invoke<SystemIntegrationStatus>("uninstall_system_context_menu"),

  detectEngineStatus: () => invoke<EngineStatus>("detect_engine_status"),

  isSupportedArchive: (path: string) =>
    invoke<boolean>("is_supported_archive", { path }),

  inspectArchive: (path: string, encoding?: string, password?: string) =>
    invoke<ArchiveInfo>("inspect_archive", {
      path,
      encoding: encoding && encoding !== "auto" ? encoding : null,
      password: password || null,
    }),

  extractArchive: (options: ExtractOptions) =>
    invoke<ExtractResult>("extract_archive", { options }),

  compressFiles: (options: CompressOptions) =>
    invoke<CompressResult>("compress_files", { options }),

  previewEntry: (
    archivePath: string,
    entryPath: string,
    password?: string,
    encoding?: string
  ) =>
    invoke<string>("preview_entry", {
      archivePath,
      entryPath,
      password: password || null,
      encoding: encoding && encoding !== "auto" ? encoding : null,
    }),

  suggestCompressOutput: (paths: string[], format: CompressFormat) =>
    invoke<string>("suggest_compress_output", { paths, format }),
};

/** 格式化文件大小 */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const idx = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1
  );
  const val = bytes / Math.pow(1024, idx);
  return `${val >= 100 || idx === 0 ? val.toFixed(0) : val.toFixed(1)} ${units[idx]}`;
}

/** 获取路径父目录 */
export function dirname(fullPath: string): string {
  const normalized = fullPath.replace(/\\/g, "/");
  const idx = normalized.lastIndexOf("/");
  if (idx <= 0) return fullPath;
  return fullPath.slice(0, idx);
}
