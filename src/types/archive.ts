export type CompressFormat =
  | "zip"
  | "seven-z"
  | "tar-gz"
  | "tar-bz2"
  | "tar-xz"
  | "tar-zst"
  | "tar"
  | "dmg";

export type ExtractMode = "smart" | "here" | "subfolder";

export type LaunchMode =
  | "open"
  | "extract-smart"
  | "extract-here"
  | "extract-subfolder"
  | "extract-to"
  | "compress-zip"
  | "compress-7z"
  | "compress-dialog";

export interface ArchiveEntry {
  path: string;
  name: string;
  isDir: boolean;
  size: number;
  compressedSize: number;
  modified: string | null;
  encrypted: boolean;
  index: number;
}

export interface ArchiveInfo {
  path: string;
  fileName: string;
  format: string;
  formatLabel: string;
  archiveSize: number;
  totalUncompressedSize: number;
  fileCount: number;
  dirCount: number;
  isEncrypted: boolean;
  detectedEncoding: string;
  engine: string;
  smartExtractWillCreateFolder: boolean;
  suggestedFolderName: string;
  entries: ArchiveEntry[];
}

export interface ExtractOptions {
  archivePath: string;
  destDir: string;
  smartExtract: boolean;
  password?: string;
  encoding?: string;
  selectedPaths?: string[];
  overwrite: boolean;
  extractMode?: ExtractMode;
  taskId?: string;
}

export interface ExtractResult {
  outputDir: string;
  extractedFiles: number;
  totalBytes: number;
  elapsedMs: number;
  primaryTarget?: string | null;
  primaryTargetIsDir?: boolean;
}

export interface ExtractConflictInfo {
  hasConflict: boolean;
  conflictPath: string;
  suggestedRenameDir: string;
}

export interface PendingLaunchPayload {
  paths: string[];
  mode: LaunchMode;
  standalone: boolean;
}

export interface CompressOptions {
  inputPaths: string[];
  outputPath: string;
  format: CompressFormat;
  level: number;
  password?: string;
  excludeJunk: boolean;
  taskId?: string;
}

export interface CompressResult {
  outputPath: string;
  inputFiles: number;
  originalBytes: number;
  archiveBytes: number;
  elapsedMs: number;
}

export interface TaskProgress {
  taskId: string;
  kind: "extract" | "compress";
  currentFile: string;
  processedItems: number;
  totalItems: number;
  processedBytes: number;
  totalBytes: number;
  currentFileBytes: number;
  currentFileTotalBytes: number;
  currentFilePercent: number;
  percent: number;
  elapsedMs: number;
  speedBps: number;
  paused: boolean;
  done: boolean;
}

export type TaskStatus =
  | "running"
  | "paused"
  | "completed"
  | "cancelled"
  | "error";

export interface ActiveTask {
  taskId: string;
  kind: "extract" | "compress";
  title: string;
  archivePath: string;
  destDir: string;
  primaryTarget?: string | null;
  primaryTargetIsDir?: boolean;
  standaloneWindow: boolean;
  status: TaskStatus;
  progress: TaskProgress;
  openFolderAfterDone: boolean;
  autoCloseAfterDone: boolean;
  deleteSourceAfterExtract: boolean;
  summary?: string;
  errorMessage?: string;
  extractedFiles?: number;
  inputFiles?: number;
  originalBytes?: number;
  archiveBytes?: number;
  elapsedMs?: number;
}

export interface EngineStatus {
  nativeReady: boolean;
  sevenzCliReady: boolean;
  sevenzCliPath: string | null;
  supportedReadFormats: string[];
  supportedWriteFormats: string[];
}

export type DoubleClickAction = "smart_extract" | "browse";

export interface AppSettings {
  doubleClickAction: DoubleClickAction;
  closeToTray: boolean;
  excludeJunk: boolean;
  openFolderAfterExtract: boolean;
  openFolderAfterCompress: boolean;
  autoCloseProgressWindow: boolean;
  defaultCompressFormat: CompressFormat;
}

export const DEFAULT_APP_SETTINGS: AppSettings = {
  doubleClickAction: "browse",
  closeToTray: true,
  excludeJunk: true,
  openFolderAfterExtract: true,
  openFolderAfterCompress: false,
  autoCloseProgressWindow: false,
  defaultCompressFormat: "zip",
};

export interface SystemIntegrationStatus {
  platform: string;
  contextMenuInstalled: boolean;
  executablePath: string;
  hint: string;
}
