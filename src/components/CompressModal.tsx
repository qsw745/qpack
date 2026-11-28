import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  FilePlus2,
  FolderPlus,
  Lock,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import { api } from "../lib/api";
import { useAppStore } from "../stores/useAppStore";
import type { CompressFormat, CompressOptions } from "../types/archive";

const FORMAT_OPTIONS: {
  value: CompressFormat;
  label: string;
  desc: string;
  supportsEncryption: boolean;
}[] = [
  {
    value: "zip",
    label: "ZIP",
    desc: "通用性最强，支持 AES-256 加密",
    supportsEncryption: true,
  },
  {
    value: "seven-z",
    label: "7Z",
    desc: "高压缩率 LZMA2，支持 AES-256 加密",
    supportsEncryption: true,
  },
  {
    value: "tar-zst",
    label: "TAR.ZST",
    desc: "极速压缩，现代高性能格式",
    supportsEncryption: false,
  },
  {
    value: "tar-gz",
    label: "TAR.GZ",
    desc: "Linux / macOS 标准格式",
    supportsEncryption: false,
  },
  {
    value: "tar-xz",
    label: "TAR.XZ",
    desc: "LZMA2 超高压缩率，适合分发源码与归档",
    supportsEncryption: false,
  },
  {
    value: "tar-bz2",
    label: "TAR.BZ2",
    desc: "经典高压缩率 Bzip2 格式",
    supportsEncryption: false,
  },
  {
    value: "tar",
    label: "TAR",
    desc: "仅打包不压缩，速度最快",
    supportsEncryption: false,
  },
  {
    value: "dmg",
    label: "DMG",
    desc: "macOS 原生压缩磁盘映像（系统内置引擎，无需额外软件）",
    supportsEncryption: false,
  },
];

interface CompressModalProps {
  onStartCompress: (options: CompressOptions) => void;
}

