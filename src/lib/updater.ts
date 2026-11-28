import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { api } from "./api";

export const MICROSOFT_STORE_WEB_URL =
  "https://apps.microsoft.com/detail/9NHLSGG1BZMZ";
export const MICROSOFT_STORE_DEEP_LINK =
  "ms-windows-store://pdp/?productid=9NHLSGG1BZMZ";

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
  percent: number | null;
}

let cachedStoreBuild: boolean | null = null;

/**
 * 判断当前是否由 Microsoft Store 托管分发与更新（MSIX 商店版）
 */
export async function isStoreManagedUpdate(): Promise<boolean> {
  if (cachedStoreBuild !== null) {
    return cachedStoreBuild;
  }
  try {
    cachedStoreBuild = await api.isMicrosoftStoreBuild();
  } catch {
    cachedStoreBuild = false;
  }
  return cachedStoreBuild;
}

/**
 * 检查是否有新版本可用；Microsoft Store 版由微软商店统一托管更新，直接跳过外部自更新
 */
export async function checkForAppUpdate(): Promise<Update | null> {
  if (await isStoreManagedUpdate()) {
    return null;
  }
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
          percent:
            total && total > 0
              ? Math.min(100, Math.round((downloaded / total) * 100))
              : null,
        });
        break;
      case "Finished":
        onProgress?.({ downloaded: total ?? downloaded, total, percent: 100 });
        break;
    }
  });

  await relaunch();
}
