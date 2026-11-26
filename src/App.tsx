import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  CheckCircle2,
  FolderOpen,
  FolderOutput,
  FolderPlus,
  Languages,
  Loader2,
  PackageOpen,
  Settings,
  Sparkles,
  X,
  AlertCircle,
} from "lucide-react";
import { ArchiveBrowser } from "./components/ArchiveBrowser";
import { CompressModal } from "./components/CompressModal";
import { DropZoneHome } from "./components/DropZoneHome";
import { PasswordModal } from "./components/PasswordModal";
import { SettingsModal } from "./components/SettingsModal";
import { UpdateBanner } from "./components/UpdateBanner";
import { api, dirname } from "./lib/api";
import { useAppStore } from "./stores/useAppStore";
import type { TaskProgress } from "./types/archive";

const ENCODING_OPTIONS = [
  { value: "auto", label: "编码：自动识别" },
  { value: "utf-8", label: "UTF-8" },
  { value: "gbk", label: "简体中文 (GBK)" },
  { value: "big5", label: "繁体中文 (Big5)" },
  { value: "shift_jis", label: "日文 (Shift-JIS)" },
  { value: "euc-kr", label: "韩文 (EUC-KR)" },
];

export default function App() {
  const archive = useAppStore((s) => s.archive);
  const loading = useAppStore((s) => s.loading);
  const encodingOverride = useAppStore((s) => s.encodingOverride);
  const selectedPaths = useAppStore((s) => s.selectedPaths);
  const progress = useAppStore((s) => s.progress);
  const toast = useAppStore((s) => s.toast);

  const setEngineStatus = useAppStore((s) => s.setEngineStatus);
  const setArchive = useAppStore((s) => s.setArchive);
  const setLoading = useAppStore((s) => s.setLoading);
  const setEncodingOverride = useAppStore((s) => s.setEncodingOverride);
  const setProgress = useAppStore((s) => s.setProgress);
  const showToast = useAppStore((s) => s.showToast);
  const clearToast = useAppStore((s) => s.clearToast);
  const openCompressModal = useAppStore((s) => s.openCompressModal);
  const resetArchiveView = useAppStore((s) => s.resetArchiveView);

  const [isDragging, setIsDragging] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [passwordPrompt, setPasswordPrompt] = useState<{
    open: boolean;
    title: string;
    onConfirm: (pwd: string) => void;
  }>({ open: false, title: "", onConfirm: () => {} });

  const [conflictPrompt, setConflictPrompt] = useState<{
    open: boolean;
    conflictPath: string;
    suggestedRenameDir: string;
    onOverwrite: () => void;
    onSkip: () => void;
    onRename: () => void;
  } | null>(null);

  // 初始化引擎状态，并全局禁用 WebView 浏览器默认右键菜单（保留输入框原生右键）
  useEffect(() => {
    api.detectEngineStatus().then(setEngineStatus).catch(() => {});

    const blockDefaultContextMenu = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      ) {
        return;
      }
      e.preventDefault();
    };
    window.addEventListener("contextmenu", blockDefaultContextMenu);
    return () => window.removeEventListener("contextmenu", blockDefaultContextMenu);
  }, [setEngineStatus]);

  // 监听后端进度事件
  useEffect(() => {
    const unlistenPromise = listen<TaskProgress>("qpack://progress", (event) => {
      const payload = event.payload;
      if (payload.done) {
        setProgress(null);
      } else {
        setProgress(payload);
      }
    });
    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, [setProgress]);

  // Toast 自动消失
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(clearToast, 6000);
    return () => clearTimeout(timer);
  }, [toast, clearToast]);

  // 打开并解析指定压缩包
  const loadArchive = useCallback(
    async (path: string, enc?: string, pwd?: string) => {
      setLoading(true);
      try {
        const info = await api.inspectArchive(path, enc, pwd);
        setArchive(info);
      } catch (err) {
        const msg = String(err);
        if (msg.includes("密码")) {
          setPasswordPrompt({
            open: true,
            title: "该压缩包目录已加密，请输入密码以查看内容",
            onConfirm: (password) => {
              setPasswordPrompt((p) => ({ ...p, open: false }));
              loadArchive(path, enc, password);
            },
          });
        } else {
          showToast({ type: "error", title: "无法打开压缩包", detail: msg });
        }
      } finally {
        setLoading(false);
      }
    },
    [setArchive, setLoading, showToast]
  );

  // 处理拖入的文件路径
  const handleDroppedPaths = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      if (paths.length === 1) {
        try {
          const isArc = await api.isSupportedArchive(paths[0]);
          if (isArc) {
            setEncodingOverride("auto");
            await loadArchive(paths[0], "auto");
            return;
          }
        } catch {
          // fallback to compress modal
        }
      }
      openCompressModal(paths);
    },
    [loadArchive, openCompressModal, setEncodingOverride]
  );

  // 注册 Tauri 原生拖放监听
  useEffect(() => {
    const unlistenPromise = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over" || event.payload.type === "enter") {
        setIsDragging(true);
      } else if (event.payload.type === "drop") {
        setIsDragging(false);
        handleDroppedPaths(event.payload.paths);
      } else {
        setIsDragging(false);
      }
    });
    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, [handleDroppedPaths]);

  // 接管系统“打开方式”/双击冷启动路径、右键自动解压与热唤起事件
  useEffect(() => {
    api
      .takePendingLaunchPaths()
      .then(async (payload) => {
        if (!payload?.paths?.length) return;
        if (payload.autoExtract && payload.paths.length === 1) {
          const targetPath = payload.paths[0];
          const ok = await api.isSupportedArchive(targetPath).catch(() => false);
          if (ok) {
            await loadArchive(targetPath, "auto");
            runExtract(dirname(targetPath), false, undefined, targetPath);
            return;
          }
        }
        handleDroppedPaths(payload.paths);
      })
      .catch(() => {});

    const unlistenOpen = listen<string[]>("qpack://open-paths", (event) => {
      if (event.payload?.length) {
        handleDroppedPaths(event.payload);
      }
    });
    const unlistenCompress = listen("qpack://trigger-compress", () => {
      openCompressModal([]);
    });

    return () => {
      unlistenOpen.then((fn) => fn());
      unlistenCompress.then((fn) => fn());
    };
  }, [handleDroppedPaths, openCompressModal]);

  const handlePickArchive = async () => {
    const picked = await open({
      multiple: false,
      title: "选择要打开的压缩包",
      filters: [
        {
          name: "压缩包",
          extensions: [
            "zip",
            "7z",
            "rar",
            "tar",
            "gz",
            "tgz",
            "bz2",
            "xz",
            "zst",
            "iso",
            "cab",
            "dmg",
          ],
        },
      ],
    });
    if (typeof picked === "string") {
      setEncodingOverride("auto");
      await loadArchive(picked, "auto");
    }
  };

  // 真正执行解压（含实时进度条、100% 完成反馈与自动打开结果目录）
  const executeExtractNow = async (
    targetArchivePath: string,
    destDir: string,
    smartExtract: boolean,
    selection: string[] | undefined,
    overwrite: boolean,
    password?: string
  ) => {
    const estimatedTotal = selection?.length || archive?.entries.length || 1;
    setProgress({
      taskId: "extract-pending",
      kind: "extract",
      currentFile: "正在准备解压...",
      processedItems: 0,
      totalItems: estimatedTotal,
      percent: 0,
      done: false,
    });
    try {
      const res = await api.extractArchive({
        archivePath: targetArchivePath,
        destDir,
        smartExtract,
        password,
        encoding: encodingOverride,
        selectedPaths: selection,
        overwrite,
      });
      // 短暂展示 100% 完成状态，让用户清晰感知解压已完成
      setProgress({
        taskId: "extract-done",
        kind: "extract",
        currentFile: `解压完成，共 ${res.extractedFiles} 个文件`,
        processedItems: res.extractedFiles,
        totalItems: Math.max(res.extractedFiles, 1),
        percent: 100,
        done: false,
      });
      await new Promise((r) => setTimeout(r, 320));
      setProgress(null);

      // 自动在系统资源管理器 / 访达中打开解压出的文件夹
      api.openOutputDirectory(res.outputDir).catch(() => {});

      showToast({
        type: "success",
        title: `解压完成（共 ${res.extractedFiles} 个文件）`,
        detail: `已解压至 ${res.outputDir}（耗时 ${(res.elapsedMs / 1000).toFixed(2)}s）`,
        actionPath: res.outputDir,
      });
    } catch (err) {
      setProgress(null);
      const msg = String(err);
      if (msg.includes("密码")) {
        setPasswordPrompt({
          open: true,
          title: "压缩包已加密，请输入解压密码",
          onConfirm: (pwd) => {
            setPasswordPrompt((p) => ({ ...p, open: false }));
            executeExtractNow(
              targetArchivePath,
              destDir,
              smartExtract,
              selection,
              overwrite,
              pwd
            );
          },
        });
      } else {
        showToast({ type: "error", title: "解压失败", detail: msg });
      }
    }
  };

  // 执行解压入口（先检测同名文件/目录冲突，再决定覆盖、跳过或自动新建目录）
  const runExtract = async (
    destDir: string,
    onlySelected: boolean,
    password?: string,
    overrideArchivePath?: string
  ) => {
    const targetArchivePath = overrideArchivePath || archive?.path;
    if (!targetArchivePath) return;
    const selection = onlySelected ? Array.from(selectedPaths) : undefined;
    const smartExtract = !onlySelected;

    try {
      const conflict = await api.checkExtractConflict({
        archivePath: targetArchivePath,
        destDir,
        smartExtract,
        password,
        encoding: encodingOverride,
        selectedPaths: selection,
        overwrite: true,
      });

      if (conflict.hasConflict) {
        setConflictPrompt({
          open: true,
          conflictPath: conflict.conflictPath,
          suggestedRenameDir: conflict.suggestedRenameDir,
          onRename: () => {
            setConflictPrompt(null);
            executeExtractNow(
              targetArchivePath,
              conflict.suggestedRenameDir,
              false,
              selection,
              true,
              password
            );
          },
          onOverwrite: () => {
            setConflictPrompt(null);
            executeExtractNow(
              targetArchivePath,
              destDir,
              smartExtract,
              selection,
              true,
              password
            );
          },
          onSkip: () => {
            setConflictPrompt(null);
            executeExtractNow(
              targetArchivePath,
              destDir,
              smartExtract,
              selection,
              false,
              password
            );
          },
        });
        return;
      }
    } catch {
      // 冲突检测失败（例如加密包需先输入密码）时直接进入解压流程处理
    }

    await executeExtractNow(
      targetArchivePath,
      destDir,
      smartExtract,
      selection,
      true,
      password
    );
  };

  // 一键智能解压到同级目录
  const handleQuickSmartExtract = () => {
    if (!archive) return;
    runExtract(dirname(archive.path), false);
  };

  // 选择目标目录解压（支持全量或选中项）
  const handleExtractToFolder = async (onlySelected: boolean) => {
    if (!archive) return;
    const dest = await open({
      directory: true,
      multiple: false,
      defaultPath: dirname(archive.path),
      title: onlySelected ? "选择选中文件的解压目录" : "选择解压目标目录",
    });
    if (typeof dest === "string") {
      runExtract(dest, onlySelected);
    }
  };

  // 快速预览单个文件
  const handlePreviewFile = async (entryPath: string, password?: string) => {
    if (!archive) return;
    try {
      const tempFile = await api.previewEntry(
        archive.path,
        entryPath,
        password,
        encodingOverride
      );
      await openPath(tempFile);
    } catch (err) {
      const msg = String(err);
      if (msg.includes("密码")) {
        setPasswordPrompt({
          open: true,
          title: "文件已加密，请输入密码以预览",
          onConfirm: (pwd) => {
            setPasswordPrompt((p) => ({ ...p, open: false }));
            handlePreviewFile(entryPath, pwd);
          },
        });
      } else {
        showToast({ type: "error", title: "预览失败", detail: msg });
      }
    }
  };

  const handleEncodingChange = async (enc: string) => {
    setEncodingOverride(enc);
    if (archive) {
      await loadArchive(archive.path, enc);
    }
  };

  return (
    <div className="flex h-screen flex-col bg-white text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      {/* 顶部极简工具栏 */}
      <header className="flex h-13 shrink-0 items-center justify-between gap-3 border-b border-zinc-200/80 bg-white/90 px-4 backdrop-blur-md dark:border-zinc-800 dark:bg-zinc-950/90">
        <div className="flex min-w-0 items-center gap-3">
          <button
            type="button"
            onClick={resetArchiveView}
            className="flex shrink-0 cursor-pointer items-center gap-2 rounded-lg px-1.5 py-1 transition hover:bg-zinc-100 dark:hover:bg-zinc-900"
          >
            <img
              src="/logo.png"
              alt="QPack"
              className="h-7 w-7 object-contain"
            />
            <span className="text-sm font-semibold tracking-tight">QPack</span>
          </button>

          {archive && (
            <>
              <span className="text-zinc-300 dark:text-zinc-700">/</span>
              <div className="flex min-w-0 items-center gap-2">
                <span
                  className="truncate text-sm font-medium text-zinc-800 dark:text-zinc-200"
                  title={archive.path}
                >
                  {archive.fileName}
                </span>
                <span className="shrink-0 rounded-md bg-blue-50 px-1.5 py-0.5 font-mono text-[11px] font-medium text-blue-700 dark:bg-blue-950/60 dark:text-blue-300">
                  {archive.formatLabel}
                </span>
              </div>
            </>
          )}
        </div>

        {/* 右侧操作区 */}
        <div className="flex shrink-0 items-center gap-2">
          {archive ? (
            <>
              {/* 编码切换（防乱码） */}
              <div className="relative flex items-center">
                <Languages className="pointer-events-none absolute left-2.5 h-3.5 w-3.5 text-zinc-400" />
                <select
                  value={encodingOverride}
                  onChange={(e) => handleEncodingChange(e.target.value)}
                  title={`当前识别编码：${archive.detectedEncoding}`}
                  className="cursor-pointer appearance-none rounded-lg border border-zinc-200 bg-white py-1.5 pr-6 pl-7 text-xs text-zinc-700 outline-none hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-300"
                >
                  {ENCODING_OPTIONS.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.value === "auto"
                        ? `自动 (${archive.detectedEncoding})`
                        : opt.label}
                    </option>
                  ))}
                </select>
              </div>

              {selectedPaths.size > 0 && (
                <button
                  type="button"
                  onClick={() => handleExtractToFolder(true)}
                  className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-blue-200 bg-blue-50 px-3 py-1.5 text-xs font-medium text-blue-700 transition hover:bg-blue-100 dark:border-blue-900 dark:bg-blue-950/50 dark:text-blue-300"
                >
                  <FolderOutput className="h-3.5 w-3.5" />
                  解压选中 ({selectedPaths.size})
                </button>
              )}

              <button
                type="button"
                onClick={() => handleExtractToFolder(false)}
                className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-200 bg-white px-3 py-1.5 text-xs font-medium text-zinc-700 transition hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
              >
                <FolderOpen className="h-3.5 w-3.5" />
                解压到...
              </button>

              <button
                type="button"
                onClick={handleQuickSmartExtract}
                className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg bg-blue-600 px-3.5 py-1.5 text-xs font-medium text-white shadow-xs transition hover:bg-blue-700 active:scale-[0.99]"
              >
                <Sparkles className="h-3.5 w-3.5" />
                智能解压
              </button>
            </>
          ) : (
            <>
              <button
                type="button"
                onClick={handlePickArchive}
                className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-200 bg-white px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
              >
                <PackageOpen className="h-3.5 w-3.5" />
                打开
              </button>
              <button
                type="button"
                onClick={() => openCompressModal([])}
                className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700"
              >
                <FolderPlus className="h-3.5 w-3.5" />
                新建压缩
              </button>
            </>
          )}

          <button
            type="button"
            onClick={() => setSettingsOpen(true)}
            title="偏好设置与系统集成"
            className="cursor-pointer rounded-lg p-1.5 text-zinc-500 transition hover:bg-zinc-100 hover:text-zinc-800 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
          >
            <Settings className="h-4 w-4" />
          </button>
        </div>
      </header>

      {/* 自动更新提示条 */}
      <UpdateBanner />

      {/* 主内容区 */}
      <main className="relative flex flex-1 flex-col overflow-hidden">
        {loading ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-3 text-sm text-zinc-500">
            <Loader2 className="h-6 w-6 animate-spin text-blue-600" />
            <span>正在解析压缩包...</span>
          </div>
        ) : archive ? (
          <ArchiveBrowser
            onPreviewFile={(p) => handlePreviewFile(p)}
            onExtractSelected={() => handleExtractToFolder(true)}
            onSmartExtractAll={handleQuickSmartExtract}
            onExtractAllToFolder={() => handleExtractToFolder(false)}
          />
        ) : (
          <DropZoneHome
            isDragging={isDragging}
            onPickArchive={handlePickArchive}
            onStartCompress={() => openCompressModal([])}
          />
        )}

        {/* 拖放覆盖提示（浏览状态下拖入新文件） */}
        {isDragging && archive && (
          <div className="pointer-events-none absolute inset-0 z-30 flex items-center justify-center bg-blue-600/10 backdrop-blur-[2px]">
            <div className="rounded-2xl border-2 border-dashed border-blue-500 bg-white/95 px-8 py-6 text-center shadow-xl dark:bg-zinc-900/95">
              <p className="text-sm font-semibold text-blue-600 dark:text-blue-400">
                松开以打开新压缩包或创建压缩
              </p>
            </div>
          </div>
        )}
      </main>

      {/* 居中实时解压/压缩进度弹窗 */}
      {progress && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <div className="w-full max-w-md rounded-2xl border border-zinc-200 bg-white p-5 shadow-2xl dark:border-zinc-800 dark:bg-zinc-900">
            <div className="mb-3 flex items-center justify-between gap-3">
              <div className="flex items-center gap-2.5">
                <Loader2 className="h-4 w-4 animate-spin text-blue-600" />
                <span className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
                  {progress.kind === "extract" ? "正在解压文件..." : "正在打包压缩..."}
                </span>
              </div>
              <span className="font-mono text-sm font-bold text-blue-600 dark:text-blue-400">
                {Math.round(progress.percent)}%
              </span>
            </div>

            <div className="h-2.5 w-full overflow-hidden rounded-full bg-zinc-100 dark:bg-zinc-800">
              <div
                className="h-full rounded-full bg-gradient-to-r from-blue-600 to-sky-400 transition-all duration-150"
                style={{ width: `${Math.max(progress.percent, 2)}%` }}
              />
            </div>

            <div className="mt-2.5 flex items-center justify-between gap-3 text-[11px] text-zinc-500 dark:text-zinc-400">
              <span className="min-w-0 truncate">
                {progress.currentFile || "正在处理..."}
              </span>
              {progress.totalItems > 0 && (
                <span className="shrink-0 font-mono">
                  {progress.processedItems} / {progress.totalItems} 项
                </span>
              )}
            </div>
          </div>
        </div>
      )}

      {/* 浮动通知 Toast */}
      {toast && !progress && (
        <div className="fixed right-4 bottom-4 z-40 flex max-w-md items-start gap-3 rounded-xl border border-zinc-200 bg-white/95 p-3.5 shadow-lg backdrop-blur-md dark:border-zinc-800 dark:bg-zinc-900/95">
          {toast.type === "success" ? (
            <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-emerald-500" />
          ) : (
            <AlertCircle className="mt-0.5 h-4 w-4 shrink-0 text-red-500" />
          )}
          <div className="min-w-0 flex-1 text-xs">
            <p className="font-medium text-zinc-900 dark:text-zinc-100">
              {toast.title}
            </p>
            {toast.detail && (
              <p className="mt-0.5 break-all text-zinc-500 dark:text-zinc-400">
                {toast.detail}
              </p>
            )}
            {toast.actionPath && (
              <button
                type="button"
                onClick={() => revealItemInDir(toast.actionPath!)}
                className="mt-2 inline-flex cursor-pointer items-center gap-1 font-medium text-blue-600 hover:underline dark:text-blue-400"
              >
                <FolderOpen className="h-3.5 w-3.5" />
                在文件管理器中查看
              </button>
            )}
          </div>
          <button
            type="button"
            onClick={clearToast}
            className="cursor-pointer text-zinc-400 hover:text-zinc-600"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      )}

      {/* 弹窗 */}
      <CompressModal />
      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <PasswordModal
        open={passwordPrompt.open}
        title={passwordPrompt.title}
        onCancel={() => setPasswordPrompt((p) => ({ ...p, open: false }))}
        onConfirm={passwordPrompt.onConfirm}
      />

      {/* 解压目标已存在冲突确认弹窗 */}
      {conflictPrompt?.open && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-4"
          onClick={(e) => {
            if (e.target === e.currentTarget) setConflictPrompt(null);
          }}
        >
          <div className="w-full max-w-md rounded-2xl border border-zinc-200 bg-white p-5 shadow-2xl dark:border-zinc-800 dark:bg-zinc-950">
            <div className="flex items-start gap-3">
              <AlertCircle className="mt-0.5 h-5 w-5 shrink-0 text-amber-500" />
              <div className="min-w-0 flex-1">
                <h3 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
                  目标位置已存在同名文件或文件夹
                </h3>
                <p
                  className="mt-1.5 truncate rounded-lg bg-zinc-100 px-2.5 py-1.5 font-mono text-[11px] text-zinc-600 dark:bg-zinc-900 dark:text-zinc-300"
                  title={conflictPrompt.conflictPath}
                >
                  {conflictPrompt.conflictPath}
                </p>
                <p className="mt-2 text-xs text-zinc-500 dark:text-zinc-400">
                  请选择如何处理冲突：
                </p>
              </div>
            </div>

            <div className="mt-4 flex flex-col gap-2">
              <button
                type="button"
                onClick={conflictPrompt.onRename}
                className="w-full cursor-pointer rounded-xl bg-blue-600 px-3.5 py-2 text-left text-xs font-medium text-white transition hover:bg-blue-700"
              >
                自动新建目录解压（推荐）
                <span className="mt-0.5 block truncate text-[11px] font-normal text-blue-100">
                  解压到：{conflictPrompt.suggestedRenameDir}
                </span>
              </button>

              <div className="grid grid-cols-2 gap-2">
                <button
                  type="button"
                  onClick={conflictPrompt.onOverwrite}
                  className="cursor-pointer rounded-xl border border-zinc-200 bg-white px-3 py-2 text-xs font-medium text-zinc-800 transition hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
                >
                  覆盖已有文件
                </button>
                <button
                  type="button"
                  onClick={conflictPrompt.onSkip}
                  className="cursor-pointer rounded-xl border border-zinc-200 bg-white px-3 py-2 text-xs font-medium text-zinc-800 transition hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
                >
                  跳过已有文件
                </button>
              </div>

              <button
                type="button"
                onClick={() => setConflictPrompt(null)}
                className="mt-0.5 w-full cursor-pointer rounded-xl py-1.5 text-xs text-zinc-500 transition hover:bg-zinc-100 dark:text-zinc-400 dark:hover:bg-zinc-900"
              >
                取消
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
