use crate::error::{QPackError, QResult};
use crate::models::{ArchiveEntry, ArchiveFormat, ExtractMode};
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

/// 标准化部分提取路径筛选集合
pub fn build_selection_set(selected: &Option<Vec<String>>) -> Option<HashSet<String>> {
    selected.as_ref().and_then(|list| {
        if list.is_empty() {
            None
        } else {
            Some(
                list.iter()
                    .map(|p| p.trim_matches('/').replace('\\', "/"))
                    .collect(),
            )
        }
    })
}

/// 判断条目是否命中选中集合（选中某个文件夹时自动包含其下所有子文件）
pub fn matches_selection(entry_path: &str, selection: &Option<HashSet<String>>) -> bool {
    let Some(set) = selection else {
        return true;
    };
    let clean = entry_path.trim_matches('/');
    if set.contains(clean) {
        return true;
    }
    for sel in set {
        if clean.starts_with(&format!("{}/", sel)) {
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

/// 根据 ExtractMode 计算最终解压输出目录
/// 返回 `(output_dir, created_subfolder)`
pub fn resolve_extract_output_dir_with_mode(
    archive_path: &Path,
    dest_dir: &Path,
    entries: &[ArchiveEntry],
    mode: ExtractMode,
    has_selection: bool,
) -> (PathBuf, bool) {
    if has_selection || mode == ExtractMode::Here {
        return (dest_dir.to_path_buf(), false);
    }

    let file_name = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("archive");
    let archive_stem = ArchiveFormat::strip_archive_extension(file_name);

    match mode {
        ExtractMode::Here => (dest_dir.to_path_buf(), false),
        ExtractMode::Subfolder => (dest_dir.join(archive_stem), true),
        ExtractMode::Smart => {
            let (will_create, suggested) = analyze_smart_extract(archive_path, entries);
            if !will_create {
                (dest_dir.to_path_buf(), false)
            } else {
                (dest_dir.join(suggested), true)
            }
        }
    }
}

/// 兼容旧调用：根据是否开启智能解压，计算最终输出目录
pub fn resolve_extract_output_dir(
    archive_path: &Path,
    dest_dir: &Path,
    entries: &[ArchiveEntry],
    smart_extract: bool,
    has_selection: bool,
) -> PathBuf {
    let mode = if smart_extract {
        ExtractMode::Smart
    } else {
        ExtractMode::Here
    };
    resolve_extract_output_dir_with_mode(archive_path, dest_dir, entries, mode, has_selection).0
}

/// 解析解压完成后最适合在文件管理器中打开或选中的目标路径
/// 返回 `(primary_target, is_dir)`
pub fn resolve_primary_target(
    output_dir: &Path,
    created_subfolder: bool,
    entries: &[ArchiveEntry],
    selected_paths: &Option<Vec<String>>,
) -> (Option<String>, bool) {
    if created_subfolder {
        return (Some(output_dir.to_string_lossy().into_owned()), true);
    }

    let selection = build_selection_set(selected_paths);
    let mut top_levels: HashSet<String> = HashSet::new();
    let mut single_is_dir = false;

    for entry in entries {
        if is_system_junk(&entry.path) {
            continue;
        }
        if !matches_selection(&entry.path, &selection) {
            continue;
        }
        let trimmed = entry.path.trim_matches('/');
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(2, '/');
        if let Some(first) = parts.next() {
            let has_child = parts.next().is_some();
            if top_levels.insert(first.to_string()) && top_levels.len() == 1 {
                single_is_dir = has_child || entry.is_dir;
            } else if top_levels.len() == 1 && (has_child || entry.is_dir) {
                single_is_dir = true;
            }
        }
    }

    if top_levels.len() == 1 {
        let single_name = top_levels.into_iter().next().unwrap();
        let target = output_dir.join(&single_name);
        let is_dir = if target.exists() {
            target.is_dir()
        } else {
            single_is_dir
        };
        return (Some(target.to_string_lossy().into_owned()), is_dir);
    }

    (Some(output_dir.to_string_lossy().into_owned()), true)
}
