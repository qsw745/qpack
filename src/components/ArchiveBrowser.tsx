import { useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  ArrowUpDown,
  CheckSquare,
  ChevronRight,
  Copy,
  Eye,
  File,
  FileArchive,
  FileCode,
  FileImage,
  FileText,
  Folder,
  FolderOpen,
  FolderOutput,
  Home,
  Lock,
  Search,
  Sparkles,
  Square,
  X,
} from "lucide-react";
import { useAppStore } from "../stores/useAppStore";
import { formatBytes } from "../lib/api";
import type { ArchiveEntry } from "../types/archive";

interface DisplayItem {
  key: string;
  path: string;
  name: string;
  isDir: boolean;
  size: number;
  compressedSize: number;
  modified: string | null;
  encrypted: boolean;
  childCount?: number;
}

type SortKey = "name" | "size" | "modified";

interface ArchiveBrowserProps {
  onPreviewFile: (entryPath: string) => void;
  onExtractSelected: () => void;
  onSmartExtractAll: () => void;
  onExtractAllToFolder: () => void;
}

interface ContextMenuState {
  x: number;
  y: number;
  item: DisplayItem | null;
}

function getFileIcon(name: string, isDir: boolean) {
  if (isDir) return <Folder className="h-4 w-4 text-blue-500" />;
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"].includes(ext)) {
    return <FileImage className="h-4 w-4 text-emerald-500" />;
  }
  if (["zip", "7z", "rar", "tar", "gz", "zst", "xz"].includes(ext)) {
    return <FileArchive className="h-4 w-4 text-amber-500" />;
  }
  if (
    ["ts", "tsx", "js", "jsx", "rs", "py", "go", "json", "toml", "yaml", "yml", "html", "css"].includes(
      ext
    )
  ) {
    return <FileCode className="h-4 w-4 text-indigo-500" />;
  }
  if (["md", "txt", "pdf", "doc", "docx", "csv"].includes(ext)) {
    return <FileText className="h-4 w-4 text-sky-500" />;
  }
  return <File className="h-4 w-4 text-zinc-400" />;
}

/** 将扁平条目按当前目录层级聚合为直接子项（支持隐式目录） */
function buildDirectoryItems(
  entries: ArchiveEntry[],
  currentDir: string,
  searchQuery: string
): DisplayItem[] {
  const q = searchQuery.trim().toLowerCase();

  // 搜索模式下直接跨层级全局检索
  if (q) {
    return entries
      .filter(
        (e) =>
          e.name.toLowerCase().includes(q) || e.path.toLowerCase().includes(q)
      )
      .map((e) => ({
        key: `${e.index}-${e.path}`,
        path: e.path.replace(/\/$/, ""),
        name: e.path.replace(/\/$/, ""),
        isDir: e.isDir,
        size: e.size,
        compressedSize: e.compressedSize,
        modified: e.modified,
        encrypted: e.encrypted,
      }));
  }

  const prefix = currentDir ? `${currentDir.replace(/\/$/, "")}/` : "";
  const map = new Map<string, DisplayItem>();

  for (const entry of entries) {
    const clean = entry.path.replace(/^\/+/, "").replace(/\/+$/, "");
    if (!clean) continue;

    if (prefix && !clean.startsWith(prefix)) continue;
    const rest = prefix ? clean.slice(prefix.length) : clean;
    if (!rest) continue;

    const slashIdx = rest.indexOf("/");
    if (slashIdx === -1) {
      // 直接子文件或目录
      const existing = map.get(rest);
      if (existing) {
        if (entry.isDir) existing.isDir = true;
        if (!existing.modified && entry.modified) existing.modified = entry.modified;
      } else {
        map.set(rest, {
          key: clean,
          path: clean,
          name: rest,
          isDir: entry.isDir,
          size: entry.size,
          compressedSize: entry.compressedSize,
          modified: entry.modified,
          encrypted: entry.encrypted,
          childCount: entry.isDir ? 0 : undefined,
        });
      }
    } else {
      // 隐式子目录
      const folderName = rest.slice(0, slashIdx);
      const folderPath = prefix ? `${prefix}${folderName}` : folderName;
      const existing = map.get(folderName);
      if (existing) {
        existing.isDir = true;
        existing.size += entry.size;
        existing.compressedSize += entry.compressedSize;
        existing.childCount = (existing.childCount ?? 0) + (entry.isDir ? 0 : 1);
        if (entry.encrypted) existing.encrypted = true;
      } else {
        map.set(folderName, {
          key: folderPath,
          path: folderPath,
          name: folderName,
          isDir: true,
          size: entry.size,
          compressedSize: entry.compressedSize,
          modified: entry.modified,
          encrypted: entry.encrypted,
          childCount: entry.isDir ? 0 : 1,
        });
      }
    }
  }

  return Array.from(map.values());
}

