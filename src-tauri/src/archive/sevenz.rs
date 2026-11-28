use crate::archive::task_control::ProgressTracker;
use crate::error::{QPackError, QResult};
use crate::models::{ArchiveEntry, ExtractOptions, ExtractResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn new_hidden_command(bin: &Path) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(bin);
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// 探测内置或系统安装的 7zz / 7z 可执行文件路径（优先使用 QPack 内置静态引擎，开箱即用无需安装）
pub fn find_sevenz_binary() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["7z.exe", "7zz.exe"]
    } else {
        &["7zz", "7z"]
    };

    // 1. 优先检查应用包内嵌 binaries 目录（macOS Resources/binaries 与 Windows $INSTDIR/binaries）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let mut search_dirs = vec![
                dir.join("binaries"),
                dir.join("resources").join("binaries"),
                dir.to_path_buf(),
            ];

            // macOS .app/Contents/MacOS/qpack -> ../Resources/binaries/7zz
            if let Some(contents_dir) = dir.parent() {
                search_dirs.insert(0, contents_dir.join("Resources").join("binaries"));
                search_dirs.push(contents_dir.join("Resources"));
            }

            for base in search_dirs {
                for name in names {
                    let candidate = base.join(name);
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    // 2. 开发与单元测试环境：检查源码树内嵌目录 src-tauri/binaries/{macos,windows}
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dev_sub = if cfg!(windows) {
        manifest_dir.join("binaries").join("windows")
    } else {
        manifest_dir.join("binaries").join("macos")
    };
    for name in names {
        let candidate = dev_sub.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // 3. 回退检查系统 PATH 中的 7zz / 7z
    let candidates = if cfg!(windows) {
        vec!["7z.exe", "7zz.exe"]
    } else {
        vec!["7zz", "7z", "/opt/homebrew/bin/7zz", "/usr/local/bin/7zz"]
    };

    for bin in candidates {
        if new_hidden_command(Path::new(bin))
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some(PathBuf::from(bin));
        }
    }

    None
}

/// 使用内置 7-Zip 引擎 (-slt) 解析压缩包条目列表（支持 RAR / ISO / CAB / DMG / WIM / RPM / DEB / LZH / ARJ 等 40+ 格式）
pub fn inspect_with_sevenz_cli(path: &Path, password: Option<&str>) -> QResult<Vec<ArchiveEntry>> {
    let bin = find_sevenz_binary().ok_or_else(|| {
        QPackError::EngineUnavailable("未检测到内置 7-Zip 扩展引擎，请重新安装 QPack".into())
    })?;

    let mut cmd = new_hidden_command(&bin);
    cmd.args(["l", "-slt", "-ba", "-sccUTF-8"]);
    if let Some(pwd) = password.filter(|p| !p.is_empty()) {
        cmd.arg(format!("-p{}", pwd));
    } else {
        cmd.arg("-p");
    }
    cmd.arg(path);

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lower = stderr.to_lowercase();
        if lower.contains("wrong password") || lower.contains("can not open encrypted archive") {
            return Err(QPackError::InvalidPassword);
        }
        return Err(QPackError::ArchiveError(stderr.trim().to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    let mut cur_path: Option<String> = None;
    let mut cur_size: u64 = 0;
    let mut cur_packed: u64 = 0;
    let mut cur_is_dir = false;
    let mut cur_modified: Option<String> = None;
    let mut cur_encrypted = false;

    let flush_entry = |entries: &mut Vec<ArchiveEntry>,
                       cur_path: &mut Option<String>,
                       cur_size: &mut u64,
                       cur_packed: &mut u64,
                       cur_is_dir: &mut bool,
                       cur_modified: &mut Option<String>,
                       cur_encrypted: &mut bool| {
        if let Some(p) = cur_path.take() {
            let normalized = p.replace('\\', "/");
            let name = normalized
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(&normalized)
                .to_string();
            let index = entries.len();
            entries.push(ArchiveEntry {
                path: normalized,
                name,
                is_dir: *cur_is_dir,
                size: *cur_size,
                compressed_size: *cur_packed,
                modified: cur_modified.take(),
                encrypted: *cur_encrypted,
                index,
            });
        }
        *cur_size = 0;
        *cur_packed = 0;
        *cur_is_dir = false;
        *cur_modified = None;
        *cur_encrypted = false;
    };

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_entry(
                &mut entries,
                &mut cur_path,
                &mut cur_size,
                &mut cur_packed,
                &mut cur_is_dir,
                &mut cur_modified,
                &mut cur_encrypted,
            );
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(" = ") {
            match k {
                "Path" => cur_path = Some(v.to_string()),
                "Size" => cur_size = v.parse().unwrap_or(0),
                "Packed Size" => cur_packed = v.parse().unwrap_or(0),
                "Folder" => cur_is_dir = v == "+",
                "Modified" => cur_modified = Some(v.to_string()),
                "Encrypted" => cur_encrypted = v == "+",
                _ => {}
            }
        }
    }

    flush_entry(
        &mut entries,
        &mut cur_path,
        &mut cur_size,
        &mut cur_packed,
        &mut cur_is_dir,
        &mut cur_modified,
        &mut cur_encrypted,
    );

    Ok(entries)
}

/// 使用内置 7-Zip 引擎解压（支持 RAR / ISO / CAB / DMG / WIM / RPM / DEB 等及中途取消）
pub fn extract_with_sevenz_cli(
    archive_path: &Path,
    output_dir: &Path,
    options: &ExtractOptions,
    tracker: &mut ProgressTracker,
) -> QResult<ExtractResult> {
    let started = Instant::now();
    let bin = find_sevenz_binary().ok_or_else(|| {
        QPackError::EngineUnavailable("未检测到内置 7-Zip 扩展引擎，请重新安装 QPack".into())
    })?;

    // 预估条目数与总大小，供进度弹窗完成后展示准确摘要
    let (est_files, est_bytes) = inspect_with_sevenz_cli(archive_path, options.password.as_deref())
        .map(|list| {
            let mut files = 0usize;
            let mut bytes = 0u64;
            for e in list {
                if !e.is_dir {
                    files += 1;
                    bytes = bytes.saturating_add(e.size);
                }
            }
            (files, bytes)
        })
        .unwrap_or((0, 0));

    fs::create_dir_all(output_dir)?;
    let archive_name = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("archive");
    tracker.set_totals(est_files.max(1), est_bytes);
    tracker.checkpoint()?;
    tracker.begin_file(&format!("正在解压 {}...", archive_name), est_bytes);

    let mut cmd = new_hidden_command(&bin);
    cmd.arg("x");
    cmd.arg(if options.overwrite { "-aoa" } else { "-aos" });
    cmd.arg("-y");
    if let Some(pwd) = options.password.as_deref().filter(|p| !p.is_empty()) {
        cmd.arg(format!("-p{}", pwd));
    } else {
        cmd.arg("-p");
    }
    cmd.arg(format!("-o{}", output_dir.to_string_lossy()));
    cmd.arg(archive_path);

    if let Some(selected) = &options.selected_paths {
        for item in selected {
            cmd.arg(item);
        }
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;

    loop {
        if tracker.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(QPackError::Cancelled);
        }
        if child.try_wait()?.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(80));
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lower = stderr.to_lowercase();
        if lower.contains("wrong password")
            || lower.contains("can not open encrypted archive")
            || lower.contains("data error in encrypted file")
        {
            return Err(QPackError::InvalidPassword);
        }
        return Err(QPackError::ArchiveError(stderr.trim().to_string()));
    }

    tracker.finish_file();
    tracker.finish();

    Ok(ExtractResult {
        output_dir: output_dir.to_string_lossy().into_owned(),
        extracted_files: est_files,
        total_bytes: est_bytes,
        elapsed_ms: started.elapsed().as_millis() as u64,
        primary_target: None,
        primary_target_is_dir: true,
    })
}
