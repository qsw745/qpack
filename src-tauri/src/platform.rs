use crate::error::{QPackError, QResult};
use serde::Serialize;
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
        let _ = Command::new("explorer").arg(path).spawn();
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

/// 发送轻量系统通知（用于后台静默解压/压缩完成提示）
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

/// 查询当前平台的系统集成状态
pub fn get_integration_status() -> SystemIntegrationStatus {
    let exe = current_exe_str().unwrap_or_default();

    #[cfg(target_os = "macos")]
    {
        let installed = macos_services_dir()
            .map(|d| d.join("使用 QPack 智能解压.workflow").exists())
            .unwrap_or(false);
        SystemIntegrationStatus {
            platform: "macos".into(),
            context_menu_installed: installed,
            executable_path: exe,
            hint: "安装后可在访达右键 ->「快速操作」中直接解压或压缩；在访达「显示简介 -> 打开方式」设为默认后，双击压缩包即可直接后台静默解压。".into(),
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let installed = Command::new("reg")
            .args(["query", r"HKCU\Software\Classes\*\shell\QPackCompress"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        SystemIntegrationStatus {
            platform: "windows".into(),
            context_menu_installed: installed,
            executable_path: exe,
            hint: "注册后可在 Windows 资源管理器右键菜单直接压缩/解压（仅写入当前用户注册表，无需管理员权限）。Windows 11 请点击右键菜单底部的「显示更多选项」，或按住 Shift + 右键直接展开。".into(),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        SystemIntegrationStatus {
            platform: "other".into(),
            context_menu_installed: false,
            executable_path: exe,
            hint: "当前系统支持命令行参数 --extract 与 --compress。".into(),
        }
    }
}

/// 一键安装系统右键菜单 / 访达快速操作
pub fn install_context_menu() -> QResult<SystemIntegrationStatus> {
    #[cfg(target_os = "macos")]
    {
        install_macos_quick_actions()?;
    }

    #[cfg(target_os = "windows")]
    {
        install_windows_context_menu()?;
    }

    Ok(get_integration_status())
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

/// 首次启动时自动静默注册系统右键菜单（macOS 访达快速操作 / Windows 资源管理器右键）
pub fn ensure_system_context_menu_on_startup() {
    #[cfg(target_os = "macos")]
    {
        if let Ok(exe) = current_exe_str() {
            if exe.contains(".app/Contents/MacOS/") {
                let status = get_integration_status();
                if !status.context_menu_installed {
                    let _ = install_macos_quick_actions();
                }
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
        &format!("\"{}\" --extract \"$@\"", escaped_exe),
    )?;
    write_macos_workflow(
        &dir,
        "使用 QPack 压缩为 ZIP",
        &format!("\"{}\" --compress \"$@\"", escaped_exe),
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
    for name in ["使用 QPack 智能解压.workflow", "使用 QPack 压缩为 ZIP.workflow"] {
        let p = dir.join(name);
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
// Windows: 写入 HKCU\Software\Classes 注册表（免管理员权限）
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
fn install_windows_context_menu() -> QResult<()> {
    let exe = current_exe_str()?;
    let extract_cmd = format!("\"{}\" --extract \"%1\"", exe);
    let compress_cmd = format!("\"{}\" --compress \"%1\"", exe);
    let icon_val = format!("\"{}\",0", exe);

    // 1. 任意文件右键 -> 使用 QPack 压缩为 ZIP
    run_reg(&[
        "add",
        r"HKCU\Software\Classes\*\shell\QPackCompress",
        "/ve",
        "/d",
        "使用 QPack 压缩为 ZIP",
        "/f",
    ])?;
    run_reg(&[
        "add",
        r"HKCU\Software\Classes\*\shell\QPackCompress",
        "/v",
        "Icon",
        "/d",
        &icon_val,
        "/f",
    ])?;
    run_reg(&[
        "add",
        r"HKCU\Software\Classes\*\shell\QPackCompress\command",
        "/ve",
        "/d",
        &compress_cmd,
        "/f",
    ])?;

    // 2. 文件夹右键 -> 使用 QPack 压缩为 ZIP
    run_reg(&[
        "add",
        r"HKCU\Software\Classes\Directory\shell\QPackCompress",
        "/ve",
        "/d",
        "使用 QPack 压缩为 ZIP",
        "/f",
    ])?;
    run_reg(&[
        "add",
        r"HKCU\Software\Classes\Directory\shell\QPackCompress",
        "/v",
        "Icon",
        "/d",
        &icon_val,
        "/f",
    ])?;
    run_reg(&[
        "add",
        r"HKCU\Software\Classes\Directory\shell\QPackCompress\command",
        "/ve",
        "/d",
        &compress_cmd,
        "/f",
    ])?;

    // 3. 任意文件右键 -> 使用 QPack 智能解压
    run_reg(&[
        "add",
        r"HKCU\Software\Classes\*\shell\QPackExtract",
        "/ve",
        "/d",
        "使用 QPack 智能解压",
        "/f",
    ])?;
    run_reg(&[
        "add",
        r"HKCU\Software\Classes\*\shell\QPackExtract",
        "/v",
        "Icon",
        "/d",
        &icon_val,
        "/f",
    ])?;
    run_reg(&[
        "add",
        r"HKCU\Software\Classes\*\shell\QPackExtract\command",
        "/ve",
        "/d",
        &extract_cmd,
        "/f",
    ])?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn uninstall_windows_context_menu() -> QResult<()> {
    for key in [
        r"HKCU\Software\Classes\*\shell\QPackCompress",
        r"HKCU\Software\Classes\Directory\shell\QPackCompress",
        r"HKCU\Software\Classes\*\shell\QPackExtract",
    ] {
        let _ = Command::new("reg").args(["delete", key, "/f"]).status();
    }
    Ok(())
}
