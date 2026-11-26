export type CompressFormat = "zip" | "seven-z" | "tar-gz" | "tar-zst" | "tar";

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
}

export interface ExtractResult {
  outputDir: string;
  extractedFiles: number;
  totalBytes: number;
  elapsedMs: number;
}

export interface ExtractConflictInfo {
  hasConflict: boolean;
  conflictPath: string;
  suggestedRenameDir: string;
}

export interface PendingLaunchPayload {
  paths: string[];
  autoExtract: boolean;
}

export interface CompressOptions {
  inputPaths: string[];
  outputPath: string;
  format: CompressFormat;
  level: number;
  password?: string;
  excludeJunk: boolean;
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
  percent: number;
  done: boolean;
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
}

export interface SystemIntegrationStatus {
  platform: string;
  contextMenuInstalled: boolean;
  executablePath: string;
  hint: string;
}
