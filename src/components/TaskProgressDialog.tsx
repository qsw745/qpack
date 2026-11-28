import { useEffect, useRef, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Archive,
  Check,
  FolderOpen,
  FolderSearch,
  Menu,
  Pause,
  Pin,
  Play,
  Settings,
  Trash2,
  X,
  AppWindow,
} from "lucide-react";
import { api, formatBytes, formatDuration, formatSpeed } from "../lib/api";
import { useAppStore } from "../stores/useAppStore";

interface TaskProgressDialogProps {
  onClose: () => void;
  onBrowseArchive: (archivePath: string) => void;
  onOpenSettings: () => void;
}

interface IconToggleButtonProps {
  title: string;
  active?: boolean;
  accent?: "emerald" | "red" | "amber" | "blue";
  onClick: () => void;
  disabled?: boolean;
  children: ReactNode;
}

/** Bandizip 风格底部快捷图标按钮（支持激活勾选角标） */
function IconToggleButton({
  title,
  active,
  accent = "emerald",
  onClick,
  disabled,
  children,
}: IconToggleButtonProps) {
  const badgeColor =
    accent === "red"
      ? "bg-red-500"
      : accent === "amber"
      ? "bg-amber-500"
      : accent === "blue"
      ? "bg-blue-600"
      : "bg-emerald-600";

  return (
    <button
      type="button"
      title={title}
      disabled={disabled}
      onClick={onClick}
      className={`relative flex h-8 w-8 cursor-pointer items-center justify-center rounded-lg border transition disabled:cursor-not-allowed disabled:opacity-40 ${
        active
          ? "border-emerald-300 bg-emerald-50/90 text-emerald-700 dark:border-emerald-800 dark:bg-emerald-950/60 dark:text-emerald-300"
          : "border-transparent text-zinc-600 hover:border-zinc-200 hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:border-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      }`}
    >
      {children}
      {active && (
        <span
          className={`absolute -right-0.5 -bottom-0.5 flex h-3.5 w-3.5 items-center justify-center rounded-full text-white shadow-xs ${badgeColor}`}
        >
          <Check className="h-2.5 w-2.5 stroke-[3]" />
        </span>
      )}
    </button>
  );
}

