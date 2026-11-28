import { Archive, FolderPlus, PackageOpen, ShieldCheck, Sparkles } from "lucide-react";
import { useAppStore } from "../stores/useAppStore";

interface DropZoneHomeProps {
  isDragging: boolean;
  onPickArchive: () => void;
  onStartCompress: () => void;
}

export function DropZoneHome({
  isDragging,
  onPickArchive,
  onStartCompress,
}: DropZoneHomeProps) {
  const engineStatus = useAppStore((s) => s.engineStatus);

  return (
    <div className="flex flex-1 flex-col items-center justify-center p-8">
      <div
        className={`flex w-full max-w-2xl flex-col items-center justify-center rounded-2xl border-2 border-dashed px-8 py-14 transition-all duration-200 ${
          isDragging
            ? "scale-[1.01] border-blue-500 bg-blue-50/70 dark:bg-blue-950/30"
            : "border-zinc-200 bg-zinc-50/60 hover:border-zinc-300 dark:border-zinc-800 dark:bg-zinc-900/40 dark:hover:border-zinc-700"
        }`}
      >
        <div className="mb-5 flex h-20 w-20 items-center justify-center">
          <img
            src="/logo.png"
            alt="QPack"
            onError={(e) => {
              e.currentTarget.style.display = "none";
            }}
            className="h-20 w-20 object-contain drop-shadow-md"
          />
        </div>

        <h1 className="text-xl font-semibold tracking-tight text-zinc-900 dark:text-zinc-100">
          {isDragging ? "松开以处理文件" : "拖入压缩包或任意文件"}
        </h1>
        <p className="mt-2 max-w-md text-center text-sm text-zinc-500 dark:text-zinc-400">
          拖入压缩包立即浏览与智能解压；拖入普通文件或文件夹快速打包压缩。
        </p>

        <div className="mt-7 flex flex-wrap items-center justify-center gap-3">
          <button
            type="button"
            onClick={onPickArchive}
            className="inline-flex cursor-pointer items-center gap-2 rounded-xl bg-blue-600 px-5 py-2.5 text-sm font-medium text-white shadow-sm transition hover:bg-blue-700 active:scale-[0.99]"
          >
            <PackageOpen className="h-4 w-4" />
            打开压缩包
          </button>

          <button
            type="button"
            onClick={onStartCompress}
            className="inline-flex cursor-pointer items-center gap-2 rounded-xl border border-zinc-200 bg-white px-5 py-2.5 text-sm font-medium text-zinc-700 shadow-xs transition hover:bg-zinc-50 active:scale-[0.99] dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
          >
            <FolderPlus className="h-4 w-4" />
            新建压缩包
          </button>
        </div>

        <div className="mt-8 grid w-full max-w-lg grid-cols-3 gap-3 text-xs text-zinc-500 dark:text-zinc-400">
          <div className="flex items-center justify-center gap-1.5 rounded-lg bg-white/80 px-2.5 py-2 dark:bg-zinc-900/70">
            <Sparkles className="h-3.5 w-3.5 text-blue-500" />
            <span>智能防解压炸弹</span>
          </div>
          <div className="flex items-center justify-center gap-1.5 rounded-lg bg-white/80 px-2.5 py-2 dark:bg-zinc-900/70">
            <ShieldCheck className="h-3.5 w-3.5 text-emerald-500" />
            <span>中文防乱码修复</span>
          </div>
          <div className="flex items-center justify-center gap-1.5 rounded-lg bg-white/80 px-2.5 py-2 dark:bg-zinc-900/70">
            <Archive className="h-3.5 w-3.5 text-indigo-500" />
            <span>AES-256 加密</span>
          </div>
        </div>
      </div>

      {/* 底部引擎与支持格式状态 */}
      <div className="mt-6 flex flex-wrap items-center justify-center gap-1.5 text-xs text-zinc-400 dark:text-zinc-500">
        <span>开箱支持：</span>
        {[
          "zip",
          "7z",
          "rar",
          "tar.gz",
          "tar.zst",
          "tar.xz",
          "tar.bz2",
          "iso",
          "dmg",
          "cab",
          "wim",
        ].map((fmt) => (
          <span
            key={fmt}
            className="rounded-md bg-zinc-100 px-1.5 py-0.5 font-mono text-[11px] text-zinc-600 uppercase dark:bg-zinc-900 dark:text-zinc-400"
          >
            {fmt}
          </span>
        ))}
        {engineStatus?.sevenzCliReady && (
          <span className="ml-1 text-emerald-600 dark:text-emerald-400">
            · 双引擎已内置（免装 7-Zip）
          </span>
        )}
      </div>
    </div>
  );
}
