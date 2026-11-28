import { useEffect, useState } from "react";
import {
  ArrowDownToLine,
  CheckCircle2,
  FolderCog,
  Loader2,
  MonitorCog,
  RefreshCw,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import type { Update } from "@tauri-apps/plugin-updater";
import { api } from "../lib/api";
import { checkForAppUpdate, downloadAndInstallUpdate } from "../lib/updater";
import { useAppStore } from "../stores/useAppStore";
import type {
  AppSettings,
  CompressFormat,
  DoubleClickAction,
  SystemIntegrationStatus,
} from "../types/archive";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

const DEFAULT_FORMATS: { value: CompressFormat; label: string }[] = [
  { value: "zip", label: "ZIP" },
  { value: "seven-z", label: "7Z" },
  { value: "tar-zst", label: "TAR.ZST" },
  { value: "tar-gz", label: "TAR.GZ" },
  { value: "tar-xz", label: "TAR.XZ" },
  { value: "tar-bz2", label: "TAR.BZ2" },
  { value: "tar", label: "TAR" },
];

export function SettingsModal({ open, onClose }: SettingsModalProps) {
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);
  const showToast = useAppStore((s) => s.showToast);

  const [integration, setIntegration] =
    useState<SystemIntegrationStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [refreshingIcons, setRefreshingIcons] = useState(false);
  const [appVersion, setAppVersion] = useState("0.1.5");
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [availableUpdate, setAvailableUpdate] = useState<Update | null>(null);
  const [updateMessage, setUpdateMessage] = useState<string | null>(null);
  const [updating, setUpdating] = useState(false);
  const [updatePercent, setUpdatePercent] = useState<number | null>(null);

  useEffect(() => {
    if (!open) return;
    api.getAppSettings().then(setSettings).catch(() => {});
    api.getSystemIntegrationStatus().then(setIntegration).catch(() => {});
    getVersion().then(setAppVersion).catch(() => {});
  }, [open, setSettings]);

  if (!open) return null;

  const saveSettings = async (next: AppSettings) => {
    setSettings(next);
    try {
      await api.updateAppSettings(next);
    } catch (err) {
      showToast({ type: "error", title: "保存设置失败", detail: String(err) });
    }
  };

  const handleToggleContextMenu = async () => {
    if (!integration) return;
    setBusy(true);
    try {
      const updated = integration.contextMenuInstalled
        ? await api.uninstallSystemContextMenu()
        : await api.installSystemContextMenu();
      setIntegration(updated);
      showToast({
        type: "success",
        title: updated.contextMenuInstalled
          ? "系统右键集成已启用"
          : "系统右键集成已移除",
        detail: updated.contextMenuInstalled
          ? updated.platform === "macos"
            ? "现在可在访达右键 ->「快速操作」中直接使用 QPack（支持智能解压、ZIP/7Z 压缩与高级压缩）"
            : "现在可在 Windows 资源管理器右键「QPack」级联菜单中直接解压或压缩多种格式"
          : undefined,
      });
    } catch (err) {
      showToast({
        type: "error",
        title: "操作失败",
        detail: String(err),
      });
    } finally {
      setBusy(false);
    }
  };

  const handleRefreshFileIcons = async () => {
    setRefreshingIcons(true);
    try {
      await api.refreshFileIcons();
      showToast({
        type: "success",
        title: "压缩包文件图标已刷新",
        detail: "已为 ZIP / 7Z / RAR / TAR / GZ / ZST 等格式绑定专属图标并刷新系统缓存",
      });
    } catch (err) {
      showToast({
        type: "error",
        title: "刷新图标失败",
        detail: String(err),
      });
    } finally {
      setRefreshingIcons(false);
    }
  };

  const setAction = (doubleClickAction: DoubleClickAction) =>
    saveSettings({ ...settings, doubleClickAction });

  const handleCheckUpdate = async () => {
    setCheckingUpdate(true);
    setUpdateMessage(null);
    try {
      const found = await checkForAppUpdate();
      if (found) {
        setAvailableUpdate(found);
        setUpdateMessage(
          `发现新版本 v${found.version}${found.body ? `：${found.body}` : ""}`
        );
      } else {
        setAvailableUpdate(null);
        setUpdateMessage(`当前 v${appVersion} 已是最新版本`);
      }
    } catch (err) {
      setUpdateMessage(`检查更新失败：${String(err)}`);
    } finally {
      setCheckingUpdate(false);
    }
  };

  const handleInstallUpdate = async () => {
    if (!availableUpdate) return;
    setUpdating(true);
    setUpdatePercent(0);
    try {
      await downloadAndInstallUpdate(availableUpdate, (p) => {
        setUpdatePercent(p.percent);
      });
    } catch (err) {
      setUpdating(false);
      showToast({
        type: "error",
        title: "下载或安装更新失败",
        detail: String(err),
      });
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="flex max-h-[88vh] w-full max-w-lg flex-col overflow-hidden rounded-2xl border border-zinc-200 bg-white shadow-2xl dark:border-zinc-800 dark:bg-zinc-950">
        {/* 头部 */}
        <div className="flex shrink-0 items-center justify-between border-b border-zinc-200 px-5 py-3.5 dark:border-zinc-800">
          <div className="flex items-center gap-2 text-sm font-semibold text-zinc-900 dark:text-zinc-100">
            <MonitorCog className="h-4 w-4 text-blue-600" />
            <span>偏好设置与系统集成</span>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="cursor-pointer rounded-lg p-1 text-zinc-400 hover:bg-zinc-100 hover:text-zinc-600 dark:hover:bg-zinc-800"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* 主体 */}
        <div className="flex-1 space-y-5 overflow-y-auto p-5 text-xs">
          {/* 双击 / 打开方式行为 */}
          <div>
            <p className="mb-2 font-medium text-zinc-700 dark:text-zinc-300">
              在访达 / 资源管理器中双击压缩包时的行为
            </p>
            <div className="grid grid-cols-2 gap-2.5">
              <button
                type="button"
                onClick={() => setAction("smart_extract")}
                className={`cursor-pointer rounded-xl border p-3 text-left transition ${
                  settings.doubleClickAction === "smart_extract"
                    ? "border-blue-500 bg-blue-50/70 dark:bg-blue-950/40"
                    : "border-zinc-200 hover:bg-zinc-50 dark:border-zinc-800 dark:hover:bg-zinc-900"
                }`}
              >
                <div className="flex items-center gap-1.5 font-medium text-zinc-900 dark:text-zinc-100">
                  <Sparkles className="h-3.5 w-3.5 text-blue-600" />
                  <span>弹出进度窗智能解压</span>
                </div>
                <p className="mt-1 text-[11px] leading-relaxed text-zinc-500 dark:text-zinc-400">
                  弹出轻量进度窗口直接原地智能解压，支持暂停与取消（类似 Bandizip）
                </p>
              </button>

              <button
                type="button"
                onClick={() => setAction("browse")}
                className={`cursor-pointer rounded-xl border p-3 text-left transition ${
                  settings.doubleClickAction === "browse"
                    ? "border-blue-500 bg-blue-50/70 dark:bg-blue-950/40"
                    : "border-zinc-200 hover:bg-zinc-50 dark:border-zinc-800 dark:hover:bg-zinc-900"
                }`}
              >
                <div className="font-medium text-zinc-900 dark:text-zinc-100">
                  打开窗口预览内容
                </div>
                <p className="mt-1 text-[11px] leading-relaxed text-zinc-500 dark:text-zinc-400">
                  打开 QPack 主窗口浏览文件列表，支持搜索与部分提取
                </p>
              </button>
            </div>
          </div>

          {/* 完成后动作与进度窗口设置 */}
          <div>
            <p className="mb-2 font-medium text-zinc-700 dark:text-zinc-300">
              解压 / 压缩完成后的行为
            </p>
            <div className="space-y-2">
              <label className="flex cursor-pointer items-center justify-between rounded-xl border border-zinc-200/80 px-3.5 py-2.5 dark:border-zinc-800">
                <div>
                  <p className="font-medium text-zinc-800 dark:text-zinc-200">
                    解压完成后自动打开文件管理器定位文件
                  </p>
                  <p className="mt-0.5 text-[11px] text-zinc-500 dark:text-zinc-400">
                    解压为目录时自动打开该文件夹，解压为单文件时高亮选中该文件
                  </p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.openFolderAfterExtract}
                  onChange={(e) =>
                    saveSettings({
                      ...settings,
                      openFolderAfterExtract: e.target.checked,
                    })
                  }
                  className="h-4 w-4 cursor-pointer accent-blue-600"
                />
              </label>

              <label className="flex cursor-pointer items-center justify-between rounded-xl border border-zinc-200/80 px-3.5 py-2.5 dark:border-zinc-800">
                <div>
                  <p className="font-medium text-zinc-800 dark:text-zinc-200">
                    压缩完成后自动打开文件管理器并选中压缩包
                  </p>
                  <p className="mt-0.5 text-[11px] text-zinc-500 dark:text-zinc-400">
                    开启后将在生成压缩包后自动定位并高亮选中该文件
                  </p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.openFolderAfterCompress}
                  onChange={(e) =>
                    saveSettings({
                      ...settings,
                      openFolderAfterCompress: e.target.checked,
                    })
                  }
                  className="h-4 w-4 cursor-pointer accent-blue-600"
                />
              </label>

              <label className="flex cursor-pointer items-center justify-between rounded-xl border border-zinc-200/80 px-3.5 py-2.5 dark:border-zinc-800">
                <div>
                  <p className="font-medium text-zinc-800 dark:text-zinc-200">
                    任务完成后自动关闭进度窗口
                  </p>
                  <p className="mt-0.5 text-[11px] text-zinc-500 dark:text-zinc-400">
                    关闭此项时将保留完成界面供查看摘要与手动定位（也可在进度窗底部一键切换）
                  </p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.autoCloseProgressWindow}
                  onChange={(e) =>
                    saveSettings({
                      ...settings,
                      autoCloseProgressWindow: e.target.checked,
                    })
                  }
                  className="h-4 w-4 cursor-pointer accent-blue-600"
                />
              </label>
            </div>
          </div>

          {/* 默认压缩格式与通用选项 */}
          <div>
            <p className="mb-2 font-medium text-zinc-700 dark:text-zinc-300">
              默认压缩与常驻选项
            </p>
            <div className="space-y-2">
              <div className="flex items-center justify-between rounded-xl border border-zinc-200/80 px-3.5 py-2.5 dark:border-zinc-800">
                <div>
                  <p className="font-medium text-zinc-800 dark:text-zinc-200">
                    新建压缩包默认格式
                  </p>
                  <p className="mt-0.5 text-[11px] text-zinc-500 dark:text-zinc-400">
                    打开新建压缩窗口时自动选中的压缩格式
                  </p>
                </div>
                <select
                  value={settings.defaultCompressFormat}
                  onChange={(e) =>
                    saveSettings({
                      ...settings,
                      defaultCompressFormat: e.target.value as CompressFormat,
                    })
                  }
                  className="cursor-pointer rounded-lg border border-zinc-200 bg-white px-2.5 py-1.5 text-xs font-medium text-zinc-800 outline-none focus:border-blue-500 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200"
                >
                  {DEFAULT_FORMATS.map((f) => (
                    <option key={f.value} value={f.value}>
                      {f.label}
                    </option>
                  ))}
                </select>
              </div>

              <label className="flex cursor-pointer items-center justify-between rounded-xl border border-zinc-200/80 px-3.5 py-2.5 dark:border-zinc-800">
                <div>
                  <p className="font-medium text-zinc-800 dark:text-zinc-200">
                    压缩时自动过滤系统垃圾文件
                  </p>
                  <p className="mt-0.5 text-[11px] text-zinc-500 dark:text-zinc-400">
                    自动排除 .DS_Store、__MACOSX、Thumbs.db
                  </p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.excludeJunk}
                  onChange={(e) =>
                    saveSettings({ ...settings, excludeJunk: e.target.checked })
                  }
                  className="h-4 w-4 cursor-pointer accent-blue-600"
                />
              </label>

              <label className="flex cursor-pointer items-center justify-between rounded-xl border border-zinc-200/80 px-3.5 py-2.5 dark:border-zinc-800">
                <div>
                  <p className="font-medium text-zinc-800 dark:text-zinc-200">
                    关闭主窗口时在后台托盘 / 菜单栏运行
                  </p>
                  <p className="mt-0.5 text-[11px] text-zinc-500 dark:text-zinc-400">
                    保持极低内存驻留，随时从托盘快速唤起
                  </p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.closeToTray}
                  onChange={(e) =>
                    saveSettings({ ...settings, closeToTray: e.target.checked })
                  }
                  className="h-4 w-4 cursor-pointer accent-blue-600"
                />
              </label>
            </div>
          </div>

          {/* 系统右键菜单 / 访达快速操作 */}
          <div className="rounded-xl border border-zinc-200 bg-zinc-50/80 p-3.5 dark:border-zinc-800 dark:bg-zinc-900/50">
            <div className="flex items-center justify-between gap-3">
              <div className="flex items-center gap-2">
                <FolderCog className="h-4 w-4 text-indigo-500" />
                <span className="font-medium text-zinc-900 dark:text-zinc-100">
                  {integration?.platform === "macos"
                    ? "访达右键「快速操作」集成"
                    : "Windows 资源管理器右键菜单集成"}
                </span>
              </div>

              <div className="flex shrink-0 items-center gap-1.5">
                <button
                  type="button"
                  disabled={refreshingIcons}
                  onClick={handleRefreshFileIcons}
                  title="为 ZIP / 7Z / RAR / TAR / GZ / ZST 等格式注册专属图标并刷新系统缓存"
                  className="inline-flex cursor-pointer items-center gap-1 rounded-lg border border-zinc-300 bg-white px-2.5 py-1.5 text-xs font-medium text-zinc-700 transition hover:bg-zinc-50 disabled:opacity-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
                >
                  <RefreshCw
                    className={`h-3.5 w-3.5 ${refreshingIcons ? "animate-spin" : ""}`}
                  />
                  刷新图标
                </button>

                <button
                  type="button"
                  disabled={busy || !integration}
                  onClick={handleToggleContextMenu}
                  className={`inline-flex shrink-0 cursor-pointer items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition disabled:opacity-50 ${
                    integration?.contextMenuInstalled
                      ? "border border-zinc-300 bg-white text-zinc-700 hover:bg-red-50 hover:text-red-600 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-300"
                      : "bg-blue-600 text-white hover:bg-blue-700"
                  }`}
                >
                  {integration?.contextMenuInstalled ? (
                    <>
                      <Trash2 className="h-3.5 w-3.5" />
                      移除菜单
                    </>
                  ) : (
                    <>
                      <CheckCircle2 className="h-3.5 w-3.5" />
                      一键启用
                    </>
                  )}
                </button>
              </div>
            </div>

            {integration?.hint && (
              <p className="mt-2 text-[11px] leading-relaxed text-zinc-500 dark:text-zinc-400">
                {integration.hint}
              </p>
            )}
          </div>

          {/* 软件版本与自动更新 */}
          <div className="rounded-xl border border-zinc-200 bg-zinc-50/80 p-3.5 dark:border-zinc-800 dark:bg-zinc-900/50">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="font-medium text-zinc-900 dark:text-zinc-100">
                  软件版本与自动更新
                </p>
                <p className="mt-0.5 text-[11px] text-zinc-500 dark:text-zinc-400">
                  当前版本 v{appVersion} · 启动时自动静默检查更新
                </p>
              </div>

              {availableUpdate ? (
                <button
                  type="button"
                  disabled={updating}
                  onClick={handleInstallUpdate}
                  className="inline-flex shrink-0 cursor-pointer items-center gap-1.5 rounded-lg bg-blue-600 px-3 py-1.5 text-xs font-medium text-white transition hover:bg-blue-700 disabled:opacity-60"
                >
                  {updating ? (
                    <>
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                      {updatePercent !== null ? `${updatePercent}%` : "更新中..."}
                    </>
                  ) : (
                    <>
                      <ArrowDownToLine className="h-3.5 w-3.5" />
                      立即更新
                    </>
                  )}
                </button>
              ) : (
                <button
                  type="button"
                  disabled={checkingUpdate}
                  onClick={handleCheckUpdate}
                  className="inline-flex shrink-0 cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-300 bg-white px-3 py-1.5 text-xs font-medium text-zinc-700 transition hover:bg-zinc-50 disabled:opacity-60 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
                >
                  <RefreshCw
                    className={`h-3.5 w-3.5 ${checkingUpdate ? "animate-spin" : ""}`}
                  />
                  {checkingUpdate ? "检查中..." : "检查更新"}
                </button>
              )}
            </div>

            {updateMessage && (
              <p className="mt-2 text-[11px] leading-relaxed text-blue-600 dark:text-blue-400">
                {updateMessage}
              </p>
            )}
          </div>
        </div>

        <div className="flex shrink-0 justify-end border-t border-zinc-200 bg-zinc-50/70 px-5 py-3 dark:border-zinc-800 dark:bg-zinc-900/50">
          <button
            type="button"
            onClick={onClose}
            className="cursor-pointer rounded-xl bg-zinc-900 px-4 py-1.5 text-xs font-medium text-white hover:bg-zinc-800 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200"
          >
            完成
          </button>
        </div>
      </div>
    </div>
  );
}