export function CompressModal({ onStartCompress }: CompressModalProps) {
  const isOpen = useAppStore((s) => s.compressModalOpen);
  const inputs = useAppStore((s) => s.compressInputs);
  const settings = useAppStore((s) => s.settings);
  const engineStatus = useAppStore((s) => s.engineStatus);
  const closeModal = useAppStore((s) => s.closeCompressModal);
  const setInputs = useAppStore((s) => s.setCompressInputs);

  const availableFormats = FORMAT_OPTIONS.filter(
    (opt) =>
      opt.value !== "dmg" ||
      (engineStatus?.supportedWriteFormats.includes("dmg") ?? false)
  );

  const [format, setFormat] = useState<CompressFormat>(
    settings.defaultCompressFormat || "zip"
  );
  const [outputPath, setOutputPath] = useState("");
  const [level, setLevel] = useState(6);
  const [password, setPassword] = useState("");
  const [excludeJunk, setExcludeJunk] = useState(settings.excludeJunk);

  // 每次打开弹窗时应用用户默认压缩格式与垃圾过滤偏好
  useEffect(() => {
    if (!isOpen) return;
    setFormat(settings.defaultCompressFormat || "zip");
    setExcludeJunk(settings.excludeJunk);
    setPassword("");
  }, [isOpen, settings.defaultCompressFormat, settings.excludeJunk]);

  useEffect(() => {
    if (!isOpen) return;
    if (inputs.length > 0) {
      api
        .suggestCompressOutput(inputs, format)
        .then(setOutputPath)
        .catch(() => {});
    } else {
      setOutputPath("");
    }
  }, [isOpen, inputs, format]);

  if (!isOpen) return null;

  const supportsEncryption = format === "zip" || format === "seven-z";

  const addFiles = async (directory: boolean) => {
    const picked = await open({
      multiple: true,
      directory,
      title: directory ? "选择要压缩的文件夹" : "选择要压缩的文件",
    });
    if (!picked) return;
    const arr = Array.isArray(picked) ? picked : [picked];
    const merged = Array.from(new Set([...inputs, ...arr]));
    setInputs(merged);
  };

  const pickOutputPath = async () => {
    const extMap: Record<CompressFormat, string> = {
      zip: "zip",
      "seven-z": "7z",
      "tar-gz": "tar.gz",
      "tar-bz2": "tar.bz2",
      "tar-xz": "tar.xz",
      "tar-zst": "tar.zst",
      tar: "tar",
      dmg: "dmg",
    };
    const chosen = await save({
      defaultPath: outputPath || `archive.${extMap[format]}`,
      title: "选择压缩包保存位置",
    });
    if (chosen) setOutputPath(chosen);
  };

  const handleStart = () => {
    if (inputs.length === 0 || !outputPath.trim()) return;
    const opts: CompressOptions = {
      inputPaths: inputs,
      outputPath: outputPath.trim(),
      format,
      level,
      password:
        supportsEncryption && password.trim() ? password.trim() : undefined,
      excludeJunk,
    };
    closeModal();
    setPassword("");
    onStartCompress(opts);
  };

  const activeFormatMeta = FORMAT_OPTIONS.find((f) => f.value === format);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-xs"
      onClick={(e) => {
        if (e.target === e.currentTarget) closeModal();
      }}
    >
      <div className="flex w-full max-w-xl flex-col overflow-hidden rounded-2xl border border-zinc-200 bg-white shadow-2xl dark:border-zinc-800 dark:bg-zinc-950">
        {/* 头部 */}
        <div className="flex items-center justify-between border-b border-zinc-200 px-5 py-3.5 dark:border-zinc-800">
          <h2 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
            新建压缩包
          </h2>
          <button
            type="button"
            onClick={closeModal}
            className="cursor-pointer rounded-lg p-1 text-zinc-400 hover:bg-zinc-100 hover:text-zinc-600 dark:hover:bg-zinc-800"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* 内容 */}
        <div className="space-y-4 p-5 text-sm">
          {/* 待压缩列表 */}
          <div>
            <div className="mb-2 flex items-center justify-between">
              <span className="text-xs font-medium text-zinc-500">
                待压缩文件 ({inputs.length})
              </span>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => addFiles(false)}
                  className="inline-flex cursor-pointer items-center gap-1 rounded-lg border border-zinc-200 px-2.5 py-1 text-xs text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800"
                >
                  <FilePlus2 className="h-3.5 w-3.5" />
                  添加文件
                </button>
                <button
                  type="button"
                  onClick={() => addFiles(true)}
                  className="inline-flex cursor-pointer items-center gap-1 rounded-lg border border-zinc-200 px-2.5 py-1 text-xs text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800"
                >
                  <FolderPlus className="h-3.5 w-3.5" />
                  添加文件夹
                </button>
              </div>
            </div>

            <div className="max-h-32 overflow-y-auto rounded-xl border border-zinc-200 bg-zinc-50/70 p-2 dark:border-zinc-800 dark:bg-zinc-900/50">
              {inputs.length === 0 ? (
                <p className="py-4 text-center text-xs text-zinc-400">
                  点击右上角添加，或直接将文件 / 文件夹 / DMG 拖入此窗口
                </p>
              ) : (
                <ul className="space-y-1">
                  {inputs.map((p) => (
                    <li
                      key={p}
                      className="flex items-center justify-between gap-2 rounded-lg bg-white px-2.5 py-1.5 text-xs dark:bg-zinc-900"
                    >
                      <span
                        className="truncate text-zinc-700 dark:text-zinc-300"
                        title={p}
                      >
                        {p}
                      </span>
                      <button
                        type="button"
                        onClick={() => setInputs(inputs.filter((x) => x !== p))}
                        className="shrink-0 cursor-pointer text-zinc-400 hover:text-red-500"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>

          {/* 格式选择 */}
          <div>
            <div className="mb-2 flex items-center justify-between">
              <label className="text-xs font-medium text-zinc-500">
                目标格式
              </label>
              {activeFormatMeta && (
                <span className="text-[11px] text-zinc-400">
                  {activeFormatMeta.desc}
                </span>
              )}
            </div>
            <div className="grid grid-cols-4 gap-2">
              {availableFormats.map((opt) => (
                <button
                  key={opt.value}
                  type="button"
                  onClick={() => setFormat(opt.value)}
                  title={opt.desc}
                  className={`cursor-pointer rounded-xl border px-2 py-2 text-xs font-medium transition ${
                    format === opt.value
                      ? "border-blue-500 bg-blue-50 text-blue-700 dark:bg-blue-950/50 dark:text-blue-300"
                      : "border-zinc-200 bg-white text-zinc-600 hover:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-400"
                  }`}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </div>

          {/* 输出路径 */}
          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-500">
              保存到
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={outputPath}
                onChange={(e) => setOutputPath(e.target.value)}
                placeholder="选择压缩包输出路径..."
                className="flex-1 rounded-xl border border-zinc-200 bg-white px-3 py-2 text-xs text-zinc-800 outline-none focus:border-blue-500 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200"
              />
              <button
                type="button"
                onClick={pickOutputPath}
                className="cursor-pointer rounded-xl border border-zinc-200 px-3 py-2 text-xs font-medium text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800"
              >
                浏览
              </button>
            </div>
          </div>

          {/* 压缩等级与密码 */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <div className="mb-1.5 flex items-center justify-between text-xs">
                <span className="font-medium text-zinc-500">压缩等级</span>
                <span className="font-mono text-zinc-700 dark:text-zinc-300">
                  {level === 0
                    ? "仅存储 / 极速"
                    : level >= 8
                    ? `${level} (极限)`
                    : `${level} (均衡)`}
                </span>
              </div>
              <input
                type="range"
                min={0}
                max={9}
                value={level}
                onChange={(e) => setLevel(Number(e.target.value))}
                disabled={format === "tar"}
                className="w-full accent-blue-600"
              />
            </div>

            <div>
              <label className="mb-1.5 flex items-center gap-1 text-xs font-medium text-zinc-500">
                <Lock className="h-3 w-3" />
                <span>加密密码（ZIP / 7Z AES-256）</span>
              </label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                disabled={!supportsEncryption}
                placeholder={
                  supportsEncryption ? "留空则不加密" : "当前格式不支持加密"
                }
                className="w-full rounded-xl border border-zinc-200 bg-white px-3 py-1.5 text-xs text-zinc-800 outline-none focus:border-blue-500 disabled:opacity-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200"
              />
            </div>
          </div>

          {/* 过滤垃圾文件 */}
          <label className="flex cursor-pointer items-center gap-2 rounded-xl bg-zinc-50 px-3 py-2.5 text-xs text-zinc-600 dark:bg-zinc-900/70 dark:text-zinc-400">
            <input
              type="checkbox"
              checked={excludeJunk}
              onChange={(e) => setExcludeJunk(e.target.checked)}
              className="h-3.5 w-3.5 accent-blue-600"
            />
            <Sparkles className="h-3.5 w-3.5 text-blue-500" />
            <span>自动排除系统隐藏垃圾文件（.DS_Store / __MACOSX / Thumbs.db）</span>
          </label>
        </div>

        {/* 底部按钮 */}
        <div className="flex items-center justify-end gap-2 border-t border-zinc-200 bg-zinc-50/70 px-5 py-3 dark:border-zinc-800 dark:bg-zinc-900/50">
          <button
            type="button"
            onClick={closeModal}
            className="cursor-pointer rounded-xl px-4 py-2 text-xs font-medium text-zinc-600 hover:bg-zinc-200/70 dark:text-zinc-400 dark:hover:bg-zinc-800"
          >
            取消
          </button>
          <button
            type="button"
            disabled={inputs.length === 0 || !outputPath.trim()}
            onClick={handleStart}
            className="cursor-pointer rounded-xl bg-blue-600 px-5 py-2 text-xs font-medium text-white shadow-xs transition hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
          >
            开始压缩
          </button>
        </div>
      </div>
    </div>
  );
}
