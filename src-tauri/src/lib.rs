pub mod archive;
pub mod commands;
pub mod error;
pub mod models;
pub mod platform;
pub mod settings;

use crate::commands::PendingLaunchState;
use crate::error::QPackError;
use crate::models::{ArchiveFormat, CompressFormat, CompressOptions, ExtractOptions};
use crate::settings::DoubleClickAction;
use std::path::{Path, PathBuf};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(target_os = "macos")]
use tauri::RunEvent;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

/// 无界面后台执行智能解压；若压缩包加密返回需要交由 GUI 打开的路径
fn headless_smart_extract(paths: &[String]) -> Vec<String> {
    let mut need_gui = Vec::new();

    for raw in paths {
        let p = Path::new(raw);
        if !p.is_file() || ArchiveFormat::from_path(p) == ArchiveFormat::Unknown {
            need_gui.push(raw.clone());
            continue;
        }
        let dest = p
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let opts = ExtractOptions {
            archive_path: raw.clone(),
            dest_dir: dest.to_string_lossy().into_owned(),
            smart_extract: true,
            password: None,
            encoding: None,
            selected_paths: None,
            overwrite: true,
        };
        let mut noop = |_: &str, _: usize, _: usize| {};
        match archive::extract_archive(&opts, &mut noop) {
            Ok(res) => {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("压缩包");
                platform::notify_user(
                    "QPack 解压完成",
                    &format!("{} 已智能解压至 {}", name, res.output_dir),
                );
                platform::open_directory(&res.output_dir);
            }
            Err(QPackError::InvalidPassword) => {
                // 加密压缩包交由 GUI 弹出密码框
                need_gui.push(raw.clone());
            }
            Err(err) => {
                platform::notify_user("QPack 解压失败", &err.to_string());
            }
        }
    }

    need_gui
}

/// 无界面后台执行快速压缩为 ZIP
fn headless_quick_compress(paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    let cfg = settings::load_settings();
    let out = commands::suggest_output_path(paths, CompressFormat::Zip);
    let opts = CompressOptions {
        input_paths: paths.to_vec(),
        output_path: out.clone(),
        format: CompressFormat::Zip,
        level: 6,
        password: None,
        exclude_junk: cfg.exclude_junk,
    };
    let mut noop = |_: &str, _: usize, _: usize| {};
    match archive::compress_files(&opts, &mut noop) {
        Ok(res) => {
            let name = Path::new(&res.output_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("archive.zip");
            platform::notify_user(
                "QPack 压缩完成",
                &format!("已生成 {}（共 {} 个文件）", name, res.input_files),
            );
        }
        Err(err) => {
            platform::notify_user("QPack 压缩失败", &err.to_string());
        }
    }
}

/// 处理命令行参数：若可直接后台静默完成则返回 None（直接退出进程），否则返回 (需交给 GUI 的路径, 是否自动触发解压)
fn handle_cli_args_before_gui() -> Option<(Vec<String>, bool)> {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("-psn_"))
        .collect();

    if args.is_empty() {
        return Some((Vec::new(), false));
    }

    match args[0].as_str() {
        "--extract" => {
            // 右键菜单触发解压时唤起窗口展示实时进度与冲突提示，解压完自动打开目标目录
            let files: Vec<String> = args[1..].to_vec();
            return Some((files, true));
        }
        "--compress" => {
            let files: Vec<String> = args[1..].to_vec();
            headless_quick_compress(&files);
            if let Some(first) = files.first() {
                if let Some(parent) = Path::new(first).parent() {
                    platform::open_directory(&parent.to_string_lossy());
                }
            }
            return None;
        }
        _ => {
            let cfg = settings::load_settings();
            if cfg.double_click_action == DoubleClickAction::SmartExtract
                && args
                    .iter()
                    .all(|p| Path::new(p).is_file() && ArchiveFormat::from_path(Path::new(p)) != ArchiveFormat::Unknown)
            {
                let need_gui = headless_smart_extract(&args);
                if need_gui.is_empty() {
                    return None;
                }
                return Some((need_gui, false));
            }
            return Some((args, false));
        }
    }
}

