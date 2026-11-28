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

  pauseTask: (taskId: string) => invoke<boolean>("pause_task", { taskId }),

  resumeTask: (taskId: string) => invoke<boolean>("resume_task", { taskId }),

  cancelTask: (taskId: string) => invoke<boolean>("cancel_task", { taskId }),

  switchToMainWindow: () => invoke<void>("switch_to_main_window"),

  checkExtractConflict: (options: ExtractOptions) =>
    invoke<ExtractConflictInfo>("check_extract_conflict", { options }),

  openOutputDirectory: (path: string) =>
    invoke<void>("open_output_directory", { path }),

  revealInFileManager: (path: string) =>
    invoke<void>("reveal_in_file_manager", { path }),

  trashPaths: (paths: string[]) => invoke<void>("trash_paths", { paths }),

  getAppSettings: () => invoke<AppSettings>("get_app_settings"),

  updateAppSettings: (newSettings: AppSettings) =>
    invoke<AppSettings>("update_app_settings", { newSettings }),

  getSystemIntegrationStatus: () =>
    invoke<SystemIntegrationStatus>("get_system_integration_status"),

  installSystemContextMenu: () =>
    invoke<SystemIntegrationStatus>("install_system_context_menu"),

  uninstallSystemContextMenu: () =>
    invoke<SystemIntegrationStatus>("uninstall_system_context_menu"),

  refreshFileIcons: () => invoke<void>("refresh_file_icons"),

  isMicrosoftStoreBuild: () => invoke<boolean>("is_microsoft_store_build"),

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
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const idx = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1
  );
  const val = bytes / Math.pow(1024, idx);
  return `${val >= 100 || idx === 0 ? val.toFixed(0) : val.toFixed(1)} ${units[idx]}`;
}

/** 格式化传输速度 */
export function formatSpeed(bps: number): string {
  if (!Number.isFinite(bps) || bps <= 0) return "—";
  return `${formatBytes(bps)}/s`;
}

/** 格式化时长为 HH:MM:SS */
export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "00:00:00";
  const totalSec = Math.floor(ms / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  return [h, m, s].map((v) => String(v).padStart(2, "0")).join(":");
}

/** 获取路径父目录 */
export function dirname(fullPath: string): string {
  const normalized = fullPath.replace(/\\/g, "/");
  const idx = normalized.lastIndexOf("/");
  if (idx <= 0) return fullPath;
  return fullPath.slice(0, idx);
}

/** 获取路径最后一段文件名 */
export function basename(fullPath: string): string {
  const normalized = fullPath.replace(/\\/g, "/").replace(/\/+$/, "");
  const idx = normalized.lastIndexOf("/");
  return idx === -1 ? normalized : normalized.slice(idx + 1);
}
