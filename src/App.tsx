import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import {
  AlertCircle,
  CheckCircle2,
  ChevronDown,
  Folder,
  FolderOpen,
  FolderOutput,
  FolderPlus,
  Languages,
  Loader2,
  PackageOpen,
  Settings,
  Sparkles,
  X,
} from "lucide-react";
import { ArchiveBrowser } from "./components/ArchiveBrowser";
import { CompressModal } from "./components/CompressModal";
import { DropZoneHome } from "./components/DropZoneHome";
import { PasswordModal } from "./components/PasswordModal";
import { SettingsModal } from "./components/SettingsModal";
import { TaskProgressDialog } from "./components/TaskProgressDialog";
import { UpdateBanner } from "./components/UpdateBanner";
import { api, basename, dirname, formatBytes } from "./lib/api";
import { useAppStore } from "./stores/useAppStore";
import type {
  CompressFormat,
  CompressOptions,
  ExtractConflictInfo,
  ExtractMode,
  PendingLaunchPayload,
  TaskProgress,
} from "./types/archive";

const ENCODING_OPTIONS = [
  { value: "auto", label: "编码：自动识别" },
  { value: "utf-8", label: "UTF-8" },
  { value: "gbk", label: "简体中文 (GBK)" },
  { value: "big5", label: "繁体中文 (Big5)" },
  { value: "shift_jis", label: "日文 (Shift-JIS)" },
  { value: "euc-kr", label: "韩文 (EUC-KR)" },
];

type ConflictChoice = "rename" | "overwrite" | "skip" | "cancel";

function createInitialProgress(
  taskId: string,
  kind: "extract" | "compress",
  currentFile = "正在准备..."
): TaskProgress {
  return {
    taskId,
    kind,
    currentFile,
    processedItems: 0,
    totalItems: 0,
    processedBytes: 0,
    totalBytes: 0,
    currentFileBytes: 0,
    currentFileTotalBytes: 0,
    currentFilePercent: 0,
    percent: 0,
    elapsedMs: 0,
    speedBps: 0,
    paused: false,
    done: false,
  };
}