/// 唤起并聚焦主窗口
fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

/// 处理运行期间通过 macOS 访达双击 / “打开方式”传入的文件列表
#[cfg(target_os = "macos")]
fn handle_opened_paths(app: &AppHandle, paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }

    let cfg = settings::load_settings();
    let all_archives = paths
        .iter()
        .all(|p| Path::new(p).is_file() && ArchiveFormat::from_path(Path::new(p)) != ArchiveFormat::Unknown);

    if all_archives && cfg.double_click_action == DoubleClickAction::SmartExtract {
        let handle = app.clone();
        std::thread::spawn(move || {
            let need_gui = headless_smart_extract(&paths);
            if !need_gui.is_empty() {
                show_main_window(&handle);
                let _ = handle.emit("qpack://open-paths", need_gui);
            }
        });
    } else {
        show_main_window(app);
        let _ = app.emit("qpack://open-paths", paths);
    }
}

/// 构建跨平台系统托盘 / macOS 菜单栏常驻图标
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "打开 QPack", true, None::<&str>)?;
    let compress_item = MenuItem::with_id(app, "compress", "新建压缩包...", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出 QPack", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_item, &compress_item, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("QPack - 极简跨平台解压缩工具")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "compress" => {
                show_main_window(app);
                let _ = app.emit("qpack://trigger-compress", ());
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    #[cfg(target_os = "windows")]
    let tray_icon_bytes: &[u8] = include_bytes!("../icons/win-tray-64.png");
    #[cfg(not(target_os = "windows"))]
    let tray_icon_bytes: &[u8] = include_bytes!("../icons/64x64.png");

    if let Ok(tray_icon) = tauri::image::Image::from_bytes(tray_icon_bytes) {
        builder = builder.icon(tray_icon);
    } else if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 优先处理静默命令行模式（双击静默解压 / 访达快速操作 / Windows 右键菜单）
    let Some((initial_paths, auto_extract)) = handle_cli_args_before_gui() else {
        return;
    };

    let app = tauri::Builder::default()
        .manage(PendingLaunchState {
            paths: std::sync::Mutex::new(initial_paths),
            auto_extract: std::sync::Mutex::new(auto_extract),
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // 显式为主窗口设置高清图标并确保在任务栏正常显示（Windows 使用满幅图标避免偏小）
            if let Some(win) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                let win_icon_bytes: &[u8] = include_bytes!("../icons/win-taskbar-128.png");
                #[cfg(not(target_os = "windows"))]
                let win_icon_bytes: &[u8] = include_bytes!("../icons/128x128.png");

                if let Ok(win_icon) = tauri::image::Image::from_bytes(win_icon_bytes) {
                    let _ = win.set_icon(win_icon);
                }
                let _ = win.set_skip_taskbar(false);
            }
            setup_tray(app.handle())?;
            platform::ensure_system_context_menu_on_startup();
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let cfg = settings::load_settings();
                if cfg.close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::take_pending_launch_paths,
            commands::check_extract_conflict,
            commands::open_output_directory,
            commands::get_app_settings,
            commands::update_app_settings,
            commands::get_system_integration_status,
            commands::install_system_context_menu,
            commands::uninstall_system_context_menu,
            commands::detect_engine_status,
            commands::is_supported_archive,
            commands::inspect_archive,
            commands::extract_archive,
            commands::compress_files,
            commands::preview_entry,
            commands::suggest_compress_output,
        ])
        .build(tauri::generate_context!())
        .expect("初始化 QPack 应用失败");

    app.run(|app_handle, event| {
        #[cfg(target_os = "macos")]
        match event {
            RunEvent::Opened { urls } => {
                let paths: Vec<String> = urls
                    .into_iter()
                    .filter_map(|u| u.to_file_path().ok())
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                handle_opened_paths(app_handle, paths);
            }
            RunEvent::Reopen {
                has_visible_windows,
                ..
            } => {
                if !has_visible_windows {
                    show_main_window(app_handle);
                }
            }
            _ => {}
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (app_handle, event);
        }
    });
}
