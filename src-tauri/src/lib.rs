pub mod archive;
pub mod commands;
pub mod error;
pub mod models;
pub mod platform;
pub mod settings;

use crate::archive::task_control::TaskRegistry;
use crate::commands::{
    PendingLaunchPayload, PendingLaunchState, StandaloneModeState, TrayCreatedState,
};
use crate::models::ArchiveFormat;
use crate::settings::DoubleClickAction;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(target_os = "macos")]
use tauri::RunEvent;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, Size, WindowEvent};

/// 判断给定路径列表是否全部为受支持的压缩包文件
fn are_all_archives(paths: &[String]) -> bool {
    !paths.is_empty()
        && paths.iter().all(|p| {
            let path = Path::new(p);
            path.is_file() && ArchiveFormat::from_path(path) != ArchiveFormat::Unknown
        })
}

/// 解析命令行参数为统一的启动任务载荷（所有右键与双击任务均通过图形进度窗口呈现）
fn parse_cli_launch() -> PendingLaunchPayload {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("-psn_"))
        .collect();

    if args.is_empty() {
        return PendingLaunchPayload::default();
    }

    let first = args[0].as_str();
    let rest = args[1..].to_vec();

    match first {
        "--extract" | "--extract-smart" => PendingLaunchPayload {
            paths: rest,
            mode: "extract-smart".into(),
            standalone: true,
        },
        "--extract-here" => PendingLaunchPayload {
            paths: rest,
            mode: "extract-here".into(),
            standalone: true,
        },
        "--extract-subfolder" => PendingLaunchPayload {
            paths: rest,
            mode: "extract-subfolder".into(),
            standalone: true,
        },
        "--extract-to" => PendingLaunchPayload {
            paths: rest,
            mode: "extract-to".into(),
            standalone: true,
        },
        "--compress" | "--compress-zip" => PendingLaunchPayload {
            paths: rest,
            mode: "compress-zip".into(),
            standalone: true,
        },
        "--compress-7z" => PendingLaunchPayload {
            paths: rest,
            mode: "compress-7z".into(),
            standalone: true,
        },
        "--compress-dialog" => PendingLaunchPayload {
            paths: rest,
            mode: "compress-dialog".into(),
            standalone: false,
        },
        _ => {
            let cfg = settings::load_settings();
            if cfg.double_click_action == DoubleClickAction::SmartExtract
                && are_all_archives(&args)
            {
                PendingLaunchPayload {
                    paths: args,
                    mode: "extract-smart".into(),
                    standalone: true,
                }
            } else {
                PendingLaunchPayload {
                    paths: args,
                    mode: "open".into(),
                    standalone: false,
                }
            }
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
    let all_archives = are_all_archives(&paths);
    let mode = if all_archives && cfg.double_click_action == DoubleClickAction::SmartExtract {
        "extract-smart".to_string()
    } else {
        "open".to_string()
    };

    let standalone_state = app.state::<StandaloneModeState>();
    let payload = PendingLaunchPayload {
        paths,
        mode,
        standalone: standalone_state.0.load(Ordering::SeqCst),
    };

    let pending = app.state::<PendingLaunchState>();
    if pending.frontend_ready.load(Ordering::SeqCst) {
        show_main_window(app);
        let _ = app.emit("qpack://launch", payload);
    } else {
        *pending.payload.lock().unwrap() = Some(payload);
        show_main_window(app);
    }
}

/// 构建跨平台系统托盘 / macOS 菜单栏常驻图标
pub(crate) fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
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
    let initial_launch = parse_cli_launch();
    let is_standalone = initial_launch.standalone && !initial_launch.paths.is_empty();
    let has_initial_payload = !initial_launch.paths.is_empty();

    let app = tauri::Builder::default()
        .manage(PendingLaunchState {
            payload: Mutex::new(if has_initial_payload {
                Some(initial_launch)
            } else {
                None
            }),
            frontend_ready: AtomicBool::new(false),
        })
        .manage(StandaloneModeState(AtomicBool::new(is_standalone)))
        .manage(TrayCreatedState(AtomicBool::new(false)))
        .manage(TaskRegistry::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            if let Some(win) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                let win_icon_bytes: &[u8] = include_bytes!("../icons/win-taskbar-128.png");
                #[cfg(not(target_os = "windows"))]
                let win_icon_bytes: &[u8] = include_bytes!("../icons/128x128.png");

                if let Ok(win_icon) = tauri::image::Image::from_bytes(win_icon_bytes) {
                    let _ = win.set_icon(win_icon);
                }
                let _ = win.set_skip_taskbar(false);

                if is_standalone {
                    // 独立紧凑进度窗口（类似 Bandizip 进度对话框）
                    let _ = win.set_min_size(Some(Size::Logical(LogicalSize::new(460.0, 300.0))));
                    let _ = win.set_size(Size::Logical(LogicalSize::new(540.0, 350.0)));
                    let _ = win.set_title("QPack");
                    let _ = win.center();
                } else {
                    let _ = win.center();
                }

                let _ = win.show();
                let _ = win.set_focus();
            }

            if !is_standalone {
                let tray_state = app.state::<TrayCreatedState>();
                if !tray_state.0.swap(true, Ordering::SeqCst) {
                    setup_tray(app.handle())?;
                }
                platform::ensure_system_context_menu_on_startup();
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let standalone = window
                    .app_handle()
                    .state::<StandaloneModeState>()
                    .0
                    .load(Ordering::SeqCst);
                // 独立任务窗关闭时直接退出，避免托盘残留
                if standalone {
                    return;
                }
                let cfg = settings::load_settings();
                if cfg.close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::take_pending_launch_paths,
            commands::pause_task,
            commands::resume_task,
            commands::cancel_task,
            commands::switch_to_main_window,
            commands::check_extract_conflict,
            commands::open_output_directory,
            commands::reveal_in_file_manager,
            commands::trash_paths,
            commands::get_app_settings,
            commands::update_app_settings,
            commands::get_system_integration_status,
            commands::install_system_context_menu,
            commands::uninstall_system_context_menu,
            commands::refresh_file_icons,
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
