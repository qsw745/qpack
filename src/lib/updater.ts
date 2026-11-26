import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
  percent: number | null;
}

/**
 * 检查是否有新版本可用；无更新返回 null
 */
export async function checkForAppUpdate(): Promise<Update | null> {
  const update = await check();
  return update ?? null;
}

/**
 * 下载并安装更新，完成后自动重启 QPack
 */
export async function downloadAndInstallUpdate(
  update: Update,
  onProgress?: (progress: UpdateProgress) => void
): Promise<void> {
  let downloaded = 0;
  let total: number | null = null;

  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        total = event.data.contentLength ?? null;
        downloaded = 0;
        onProgress?.({ downloaded, total, percent: total ? 0 : null });
        break;
      case "Progress":
        downloaded += event.data.chunkLength;
        onProgress?.({
          downloaded,
          total,
          percent: total && total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : null,
        });
        break;
      case "Finished":
        onProgress?.({ downloaded: total ?? downloaded, total, percent: 100 });
        break;
    }
  });

  await relaunch();
}
