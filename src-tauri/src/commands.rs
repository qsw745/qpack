use crate::archive;
use crate::error::{QPackError, QResult};
use crate::models::{
    ArchiveFormat, ArchiveInfo, CompressFormat, CompressOptions, CompressResult, EngineStatus,
    ExtractOptions, ExtractResult, TaskProgress,
};
use crate::platform::{self, SystemIntegrationStatus};
use crate::settings::{self, AppSettings};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

/// 存放应用冷启动时通过命令行/系统打开传入的待处理路径，供前端就绪后拉取
#[derive(Default)]
pub struct PendingLaunchState {
    pub paths: Mutex<Vec<String>>,
    pub auto_extract: Mutex<bool>,
}

/// 冷启动拉取结果
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingLaunchPayload {
    pub paths: Vec<String>,
    pub auto_extract: bool,
}

/// 读取冷启动待打开路径并清空
#[tauri::command]
pub fn take_pending_launch_paths(state: State<'_, PendingLaunchState>) -> PendingLaunchPayload {
    let mut paths_guard = state.paths.lock().unwrap();
    let mut auto_guard = state.auto_extract.lock().unwrap();
    let paths = std::mem::take(&mut *paths_guard);
    let auto_extract = std::mem::take(&mut *auto_guard);
    PendingLaunchPayload {
        paths,
        auto_extract,
    }
}

/// 检测解压目标目录是否已存在同名文件/文件夹冲突
#[tauri::command]
pub async fn check_extract_conflict(
    options: ExtractOptions,
) -> QResult<archive::ExtractConflictInfo> {
    tokio::task::spawn_blocking(move || archive::check_extract_conflict(&options))
        .await
        .map_err(|e| QPackError::ArchiveError(e.to_string()))?
}

/// 在系统文件管理器中打开目录
#[tauri::command]
pub fn open_output_directory(path: String) {
    platform::open_directory(&path);
}

/// 获取用户偏好设置
#[tauri::command]
pub async fn get_app_settings() -> AppSettings {
    tokio::task::spawn_blocking(settings::load_settings)
        .await
        .unwrap_or_default()
}

/// 保存用户偏好设置
#[tauri::command]
pub async fn update_app_settings(new_settings: AppSettings) -> QResult<AppSettings> {
    tokio::task::spawn_blocking(move || {
        settings::save_settings(&new_settings)?;
        Ok(new_settings)
    })
    .await
    .map_err(|e| QPackError::ArchiveError(e.to_string()))?
}

/// 获取系统右键菜单 / 访达快速操作集成状态
#[tauri::command]
pub async fn get_system_integration_status() -> SystemIntegrationStatus {
    tokio::task::spawn_blocking(platform::get_integration_status)
        .await
        .unwrap_or_else(|_| platform::get_integration_status())
}

/// 安装系统右键菜单 / 访达快速操作
#[tauri::command]
pub async fn install_system_context_menu() -> QResult<SystemIntegrationStatus> {
    tokio::task::spawn_blocking(platform::install_context_menu)
        .await
        .map_err(|e| QPackError::ArchiveError(e.to_string()))?
}

/// 卸载系统右键菜单 / 访达快速操作
#[tauri::command]
pub async fn uninstall_system_context_menu() -> QResult<SystemIntegrationStatus> {
    tokio::task::spawn_blocking(platform::uninstall_context_menu)
        .await
        .map_err(|e| QPackError::ArchiveError(e.to_string()))?
}

/// 探测引擎状态与支持格式
#[tauri::command]
pub fn detect_engine_status() -> EngineStatus {
    archive::get_engine_status()
}

/// 检查路径是压缩包还是普通文件/文件夹
#[tauri::command]
pub fn is_supported_archive(path: String) -> bool {
    let p = Path::new(&path);
    p.is_file() && ArchiveFormat::from_path(p) != ArchiveFormat::Unknown
}

/// 打开并解析压缩包内容（支持指定编码与密码）
#[tauri::command]
pub async fn inspect_archive(
    path: String,
    encoding: Option<String>,
    password: Option<String>,
) -> QResult<ArchiveInfo> {
    tokio::task::spawn_blocking(move || {
        archive::inspect_archive(Path::new(&path), encoding.as_deref(), password.as_deref())
    })
    .await
    .map_err(|e| QPackError::ArchiveError(e.to_string()))?
}

