//! macOS 原生 DMG 磁盘映像读写引擎（基于系统内置 /usr/bin/hdiutil，零外部依赖）

#[cfg(target_os = "macos")]
use crate::archive::smart::{build_selection_set, is_system_junk, matches_selection, safe_join};
#[cfg(target_os = "macos")]
use crate::archive::task_control::{copy_with_progress, ProgressTracker};
#[cfg(target_os = "macos")]
use crate::error::{QPackError, QResult};
#[cfg(target_os = "macos")]
use crate::models::{ArchiveEntry, CompressOptions, CompressResult, ExtractOptions, ExtractResult};
#[cfg(target_os = "macos")]
use chrono::{DateTime, Utc};
#[cfg(target_os = "macos")]
use std::fs::{self, File};
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
#[cfg(target_os = "macos")]
use std::thread;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant, UNIX_EPOCH};
#[cfg(target_os = "macos")]
use uuid::Uuid;
#[cfg(target_os = "macos")]
use walkdir::WalkDir;

#[cfg(target_os = "macos")]
struct MountedDmg {
    mount_point: PathBuf,
}

#[cfg(target_os = "macos")]
impl MountedDmg {
    fn attach(dmg_path: &Path) -> QResult<Self> {
        let mount_point = std::env::temp_dir().join(format!("qpack-dmg-{}", Uuid::new_v4()));
        fs::create_dir_all(&mount_point)?;

        let output = Command::new("/usr/bin/hdiutil")
            .args([
                "attach",
                "-nobrowse",
                "-readonly",
                "-noverify",
                "-noautoopen",
                "-mountpoint",
            ])
            .arg(&mount_point)
            .arg(dmg_path)
            .stdin(Stdio::null())
            .output()?;

        if !output.status.success() {
            let _ = fs::remove_dir_all(&mount_point);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let msg = if !stderr.trim().is_empty() {
                stderr.trim().to_string()
            } else {
                stdout.trim().to_string()
            };
            if msg.to_lowercase().contains("authentication")
                || msg.to_lowercase().contains("passphrase")
                || msg.to_lowercase().contains("password")
            {
                return Err(QPackError::InvalidPassword);
            }
            return Err(QPackError::ArchiveError(format!(
                "无法挂载 DMG 映像: {}",
                msg
            )));
        }

        Ok(Self { mount_point })
    }
}

#[cfg(target_os = "macos")]
impl Drop for MountedDmg {
    fn drop(&mut self) {
        let _ = Command::new("/usr/bin/hdiutil")
            .args(["detach", "-force", "-quiet"])
            .arg(&self.mount_point)
            .status();
        let _ = fs::remove_dir_all(&self.mount_point);
    }
}

