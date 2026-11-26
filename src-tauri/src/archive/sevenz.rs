use crate::archive::native::ProgressCallback;
use crate::error::{QPackError, QResult};
use crate::models::{ArchiveEntry, ExtractOptions, ExtractResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// 探测内置或系统安装的 7zz / 7z 可执行文件路径
pub fn find_sevenz_binary() -> Option<PathBuf> {
    // 1. 优先检查应用同级 binaries 目录（打包 Sidecar 场景）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["7zz", "7zz.exe", "7z", "7z.exe"] {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
                let nested = dir.join("binaries").join(name);
                if nested.is_file() {
                    return Some(nested);
                }
            }
        }
    }

    // 2. 检查系统 PATH 中的 7zz / 7z
    let candidates = if cfg!(windows) {
        vec!["7zz.exe", "7z.exe"]
    } else {
        vec!["7zz", "7z", "/opt/homebrew/bin/7zz", "/usr/local/bin/7zz"]
    };

    for bin in candidates {
        if Command::new(bin).arg("--help").output().is_ok() {
            return Some(PathBuf::from(bin));
        }
    }

    None
}

/// 使用 7zz -slt 解析压缩包条目列表（用于 RAR / ISO / DMG / CAB 等格式）
pub fn inspect_with_sevenz_cli(path: &Path, password: Option<&str>) -> QResult<Vec<ArchiveEntry>> {
    let bin = find_sevenz_binary().ok_or_else(|| {
        QPackError::EngineUnavailable(
            "未检测到 7zz 引擎。请安装 7-Zip (macOS: brew install sevenzip) 或将 7zz 放入应用 binaries 目录".into(),
        )
    })?;

    let mut cmd = Command::new(&bin);
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
        if stderr.to_lowercase().contains("wrong password") {
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

/// 使用 7zz 命令行解压（支持 RAR / ISO / DMG / CAB 及部分提取）
pub fn extract_with_sevenz_cli(
    archive_path: &Path,
    output_dir: &Path,
    options: &ExtractOptions,
    on_progress: ProgressCallback<'_>,
) -> QResult<ExtractResult> {
    let started = Instant::now();
    let bin = find_sevenz_binary().ok_or_else(|| {
        QPackError::EngineUnavailable(
            "未检测到 7zz 引擎。请安装 7-Zip (macOS: brew install sevenzip) 或将 7zz 放入应用 binaries 目录".into(),
        )
    })?;

    fs::create_dir_all(output_dir)?;
    on_progress("正在调用 7zz 引擎解压...", 0, 100);

    let mut cmd = Command::new(&bin);
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

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.to_lowercase().contains("wrong password") {
            return Err(QPackError::InvalidPassword);
        }
        return Err(QPackError::ArchiveError(stderr.trim().to_string()));
    }

    on_progress("解压完成", 100, 100);

    Ok(ExtractResult {
        output_dir: output_dir.to_string_lossy().into_owned(),
        extracted_files: 0,
        total_bytes: 0,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}
