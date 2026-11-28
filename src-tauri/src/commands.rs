use crate::archive;
use crate::archive::task_control::{ProgressSnapshot, TaskController, TaskRegistry};
use crate::error::{QPackError, QResult};
use crate::models::{
    ArchiveFormat, ArchiveInfo, CompressFormat, CompressOptions, CompressResult, EngineStatus,
    ExtractMode, ExtractOptions, ExtractResult, TaskProgress,
};
use crate::platform::{self, SystemIntegrationStatus};
use crate::settings::{self, AppSettings};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, Size, State};
use uuid::Uuid;

/// 存放应用冷启动时通过命令行/系统打开传入的待处理任务，供前端就绪后拉取
pub struct PendingLaunchState {
    pub payload: Mutex<Option<PendingLaunchPayload>>,
    pub frontend_ready: AtomicBool,
}

impl Default for PendingLaunchState {
    fn default() -> Self {
        Self {
            payload: Mutex::new(None),
            frontend_ready: AtomicBool::new(false),
        }
    }
}

/// 标记当前窗口是否处于独立紧凑进度窗模式
#[derive(Default)]
pub struct StandaloneModeState(pub AtomicBool);

/// 标记系统托盘是否已创建，避免重复注册
#[derive(Default)]
pub struct TrayCreatedState(pub AtomicBool);

/// 冷启动 / 热唤起任务载荷
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingLaunchPayload {
    pub paths: Vec<String>,
    /// 启动模式: "open" | "extract-smart" | "extract-here" | "extract-subfolder" | "extract-to"
    ///         | "compress-zip" | "compress-7z" | "compress-dialog"
    pub mode: String,
    pub standalone: bool,
}

impl Default for PendingLaunchPayload {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            mode: "open".into(),
            standalone: false,
        }
    }
}

/// 任务注销守卫，确保任务正常结束、报错或取消时都会从 TaskRegistry 清理
struct TaskGuard<'a> {
    registry: &'a TaskRegistry,
    task_id: String,
}

impl Drop for TaskGuard<'_> {
    fn drop(&mut self) {
        self.registry.remove(&self.task_id);
    }
}

fn snapshot_to_progress(task_id: &str, kind: &str, snap: &ProgressSnapshot) -> TaskProgress {
    let percent = if snap.done {
        100.0
    } else if snap.total_bytes > 0 {
        ((snap.processed_bytes as f64 / snap.total_bytes as f64) * 100.0).min(100.0)
    } else if snap.total_items > 0 {
        ((snap.processed_items as f64 / snap.total_items as f64) * 100.0).min(100.0)
    } else {
        0.0
    };

    let current_file_percent = if snap.done {
        100.0
    } else if snap.current_file_total_bytes > 0 {
        ((snap.current_file_bytes as f64 / snap.current_file_total_bytes as f64) * 100.0).min(100.0)
    } else {
        0.0
    };

    TaskProgress {
        task_id: task_id.to_string(),
        kind: kind.to_string(),
        current_file: snap.current_file.clone(),
        processed_items: snap.processed_items,
        total_items: snap.total_items,
        processed_bytes: snap.processed_bytes,
        total_bytes: snap.total_bytes,
        current_file_bytes: snap.current_file_bytes,
        current_file_total_bytes: snap.current_file_total_bytes,
        current_file_percent,
        percent,
        elapsed_ms: snap.elapsed_ms,
        speed_bps: snap.speed_bps,
        paused: snap.paused,
        done: snap.done,
    }
}

/// 读取冷启动待处理任务并标记前端已就绪
#[tauri::command]
pub fn take_pending_launch_paths(state: State<'_, PendingLaunchState>) -> PendingLaunchPayload {
    state.frontend_ready.store(true, Ordering::SeqCst);
    let mut guard = state.payload.lock().unwrap();
    guard.take().unwrap_or_default()
}

/// 暂停进行中的任务
#[tauri::command]
pub fn pause_task(registry: State<'_, TaskRegistry>, task_id: String) -> bool {
    registry.pause(&task_id)
}

/// 恢复已暂停的任务
#[tauri::command]
pub fn resume_task(registry: State<'_, TaskRegistry>, task_id: String) -> bool {
    registry.resume(&task_id)
}

/// 取消进行中的任务
#[tauri::command]
pub fn cancel_task(registry: State<'_, TaskRegistry>, task_id: String) -> bool {
    registry.cancel(&task_id)
}