#[cfg(target_os = "macos")]
fn format_meta_time(meta: &fs::Metadata) -> Option<String> {
    let modified = meta.modified().ok()?;
    let dur = modified.duration_since(UNIX_EPOCH).ok()?;
    let dt = DateTime::<Utc>::from_timestamp(dur.as_secs() as i64, 0)?;
    Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

/// 在 macOS 上使用系统内置 hdiutil 解析 DMG 内容（零外部依赖）
#[cfg(target_os = "macos")]
pub fn inspect_dmg_macos(dmg_path: &Path) -> QResult<Vec<ArchiveEntry>> {
    let mounted = MountedDmg::attach(dmg_path)?;
    let mut entries = Vec::new();

    for entry in WalkDir::new(&mounted.mount_point).min_depth(1) {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        let Ok(rel) = path.strip_prefix(&mounted.mount_point) else {
            continue;
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if rel_str.is_empty()
            || rel_str.starts_with(".fseventsd")
            || rel_str.starts_with(".Trashes")
            || rel_str == ".background"
            || rel_str.starts_with(".background/")
            || is_system_junk(&rel_str)
        {
            continue;
        }

        let meta = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // 跳过指向 /Applications 的常见安装快捷软链，避免递归拖出系统目录
        if meta.file_type().is_symlink() {
            continue;
        }

        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&rel_str)
            .to_string();
        let index = entries.len();

        entries.push(ArchiveEntry {
            path: if is_dir {
                format!("{}/", rel_str)
            } else {
                rel_str
            },
            name,
            is_dir,
            size,
            compressed_size: size,
            modified: format_meta_time(&meta),
            encrypted: false,
            index,
        });
    }

    Ok(entries)
}

/// 在 macOS 上使用系统内置 hdiutil 解压提取 DMG 内容（支持部分提取、进度、暂停/取消）
#[cfg(target_os = "macos")]
pub fn extract_dmg_macos(
    archive_path: &Path,
    output_dir: &Path,
    options: &ExtractOptions,
    tracker: &mut ProgressTracker,
) -> QResult<ExtractResult> {
    let started = Instant::now();
    let mounted = MountedDmg::attach(archive_path)?;
    let selection = build_selection_set(&options.selected_paths);

    // 收集待提取条目
    let mut plan: Vec<(PathBuf, String, bool, u64)> = Vec::new();
    for entry in WalkDir::new(&mounted.mount_point).min_depth(1) {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        let Ok(rel) = path.strip_prefix(&mounted.mount_point) else {
            continue;
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if rel_str.is_empty()
            || rel_str.starts_with(".fseventsd")
            || rel_str.starts_with(".Trashes")
            || rel_str == ".background"
            || rel_str.starts_with(".background/")
            || is_system_junk(&rel_str)
        {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if !matches_selection(&rel_str, &selection) {
            continue;
        }
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        plan.push((path.to_path_buf(), rel_str, is_dir, size));
    }

    let total_items = plan.len();
    let total_bytes: u64 = plan
        .iter()
        .filter(|(_, _, is_dir, _)| !*is_dir)
        .map(|(_, _, _, sz)| *sz)
        .sum();
    tracker.set_totals(total_items, total_bytes);

    fs::create_dir_all(output_dir)?;

    let mut extracted_files = 0usize;
    let mut extracted_bytes = 0u64;

    for (src_path, rel_str, is_dir, size) in plan {
        tracker.checkpoint()?;
        tracker.begin_file(&rel_str, if is_dir { 0 } else { size });

        let dest_path = safe_join(output_dir, &rel_str)?;
        if is_dir {
            fs::create_dir_all(&dest_path)?;
            tracker.finish_file();
            continue;
        }

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if dest_path.exists() && !options.overwrite {
            tracker.add_bytes(size);
            tracker.finish_file();
            continue;
        }

        let mut reader = File::open(&src_path)?;
        let mut writer = File::create(&dest_path)?;
        let copy_res = copy_with_progress(&mut reader, &mut writer, tracker);
        match copy_res {
            Ok(copied) => {
                // 保留可执行权限（.app 内 MacOS 二进制必备）
                if let Ok(src_meta) = fs::metadata(&src_path) {
                    let _ = fs::set_permissions(&dest_path, src_meta.permissions());
                }
                extracted_files += 1;
                extracted_bytes += copied;
                tracker.finish_file();
            }
            Err(err) => {
                drop(writer);
                let _ = fs::remove_file(&dest_path);
                return Err(err);
            }
        }
    }

    tracker.finish();

    Ok(ExtractResult {
        output_dir: output_dir.to_string_lossy().into_owned(),
        extracted_files,
        total_bytes: extracted_bytes,
        elapsed_ms: started.elapsed().as_millis() as u64,
        primary_target: None,
        primary_target_is_dir: true,
    })
}

/// 在 macOS 上使用系统内置 hdiutil 创建压缩 DMG 镜像（零外部依赖，支持取消清理）
#[cfg(target_os = "macos")]
pub fn compress_dmg_macos(
    options: &CompressOptions,
    tracker: &mut ProgressTracker,
) -> QResult<CompressResult> {
    let started = Instant::now();
    let output_path = PathBuf::from(&options.output_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 1. 收集并复制输入文件到临时 staging 目录（应用垃圾文件过滤与字节进度）
    let staging = std::env::temp_dir().join(format!("qpack-dmg-src-{}", Uuid::new_v4()));
    fs::create_dir_all(&staging)?;

    struct StagingGuard(PathBuf);
    impl Drop for StagingGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _guard = StagingGuard(staging.clone());

    let mut plan: Vec<(PathBuf, PathBuf, String, bool, u64)> = Vec::new();
    for raw in &options.input_paths {
        let root = PathBuf::from(raw);
        if !root.exists() {
            return Err(QPackError::NotFound(raw.clone()));
        }
        let base_name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("item")
            .to_string();
        if root.is_file() {
            if options.exclude_junk && is_system_junk(&base_name) {
                continue;
            }
            let sz = fs::metadata(&root)?.len();
            plan.push((root, staging.join(&base_name), base_name, false, sz));
        } else {
            for entry in WalkDir::new(&root) {
                let entry = entry.map_err(|e| QPackError::ArchiveError(e.to_string()))?;
                let p = entry.path().to_path_buf();
                let rel = p
                    .strip_prefix(&root)
                    .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
                let arc_rel = if rel.as_os_str().is_empty() {
                    base_name.clone()
                } else {
                    format!("{}/{}", base_name, rel.to_string_lossy().replace('\\', "/"))
                };
                if options.exclude_junk && is_system_junk(&arc_rel) {
                    continue;
                }
                let is_dir = p.is_dir();
                let sz = if is_dir {
                    0
                } else {
                    fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
                };
                let dst = if rel.as_os_str().is_empty() {
                    staging.join(&base_name)
                } else {
                    staging.join(&base_name).join(rel)
                };
                plan.push((p, dst, arc_rel, is_dir, sz));
            }
        }
    }

    if plan.is_empty() {
        return Err(QPackError::ArchiveError("没有可打包的有效文件".into()));
    }

    let total_items = plan.len();
    let total_bytes: u64 = plan
        .iter()
        .filter(|(_, _, _, is_dir, _)| !*is_dir)
        .map(|(_, _, _, _, sz)| *sz)
        .sum();
    // 准备阶段占前 50%，hdiutil 压缩封装占后半段
    tracker.set_totals(total_items + 1, total_bytes.saturating_mul(2).max(1));

    let mut input_files = 0usize;
    let mut original_bytes = 0u64;

    for (src, dst, label, is_dir, sz) in plan {
        tracker.checkpoint()?;
        tracker.begin_file(&label, if is_dir { 0 } else { sz });
        if is_dir {
            fs::create_dir_all(&dst)?;
        } else {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut r = File::open(&src)?;
            let mut w = File::create(&dst)?;
            copy_with_progress(&mut r, &mut w, tracker)?;
            if let Ok(meta) = fs::metadata(&src) {
                let _ = fs::set_permissions(&dst, meta.permissions());
            }
            input_files += 1;
            original_bytes += sz;
        }
        tracker.finish_file();
    }

    // 2. 调用系统内置 hdiutil create 生成压缩 DMG 镜像（UDZO = zlib 压缩）
    tracker.checkpoint()?;
    let vol_name = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("QPack");
    tracker.begin_file(&format!("正在生成 {}.dmg 磁盘映像...", vol_name), 0);

    let format_flag = if options.level == 0 { "UDRO" } else { "UDZO" };
    let mut cmd = Command::new("/usr/bin/hdiutil");
    cmd.args([
        "create",
        "-volname",
        vol_name,
        "-srcfolder",
    ])
    .arg(&staging)
    .args(["-ov", "-format", format_flag])
    .arg(&output_path)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    loop {
        if tracker.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&output_path);
            return Err(QPackError::Cancelled);
        }
        if child.try_wait()?.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(80));
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let _ = fs::remove_file(&output_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(QPackError::ArchiveError(format!(
            "生成 DMG 失败: {}",
            stderr.trim()
        )));
    }

    tracker.finish_file();
    tracker.finish();

    let archive_bytes = fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);

    Ok(CompressResult {
        output_path: output_path.to_string_lossy().into_owned(),
        input_files,
        original_bytes,
        archive_bytes,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}
