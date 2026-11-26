use crate::error::{QPackError, QResult};
use crate::models::{ArchiveEntry, ArchiveFormat};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// 判断是否为系统垃圾文件（macOS / Windows 元数据文件）
pub fn is_system_junk(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    for segment in normalized.split('/') {
        if segment.is_empty() {
            continue;
        }
        if segment == "__MACOSX"
            || segment == ".DS_Store"
            || segment == "Thumbs.db"
            || segment.eq_ignore_ascii_case("desktop.ini")
            || segment.starts_with("._")
        {
            return true;
        }
    }
    false
}

/// 校验并拼接安全解压路径，防止 Zip Slip 路径穿越攻击
pub fn safe_join(base: &Path, relative_path: &str) -> QResult<PathBuf> {
    let cleaned = relative_path.replace('\\', "/");
    let rel = Path::new(&cleaned);
    let mut safe_rel = PathBuf::new();

    for comp in rel.components() {
        match comp {
            Component::Normal(part) => safe_rel.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(QPackError::UnsafePath(relative_path.to_string()));
            }
        }
    }

    Ok(base.join(safe_rel))
}

/// 分析压缩包顶层结构，判断是否需要“智能创建同名目录”
/// 返回 `(will_create_folder, suggested_folder_name)`
pub fn analyze_smart_extract(archive_path: &Path, entries: &[ArchiveEntry]) -> (bool, String) {
    let file_name = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("archive");
    let suggested = ArchiveFormat::strip_archive_extension(file_name);

    let mut top_levels: HashSet<String> = HashSet::new();
    let mut has_top_level_dir = false;

    for entry in entries {
        if is_system_junk(&entry.path) {
            continue;
        }
        let trimmed = entry.path.trim_matches('/');
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(2, '/');
        if let Some(first) = parts.next() {
            top_levels.insert(first.to_string());
            if parts.next().is_some() || entry.is_dir {
                has_top_level_dir = true;
            }
        }
    }

    // 若有效顶层恰好只有一个文件夹，直接解压不会造成“解压炸弹”
    if top_levels.len() == 1 && has_top_level_dir {
        let single_dir = top_levels.into_iter().next().unwrap_or(suggested);
        (false, single_dir)
    } else {
        // 顶层散落多个文件/目录，或只有一个裸文件 -> 自动创建同名子目录
        (true, suggested)
    }
}

/// 根据是否开启智能解压，计算最终输出目录（避免与现有目录冲突时自动加编号）
pub fn resolve_extract_output_dir(
    archive_path: &Path,
    dest_dir: &Path,
    entries: &[ArchiveEntry],
    smart_extract: bool,
    has_selection: bool,
) -> PathBuf {
    if !smart_extract || has_selection {
        return dest_dir.to_path_buf();
    }

    let (will_create, suggested) = analyze_smart_extract(archive_path, entries);
    if !will_create {
        return dest_dir.to_path_buf();
    }

    let candidate = dest_dir.join(&suggested);
    if !candidate.exists() {
        return candidate;
    }

    // 避免覆盖已有同名目录，自动顺延编号
    for idx in 1..=999 {
        let alt = dest_dir.join(format!("{} ({})", suggested, idx));
        if !alt.exists() {
            return alt;
        }
    }

    candidate
}