export function TaskProgressDialog({
  onClose,
  onBrowseArchive,
  onOpenSettings,
}: TaskProgressDialogProps) {
  const task = useAppStore((s) => s.activeTask);
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);
  const patchActiveTask = useAppStore((s) => s.patchActiveTask);

  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [fileElapsedMs, setFileElapsedMs] = useState(0);
  const fileStartRef = useRef<{ name: string; startedAt: number }>({
    name: "",
    startedAt: Date.now(),
  });
  const menuRef = useRef<HTMLDivElement>(null);

  // 点击外部关闭更多选项菜单
  useEffect(() => {
    if (!menuOpen) return;
    const handleOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    window.addEventListener("mousedown", handleOutside);
    return () => window.removeEventListener("mousedown", handleOutside);
  }, [menuOpen]);

  // 追踪当前单文件耗时
  useEffect(() => {
    if (!task) return;
    const curName = task.progress.currentFile;
    if (curName !== fileStartRef.current.name) {
      fileStartRef.current = { name: curName, startedAt: Date.now() };
      setFileElapsedMs(0);
    }
    if (task.status !== "running") return;
    const timer = setInterval(() => {
      setFileElapsedMs(Date.now() - fileStartRef.current.startedAt);
    }, 250);
    return () => clearInterval(timer);
  }, [task?.progress.currentFile, task?.status]);

  // 独立模式下实时同步操作系统窗口标题（仿 Bandizip：100% 正在压缩 xxx.zip - QPack）
  useEffect(() => {
    if (!task || !task.standaloneWindow) return;
    const pct = Math.round(
      task.status === "completed" ? 100 : task.progress.percent
    );
    let verb = task.kind === "compress" ? "正在压缩" : "正在解压";
    if (task.status === "paused") verb = "已暂停";
    else if (task.status === "completed")
      verb = task.kind === "compress" ? "已压缩" : "已解压";
    else if (task.status === "cancelled") verb = "已取消";
    else if (task.status === "error") verb = "操作失败";

    const winTitle = `${pct}% ${verb} ${task.title} - QPack`;
    getCurrentWindow()
      .setTitle(winTitle)
      .catch(() => {});
  }, [
    task?.standaloneWindow,
    task?.status,
    task?.kind,
    task?.title,
    task?.progress.percent,
  ]);

  // Esc / Enter 快捷键：任务完成后按 Esc 或 Enter 关闭
  useEffect(() => {
    if (!task) return;
    const isFinished =
      task.status === "completed" ||
      task.status === "cancelled" ||
      task.status === "error";
    if (!isFinished) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" || e.key === "Enter") {
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [task?.status, onClose]);

  if (!task) return null;

  const isRunning = task.status === "running";
  const isPaused = task.status === "paused";
  const isCompleted = task.status === "completed";
  const isCancelled = task.status === "cancelled";
  const isError = task.status === "error";
  const isActive = isRunning || isPaused;

  const totalPct = isCompleted
    ? 100
    : Math.max(0, Math.min(100, Math.round(task.progress.percent)));
  const filePct = isCompleted
    ? 100
    : Math.max(
        0,
        Math.min(
          100,
          Math.round(
            task.progress.currentFileTotalBytes > 0
              ? task.progress.currentFilePercent
              : task.progress.percent
          )
        )
      );

  // 估算剩余时间
  const totalElapsedMs = task.elapsedMs ?? task.progress.elapsedMs ?? 0;
  const totalRemainingMs =
    isCompleted || totalPct >= 100
      ? 0
      : task.progress.percent > 1 && totalElapsedMs > 150
      ? (totalElapsedMs * (100 - task.progress.percent)) / task.progress.percent
      : 0;

  const fileRemainingMs =
    isCompleted || filePct >= 100
      ? 0
      : task.progress.currentFilePercent > 1 && fileElapsedMs > 150
      ? (fileElapsedMs * (100 - task.progress.currentFilePercent)) /
        task.progress.currentFilePercent
      : 0;

  const barColorClass = isError
    ? "bg-red-500"
    : isCancelled
    ? "bg-zinc-400 dark:bg-zinc-600"
    : isPaused
    ? "bg-amber-500"
    : "bg-[#198019] dark:bg-emerald-600";

  const handleTogglePause = async () => {
    if (isRunning) {
      await api.pauseTask(task.taskId).catch(() => {});
      patchActiveTask({
        status: "paused",
        progress: { ...task.progress, paused: true },
      });
    } else if (isPaused) {
      await api.resumeTask(task.taskId).catch(() => {});
      patchActiveTask({
        status: "running",
        progress: { ...task.progress, paused: false },
      });
    }
  };

  const handleCancel = async () => {
    await api.cancelTask(task.taskId).catch(() => {});
  };

  const handleToggleAlwaysOnTop = async () => {
    const next = !alwaysOnTop;
    setAlwaysOnTop(next);
    setMenuOpen(false);
    await getCurrentWindow()
      .setAlwaysOnTop(next)
      .catch(() => {});
  };

  const handleToggleOpenFolder = () => {
    const next = !task.openFolderAfterDone;
    patchActiveTask({ openFolderAfterDone: next });
    const nextSettings =
      task.kind === "extract"
        ? { ...settings, openFolderAfterExtract: next }
        : { ...settings, openFolderAfterCompress: next };
    setSettings(nextSettings);
    api.updateAppSettings(nextSettings).catch(() => {});
  };

  const handleToggleAutoClose = () => {
    const next = !task.autoCloseAfterDone;
    patchActiveTask({ autoCloseAfterDone: next });
    const nextSettings = { ...settings, autoCloseProgressWindow: next };
    setSettings(nextSettings);
    api.updateAppSettings(nextSettings).catch(() => {});
  };

  const handleToggleDeleteSource = () => {
    patchActiveTask({
      deleteSourceAfterExtract: !task.deleteSourceAfterExtract,
    });
  };

  const handleRevealOutput = () => {
    if (task.kind === "compress") {
      api.revealInFileManager(task.archivePath).catch(() => {});
    } else {
      const target = task.primaryTarget || task.destDir;
      if (task.primaryTarget && !task.primaryTargetIsDir) {
        api.revealInFileManager(target).catch(() => {});
      } else {
        api.openOutputDirectory(target).catch(() => {});
      }
    }
  };

  const handleRevealArchive = () => {
    if (task.archivePath) {
      api.revealInFileManager(task.archivePath).catch(() => {});
    }
  };

  const headerTitle = `${totalPct}% ${
    isCompleted
      ? task.kind === "compress"
        ? "已压缩"
        : "已解压"
      : isPaused
      ? "已暂停"
      : isCancelled
      ? "已取消"
      : isError
      ? "操作失败"
      : task.kind === "compress"
      ? "正在压缩"
      : "正在解压"
  } ${task.title}`;

  const body = (
    <div
      className={`flex flex-col bg-[#f5f6f7] text-zinc-900 select-none dark:bg-zinc-950 dark:text-zinc-100 ${
        task.standaloneWindow
          ? "h-screen w-screen"
          : "w-full max-w-[530px] overflow-hidden rounded-xl border border-zinc-300/90 shadow-2xl dark:border-zinc-800"
      }`}
    >
      {/* 非独立窗口模式时显示模拟窗口标题栏 */}
      {!task.standaloneWindow && (
        <div className="flex items-center justify-between border-b border-zinc-200 bg-white px-3.5 py-2.5 dark:border-zinc-800 dark:bg-zinc-900">
          <div className="flex min-w-0 items-center gap-2">
            <img src="/logo.png" alt="QPack" className="h-4 w-4 shrink-0" />
            <span className="truncate text-xs font-medium text-zinc-800 dark:text-zinc-100">
              {headerTitle}
            </span>
          </div>
          <button
            type="button"
            onClick={isActive ? handleCancel : onClose}
            title={isActive ? "取消任务" : "关闭"}
            className="cursor-pointer rounded p-0.5 text-zinc-400 hover:bg-zinc-100 hover:text-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      )}

      {/* 核心内容区：双进度条 + 当前文件 + 状态盒 */}
      <div className="flex flex-1 flex-col gap-3 px-4 pt-3.5 pb-3">
        {/* 进度条 1：当前文件进度 */}
        <div>
          <div className="mb-1 flex items-center justify-between text-xs tabular-nums text-zinc-700 dark:text-zinc-300">
            <span className="font-medium">{filePct}%</span>
            <span className="font-mono text-[11px] text-zinc-500 dark:text-zinc-400">
              {formatDuration(isCompleted ? totalElapsedMs : fileElapsedMs)} /{" "}
              {formatDuration(fileRemainingMs)}
            </span>
          </div>
          <div className="h-5 w-full overflow-hidden rounded-[3px] border border-zinc-300/90 bg-zinc-200/80 dark:border-zinc-700 dark:bg-zinc-800">
            <div
              className={`h-full transition-all duration-150 ${barColorClass}`}
              style={{ width: `${filePct}%` }}
            />
          </div>
        </div>

        {/* 进度条 2：总进度 */}
        <div>
          <div className="mb-1 flex items-center justify-between text-xs tabular-nums text-zinc-700 dark:text-zinc-300">
            <span className="font-medium">{totalPct}%</span>
            <span className="font-mono text-[11px] text-zinc-500 dark:text-zinc-400">
              {formatDuration(totalElapsedMs)} / {formatDuration(totalRemainingMs)}
            </span>
          </div>
          <div className="h-5 w-full overflow-hidden rounded-[3px] border border-zinc-300/90 bg-zinc-200/80 dark:border-zinc-700 dark:bg-zinc-800">
            <div
              className={`h-full transition-all duration-150 ${barColorClass}`}
              style={{ width: `${totalPct}%` }}
            />
          </div>
        </div>

        {/* 当前处理文件名 */}
        <div
          className="h-4 truncate text-xs text-zinc-700 dark:text-zinc-300"
          title={task.progress.currentFile || task.archivePath}
        >
          {task.progress.currentFile ||
            (isCompleted ? task.title : "正在准备...")}
        </div>

        {/* 中间带边框状态信息盒（仿 Bandizip 状态面板） */}
        <div className="flex min-h-[76px] flex-1 flex-col items-center justify-center rounded-[4px] border border-zinc-300/80 bg-white/80 px-4 py-2.5 text-center dark:border-zinc-800 dark:bg-zinc-900/70">
          {isCompleted ? (
            <>
              <p className="text-[13px] font-medium text-zinc-800 dark:text-zinc-100">
                {task.kind === "compress"
                  ? "压缩操作已成功完成。"
                  : "解压操作已成功完成。"}
              </p>
              {task.summary && (
                <p className="mt-1 text-[11px] text-zinc-500 dark:text-zinc-400">
                  {task.summary}
                </p>
              )}
            </>
          ) : isPaused ? (
            <>
              <p className="text-[13px] font-medium text-amber-600 dark:text-amber-400">
                操作已暂停
              </p>
              <p className="mt-1 text-[11px] text-zinc-500 dark:text-zinc-400">
                已完成 {formatBytes(task.progress.processedBytes)}
                {task.progress.totalBytes > 0
                  ? ` / ${formatBytes(task.progress.totalBytes)}`
                  : ""}
                {" · "}点击右下角「继续」恢复
              </p>
            </>
          ) : isCancelled ? (
            <p className="text-[13px] font-medium text-zinc-600 dark:text-zinc-300">
              操作已取消，已自动清理未完成的临时文件。
            </p>
          ) : isError ? (
            <>
              <p className="text-[13px] font-medium text-red-600 dark:text-red-400">
                {task.kind === "compress" ? "压缩失败" : "解压失败"}
              </p>
              {task.errorMessage && (
                <p className="mt-1 line-clamp-2 text-[11px] break-all text-red-500/90 dark:text-red-400/80">
                  {task.errorMessage}
                </p>
              )}
            </>
          ) : (
            <div className="grid w-full grid-cols-3 gap-2 text-xs">
              <div className="flex flex-col items-center">
                <span className="text-[10px] text-zinc-400">处理速度</span>
                <span className="mt-0.5 font-mono font-medium text-zinc-800 dark:text-zinc-200">
                  {formatSpeed(task.progress.speedBps)}
                </span>
              </div>
              <div className="flex flex-col items-center border-x border-zinc-200/80 dark:border-zinc-800">
                <span className="text-[10px] text-zinc-400">已处理大小</span>
                <span className="mt-0.5 font-mono font-medium text-zinc-800 dark:text-zinc-200">
                  {formatBytes(task.progress.processedBytes)}
                  {task.progress.totalBytes > 0
                    ? ` / ${formatBytes(task.progress.totalBytes)}`
                    : ""}
                </span>
              </div>
              <div className="flex flex-col items-center">
                <span className="text-[10px] text-zinc-400">文件数量</span>
                <span className="mt-0.5 font-mono font-medium text-zinc-800 dark:text-zinc-200">
                  {task.progress.processedItems} /{" "}
                  {Math.max(task.progress.totalItems, task.progress.processedItems)}
                </span>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* 底部工具栏（仿 Bandizip 图标按钮组 + 右侧操作按钮） */}
      <div className="flex items-center justify-between gap-2 border-t border-zinc-200/90 bg-[#f0f1f3] px-3.5 py-2.5 dark:border-zinc-800 dark:bg-zinc-900">
        {/* 左侧快捷图标栏 */}
        <div className="relative flex items-center gap-1" ref={menuRef}>
          {/* 1. 打开并选中压缩包 */}
          <IconToggleButton
            title={
              task.kind === "compress"
                ? "在文件管理器中定位并选中生成的压缩包"
                : "在文件管理器中定位源压缩包"
            }
            onClick={handleRevealArchive}
            disabled={task.kind === "compress" && !isCompleted}
          >
            <Archive className="h-4 w-4 text-amber-500" />
          </IconToggleButton>

          {/* 2. 解压模式：立即打开目标文件夹 */}
          {task.kind === "extract" && (
            <IconToggleButton
              title="打开解压后的目标文件夹"
              onClick={handleRevealOutput}
            >
              <FolderOpen className="h-4 w-4 text-amber-500" />
            </IconToggleButton>
          )}

          {/* 3. 完成后自动打开文件管理器定位文件/文件夹 开关 */}
          <IconToggleButton
            title={
              task.kind === "extract"
                ? `解压完成后自动打开文件夹（当前：${
                    task.openFolderAfterDone ? "开启" : "关闭"
                  }）`
                : `压缩完成后自动定位压缩包（当前：${
                    task.openFolderAfterDone ? "开启" : "关闭"
                  }）`
            }
            active={task.openFolderAfterDone}
            accent="emerald"
            onClick={handleToggleOpenFolder}
          >
            <FolderSearch className="h-4 w-4" />
          </IconToggleButton>

          {/* 4. 完成后自动关闭窗口 开关（经典 Bandizip 红叉+勾选图标） */}
          <IconToggleButton
            title={`完成后自动关闭此窗口（当前：${
              task.autoCloseAfterDone ? "开启" : "关闭"
            }）`}
            active={task.autoCloseAfterDone}
            accent="red"
            onClick={handleToggleAutoClose}
          >
            <span className="flex h-4 w-4 items-center justify-center rounded-[3px] bg-red-500/90 text-white">
              <X className="h-3 w-3 stroke-[2.8]" />
            </span>
          </IconToggleButton>

          {/* 5. 解压模式：完成后将源压缩包移入回收站/废纸篓 */}
          {task.kind === "extract" && (
            <IconToggleButton
              title={`解压成功后将原压缩包移入回收站（当前：${
                task.deleteSourceAfterExtract ? "开启" : "关闭"
              }）`}
              active={task.deleteSourceAfterExtract}
              accent="amber"
              onClick={handleToggleDeleteSource}
              disabled={!isActive}
            >
              <Trash2 className="h-4 w-4" />
            </IconToggleButton>
          )}

          {/* 6. 更多菜单 ☰ */}
          <IconToggleButton
            title="更多选项"
            active={menuOpen || alwaysOnTop}
            accent="blue"
            onClick={() => setMenuOpen((v) => !v)}
          >
            <Menu className="h-4 w-4" />
          </IconToggleButton>

          {menuOpen && (
            <div className="absolute bottom-10 left-0 z-50 w-48 overflow-hidden rounded-xl border border-zinc-200 bg-white py-1 text-xs text-zinc-700 shadow-xl dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200">
              <button
                type="button"
                onClick={handleToggleAlwaysOnTop}
                className="flex w-full cursor-pointer items-center justify-between px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
              >
                <span className="flex items-center gap-2">
                  <Pin className="h-3.5 w-3.5 text-zinc-500" />
                  窗口始终置顶
                </span>
                {alwaysOnTop && <Check className="h-3.5 w-3.5 text-blue-600" />}
              </button>

              {task.archivePath && (
                <button
                  type="button"
                  disabled={task.kind === "compress" && !isCompleted}
                  onClick={() => {
                    setMenuOpen(false);
                    onBrowseArchive(task.archivePath);
                  }}
                  className="flex w-full cursor-pointer items-center gap-2 px-3 py-2 text-left hover:bg-zinc-100 disabled:opacity-40 dark:hover:bg-zinc-800"
                >
                  <AppWindow className="h-3.5 w-3.5 text-zinc-500" />
                  <span>在 QPack 主窗口预览</span>
                </button>
              )}

              <div className="my-1 border-t border-zinc-100 dark:border-zinc-800" />

              <button
                type="button"
                onClick={() => {
                  setMenuOpen(false);
                  onOpenSettings();
                }}
                className="flex w-full cursor-pointer items-center gap-2 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
              >
                <Settings className="h-3.5 w-3.5 text-zinc-500" />
                <span>偏好设置...</span>
              </button>
            </div>
          )}
        </div>

        {/* 右侧操作按钮 */}
        <div className="flex items-center gap-2">
          {isActive ? (
            <>
              <button
                type="button"
                onClick={handleTogglePause}
                className="inline-flex min-w-[74px] cursor-pointer items-center justify-center gap-1.5 rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-xs font-medium text-zinc-800 shadow-2xs transition hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-100 dark:hover:bg-zinc-700"
              >
                {isPaused ? (
                  <>
                    <Play className="h-3.5 w-3.5 text-emerald-600" />
                    继续
                  </>
                ) : (
                  <>
                    <Pause className="h-3.5 w-3.5 text-amber-600" />
                    暂停
                  </>
                )}
              </button>
              <button
                type="button"
                onClick={handleCancel}
                className="min-w-[74px] cursor-pointer rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-xs font-medium text-zinc-800 shadow-2xs transition hover:border-red-300 hover:bg-red-50 hover:text-red-600 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-100 dark:hover:border-red-800 dark:hover:bg-red-950/50 dark:hover:text-red-400"
              >
                取消
              </button>
            </>
          ) : (
            <>
              {isCompleted && (
                <button
                  type="button"
                  onClick={handleRevealOutput}
                  className="cursor-pointer rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-xs font-medium text-zinc-800 shadow-2xs transition hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-100 dark:hover:bg-zinc-700"
                >
                  {task.kind === "compress" ? "定位文件" : "打开位置"}
                </button>
              )}
              <button
                type="button"
                onClick={onClose}
                className="min-w-[78px] cursor-pointer rounded-md border border-blue-500 bg-white px-4 py-1.5 text-xs font-medium text-zinc-900 shadow-2xs transition hover:bg-blue-50 dark:border-blue-500 dark:bg-zinc-800 dark:text-zinc-100 dark:hover:bg-blue-950/40"
              >
                关闭
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );

  if (task.standaloneWindow) {
    return body;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-4 backdrop-blur-[1px]">
      {body}
    </div>
  );
}
