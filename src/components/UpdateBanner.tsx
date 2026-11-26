import { useEffect, useState } from "react";
import { ArrowDownToLine, Loader2, Sparkles, X } from "lucide-react";
import type { Update } from "@tauri-apps/plugin-updater";
import { checkForAppUpdate, downloadAndInstallUpdate } from "../lib/updater";
import { useAppStore } from "../stores/useAppStore";

export function UpdateBanner() {
  const showToast = useAppStore((s) => s.showToast);
  const [update, setUpdate] = useState<Update | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [percent, setPercent] = useState<number | null>(null);

  // 应用启动 3 秒后静默检查更新，离线或未部署 latest.json 时静默跳过
  useEffect(() => {
    const timer = window.setTimeout(() => {
      checkForAppUpdate()
        .then((found) => {
          if (found) setUpdate(found);
        })
        .catch(() => {});
    }, 3000);
    return () => window.clearTimeout(timer);
  }, []);

  if (!update || dismissed) return null;

  const handleInstall = async () => {
    setInstalling(true);
    setPercent(0);
    try {
      await downloadAndInstallUpdate(update, (p) => {
        setPercent(p.percent);
      });
    } catch (err) {
      setInstalling(false);
      showToast({
        type: "error",
        title: "下载或安装更新失败",
        detail: String(err),
      });
    }
  };

  return (
    <div className="flex shrink-0 items-center justify-between gap-3 border-b border-blue-200/80 bg-blue-50/90 px-4 py-2 text-xs text-blue-900 dark:border-blue-900/60 dark:bg-blue-950/60 dark:text-blue-100">
      <div className="flex min-w-0 items-center gap-2">
        <Sparkles className="h-3.5 w-3.5 shrink-0 text-blue-600 dark:text-blue-400" />
        <span className="font-semibold">发现新版本 v{update.version}</span>
        {update.body && (
          <span className="truncate text-blue-700/90 dark:text-blue-300/90">
            · {update.body}
          </span>
        )}
      </div>

      <div className="flex shrink-0 items-center gap-2">
        <button
          type="button"
          disabled={installing}
          onClick={handleInstall}
          className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg bg-blue-600 px-3 py-1 text-xs font-medium text-white transition hover:bg-blue-700 disabled:opacity-70"
        >
          {installing ? (
            <>
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              {percent !== null ? `正在更新 ${percent}%` : "正在下载更新..."}
            </>
          ) : (
            <>
              <ArrowDownToLine className="h-3.5 w-3.5" />
              立即更新并重启
            </>
          )}
        </button>

        {!installing && (
          <button
            type="button"
            onClick={() => setDismissed(true)}
            className="cursor-pointer rounded-md p-1 text-blue-500 hover:bg-blue-100 dark:hover:bg-blue-900/60"
            title="稍后提醒"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        )}
      </div>
    </div>
  );
}