/// 执行解压并实时通过事件推送进度
#[tauri::command]
pub async fn extract_archive(
    app: AppHandle,
    options: ExtractOptions,
) -> QResult<ExtractResult> {
    let task_id = Uuid::new_v4().to_string();

    tokio::task::spawn_blocking(move || {
        let mut last_percent: i32 = -1;
        let mut emit_progress = |current_file: &str, processed: usize, total: usize| {
            let pct = if total == 0 {
                100.0
            } else {
                ((processed as f64 / total as f64) * 100.0).min(100.0)
            };
            let bucket = pct as i32;
            if bucket != last_percent || processed >= total {
                last_percent = bucket;
                let _ = app.emit(
                    "qpack://progress",
                    TaskProgress {
                        task_id: task_id.clone(),
                        kind: "extract".into(),
                        current_file: current_file.to_string(),
                        processed_items: processed,
                        total_items: total,
                        percent: pct,
                        done: processed >= total,
                    },
                );
            }
        };

        archive::extract_archive(&options, &mut emit_progress)
    })
    .await
    .map_err(|e| QPackError::ArchiveError(e.to_string()))?
}

/// 执行压缩并实时通过事件推送进度
#[tauri::command]
pub async fn compress_files(
    app: AppHandle,
    options: CompressOptions,
) -> QResult<CompressResult> {
    let task_id = Uuid::new_v4().to_string();

    tokio::task::spawn_blocking(move || {
        let mut last_percent: i32 = -1;
        let mut emit_progress = |current_file: &str, processed: usize, total: usize| {
            let pct = if total == 0 {
                100.0
            } else {
                ((processed as f64 / total as f64) * 100.0).min(100.0)
            };
            let bucket = pct as i32;
            if bucket != last_percent || processed >= total {
                last_percent = bucket;
                let _ = app.emit(
                    "qpack://progress",
                    TaskProgress {
                        task_id: task_id.clone(),
                        kind: "compress".into(),
                        current_file: current_file.to_string(),
                        processed_items: processed,
                        total_items: total,
                        percent: pct,
                        done: processed >= total,
                    },
                );
            }
        };

        archive::compress_files(&options, &mut emit_progress)
    })
    .await
    .map_err(|e| QPackError::ArchiveError(e.to_string()))?
}

/// 提取单个文件到临时目录并返回路径，供系统默认应用快速预览
#[tauri::command]
pub async fn preview_entry(
    archive_path: String,
    entry_path: String,
    password: Option<String>,
    encoding: Option<String>,
) -> QResult<String> {
    tokio::task::spawn_blocking(move || {
        let temp_root = std::env::temp_dir()
            .join("qpack-preview")
            .join(Uuid::new_v4().to_string());

        let opts = ExtractOptions {
            archive_path,
            dest_dir: temp_root.to_string_lossy().into_owned(),
            smart_extract: false,
            password,
            encoding,
            selected_paths: Some(vec![entry_path.clone()]),
            overwrite: true,
        };

        let mut noop = |_: &str, _: usize, _: usize| {};
        archive::extract_archive(&opts, &mut noop)?;

        let extracted = archive::smart::safe_join(&temp_root, &entry_path)?;
        if !extracted.exists() {
            return Err(QPackError::NotFound(entry_path));
        }

        Ok(extracted.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| QPackError::ArchiveError(e.to_string()))?
}

/// 根据输入路径列表，智能推荐默认压缩包输出路径
#[tauri::command]
pub fn suggest_compress_output(paths: Vec<String>, format: CompressFormat) -> String {
    suggest_output_path(&paths, format)
}

pub fn suggest_output_path(paths: &[String], format: CompressFormat) -> String {
    let ext = match format {
        CompressFormat::Zip => "zip",
        CompressFormat::SevenZ => "7z",
        CompressFormat::TarGz => "tar.gz",
        CompressFormat::TarZst => "tar.zst",
        CompressFormat::Tar => "tar",
    };

    let Some(first) = paths.first() else {
        return format!("archive.{}", ext);
    };

    let first_path = PathBuf::from(first);
    let parent = first_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let stem = if paths.len() == 1 {
        first_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("archive")
            .to_string()
    } else {
        parent
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("archive")
            .to_string()
    };

    parent
        .join(format!("{}.{}", stem, ext))
        .to_string_lossy()
        .into_owned()
}