export function ArchiveBrowser({
  onPreviewFile,
  onExtractSelected,
  onSmartExtractAll,
  onExtractAllToFolder,
}: ArchiveBrowserProps) {
  const archive = useAppStore((s) => s.archive);
  const currentDir = useAppStore((s) => s.currentDir);
  const searchQuery = useAppStore((s) => s.searchQuery);
  const selectedPaths = useAppStore((s) => s.selectedPaths);
  const setCurrentDir = useAppStore((s) => s.setCurrentDir);
  const setSearchQuery = useAppStore((s) => s.setSearchQuery);
  const toggleSelect = useAppStore((s) => s.toggleSelect);
  const selectAll = useAppStore((s) => s.selectAll);
  const clearSelection = useAppStore((s) => s.clearSelection);
  const showToast = useAppStore((s) => s.showToast);

  const [sortKey, setSortKey] = useState<SortKey>("name");
  const [sortAsc, setSortAsc] = useState(true);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const scrollParentRef = useRef<HTMLDivElement>(null);

  // 点击页面空白处、滚动或按 Esc 时关闭右键菜单
  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("click", close);
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("blur", close);
    };
  }, [contextMenu]);

  const items = useMemo(() => {
    if (!archive) return [];
    const list = buildDirectoryItems(archive.entries, currentDir, searchQuery);
    return list.sort((a, b) => {
      // 文件夹始终排在前面
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      let cmp = 0;
      if (sortKey === "name") {
        cmp = a.name.localeCompare(b.name, "zh-CN", { numeric: true });
      } else if (sortKey === "size") {
        cmp = a.size - b.size;
      } else {
        cmp = (a.modified ?? "").localeCompare(b.modified ?? "");
      }
      return sortAsc ? cmp : -cmp;
    });
  }, [archive, currentDir, searchQuery, sortKey, sortAsc]);

  const rowVirtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollParentRef.current,
    estimateSize: () => 38,
    overscan: 16,
  });

  if (!archive) return null;

  const breadcrumbs = currentDir ? currentDir.split("/").filter(Boolean) : [];
  const allSelected =
    items.length > 0 && items.every((item) => selectedPaths.has(item.path));

  const toggleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortAsc(!sortAsc);
    } else {
      setSortKey(key);
      setSortAsc(key === "name");
    }
  };

  const ratio =
    archive.totalUncompressedSize > 0
      ? Math.max(
          0,
          Math.round(
            (1 - archive.archiveSize / archive.totalUncompressedSize) * 100
          )
        )
      : 0;

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {/* 二级工具条：面包屑导航 + 搜索框 */}
      <div className="flex items-center justify-between gap-4 border-b border-zinc-200/80 bg-zinc-50/70 px-4 py-2 dark:border-zinc-800 dark:bg-zinc-900/50">
        <div className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto text-xs">
          <button
            type="button"
            onClick={() => setCurrentDir("")}
            className={`inline-flex shrink-0 cursor-pointer items-center gap-1 rounded-md px-2 py-1 transition ${
              breadcrumbs.length === 0
                ? "bg-white font-medium text-zinc-900 shadow-2xs dark:bg-zinc-800 dark:text-zinc-100"
                : "text-zinc-500 hover:bg-zinc-200/60 hover:text-zinc-800 dark:text-zinc-400 dark:hover:bg-zinc-800"
            }`}
          >
            <Home className="h-3.5 w-3.5" />
            <span>根目录</span>
          </button>

          {breadcrumbs.map((seg, idx) => {
            const target = breadcrumbs.slice(0, idx + 1).join("/");
            const isLast = idx === breadcrumbs.length - 1;
            return (
              <div key={target} className="flex shrink-0 items-center gap-1">
                <ChevronRight className="h-3.5 w-3.5 text-zinc-400" />
                <button
                  type="button"
                  onClick={() => setCurrentDir(target)}
                  className={`cursor-pointer rounded-md px-2 py-1 transition ${
                    isLast
                      ? "bg-white font-medium text-zinc-900 shadow-2xs dark:bg-zinc-800 dark:text-zinc-100"
                      : "text-zinc-500 hover:bg-zinc-200/60 hover:text-zinc-800 dark:text-zinc-400 dark:hover:bg-zinc-800"
                  }`}
                >
                  {seg}
                </button>
              </div>
            );
          })}
        </div>

        {/* 实时搜索 */}
        <div className="relative w-56 shrink-0">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 h-3.5 w-3.5 -translate-y-1/2 text-zinc-400" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="搜索包内文件..."
            className="w-full rounded-lg border border-zinc-200 bg-white py-1.5 pr-7 pl-8 text-xs text-zinc-800 outline-none transition focus:border-blue-500 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200"
          />
          {searchQuery && (
            <button
              type="button"
              onClick={() => setSearchQuery("")}
              className="absolute top-1/2 right-2 -translate-y-1/2 cursor-pointer text-zinc-400 hover:text-zinc-600"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
      </div>

      {/* 表头 */}
      <div className="grid grid-cols-[36px_1fr_110px_160px_68px] items-center border-b border-zinc-200/80 bg-white px-3 py-2 text-xs font-medium text-zinc-500 select-none dark:border-zinc-800 dark:bg-zinc-950 dark:text-zinc-400">
        <div className="flex justify-center">
          <input
            type="checkbox"
            checked={allSelected}
            onChange={() =>
              allSelected
                ? clearSelection()
                : selectAll(items.map((i) => i.path))
            }
            className="h-3.5 w-3.5 cursor-pointer rounded accent-blue-600"
          />
        </div>
        <button
          type="button"
          onClick={() => toggleSort("name")}
          className="flex cursor-pointer items-center gap-1 text-left hover:text-zinc-800 dark:hover:text-zinc-200"
        >
          <span>名称</span>
          <ArrowUpDown className="h-3 w-3 opacity-60" />
        </button>
        <button
          type="button"
          onClick={() => toggleSort("size")}
          className="flex cursor-pointer items-center justify-end gap-1 hover:text-zinc-800 dark:hover:text-zinc-200"
        >
          <span>大小</span>
          <ArrowUpDown className="h-3 w-3 opacity-60" />
        </button>
        <button
          type="button"
          onClick={() => toggleSort("modified")}
          className="flex cursor-pointer items-center justify-end gap-1 pr-2 hover:text-zinc-800 dark:hover:text-zinc-200"
        >
          <span>修改时间</span>
          <ArrowUpDown className="h-3 w-3 opacity-60" />
        </button>
        <div className="text-center">操作</div>
      </div>

      {/* 虚拟滚动列表主体 */}
      <div
        ref={scrollParentRef}
        onScroll={() => {
          if (contextMenu) setContextMenu(null);
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          const menuW = 220;
          const menuH = 190;
          setContextMenu({
            x: Math.min(e.clientX, window.innerWidth - menuW - 8),
            y: Math.min(e.clientY, window.innerHeight - menuH - 8),
            item: null,
          });
        }}
        className="flex-1 overflow-auto bg-white dark:bg-zinc-950"
      >
        {items.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-sm text-zinc-400">
            <p>{searchQuery ? "没有匹配的文件" : "当前目录为空"}</p>
          </div>
        ) : (
          <div
            style={{
              height: `${rowVirtualizer.getTotalSize()}px`,
              position: "relative",
              width: "100%",
            }}
          >
            {rowVirtualizer.getVirtualItems().map((vRow) => {
              const item = items[vRow.index];
              const checked = selectedPaths.has(item.path);
              return (
                <div
                  key={item.key}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: `${vRow.size}px`,
                    transform: `translateY(${vRow.start}px)`,
                  }}
                  onDoubleClick={() => {
                    if (item.isDir) {
                      setCurrentDir(item.path);
                    } else {
                      onPreviewFile(item.path);
                    }
                  }}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    if (!selectedPaths.has(item.path)) {
                      selectAll([item.path]);
                    }
                    const menuW = 228;
                    const menuH = 290;
                    setContextMenu({
                      x: Math.min(e.clientX, window.innerWidth - menuW - 8),
                      y: Math.min(e.clientY, window.innerHeight - menuH - 8),
                      item,
                    });
                  }}
                  className={`grid grid-cols-[36px_1fr_110px_160px_68px] items-center border-b border-zinc-100/80 px-3 text-sm transition select-none dark:border-zinc-900 ${
                    checked
                      ? "bg-blue-50/70 dark:bg-blue-950/30"
                      : "hover:bg-zinc-50 dark:hover:bg-zinc-900/60"
                  }`}
                >
                  <div className="flex justify-center">
                    <input
                      type="checkbox"
                      checked={checked}
                      onChange={() => toggleSelect(item.path)}
                      className="h-3.5 w-3.5 cursor-pointer rounded accent-blue-600"
                    />
                  </div>

                  <div className="flex min-w-0 items-center gap-2.5 pr-3">
                    {getFileIcon(item.name, item.isDir)}
                    <span
                      className="truncate text-[13px] text-zinc-800 dark:text-zinc-200"
                      title={item.path}
                    >
                      {item.name}
                    </span>
                    {item.encrypted && (
                      <Lock
                        className="h-3 w-3 shrink-0 text-amber-500"
                        aria-label="已加密"
                      />
                    )}
                    {item.isDir && item.childCount !== undefined && item.childCount > 0 && (
                      <span className="shrink-0 text-[11px] text-zinc-400">
                        {item.childCount} 项
                      </span>
                    )}
                  </div>

                  <div className="text-right font-mono text-xs text-zinc-500 dark:text-zinc-400">
                    {item.isDir && item.size === 0 ? "—" : formatBytes(item.size)}
                  </div>

                  <div className="pr-2 text-right font-mono text-xs text-zinc-400 dark:text-zinc-500">
                    {item.modified ?? "—"}
                  </div>

                  <div className="flex justify-center">
                    {!item.isDir && (
                      <button
                        type="button"
                        onClick={() => onPreviewFile(item.path)}
                        title="快速预览"
                        className="cursor-pointer rounded-md p-1 text-zinc-400 transition hover:bg-zinc-200/70 hover:text-blue-600 dark:hover:bg-zinc-800 dark:hover:text-blue-400"
                      >
                        <Eye className="h-3.5 w-3.5" />
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* 底部状态栏 */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-zinc-200/80 bg-zinc-50/80 px-4 py-2 text-xs text-zinc-500 dark:border-zinc-800 dark:bg-zinc-900/70 dark:text-zinc-400">
        <div className="flex items-center gap-3">
          <span>
            共 <strong className="text-zinc-700 dark:text-zinc-200">{archive.fileCount}</strong> 个文件
            {archive.dirCount > 0 ? `，${archive.dirCount} 个文件夹` : ""}
          </span>
          <span>·</span>
          <span>
            原始 {formatBytes(archive.totalUncompressedSize)} / 压缩后{" "}
            {formatBytes(archive.archiveSize)}
            {ratio > 0 ? ` (省 ${ratio}%)` : ""}
          </span>
          <span>·</span>
          <span>{archive.engine}</span>
        </div>

        <div className="flex items-center gap-1.5 text-blue-600 dark:text-blue-400">
          <Sparkles className="h-3.5 w-3.5" />
          <span>
            {archive.smartExtractWillCreateFolder
              ? `智能解压将自动新建 "${archive.suggestedFolderName}/"`
              : "包内结构整洁，智能解压将直接展开"}
          </span>
        </div>
      </div>
      {/* 自定义文件列表右键菜单 */}
      {contextMenu && (
        <div
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
          className="fixed z-50 w-56 overflow-hidden rounded-xl border border-zinc-200 bg-white py-1.5 text-xs text-zinc-700 shadow-xl dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200"
        >
          {contextMenu.item && (
            <>
              <button
                type="button"
                onClick={() => {
                  const target = contextMenu.item!;
                  setContextMenu(null);
                  if (target.isDir) {
                    setCurrentDir(target.path);
                  } else {
                    onPreviewFile(target.path);
                  }
                }}
                className="flex w-full cursor-pointer items-center gap-2.5 px-3 py-2 text-left font-medium hover:bg-blue-50 hover:text-blue-600 dark:hover:bg-blue-950/50 dark:hover:text-blue-400"
              >
                {contextMenu.item.isDir ? (
                  <>
                    <FolderOpen className="h-3.5 w-3.5 text-blue-500" />
                    <span>打开文件夹</span>
                  </>
                ) : (
                  <>
                    <Eye className="h-3.5 w-3.5 text-blue-500" />
                    <span>快速预览</span>
                  </>
                )}
              </button>

              <button
                type="button"
                onClick={() => {
                  setContextMenu(null);
                  onExtractSelected();
                }}
                className="flex w-full cursor-pointer items-center gap-2.5 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
              >
                <FolderOutput className="h-3.5 w-3.5 text-zinc-500" />
                <span>
                  解压选中项到…
                  {selectedPaths.size > 1 ? ` (${selectedPaths.size} 项)` : ""}
                </span>
              </button>

              <div className="my-1 border-t border-zinc-100 dark:border-zinc-800" />
            </>
          )}

          <button
            type="button"
            onClick={() => {
              setContextMenu(null);
              onSmartExtractAll();
            }}
            className="flex w-full cursor-pointer items-center gap-2.5 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
          >
            <Sparkles className="h-3.5 w-3.5 text-blue-500" />
            <span>全部智能解压到同级目录</span>
          </button>

          <button
            type="button"
            onClick={() => {
              setContextMenu(null);
              onExtractAllToFolder();
            }}
            className="flex w-full cursor-pointer items-center gap-2.5 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
          >
            <FolderOutput className="h-3.5 w-3.5 text-zinc-500" />
            <span>全部解压到指定目录…</span>
          </button>

          {contextMenu.item && (
            <>
              <div className="my-1 border-t border-zinc-100 dark:border-zinc-800" />
              <button
                type="button"
                onClick={() => {
                  const name = contextMenu.item!.name;
                  setContextMenu(null);
                  navigator.clipboard
                    .writeText(name)
                    .then(() =>
                      showToast({ type: "success", title: "已复制文件名", detail: name })
                    )
                    .catch(() => {});
                }}
                className="flex w-full cursor-pointer items-center gap-2.5 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
              >
                <Copy className="h-3.5 w-3.5 text-zinc-400" />
                <span>复制文件名</span>
              </button>

              <button
                type="button"
                onClick={() => {
                  const p = contextMenu.item!.path;
                  setContextMenu(null);
                  navigator.clipboard
                    .writeText(p)
                    .then(() =>
                      showToast({ type: "success", title: "已复制包内路径", detail: p })
                    )
                    .catch(() => {});
                }}
                className="flex w-full cursor-pointer items-center gap-2.5 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
              >
                <Copy className="h-3.5 w-3.5 text-zinc-400" />
                <span>复制包内路径</span>
              </button>
            </>
          )}

          <div className="my-1 border-t border-zinc-100 dark:border-zinc-800" />

          <button
            type="button"
            onClick={() => {
              setContextMenu(null);
              if (allSelected) {
                clearSelection();
              } else {
                selectAll(items.map((i) => i.path));
              }
            }}
            className="flex w-full cursor-pointer items-center gap-2.5 px-3 py-2 text-left hover:bg-zinc-100 dark:hover:bg-zinc-800"
          >
            {allSelected ? (
              <>
                <Square className="h-3.5 w-3.5 text-zinc-400" />
                <span>取消全选</span>
              </>
            ) : (
              <>
                <CheckSquare className="h-3.5 w-3.5 text-zinc-400" />
                <span>全选当前目录</span>
              </>
            )}
          </button>
        </div>
      )}
    </div>
  );
}