function generateTaskId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `task-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

export default function App() {
  const archive = useAppStore((s) => s.archive);
  const loading = useAppStore((s) => s.loading);
  const encodingOverride = useAppStore((s) => s.encodingOverride);
  const selectedPaths = useAppStore((s) => s.selectedPaths);
  const activeTask = useAppStore((s) => s.activeTask);
  const standaloneMode = useAppStore((s) => s.standaloneMode);
  const toast = useAppStore((s) => s.toast);

  const setEngineStatus = useAppStore((s) => s.setEngineStatus);
  const setSettings = useAppStore((s) => s.setSettings);
  const setStandaloneMode = useAppStore((s) => s.setStandaloneMode);
  const setArchive = useAppStore((s) => s.setArchive);
  const setLoading = useAppStore((s) => s.setLoading);
  const setEncodingOverride = useAppStore((s) => s.setEncodingOverride);
  const startTask = useAppStore((s) => s.startTask);
  const updateTaskProgress = useAppStore((s) => s.updateTaskProgress);
  const patchActiveTask = useAppStore((s) => s.patchActiveTask);
  const clearActiveTask = useAppStore((s) => s.clearActiveTask);
  const showToast = useAppStore((s) => s.showToast);
  const clearToast = useAppStore((s) => s.clearToast);
  const openCompressModal = useAppStore((s) => s.openCompressModal);
  const resetArchiveView = useAppStore((s) => s.resetArchiveView);

  const [isDragging, setIsDragging] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [extractMenuOpen, setExtractMenuOpen] = useState(false);
  const extractMenuRef = useRef<HTMLDivElement>(null);

  // Promise 化密码弹窗
  const [passwordPrompt, setPasswordPrompt] = useState<{
    open: boolean;
    title: string;
    resolve: (pwd: string | null) => void;
  } | null>(null);

  // Promise 化冲突确认弹窗
  const [conflictPrompt, setConflictPrompt] = useState<{
    open: boolean;
    conflictPath: string;
    suggestedRenameDir: string;
    resolve: (choice: ConflictChoice) => void;
  } | null>(null);

  const requestPassword = useCallback((title: string): Promise<string | null> => {
    return new Promise((resolve) => {
      setPasswordPrompt({ open: true, title, resolve });
    });
  }, []);

  const requestConflictChoice = useCallback(
    (conflict: ExtractConflictInfo): Promise<ConflictChoice> => {
      return new Promise((resolve) => {
        setConflictPrompt({
          open: true,
          conflictPath: conflict.conflictPath,
          suggestedRenameDir: conflict.suggestedRenameDir,
          resolve,
        });
      });
    },
    []
  );

  // 点击外部关闭顶栏解压模式下拉菜单
  useEffect(() => {
    if (!extractMenuOpen) return;
    const onOutside = (e: MouseEvent) => {
      if (
        extractMenuRef.current &&
        !extractMenuRef.current.contains(e.target as Node)
      ) {
        setExtractMenuOpen(false);
      }
    };
    window.addEventListener("mousedown", onOutside);
    return () => window.removeEventListener("mousedown", onOutside);
  }, [extractMenuOpen]);

  // 初始化引擎状态与全局禁用浏览器原生右键菜单
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
    return () =>
      window.removeEventListener("contextmenu", blockDefaultContextMenu);
  }, [setEngineStatus]);

  // 监听后端字节级进度事件
  useEffect(() => {
    const unlistenPromise = listen<TaskProgress>("qpack://progress", (event) => {
      updateTaskProgress(event.payload);
    });
    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, [updateTaskProgress]);

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
          setLoading(false);
          const inputPwd = await requestPassword(
            "该压缩包目录已加密，请输入密码以查看内容"
          );
          if (inputPwd) {
            await loadArchive(path, enc, inputPwd);
          }
          return;
        }
        showToast({ type: "error", title: "无法打开压缩包", detail: msg });
      } finally {
        setLoading(false);
      }
    },
    [requestPassword, setArchive, setLoading, showToast]
  );

  // 处理拖入或普通打开的文件路径
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

  // 执行压缩任务（弹出 Bandizip 风格双进度条弹窗，完成后按偏好自动定位并高亮选中压缩包）
  const executeCompressNow = useCallback(
    async (options: CompressOptions, standaloneWindow: boolean) => {
      const cfg = useAppStore.getState().settings;
      const taskId = generateTaskId();
      const outTitle = basename(options.outputPath) || "archive";

      startTask({
        taskId,
        kind: "compress",
        title: outTitle,
        archivePath: options.outputPath,
        destDir: dirname(options.outputPath),
        primaryTarget: options.outputPath,
        primaryTargetIsDir: false,
        standaloneWindow,
        status: "running",
        progress: createInitialProgress(taskId, "compress", "正在扫描待压缩文件..."),
        openFolderAfterDone: cfg.openFolderAfterCompress,
        autoCloseAfterDone: cfg.autoCloseProgressWindow,
        deleteSourceAfterExtract: false,
      });

      try {
        const res = await api.compressFiles({ ...options, taskId });
        const latest = useAppStore.getState().activeTask;
        const shouldReveal =
          latest?.openFolderAfterDone ?? cfg.openFolderAfterCompress;
        const shouldAutoClose =
          latest?.autoCloseAfterDone ?? cfg.autoCloseProgressWindow;

        const savedPct =
          res.originalBytes > 0
            ? Math.max(
                0,
                Math.round((1 - res.archiveBytes / res.originalBytes) * 100)
              )
            : 0;
        const summary = `已打包 ${res.inputFiles} 个文件 · 体积 ${formatBytes(
          res.archiveBytes
        )}${savedPct > 0 ? `（省 ${savedPct}%）` : ""} · 耗时 ${(
          res.elapsedMs / 1000
        ).toFixed(2)}s`;

        patchActiveTask({
          status: "completed",
          archivePath: res.outputPath,
          primaryTarget: res.outputPath,
          primaryTargetIsDir: false,
          inputFiles: res.inputFiles,
          originalBytes: res.originalBytes,
          archiveBytes: res.archiveBytes,
          elapsedMs: res.elapsedMs,
          summary,
          progress: {
            ...(latest?.progress ?? createInitialProgress(taskId, "compress")),
            currentFile: basename(res.outputPath),
            processedItems: res.inputFiles,
            totalItems: Math.max(res.inputFiles, 1),
            processedBytes: res.originalBytes,
            totalBytes: res.originalBytes,
            currentFilePercent: 100,
            percent: 100,
            elapsedMs: res.elapsedMs,
            paused: false,
            done: true,
          },
        });

        // 压缩完成后按用户偏好自动在文件管理器中打开并高亮选中压缩包
        if (shouldReveal) {
          api.revealInFileManager(res.outputPath).catch(() => {});
        }

        if (shouldAutoClose) {
          await new Promise((r) => setTimeout(r, 380));
          if (standaloneWindow) {
            await getCurrentWindow().close().catch(() => {});
          } else {
            clearActiveTask();
            showToast({
              type: "success",
              title: `压缩完成：${basename(res.outputPath)}`,
              detail: summary,
              actionPath: res.outputPath,
              actionReveal: true,
            });
          }
        }
      } catch (err) {
        const msg = String(err);
        if (msg.includes("取消")) {
          patchActiveTask({ status: "cancelled" });
        } else {
          patchActiveTask({ status: "error", errorMessage: msg });
        }
      }
    },
    [clearActiveTask, patchActiveTask, showToast, startTask]
  );

  // 执行单个压缩包解压（支持冲突检测、密码重试、回收站清理、智能定位与自动关闭）
  const runExtractOne = useCallback(
    async (params: {
      archivePath: string;
      destDir: string;
      mode: ExtractMode;
      selectedPaths?: string[];
      standaloneWindow: boolean;
      isLastInBatch?: boolean;
      initialPassword?: string;
    }): Promise<"completed" | "cancelled" | "error"> => {
      const {
        archivePath,
        selectedPaths: selPaths,
        standaloneWindow,
        isLastInBatch = true,
      } = params;

      let destDir = params.destDir;
      let mode: ExtractMode = params.mode;
      let overwrite = true;
      let password = params.initialPassword;
      const enc = useAppStore.getState().encodingOverride;

      // 1. 冲突检测（加密包会在下一步触发密码输入）
      try {
        const conflict = await api.checkExtractConflict({
          archivePath,
          destDir,
          smartExtract: mode === "smart",
          extractMode: mode,
          password,
          encoding: enc,
          selectedPaths: selPaths,
          overwrite: true,
        });

        if (conflict.hasConflict) {
          const choice = await requestConflictChoice(conflict);
          if (choice === "cancel") {
            return "cancelled";
          }
          if (choice === "rename") {
            destDir = conflict.suggestedRenameDir;
            mode = "here";
            overwrite = true;
          } else if (choice === "overwrite") {
            overwrite = true;
          } else if (choice === "skip") {
            overwrite = false;
          }
        }
      } catch {
        // 例如 7z 文件头加密时 conflict check 会抛密码错，直接进入解压流程弹密码框
      }

      // 2. 循环执行解压（若需要密码则提示输入并重试）
      while (true) {
        const cfg = useAppStore.getState().settings;
        const taskId = generateTaskId();
        const title = basename(archivePath) || "archive";

        startTask({
          taskId,
          kind: "extract",
          title,
          archivePath,
          destDir,
          standaloneWindow,
          status: "running",
          progress: createInitialProgress(taskId, "extract", "正在读取压缩包..."),
          openFolderAfterDone: cfg.openFolderAfterExtract,
          autoCloseAfterDone: cfg.autoCloseProgressWindow,
          deleteSourceAfterExtract: false,
        });

        try {
          const res = await api.extractArchive({
            archivePath,
            destDir,
            smartExtract: mode === "smart",
            extractMode: mode,
            password,
            encoding: enc,
            selectedPaths: selPaths,
            overwrite,
            taskId,
          });

          const latest = useAppStore.getState().activeTask;
          const shouldOpen =
            latest?.openFolderAfterDone ?? cfg.openFolderAfterExtract;
          const shouldAutoClose =
            latest?.autoCloseAfterDone ?? cfg.autoCloseProgressWindow;
          const shouldTrash = latest?.deleteSourceAfterExtract ?? false;

          // 解压成功后若用户勾选“删除原压缩包”，移入系统回收站/废纸篓
          if (shouldTrash) {
            await api.trashPaths([archivePath]).catch(() => {});
          }

          const targetPath = res.primaryTarget || res.outputDir;
          const targetIsDir = res.primaryTarget
            ? Boolean(res.primaryTargetIsDir)
            : true;

          const summary = `已解压 ${res.extractedFiles} 个文件${
            res.totalBytes > 0 ? ` · ${formatBytes(res.totalBytes)}` : ""
          } · 耗时 ${(res.elapsedMs / 1000).toFixed(2)}s`;

          patchActiveTask({
            status: "completed",
            destDir: res.outputDir,
            primaryTarget: targetPath,
            primaryTargetIsDir: targetIsDir,
            extractedFiles: res.extractedFiles,
            elapsedMs: res.elapsedMs,
            summary,
            progress: {
              ...(latest?.progress ?? createInitialProgress(taskId, "extract")),
              currentFile:
                latest?.progress.currentFile &&
                latest.progress.currentFile !== "正在读取压缩包..."
                  ? latest.progress.currentFile
                  : title,
              processedItems: Math.max(
                res.extractedFiles,
                latest?.progress.totalItems ?? 0
              ),
              totalItems: Math.max(
                res.extractedFiles,
                latest?.progress.totalItems ?? 1
              ),
              processedBytes: Math.max(
                res.totalBytes,
                latest?.progress.totalBytes ?? 0
              ),
              totalBytes: Math.max(
                res.totalBytes,
                latest?.progress.totalBytes ?? 0
              ),
              currentFilePercent: 100,
              percent: 100,
              elapsedMs: res.elapsedMs,
              paused: false,
              done: true,
            },
          });

          // 批量解压仅在最后一个完成后定位，避免连弹多个资源管理器窗口
          if (shouldOpen && isLastInBatch) {
            if (targetIsDir) {
              api.openOutputDirectory(targetPath).catch(() => {});
            } else {
              api.revealInFileManager(targetPath).catch(() => {});
            }
          }

          if (shouldAutoClose && isLastInBatch) {
            await new Promise((r) => setTimeout(r, 380));
            if (standaloneWindow) {
              await getCurrentWindow().close().catch(() => {});
            } else {
              clearActiveTask();
              showToast({
                type: "success",
                title: `解压完成（共 ${res.extractedFiles} 个文件）`,
                detail: `已解压至 ${targetPath}`,
                actionPath: targetPath,
                actionReveal: !targetIsDir,
              });
            }
          }

          return "completed";
        } catch (err) {
          const msg = String(err);
          if (msg.includes("密码")) {
            clearActiveTask();
            const inputPwd = await requestPassword(
              password
                ? `密码错误，请重新输入 ${title} 的解压密码`
                : `${title} 已加密，请输入解压密码`
            );
            if (!inputPwd) {
              return "cancelled";
            }
            password = inputPwd;
            continue;
          }
          if (msg.includes("取消")) {
            patchActiveTask({ status: "cancelled" });
            return "cancelled";
          }
          patchActiveTask({ status: "error", errorMessage: msg });
          return "error";
        }
      }
    },
    [
      clearActiveTask,
      patchActiveTask,
      requestConflictChoice,
      requestPassword,
      showToast,
      startTask,
    ]
  );

  // 快捷压缩（右键菜单 / 访达快速操作触发）
  const runQuickCompress = useCallback(
    async (
      paths: string[],
      format: CompressFormat,
      standaloneWindow: boolean
    ) => {
      if (paths.length === 0) return;
      const cfg = useAppStore.getState().settings;
      const outPath = await api.suggestCompressOutput(paths, format);
      await executeCompressNow(
        {
          inputPaths: paths,
          outputPath: outPath,
          format,
          level: 6,
          excludeJunk: cfg.excludeJunk,
        },
        standaloneWindow
      );
    },
    [executeCompressNow]
  );

  // 统一处理冷启动 / 热唤起任务载荷
  const handleLaunchPayload = useCallback(
    async (payload: PendingLaunchPayload) => {
      if (!payload || !payload.paths || payload.paths.length === 0) return;

      const isStandalone = Boolean(payload.standalone);
      if (isStandalone) {
        setStandaloneMode(true);
      }

      switch (payload.mode) {
        case "compress-dialog": {
          openCompressModal(payload.paths);
          return;
        }
        case "compress-zip": {
          await runQuickCompress(payload.paths, "zip", isStandalone);
          return;
        }
        case "compress-7z": {
          await runQuickCompress(payload.paths, "seven-z", isStandalone);
          return;
        }
        case "extract-smart":
        case "extract-here":
        case "extract-subfolder":
        case "extract-to": {
          const validArchives: string[] = [];
          for (const p of payload.paths) {
            const ok = await api.isSupportedArchive(p).catch(() => false);
            if (ok) validArchives.push(p);
          }

          if (validArchives.length === 0) {
            if (isStandalone) {
              await api.switchToMainWindow().catch(() => {});
              setStandaloneMode(false);
            }
            await handleDroppedPaths(payload.paths);
            return;
          }

          let fixedDest: string | null = null;
          let mode: ExtractMode =
            payload.mode === "extract-here"
              ? "here"
              : payload.mode === "extract-subfolder"
              ? "subfolder"
              : "smart";

          if (payload.mode === "extract-to") {
            const picked = await open({
              directory: true,
              multiple: false,
              defaultPath: dirname(validArchives[0]),
              title: "选择解压目标文件夹",
            });
            if (typeof picked !== "string") {
              if (isStandalone) {
                await getCurrentWindow().close().catch(() => {});
              }
              return;
            }
            fixedDest = picked;
            mode = "smart";
          }

          for (let i = 0; i < validArchives.length; i++) {
            const arc = validArchives[i];
            const isLast = i === validArchives.length - 1;
            const status = await runExtractOne({
              archivePath: arc,
              destDir: fixedDest || dirname(arc),
              mode,
              standaloneWindow: isStandalone,
              isLastInBatch: isLast,
            });
            if (status !== "completed") {
              if (status === "cancelled" && isStandalone && !useAppStore.getState().activeTask) {
                await getCurrentWindow().close().catch(() => {});
              }
              break;
            }
            if (!isLast) {
              await new Promise((r) => setTimeout(r, 150));
            }
          }
          return;
        }
        case "open":
        default: {
          await handleDroppedPaths(payload.paths);
        }
      }
    },
    [
      handleDroppedPaths,
      openCompressModal,
      runExtractOne,
      runQuickCompress,
      setStandaloneMode,
    ]
  );

  // 注册 Tauri 原生拖放监听
  useEffect(() => {
    const unlistenPromise = getCurrentWebview().onDragDropEvent((event) => {
      const state = useAppStore.getState();
      if (state.standaloneMode) return;
      if (event.payload.type === "over" || event.payload.type === "enter") {
        setIsDragging(true);
      } else if (event.payload.type === "drop") {
        setIsDragging(false);
        // 新建压缩弹窗已打开时，任何拖入的文件（含 .dmg / .zip 等）均直接加入待压缩列表
        if (state.compressModalOpen) {
          const merged = Array.from(
            new Set([...state.compressInputs, ...event.payload.paths])
          );
          state.setCompressInputs(merged);
        } else {
          handleDroppedPaths(event.payload.paths);
        }
      } else {
        setIsDragging(false);
      }
    });
    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, [handleDroppedPaths]);

  // 启动时加载用户设置 -> 拉取冷启动任务 -> 监听热唤起事件
  useEffect(() => {
    let active = true;
    (async () => {
      try {
        const cfg = await api.getAppSettings();
        if (active) setSettings(cfg);
      } catch {
        // ignore
      }
      try {
        const payload = await api.takePendingLaunchPaths();
        if (active && payload?.paths?.length) {
          await handleLaunchPayload(payload);
        }
      } catch {
        // ignore
      }
    })();

    const unlistenLaunch = listen<PendingLaunchPayload>(
      "qpack://launch",
      (event) => {
        if (event.payload?.paths?.length) {
          handleLaunchPayload(event.payload);
        }
      }
    );
    const unlistenCompress = listen("qpack://trigger-compress", () => {
      openCompressModal([]);
    });

    return () => {
      active = false;
      unlistenLaunch.then((fn) => fn());
      unlistenCompress.then((fn) => fn());
    };
  }, [handleLaunchPayload, openCompressModal, setSettings]);

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
            "wim",
            "rpm",
            "deb",
            "lzh",
            "arj",
            "cpio",
          ],
        },
      ],
    });
    if (typeof picked === "string") {
      setEncodingOverride("auto");
      await loadArchive(picked, "auto");
    }
  };

  // 主界面中触发各类解压
  const handleExtractFromBrowser = async (mode: ExtractMode) => {
    if (!archive) return;
    setExtractMenuOpen(false);
    await runExtractOne({
      archivePath: archive.path,
      destDir: dirname(archive.path),
      mode,
      standaloneWindow: false,
    });
  };

  const handleExtractToFolder = async (onlySelected: boolean) => {
    if (!archive) return;
    setExtractMenuOpen(false);
    const dest = await open({
      directory: true,
      multiple: false,
      defaultPath: dirname(archive.path),
      title: onlySelected ? "选择选中文件的解压目录" : "选择解压目标目录",
    });
    if (typeof dest === "string") {
      await runExtractOne({
        archivePath: archive.path,
        destDir: dest,
        mode: onlySelected ? "here" : "smart",
        selectedPaths: onlySelected ? Array.from(selectedPaths) : undefined,
        standaloneWindow: false,
      });
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
        const inputPwd = await requestPassword("文件已加密，请输入密码以预览");
        if (inputPwd) {
          await handlePreviewFile(entryPath, inputPwd);
        }
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

  // 进度窗口关闭 / 切换到主窗口预览 / 打开设置
  const handleCloseProgressDialog = async () => {
    const task = useAppStore.getState().activeTask;
    if (task?.standaloneWindow) {
      await getCurrentWindow().close().catch(() => {});
    } else {
      clearActiveTask();
    }
  };

  const handleBrowseArchiveFromProgress = async (archivePath: string) => {
    const wasStandalone = useAppStore.getState().standaloneMode;
    clearActiveTask();
    if (wasStandalone) {
      await api.switchToMainWindow().catch(() => {});
      setStandaloneMode(false);
    }
    setEncodingOverride("auto");
    await loadArchive(archivePath, "auto");
  };

  const handleOpenSettingsFromProgress = async () => {
    const wasStandalone = useAppStore.getState().standaloneMode;
    if (wasStandalone) {
      clearActiveTask();
      await api.switchToMainWindow().catch(() => {});
      setStandaloneMode(false);
    }
    setSettingsOpen(true);
  };

  // 渲染通用辅助弹窗（密码框、冲突确认框）
  const renderSharedModals = () => (
    <>
      <PasswordModal
        open={Boolean(passwordPrompt?.open)}
        title={passwordPrompt?.title ?? ""}
        onCancel={() => {
          const resolve = passwordPrompt?.resolve;
          setPasswordPrompt(null);
          resolve?.(null);
        }}
        onConfirm={(pwd) => {
          const resolve = passwordPrompt?.resolve;
          setPasswordPrompt(null);
          resolve?.(pwd);
        }}
      />

      {conflictPrompt?.open && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-4"
          onClick={(e) => {
            if (e.target === e.currentTarget) {
              const resolve = conflictPrompt.resolve;
              setConflictPrompt(null);
              resolve("cancel");
            }
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
                onClick={() => {
                  const resolve = conflictPrompt.resolve;
                  setConflictPrompt(null);
                  resolve("rename");
                }}
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
                  onClick={() => {
                    const resolve = conflictPrompt.resolve;
                    setConflictPrompt(null);
                    resolve("overwrite");
                  }}
                  className="cursor-pointer rounded-xl border border-zinc-200 bg-white px-3 py-2 text-xs font-medium text-zinc-800 transition hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
                >
                  覆盖已有文件
                </button>
                <button
                  type="button"
                  onClick={() => {
                    const resolve = conflictPrompt.resolve;
                    setConflictPrompt(null);
                    resolve("skip");
                  }}
                  className="cursor-pointer rounded-xl border border-zinc-200 bg-white px-3 py-2 text-xs font-medium text-zinc-800 transition hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
                >
                  跳过已有文件
                </button>
              </div>

              <button
                type="button"
                onClick={() => {
                  const resolve = conflictPrompt.resolve;
                  setConflictPrompt(null);
                  resolve("cancel");
                }}
                className="mt-0.5 w-full cursor-pointer rounded-xl py-1.5 text-xs text-zinc-500 transition hover:bg-zinc-100 dark:text-zinc-400 dark:hover:bg-zinc-900"
              >
                取消
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );

  // 独立紧凑任务窗模式（右键菜单 / 双击智能解压）
  if (standaloneMode) {
    return (
      <div className="h-screen w-screen overflow-hidden bg-[#f5f6f7] text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
        {activeTask ? (
          <TaskProgressDialog
            onClose={handleCloseProgressDialog}
            onBrowseArchive={handleBrowseArchiveFromProgress}
            onOpenSettings={handleOpenSettingsFromProgress}
          />
        ) : (
          <div className="flex h-full w-full flex-col items-center justify-center gap-3 text-xs text-zinc-500 dark:text-zinc-400">
            <img src="/logo.png" alt="QPack" className="h-10 w-10 object-contain" />
            <div className="flex items-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin text-blue-600" />
              <span>正在准备任务...</span>
            </div>
          </div>
        )}
        {renderSharedModals()}
      </div>
    );
  }

  // 完整主窗口模式
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
                onClick={() => openCompressModal([archive.path])}
                title="将该压缩包/DMG 文件重新打包压缩为 ZIP / 7Z 等格式"
                className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-200 bg-white px-2.5 py-1.5 text-xs font-medium text-zinc-700 transition hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
              >
                <FolderPlus className="h-3.5 w-3.5" />
                打包此文件
              </button>

              <button
                type="button"
                onClick={() => handleExtractToFolder(false)}
                className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-200 bg-white px-3 py-1.5 text-xs font-medium text-zinc-700 transition hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
              >
                <FolderOpen className="h-3.5 w-3.5" />
                解压到...
              </button>

              {/* 分裂式智能解压按钮（支持下拉切换解压到此处/同名文件夹） */}
              <div className="relative flex items-center" ref={extractMenuRef}>
                <button
                  type="button"
                  onClick={() => handleExtractFromBrowser("smart")}
                  className="inline-flex cursor-pointer items-center gap-1.5 rounded-l-lg bg-blue-600 py-1.5 pr-2.5 pl-3.5 text-xs font-medium text-white shadow-xs transition hover:bg-blue-700 active:scale-[0.99]"
                >
                  <Sparkles className="h-3.5 w-3.5" />
                  智能解压
                </button>
                <button
                  type="button"
                  onClick={() => setExtractMenuOpen((v) => !v)}
                  title="更多解压方式"
                  className="inline-flex cursor-pointer items-center rounded-r-lg border-l border-blue-500 bg-blue-600 px-1.5 py-1.5 text-white shadow-xs transition hover:bg-blue-700"
                >
                  <ChevronDown className="h-3.5 w-3.5" />
                </button>

                {extractMenuOpen && (
                  <div className="absolute top-10 right-0 z-40 w-52 overflow-hidden rounded-xl border border-zinc-200 bg-white py-1 text-xs text-zinc-700 shadow-xl dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200">
                    <button
                      type="button"
                      onClick={() => handleExtractFromBrowser("smart")}
                      className="flex w-full cursor-pointer items-center gap-2 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
                    >
                      <Sparkles className="h-3.5 w-3.5 text-blue-500" />
                      <span>智能解压到同级目录</span>
                    </button>
                    <button
                      type="button"
                      onClick={() => handleExtractFromBrowser("here")}
                      className="flex w-full cursor-pointer items-center gap-2 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
                    >
                      <FolderOpen className="h-3.5 w-3.5 text-zinc-500" />
                      <span>直接解压到同级目录</span>
                    </button>
                    <button
                      type="button"
                      onClick={() => handleExtractFromBrowser("subfolder")}
                      className="flex w-full cursor-pointer items-center gap-2 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
                    >
                      <Folder className="h-3.5 w-3.5 text-zinc-500" />
                      <span className="truncate">
                        解压到 {archive.suggestedFolderName}/
                      </span>
                    </button>
                    <div className="my-1 border-t border-zinc-100 dark:border-zinc-800" />
                    <button
                      type="button"
                      onClick={() => handleExtractToFolder(false)}
                      className="flex w-full cursor-pointer items-center gap-2 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
                    >
                      <FolderOutput className="h-3.5 w-3.5 text-zinc-500" />
                      <span>解压到其他文件夹...</span>
                    </button>
                  </div>
                )}
              </div>
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
            onSmartExtractAll={() => handleExtractFromBrowser("smart")}
            onExtractHereAll={() => handleExtractFromBrowser("here")}
            onExtractSubfolderAll={() => handleExtractFromBrowser("subfolder")}
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

      {/* Bandizip 风格双进度条任务窗口 */}
      {activeTask && (
        <TaskProgressDialog
          onClose={handleCloseProgressDialog}
          onBrowseArchive={handleBrowseArchiveFromProgress}
          onOpenSettings={handleOpenSettingsFromProgress}
        />
      )}

      {/* 浮动通知 Toast */}
      {toast && !activeTask && (
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
                onClick={() => {
                  if (toast.actionReveal) {
                    api.revealInFileManager(toast.actionPath!).catch(() => {});
                  } else {
                    api.openOutputDirectory(toast.actionPath!).catch(() => {});
                  }
                }}
                className="mt-2 inline-flex cursor-pointer items-center gap-1 font-medium text-blue-600 hover:underline dark:text-blue-400"
              >
                <FolderOpen className="h-3.5 w-3.5" />
                {toast.actionReveal ? "在文件管理器中定位" : "打开文件夹"}
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
      <CompressModal
        onStartCompress={(opts) => executeCompressNow(opts, false)}
      />
      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />
      {renderSharedModals()}
    </div>
  );
}