/// 从独立紧凑任务窗切换到完整主窗口（并按需补建系统托盘）
#[tauri::command]
pub fn switch_to_main_window(
    app: AppHandle,
    standalone: State<'_, StandaloneModeState>,
    tray_created: State<'_, TrayCreatedState>,
) -> QResult<()> {
    standalone.0.store(false, Ordering::SeqCst);

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(false);
        let _ = win.set_min_size(Some(Size::Logical(LogicalSize::new(720.0, 500.0))));
        let _ = win.set_size(Size::Logical(LogicalSize::new(1020.0, 680.0)));
        let _ = win.set_title("QPack");
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
    }

    if !tray_created.0.swap(true, Ordering::SeqCst) {
        let _ = crate::setup_tray(&app);
    }

    Ok(())
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

/// 在系统文件管理器中定位并高亮选中目标文件/文件夹
#[tauri::command]
pub fn reveal_in_file_manager(path: String) {
    platform::reveal_in_file_manager(&path);
}

/// 将指定文件/文件夹移入系统废纸篓 / 回收站
#[tauri::command]
pub async fn trash_paths(paths: Vec<String>) -> QResult<()> {
    tokio::task::spawn_blocking(move || platform::trash_paths(&paths))
        .await
        .map_err(|e| QPackError::ArchiveError(e.to_string()))?
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

/// 刷新 / 修复多格式压缩包文件类型图标
#[tauri::command]
pub async fn refresh_file_icons() -> QResult<()> {
    tokio::task::spawn_blocking(|| platform::refresh_file_type_icons(true))
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

/// 执行解压并实时通过事件推送字节级进度（支持暂停 / 恢复 / 取消）
#[tauri::command]
pub async fn extract_archive(
    app: AppHandle,
    options: ExtractOptions,
) -> QResult<ExtractResult> {
    let task_id = options
        .task_id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        let registry = app_clone.state::<TaskRegistry>();
        let control = registry.register(&task_id);
        let _guard = TaskGuard {
            registry: &registry,
            task_id: task_id.clone(),
        };

        let mut emit_progress = |snap: &ProgressSnapshot| {
            let payload = snapshot_to_progress(&task_id, "extract", snap);
            let _ = app_clone.emit("qpack://progress", payload);
        };

        archive::extract_archive(&options, control, &mut emit_progress)
    })
    .await
    .map_err(|e| QPackError::ArchiveError(e.to_string()))?
}

/// 执行压缩并实时通过事件推送字节级进度（支持暂停 / 恢复 / 取消）
#[tauri::command]
pub async fn compress_files(
    app: AppHandle,
    options: CompressOptions,
) -> QResult<CompressResult> {
    let task_id = options
        .task_id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        let registry = app_clone.state::<TaskRegistry>();
        let control = registry.register(&task_id);
        let _guard = TaskGuard {
            registry: &registry,
            task_id: task_id.clone(),
        };

        let mut emit_progress = |snap: &ProgressSnapshot| {
            let payload = snapshot_to_progress(&task_id, "compress", snap);
            let _ = app_clone.emit("qpack://progress", payload);
        };

        archive::compress_files(&options, control, &mut emit_progress)
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
            extract_mode: Some(ExtractMode::Here),
            task_id: None,
        };

        let mut noop = |_: &ProgressSnapshot| {};
        archive::extract_archive(&opts, TaskController::noop(), &mut noop)?;

        let extracted = archive::smart::safe_join(&temp_root, &entry_path)?;
        if !extracted.exists() {
            return Err(QPackError::NotFound(entry_path));
        }

        Ok(extracted.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| QPackError::ArchiveError(e.to_string()))?
}

/// 根据输入路径列表，智能推荐不冲突的默认压缩包输出路径
#[tauri::command]
pub fn suggest_compress_output(paths: Vec<String>, format: CompressFormat) -> String {
    suggest_output_path(&paths, format)
}

pub fn suggest_output_path(paths: &[String], format: CompressFormat) -> String {
    let ext = match format {
        CompressFormat::Zip => "zip",
        CompressFormat::SevenZ => "7z",
        CompressFormat::TarGz => "tar.gz",
        CompressFormat::TarBz2 => "tar.bz2",
        CompressFormat::TarXz => "tar.xz",
        CompressFormat::TarZst => "tar.zst",
        CompressFormat::Tar => "tar",
        CompressFormat::Dmg => "dmg",
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
        if first_path.is_dir() {
            first_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("archive")
                .to_string()
        } else {
            first_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("archive")
                .to_string()
        }
    } else {
        parent
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("archive")
            .to_string()
    };

    let candidate = parent.join(format!("{}.{}", stem, ext));
    if !candidate.exists() {
        return candidate.to_string_lossy().into_owned();
    }

    for n in 2..=999 {
        let alt = parent.join(format!("{} ({}).{}", stem, n, ext));
        if !alt.exists() {
            return alt.to_string_lossy().into_owned();
        }
    }

    candidate.to_string_lossy().into_owned()
}
