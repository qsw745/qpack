use crate::error::{QPackError, QResult};
use serde::Serialize;
use std::path::Path;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
use std::process::Command;

/// 系统集成状态（访达快速操作 / Windows 右键菜单）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemIntegrationStatus {
    pub platform: String,
    pub context_menu_installed: bool,
    pub executable_path: String,
    pub hint: String,
}

/// 在系统文件管理器中打开指定文件夹（Windows 资源管理器 / macOS 访达）
pub fn open_directory(path: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(path).spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let normalized = path.replace('/', "\\");
        let _ = Command::new("explorer").arg(normalized).spawn();
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

/// 在系统文件管理器中打开所在目录并高亮选中指定文件或文件夹
pub fn reveal_in_file_manager(path: &str) {
    let target = Path::new(path);

    #[cfg(target_os = "macos")]
    {
        if target.exists() {
            let _ = Command::new("open").args(["-R", path]).spawn();
        } else if let Some(parent) = target.parent() {
            let _ = Command::new("open").arg(parent).spawn();
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let normalized = path.replace('/', "\\");
        if target.exists() {
            // 必须使用 raw_arg，避免 Rust 对包含引号的 /select,"..." 参数二次转义导致无法选中文件
            let _ = Command::new("explorer")
                .raw_arg(format!("/select,\"{}\"", normalized))
                .spawn();
        } else if let Some(parent) = target.parent() {
            let _ = Command::new("explorer")
                .arg(parent.to_string_lossy().replace('/', "\\"))
                .spawn();
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if target.exists() {
            let uri = format!("file://{}", path);
            let ok = Command::new("dbus-send")
                .args([
                    "--session",
                    "--dest=org.freedesktop.FileManager1",
                    "--type=method_call",
                    "/org/freedesktop/FileManager1",
                    "org.freedesktop.FileManager1.ShowItems",
                    &format!("array:string:{}", uri),
                    "string:",
                ])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                return;
            }
        }
        let fallback = if target.is_dir() {
            target
        } else {
            target.parent().unwrap_or(target)
        };
        let _ = Command::new("xdg-open").arg(fallback).spawn();
    }
}

/// 将指定文件或文件夹移入系统废纸篓 / 回收站（用于“解压完成后删除源压缩包”）
pub fn trash_paths(paths: &[String]) -> QResult<()> {
    for raw in paths {
        let p = Path::new(raw);
        if !p.exists() {
            continue;
        }

        #[cfg(target_os = "macos")]
        {
            let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
            let script = format!(
                "tell application \"Finder\" to delete POSIX file \"{}\"",
                escaped
            );
            let status = Command::new("osascript").args(["-e", &script]).status()?;
            if !status.success() {
                return Err(QPackError::ArchiveError(format!(
                    "无法将文件移入废纸篓: {}",
                    raw
                )));
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let escaped = raw.replace('/', "\\").replace('\'', "''");
            let ps_script = if p.is_dir() {
                format!(
                    "Add-Type -AssemblyName Microsoft.VisualBasic; \
                     [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory(\
                     '{}', 'OnlyErrorDialogs', 'SendToRecycleBin')",
                    escaped
                )
            } else {
                format!(
                    "Add-Type -AssemblyName Microsoft.VisualBasic; \
                     [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile(\
                     '{}', 'OnlyErrorDialogs', 'SendToRecycleBin')",
                    escaped
                )
            };
            let status = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
                .creation_flags(CREATE_NO_WINDOW)
                .status()?;
            if !status.success() {
                return Err(QPackError::ArchiveError(format!(
                    "无法将文件移入回收站: {}",
                    raw
                )));
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let ok = Command::new("gio")
                .args(["trash", raw])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
                || Command::new("trash-put")
                    .arg(raw)
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
            if !ok {
                return Err(QPackError::ArchiveError(format!(
                    "无法将文件移入回收站: {}",
                    raw
                )));
            }
        }
    }
    Ok(())
}

/// 发送轻量系统通知
pub fn notify_user(title: &str, body: &str) {
    #[cfg(target_os = "macos")]
    {
        let escaped_title = title.replace('\\', "\\\\").replace('"', "\\\"");
        let escaped_body = body.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            escaped_body, escaped_title
        );
        let _ = Command::new("osascript").args(["-e", &script]).spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = (title, body);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = Command::new("notify-send").args([title, body]).spawn();
    }
}

/// 获取当前可执行文件路径
fn current_exe_str() -> QResult<String> {
    let exe = std::env::current_exe()?;
    Ok(exe.to_string_lossy().into_owned())
}

/// 判断当前进程是否以 Microsoft Store (MSIX) 商店包身份运行
pub fn is_microsoft_store_build() -> bool {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentPackageFullName(
                package_full_name_length: *mut u32,
                package_full_name: *mut u16,
            ) -> i32;
        }
        const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
        let mut len: u32 = 0;
        let rc = unsafe { GetCurrentPackageFullName(&mut len, std::ptr::null_mut()) };
        if rc == 0 || rc == ERROR_INSUFFICIENT_BUFFER {
            return true;
        }
        if let Ok(exe) = current_exe_str() {
            if exe.to_ascii_lowercase().contains("\\windowsapps\\") {
                return true;
            }
        }
        false
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// 查询当前平台的系统集成状态
pub fn get_integration_status() -> SystemIntegrationStatus {
    let exe = current_exe_str().unwrap_or_default();

    #[cfg(target_os = "macos")]
    {
        let installed = macos_services_dir()
            .map(|d| {
                d.join("使用 QPack 智能解压.workflow").exists()
                    && d.join("使用 QPack 压缩为 7Z.workflow").exists()
            })
            .unwrap_or(false);
        SystemIntegrationStatus {
            platform: "macos".into(),
            context_menu_installed: installed,
            executable_path: exe,
            hint: "安装后可在访达右键 ->「快速操作」中直接智能解压、压缩为 ZIP / 7Z 或打开高级压缩；设为默认打开方式后，双击压缩包可直接弹出紧凑进度窗解压。".into(),
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let installed = Command::new("reg")
            .args(["query", r"HKCU\Software\Classes\*\shell\QPackMenu"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        SystemIntegrationStatus {
            platform: "windows".into(),
            context_menu_installed: installed,
            executable_path: exe,
            hint: "注册后可在 Windows 资源管理器右键「QPack」级联菜单中选择解压到此处、智能解压、解压到同名文件夹、压缩为 ZIP/7Z 或高级压缩（仅写入当前用户注册表，免管理员权限）。".into(),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        SystemIntegrationStatus {
            platform: "other".into(),
            context_menu_installed: false,
            executable_path: exe,
            hint: "当前系统支持命令行参数 --extract-smart、--extract-here、--compress-zip、--compress-7z 与 --compress-dialog。".into(),
        }
    }
}

/// 一键安装系统右键菜单 / 访达快速操作（并同步注册多格式压缩包文件图标）
pub fn install_context_menu() -> QResult<SystemIntegrationStatus> {
    #[cfg(target_os = "macos")]
    {
        install_macos_quick_actions()?;
        refresh_file_type_icons(true)?;
    }

    #[cfg(target_os = "windows")]
    {
        install_windows_context_menu()?;
        refresh_file_type_icons(true)?;
    }

    Ok(get_integration_status())
}

/// 刷新/修复系统中的压缩包文件类型图标（macOS 刷新 LaunchServices，Windows 写入专属 .ico 并通知资源管理器）
pub fn refresh_file_type_icons(force: bool) -> QResult<()> {
    #[cfg(target_os = "windows")]
    {
        register_windows_file_icons(force)?;
    }

    #[cfg(target_os = "macos")]
    {
        let _ = force;
        refresh_macos_file_icons();
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = force;
    }

    Ok(())
}

/// 一键卸载系统右键菜单 / 访达快速操作
pub fn uninstall_context_menu() -> QResult<SystemIntegrationStatus> {
    #[cfg(target_os = "macos")]
    {
        uninstall_macos_quick_actions()?;
    }

    #[cfg(target_os = "windows")]
    {
        uninstall_windows_context_menu()?;
    }

    Ok(get_integration_status())
}

// ============================================================================
// macOS: 在 ~/Library/Services/ 生成标准 Automator 快速操作 (.workflow)
// ============================================================================
#[cfg(target_os = "macos")]
fn macos_services_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Services"))
}

#[cfg(target_os = "macos")]
const MACOS_WORKFLOW_NAMES: &[&str] = &[
    "使用 QPack 智能解压",
    "使用 QPack 压缩为 ZIP",
    "使用 QPack 压缩为 7Z",
    "使用 QPack 高级压缩",
];

#[cfg(target_os = "macos")]
fn write_macos_workflow(
    services_dir: &std::path::Path,
    workflow_name: &str,
    shell_command: &str,
) -> QResult<()> {
    let wf_dir = services_dir.join(format!("{}.workflow/Contents", workflow_name));
    std::fs::create_dir_all(&wf_dir)?;

    let info_plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>NSServices</key>
	<array>
		<dict>
			<key>NSBackgroundColorName</key>
			<string>background</string>
			<key>NSIconName</key>
			<string>NSActionTemplate</string>
			<key>NSMenuItem</key>
			<dict>
				<key>default</key>
				<string>__WORKFLOW_NAME__</string>
			</dict>
			<key>NSMessage</key>
			<string>runWorkflowAsService</string>
			<key>NSRequiredContext</key>
			<dict>
				<key>NSApplicationIdentifier</key>
				<string>com.apple.finder</string>
			</dict>
			<key>NSSendFileTypes</key>
			<array>
				<string>public.item</string>
			</array>
		</dict>
	</array>
</dict>
</plist>
"#
    .replace("__WORKFLOW_NAME__", workflow_name);

    let xml_escaped_cmd = shell_command
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");

    let document_wflow = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>AMApplicationBuild</key>
	<string>523</string>
	<key>AMApplicationVersion</key>
	<string>2.10</string>
	<key>AMDocumentVersion</key>
	<string>2</string>
	<key>actions</key>
	<array>
		<dict>
			<key>action</key>
			<dict>
				<key>AMAccepts</key>
				<dict>
					<key>Container</key>
					<string>List</string>
					<key>Optional</key>
					<true/>
					<key>Types</key>
					<array>
						<string>com.apple.cocoa.string</string>
					</array>
				</dict>
				<key>AMActionVersion</key>
				<string>2.0.3</string>
				<key>AMApplication</key>
				<array>
					<string>Automator</string>
				</array>
				<key>AMParameterProperties</key>
				<dict>
					<key>COMMAND_STRING</key>
					<dict/>
					<key>CheckedForUserDefaultShell</key>
					<dict/>
					<key>inputMethod</key>
					<dict/>
					<key>shell</key>
					<dict/>
					<key>source</key>
					<dict/>
				</dict>
				<key>AMProvides</key>
				<dict>
					<key>Container</key>
					<string>List</string>
					<key>Types</key>
					<array>
						<string>com.apple.cocoa.string</string>
					</array>
				</dict>
				<key>ActionBundlePath</key>
				<string>/System/Library/Automator/Run Shell Script.action</string>
				<key>ActionName</key>
				<string>Run Shell Script</string>
				<key>ActionParameters</key>
				<dict>
					<key>COMMAND_STRING</key>
					<string>{}</string>
					<key>CheckedForUserDefaultShell</key>
					<true/>
					<key>inputMethod</key>
					<integer>1</integer>
					<key>shell</key>
					<string>/bin/zsh</string>
					<key>source</key>
					<string></string>
				</dict>
				<key>BundleIdentifier</key>
				<string>com.apple.RunShellScript</string>
				<key>CFBundleVersion</key>
				<string>2.0.3</string>
				<key>CanShowSelectedItemsWhenRun</key>
				<false/>
				<key>CanShowWhenRun</key>
				<true/>
				<key>Category</key>
				<array>
					<string>AMCategoryUtilities</string>
				</array>
				<key>Class Name</key>
				<string>RunShellScriptAction</string>
				<key>InputUUID</key>
				<string>B1A1C2D3-1111-4000-8000-000000000001</string>
				<key>Keywords</key>
				<array>
					<string>Shell</string>
					<string>Script</string>
				</array>
				<key>OutputUUID</key>
				<string>B1A1C2D3-1111-4000-8000-000000000002</string>
				<key>UUID</key>
				<string>B1A1C2D3-1111-4000-8000-000000000003</string>
				<key>UnlocalizedApplications</key>
				<array>
					<string>Automator</string>
				</array>
			</dict>
		</dict>
	</array>
	<key>connectors</key>
	<dict/>
	<key>workflowMetaData</key>
	<dict>
		<key>applicationBundleID</key>
		<string>com.apple.finder</string>
		<key>applicationBundleIDsByPath</key>
		<dict>
			<key>/System/Library/CoreServices/Finder.app</key>
			<string>com.apple.finder</string>
		</dict>
		<key>applicationPath</key>
		<string>/System/Library/CoreServices/Finder.app</string>
		<key>applicationPaths</key>
		<array>
			<string>/System/Library/CoreServices/Finder.app</string>
		</array>
		<key>inputTypeIdentifier</key>
		<string>com.apple.Automator.fileSystemObject</string>
		<key>outputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>presentationMode</key>
		<integer>15</integer>
		<key>processesInput</key>
		<false/>
		<key>serviceApplicationBundleID</key>
		<string>com.apple.finder</string>
		<key>serviceApplicationPath</key>
		<string>/System/Library/CoreServices/Finder.app</string>
		<key>serviceInputTypeIdentifier</key>
		<string>com.apple.Automator.fileSystemObject</string>
		<key>serviceOutputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>serviceProcessesInput</key>
		<false/>
		<key>systemImageName</key>
		<string>NSActionTemplate</string>
		<key>useAutomaticInputType</key>
		<false/>
		<key>workflowTypeIdentifier</key>
		<string>com.apple.Automator.servicesMenu</string>
	</dict>
</dict>
</plist>
"#,
        xml_escaped_cmd
    );

    std::fs::write(wf_dir.join("Info.plist"), info_plist)?;
    std::fs::write(wf_dir.join("document.wflow"), document_wflow)?;
    Ok(())
}

/// 解析 macOS 上最稳定的 QPack 可执行文件路径（优先使用 .app 包内路径）
#[cfg(target_os = "macos")]
fn resolve_macos_exe_str() -> QResult<String> {
    let exe = current_exe_str()?;
    if exe.contains(".app/Contents/MacOS/") {
        return Ok(exe);
    }
    let installed_app_exe = PathBuf::from("/Applications/QPack.app/Contents/MacOS/qpack");
    if installed_app_exe.exists() {
        return Ok(installed_app_exe.to_string_lossy().into_owned());
    }
    Ok(exe)
}

/// 刷新 macOS LaunchServices 数据库以立即生效 .icns 文件类型图标
#[cfg(target_os = "macos")]
fn refresh_macos_file_icons() {
    let app_bundle = current_exe_str()
        .ok()
        .and_then(|exe| {
            exe.find(".app/Contents/MacOS/")
                .map(|idx| format!("{}.app", &exe[..idx]))
        })
        .unwrap_or_else(|| "/Applications/QPack.app".into());

    if Path::new(&app_bundle).exists() {
        let _ = Command::new(
            "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister",
        )
        .args(["-f", &app_bundle])
        .spawn();
    }
}

/// 首次启动时自动静默注册系统右键菜单与多格式压缩包图标
pub fn ensure_system_context_menu_on_startup() {
    #[cfg(target_os = "macos")]
    {
        if let Ok(exe) = current_exe_str() {
            if exe.contains(".app/Contents/MacOS/") {
                let status = get_integration_status();
                if !status.context_menu_installed {
                    let _ = install_macos_quick_actions();
                }
                refresh_macos_file_icons();
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        std::thread::spawn(|| {
            let status = get_integration_status();
            if !status.context_menu_installed {
                let _ = install_windows_context_menu();
            }
            if !windows_file_icons_registered() {
                let _ = register_windows_file_icons(false);
            }
        });
    }
}

#[cfg(target_os = "macos")]
fn install_macos_quick_actions() -> QResult<()> {
    let dir = macos_services_dir()
        .ok_or_else(|| QPackError::ArchiveError("无法定位 ~/Library/Services 目录".into()))?;
    let exe = resolve_macos_exe_str()?;
    let escaped_exe = exe.replace('"', "\\\"");

    write_macos_workflow(
        &dir,
        "使用 QPack 智能解压",
        &format!("\"{}\" --extract-smart \"$@\"", escaped_exe),
    )?;
    write_macos_workflow(
        &dir,
        "使用 QPack 压缩为 ZIP",
        &format!("\"{}\" --compress-zip \"$@\"", escaped_exe),
    )?;
    write_macos_workflow(
        &dir,
        "使用 QPack 压缩为 7Z",
        &format!("\"{}\" --compress-7z \"$@\"", escaped_exe),
    )?;
    write_macos_workflow(
        &dir,
        "使用 QPack 高级压缩",
        &format!("\"{}\" --compress-dialog \"$@\"", escaped_exe),
    )?;

    // 刷新 macOS Services / 快速操作缓存
    let _ = Command::new("/System/Library/CoreServices/pbs")
        .arg("-flush")
        .status();
    let _ = Command::new("/System/Library/CoreServices/pbs")
        .arg("-update")
        .spawn();
    Ok(())
}

#[cfg(target_os = "macos")]
fn uninstall_macos_quick_actions() -> QResult<()> {
    let Some(dir) = macos_services_dir() else {
        return Ok(());
    };
    for name in MACOS_WORKFLOW_NAMES {
        let p = dir.join(format!("{}.workflow", name));
        if p.exists() {
            std::fs::remove_dir_all(p)?;
        }
    }
    let _ = Command::new("/System/Library/CoreServices/pbs")
        .arg("-update")
        .spawn();
    Ok(())
}

// ============================================================================
// Windows: 写入 HKCU\Software\Classes 级联右键菜单（免管理员权限）
// ============================================================================
#[cfg(target_os = "windows")]
fn run_reg(args: &[&str]) -> QResult<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let status = Command::new("reg")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;
    if !status.success() {
        return Err(QPackError::ArchiveError("写入 Windows 注册表失败".into()));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn reg_set(key: &str, value_name: Option<&str>, data: &str) -> QResult<()> {
    match value_name {
        Some(name) => run_reg(&["add", key, "/v", name, "/d", data, "/f"]),
        None => run_reg(&["add", key, "/ve", "/d", data, "/f"]),
    }
}

#[cfg(target_os = "windows")]
fn reg_query_default(key: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = Command::new("reg")
        .args(["query", key, "/ve"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if let Some(pos) = line.find("REG_SZ") {
            let val = line[pos + 6..].trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn notify_shell_assoc_changed() {
    #[link(name = "shell32")]
    extern "system" {
        fn SHChangeNotify(
            w_event_id: i32,
            u_flags: u32,
            dw_item1: *const std::ffi::c_void,
            dw_item2: *const std::ffi::c_void,
        );
    }
    const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
    const SHCNF_IDLIST: u32 = 0x0000;
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}

#[cfg(target_os = "windows")]
fn resolve_windows_file_icon(exe_path: &str, icon_file_name: &str) -> String {
    if let Some(exe_dir) = Path::new(exe_path).parent() {
        for candidate in [
            exe_dir.join("icons").join("file-types").join(icon_file_name),
            exe_dir
                .join("resources")
                .join("icons")
                .join("file-types")
                .join(icon_file_name),
        ] {
            if candidate.is_file() {
                return candidate.to_string_lossy().replace('/', "\\");
            }
        }
    }
    format!("\"{}\",0", exe_path)
}

#[cfg(target_os = "windows")]
fn windows_file_icons_registered() -> bool {
    let Some(val) = reg_query_default(r"HKCU\Software\Classes\QPack.7z\DefaultIcon") else {
        return false;
    };
    val.to_ascii_lowercase().contains("file-7z.ico")
}

/// 为所有受支持的压缩包扩展名注册 QPack 专属格式图标，并刷新 Windows 资源管理器图标缓存
#[cfg(target_os = "windows")]
fn register_windows_file_icons(force: bool) -> QResult<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe = current_exe_str()?;
    let open_cmd = format!("\"{}\" \"%1\"", exe);

    // 清理旧版 NSIS 写入的含空格/斜杠失效 ProgID 键
    let legacy_prog_ids = [
        "ZIP Archive",
        "7-Zip Archive",
        "RAR Archive",
        "TAR / Unix Archive",
        "Disk / Cabinet Image",
    ];
    for legacy in legacy_prog_ids {
        let key = format!(r"HKCU\Software\Classes\{}", legacy);
        let _ = Command::new("reg")
            .args(["delete", &key, "/f"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }

    // 格式分组：(ProgID, 类型描述, 图标文件名, 关联扩展名列表)
    let groups: &[(&str, &str, &str, &[&str])] = &[
        (
            "QPack.zip",
            "QPack ZIP 压缩包",
            "file-zip.ico",
            &["zip", "jar", "apk", "ipa", "epub", "cbz"],
        ),
        ("QPack.7z", "QPack 7Z 压缩包", "file-7z.ico", &["7z"]),
        ("QPack.rar", "QPack RAR 压缩包", "file-rar.ico", &["rar"]),
        ("QPack.tar", "QPack TAR 归档文件", "file-tar.ico", &["tar"]),
        (
            "QPack.gz",
            "QPack GZIP 压缩包",
            "file-gz.ico",
            &["gz", "tgz"],
        ),
        (
            "QPack.zst",
            "QPack Zstandard 压缩包",
            "file-zst.ico",
            &["zst", "tzst"],
        ),
        (
            "QPack.archive",
            "QPack 压缩包",
            "file-archive.ico",
            &[
                "bz2", "tbz2", "xz", "txz", "iso", "cab", "dmg", "wim", "rpm", "deb", "lzh",
                "arj", "cpio",
            ],
        ),
    ];

    for &(prog_id, desc, icon_name, exts) in groups {
        let prog_key = format!(r"HKCU\Software\Classes\{}", prog_id);
        let icon_key = format!(r"{}\DefaultIcon", prog_key);
        let shell_key = format!(r"{}\shell", prog_key);
        let open_key = format!(r"{}\shell\open", prog_key);
        let cmd_key = format!(r"{}\shell\open\command", prog_key);

        let icon_val = resolve_windows_file_icon(&exe, icon_name);
        reg_set(&prog_key, None, desc)?;
        reg_set(&icon_key, None, &icon_val)?;
        reg_set(&shell_key, None, "open")?;
        reg_set(&open_key, None, "使用 QPack 打开")?;
        reg_set(&cmd_key, None, &open_cmd)?;

        for ext in exts {
            let ext_key = format!(r"HKCU\Software\Classes\.{}", ext);
            let open_with_key = format!(r"{}\OpenWithProgids", ext_key);
            let _ = run_reg(&["add", &open_with_key, "/v", prog_id, "/t", "REG_NONE", "/f"]);

            let current_prog = reg_query_default(&ext_key);
            let is_legacy = current_prog
                .as_deref()
                .map(|c| legacy_prog_ids.contains(&c))
                .unwrap_or(false);
            let is_qpack = current_prog
                .as_deref()
                .map(|c| c.starts_with("QPack."))
                .unwrap_or(false);

            // 若强制刷新、尚未关联、或指向旧版损坏 ProgID，则绑定到 QPack 专属格式图标
            if force || current_prog.is_none() || is_legacy || is_qpack {
                let _ = reg_set(&ext_key, None, prog_id);
            }
        }
    }

    notify_shell_assoc_changed();
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows_cascade(
    parent_key: &str,
    icon_val: &str,
    items: &[(&str, &str, String)],
) -> QResult<()> {
    reg_set(parent_key, Some("MUIVerb"), "QPack")?;
    reg_set(parent_key, Some("Icon"), icon_val)?;
    reg_set(parent_key, Some("SubCommands"), "")?;

    for (sub_id, label, command) in items {
        let sub_key = format!(r"{}\shell\{}", parent_key, sub_id);
        let cmd_key = format!(r"{}\command", sub_key);
        reg_set(&sub_key, None, label)?;
        reg_set(&sub_key, Some("Icon"), icon_val)?;
        reg_set(&cmd_key, None, command)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows_context_menu() -> QResult<()> {
    // 先清理旧版扁平键
    let _ = uninstall_windows_context_menu();

    let exe = current_exe_str()?;
    let icon_val = format!("\"{}\",0", exe);

    let file_items = vec![
        (
            "01_extract_smart",
            "智能解压到此处",
            format!("\"{}\" --extract-smart \"%1\"", exe),
        ),
        (
            "02_extract_here",
            "解压到此处",
            format!("\"{}\" --extract-here \"%1\"", exe),
        ),
        (
            "03_extract_subfolder",
            "解压到同名文件夹",
            format!("\"{}\" --extract-subfolder \"%1\"", exe),
        ),
        (
            "04_extract_to",
            "解压到其他文件夹...",
            format!("\"{}\" --extract-to \"%1\"", exe),
        ),
        ("05_open", "用 QPack 打开...", format!("\"{}\" \"%1\"", exe)),
        (
            "06_compress_zip",
            "压缩为 ZIP",
            format!("\"{}\" --compress-zip \"%1\"", exe),
        ),
        (
            "07_compress_7z",
            "压缩为 7Z",
            format!("\"{}\" --compress-7z \"%1\"", exe),
        ),
        (
            "08_compress_dialog",
            "添加到压缩包...",
            format!("\"{}\" --compress-dialog \"%1\"", exe),
        ),
    ];

    let dir_items = vec![
        (
            "01_compress_zip",
            "压缩为 ZIP",
            format!("\"{}\" --compress-zip \"%1\"", exe),
        ),
        (
            "02_compress_7z",
            "压缩为 7Z",
            format!("\"{}\" --compress-7z \"%1\"", exe),
        ),
        (
            "03_compress_dialog",
            "添加到压缩包...",
            format!("\"{}\" --compress-dialog \"%1\"", exe),
        ),
    ];

    install_windows_cascade(
        r"HKCU\Software\Classes\*\shell\QPackMenu",
        &icon_val,
        &file_items,
    )?;
    install_windows_cascade(
        r"HKCU\Software\Classes\Directory\shell\QPackMenu",
        &icon_val,
        &dir_items,
    )?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn uninstall_windows_context_menu() -> QResult<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    for key in [
        r"HKCU\Software\Classes\*\shell\QPackMenu",
        r"HKCU\Software\Classes\Directory\shell\QPackMenu",
        r"HKCU\Software\Classes\*\shell\QPackCompress",
        r"HKCU\Software\Classes\Directory\shell\QPackCompress",
        r"HKCU\Software\Classes\*\shell\QPackExtract",
    ] {
        let _ = Command::new("reg")
            .args(["delete", key, "/f"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
    Ok(())
}
