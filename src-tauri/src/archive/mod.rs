pub mod dmg;
pub mod encoding;
pub mod native;
pub mod sevenz;
pub mod smart;
pub mod task_control;

use crate::archive::task_control::{ProgressCallback, ProgressTracker, TaskController};
use crate::error::{QPackError, QResult};
use crate::models::{
    ArchiveFormat, ArchiveInfo, CompressOptions, CompressResult, EngineStatus, ExtractMode,
    ExtractOptions, ExtractResult,
};
use std::fs;
use std::path::Path;

/// 根据 ExtractOptions 推导最终的 ExtractMode
pub fn effective_extract_mode(options: &ExtractOptions) -> ExtractMode {
    options.extract_mode.unwrap_or(if options.smart_extract {
        ExtractMode::Smart
    } else {
        ExtractMode::Here
    })
}

/// 探测当前可用引擎与支持格式
pub fn get_engine_status() -> EngineStatus {
    let sevenz_bin = sevenz::find_sevenz_binary();
    let mut read_formats = vec![
        "zip", "7z", "tar", "tar.gz", "tgz", "tar.bz2", "tar.xz", "tar.zst", "gz", "bz2", "xz",
        "zst",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();

    if cfg!(target_os = "macos") {
        read_formats.push("dmg".into());
    }

    if sevenz_bin.is_some() {
        for ext in [
            "rar", "iso", "cab", "dmg", "wim", "rpm", "deb", "lzh", "arj", "cpio",
        ] {
            if !read_formats.iter().any(|f| f == ext) {
                read_formats.push(ext.into());
            }
        }
    }

    let mut write_formats = vec![
        "zip".into(),
        "7z".into(),
        "tar.zst".into(),
        "tar.gz".into(),
        "tar.xz".into(),
        "tar.bz2".into(),
        "tar".into(),
    ];
    if cfg!(target_os = "macos") {
        write_formats.push("dmg".into());
    }

    EngineStatus {
        native_ready: true,
        sevenz_cli_ready: sevenz_bin.is_some(),
        sevenz_cli_path: sevenz_bin.map(|p| p.to_string_lossy().into_owned()),
        supported_read_formats: read_formats,
        supported_write_formats: write_formats,
    }
}

/// 统一入口：检查并解析压缩包内容
pub fn inspect_archive(
    archive_path: &Path,
    override_encoding: Option<&str>,
    password: Option<&str>,
) -> QResult<ArchiveInfo> {
    if !archive_path.exists() {
        return Err(QPackError::NotFound(
            archive_path.to_string_lossy().into_owned(),
        ));
    }

    let format = ArchiveFormat::from_path(archive_path);
    let archive_size = fs::metadata(archive_path)?.len();
    let file_name = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("archive")
        .to_string();

    let (entries, detected_encoding, engine) = match format {
        ArchiveFormat::Zip => match native::inspect_zip(archive_path, override_encoding) {
            Ok((e, enc)) => (e, enc, "Rust 原生引擎".to_string()),
            Err(_) if sevenz::find_sevenz_binary().is_some() => {
                let e = sevenz::inspect_with_sevenz_cli(archive_path, password)?;
                (e, "UTF-8".to_string(), "内置 7-Zip 引擎".to_string())
            }
            Err(err) => return Err(err),
        },
        ArchiveFormat::Tar
        | ArchiveFormat::TarGz
        | ArchiveFormat::TarBz2
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarZst => {
            let (e, enc) = native::inspect_tar(archive_path, format, override_encoding)?;
            (e, enc, "Rust 原生引擎".to_string())
        }
        ArchiveFormat::Gz | ArchiveFormat::Bz2 | ArchiveFormat::Xz | ArchiveFormat::Zst => {
            let (e, enc) = native::inspect_single_stream(archive_path)?;
            (e, enc, "Rust 原生引擎".to_string())
        }
        ArchiveFormat::SevenZ => match native::inspect_sevenz_native(archive_path, password) {
            Ok((e, enc)) => (e, enc, "Rust 原生引擎 (7z)".to_string()),
            Err(QPackError::InvalidPassword) => return Err(QPackError::InvalidPassword),
            Err(_) if sevenz::find_sevenz_binary().is_some() => {
                let e = sevenz::inspect_with_sevenz_cli(archive_path, password)?;
                (e, "UTF-8".to_string(), "内置 7-Zip 引擎".to_string())
            }
            Err(err) => return Err(err),
        },
        ArchiveFormat::Dmg => {
            #[cfg(target_os = "macos")]
            {
                let e = dmg::inspect_dmg_macos(archive_path)?;
                (e, "UTF-8".to_string(), "macOS 原生 DMG 引擎".to_string())
            }
            #[cfg(not(target_os = "macos"))]
            {
                let e = sevenz::inspect_with_sevenz_cli(archive_path, password)?;
                (e, "UTF-8".to_string(), "内置 7-Zip 引擎".to_string())
            }
        }
        ArchiveFormat::Rar
        | ArchiveFormat::Iso
        | ArchiveFormat::Cab
        | ArchiveFormat::Wim
        | ArchiveFormat::Rpm
        | ArchiveFormat::Deb
        | ArchiveFormat::Lzh
        | ArchiveFormat::Arj
        | ArchiveFormat::Cpio => {
            let e = sevenz::inspect_with_sevenz_cli(archive_path, password)?;
            (e, "UTF-8".to_string(), "内置 7-Zip 引擎".to_string())
        }
        ArchiveFormat::Unknown => {
            return Err(QPackError::UnsupportedFormat(file_name));
        }
    };

    let (will_create_folder, suggested_folder_name) =
        smart::analyze_smart_extract(archive_path, &entries);

    let mut file_count = 0usize;
    let mut dir_count = 0usize;
    let mut total_uncompressed_size = 0u64;
    let mut is_encrypted = false;

    for e in &entries {
        if e.is_dir {
            dir_count += 1;
        } else {
            file_count += 1;
            total_uncompressed_size = total_uncompressed_size.saturating_add(e.size);
        }
        if e.encrypted {
            is_encrypted = true;
        }
    }

    Ok(ArchiveInfo {
        path: archive_path.to_string_lossy().into_owned(),
        file_name,
        format,
        format_label: format.display_name().to_string(),
        archive_size,
        total_uncompressed_size,
        file_count,
        dir_count,
        is_encrypted,
        detected_encoding,
        engine,
        smart_extract_will_create_folder: will_create_folder,
        suggested_folder_name,
        entries,
    })
}

/// 解压目标冲突检测结果
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractConflictInfo {
    pub has_conflict: bool,
    pub conflict_path: String,
    pub suggested_rename_dir: String,
}

/// 为已存在的目录生成不冲突的新目录名（如 "资料" -> "资料 (2)"）
fn next_available_dir(base_dir: &Path) -> std::path::PathBuf {
    if !base_dir.exists() {
        return base_dir.to_path_buf();
    }
    let parent = base_dir.parent().unwrap_or_else(|| Path::new("."));
    let stem = base_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("extracted");
    for n in 2..1000 {
        let candidate = parent.join(format!("{} ({})", stem, n));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{}_new", stem))
}

/// 检测解压到目标目录是否会与已有文件/文件夹冲突
pub fn check_extract_conflict(options: &ExtractOptions) -> QResult<ExtractConflictInfo> {
    let archive_path = Path::new(&options.archive_path);
    if !archive_path.exists() {
        return Err(QPackError::NotFound(options.archive_path.clone()));
    }

    let info = inspect_archive(
        archive_path,
        options.encoding.as_deref(),
        options.password.as_deref(),
    )?;

    let has_selection = options
        .selected_paths
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    let dest_root = Path::new(&options.dest_dir);
    let mode = effective_extract_mode(options);
    let (output_dir, created_subfolder) = smart::resolve_extract_output_dir_with_mode(
        archive_path,
        dest_root,
        &info.entries,
        mode,
        has_selection,
    );

    // 情况 1：新建了专属目录（output_dir != dest_root）
    if created_subfolder && output_dir != dest_root {
        let has_conflict = output_dir.exists();
        let suggested = next_available_dir(&output_dir);
        return Ok(ExtractConflictInfo {
            has_conflict,
            conflict_path: output_dir.to_string_lossy().into_owned(),
            suggested_rename_dir: suggested.to_string_lossy().into_owned(),
        });
    }

    // 情况 2：直接展开到 dest_root，检查待解压的顶层条目是否已存在
    let selection = smart::build_selection_set(&options.selected_paths);
    let mut first_conflict: Option<std::path::PathBuf> = None;
    for entry in &info.entries {
        if smart::is_system_junk(&entry.path) {
            continue;
        }
        if !smart::matches_selection(&entry.path, &selection) {
            continue;
        }
        let clean = entry.path.trim_start_matches('/').trim_end_matches('/');
        if clean.is_empty() {
            continue;
        }
        let top = clean.split('/').next().unwrap_or(clean);
        let candidate = dest_root.join(top);
        if candidate.exists() {
            first_conflict = Some(candidate);
            break;
        }
    }

    if let Some(conflict_p) = first_conflict {
        let fallback_name = if info.suggested_folder_name.is_empty() {
            "extracted".to_string()
        } else {
            info.suggested_folder_name.clone()
        };
        let suggested = next_available_dir(&dest_root.join(fallback_name));
        Ok(ExtractConflictInfo {
            has_conflict: true,
            conflict_path: conflict_p.to_string_lossy().into_owned(),
            suggested_rename_dir: suggested.to_string_lossy().into_owned(),
        })
    } else {
        Ok(ExtractConflictInfo {
            has_conflict: false,
            conflict_path: output_dir.to_string_lossy().into_owned(),
            suggested_rename_dir: output_dir.to_string_lossy().into_owned(),
        })
    }
}

/// 统一入口：执行压缩包解压（含模式目录规划、任务控制与主要定位目标解析）
pub fn extract_archive(
    options: &ExtractOptions,
    control: TaskController,
    on_progress: ProgressCallback<'_>,
) -> QResult<ExtractResult> {
    let archive_path = Path::new(&options.archive_path);
    if !archive_path.exists() {
        return Err(QPackError::NotFound(options.archive_path.clone()));
    }

    let info = inspect_archive(
        archive_path,
        options.encoding.as_deref(),
        options.password.as_deref(),
    )?;

    let has_selection = options
        .selected_paths
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    let mode = effective_extract_mode(options);
    let (output_dir, created_subfolder) = smart::resolve_extract_output_dir_with_mode(
        archive_path,
        Path::new(&options.dest_dir),
        &info.entries,
        mode,
        has_selection,
    );

    let mut tracker = ProgressTracker::new(control, on_progress);

    let mut result = match info.format {
        ArchiveFormat::Zip => {
            if info.engine.contains("7-Zip") {
                sevenz::extract_with_sevenz_cli(archive_path, &output_dir, options, &mut tracker)
            } else {
                match native::extract_zip(archive_path, &output_dir, options, &mut tracker) {
                    Ok(r) => Ok(r),
                    Err(e @ QPackError::InvalidPassword) | Err(e @ QPackError::Cancelled) => Err(e),
                    Err(_) if sevenz::find_sevenz_binary().is_some() => {
                        sevenz::extract_with_sevenz_cli(
                            archive_path,
                            &output_dir,
                            options,
                            &mut tracker,
                        )
                    }
                    Err(e) => Err(e),
                }
            }
        }
        ArchiveFormat::Tar
        | ArchiveFormat::TarGz
        | ArchiveFormat::TarBz2
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarZst => native::extract_tar(
            archive_path,
            info.format,
            &output_dir,
            options,
            &mut tracker,
        ),
        ArchiveFormat::Gz | ArchiveFormat::Bz2 | ArchiveFormat::Xz | ArchiveFormat::Zst => {
            native::extract_single_stream(
                archive_path,
                info.format,
                &output_dir,
                options,
                &mut tracker,
            )
        }
        ArchiveFormat::SevenZ => {
            if info.engine.contains("7-Zip") {
                sevenz::extract_with_sevenz_cli(archive_path, &output_dir, options, &mut tracker)
            } else {
                match native::extract_sevenz_native(archive_path, &output_dir, options, &mut tracker)
                {
                    Ok(r) => Ok(r),
                    Err(e @ QPackError::InvalidPassword) | Err(e @ QPackError::Cancelled) => Err(e),
                    Err(_) if sevenz::find_sevenz_binary().is_some() => {
                        sevenz::extract_with_sevenz_cli(
                            archive_path,
                            &output_dir,
                            options,
                            &mut tracker,
                        )
                    }
                    Err(e) => Err(e),
                }
            }
        }
        ArchiveFormat::Dmg => {
            #[cfg(target_os = "macos")]
            {
                dmg::extract_dmg_macos(archive_path, &output_dir, options, &mut tracker)
            }
            #[cfg(not(target_os = "macos"))]
            {
                sevenz::extract_with_sevenz_cli(archive_path, &output_dir, options, &mut tracker)
            }
        }
        ArchiveFormat::Rar
        | ArchiveFormat::Iso
        | ArchiveFormat::Cab
        | ArchiveFormat::Wim
        | ArchiveFormat::Rpm
        | ArchiveFormat::Deb
        | ArchiveFormat::Lzh
        | ArchiveFormat::Arj
        | ArchiveFormat::Cpio => {
            sevenz::extract_with_sevenz_cli(archive_path, &output_dir, options, &mut tracker)
        }
        ArchiveFormat::Unknown => Err(QPackError::UnsupportedFormat(info.file_name)),
    }?;

    let (primary_target, primary_target_is_dir) = smart::resolve_primary_target(
        &output_dir,
        created_subfolder,
        &info.entries,
        &options.selected_paths,
    );
    result.primary_target = primary_target;
    result.primary_target_is_dir = primary_target_is_dir;

    Ok(result)
}

/// 统一入口：执行压缩
pub fn compress_files(
    options: &CompressOptions,
    control: TaskController,
    on_progress: ProgressCallback<'_>,
) -> QResult<CompressResult> {
    let mut tracker = ProgressTracker::new(control, on_progress);
    if options.format == crate::models::CompressFormat::Dmg {
        #[cfg(target_os = "macos")]
        {
            return dmg::compress_dmg_macos(options, &mut tracker);
        }
        #[cfg(not(target_os = "macos"))]
        {
            return Err(QPackError::UnsupportedFormat(
                "仅 macOS 支持创建 DMG 磁盘映像".into(),
            ));
        }
    }
    native::compress_native(options, &mut tracker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::task_control::ProgressSnapshot;
    use crate::models::{ArchiveEntry, CompressFormat};
    use std::fs;
    use std::path::PathBuf;

    fn temp_workspace(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("qpack_test_{}_{}", name, uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_safe_join_blocks_zip_slip() {
        let base = PathBuf::from("/tmp/qpack_safe");
        assert!(smart::safe_join(&base, "folder/a.txt").is_ok());
        assert!(smart::safe_join(&base, "../etc/passwd").is_err());
        assert!(smart::safe_join(&base, "/abs/path.txt").is_err());
    }

    #[test]
    fn test_smart_extract_analysis() {
        let single_folder = vec![
            ArchiveEntry {
                path: "my_project/".into(),
                name: "my_project".into(),
                is_dir: true,
                size: 0,
                compressed_size: 0,
                modified: None,
                encrypted: false,
                index: 0,
            },
            ArchiveEntry {
                path: "my_project/readme.md".into(),
                name: "readme.md".into(),
                is_dir: false,
                size: 10,
                compressed_size: 8,
                modified: None,
                encrypted: false,
                index: 1,
            },
            ArchiveEntry {
                path: "__MACOSX/._my_project".into(),
                name: "._my_project".into(),
                is_dir: false,
                size: 100,
                compressed_size: 50,
                modified: None,
                encrypted: false,
                index: 2,
            },
        ];

        let (will_create, _) = smart::analyze_smart_extract(Path::new("demo.zip"), &single_folder);
        assert!(!will_create, "单个顶层目录时不应额外套娃建目录");

        let loose_files = vec![
            ArchiveEntry {
                path: "a.txt".into(),
                name: "a.txt".into(),
                is_dir: false,
                size: 10,
                compressed_size: 10,
                modified: None,
                encrypted: false,
                index: 0,
            },
            ArchiveEntry {
                path: "b.txt".into(),
                name: "b.txt".into(),
                is_dir: false,
                size: 10,
                compressed_size: 10,
                modified: None,
                encrypted: false,
                index: 1,
            },
        ];

        let (will_create2, folder) =
            smart::analyze_smart_extract(Path::new("bundle.tar.gz"), &loose_files);
        assert!(will_create2, "散落多个文件时应自动创建同名文件夹");
        assert_eq!(folder, "bundle");
    }

    #[test]
    fn test_gbk_encoding_detection() {
        let (gbk_bytes, _, _) = encoding_rs::GB18030.encode("中文测试目录/说明文档.txt");
        let samples = vec![gbk_bytes.into_owned()];
        let enc = encoding::detect_best_encoding(&samples, None);
        let decoded = encoding::decode_entry_path(&samples[0], enc);
        assert_eq!(decoded, "中文测试目录/说明文档.txt");
    }

    #[test]
    fn test_roundtrip_zip_tar_zst_and_7z() {
        let work = temp_workspace("roundtrip");
        let src_dir = work.join("source");
        fs::create_dir_all(src_dir.join("sub")).unwrap();
        fs::write(src_dir.join("hello.txt"), "你好 QPack!").unwrap();
        fs::write(src_dir.join("sub/data.txt"), "rust + tauri").unwrap();
        fs::write(src_dir.join(".DS_Store"), "junk").unwrap();

        let formats = [
            (CompressFormat::Zip, "out.zip"),
            (CompressFormat::TarGz, "out.tar.gz"),
            (CompressFormat::TarBz2, "out.tar.bz2"),
            (CompressFormat::TarXz, "out.tar.xz"),
            (CompressFormat::TarZst, "out.tar.zst"),
            (CompressFormat::SevenZ, "out.7z"),
        ];

        for (fmt, filename) in formats {
            let out_arc = work.join(filename);
            let mut noop = |_: &ProgressSnapshot| {};
            let comp = compress_files(
                &CompressOptions {
                    input_paths: vec![src_dir.to_string_lossy().into_owned()],
                    output_path: out_arc.to_string_lossy().into_owned(),
                    format: fmt,
                    level: 6,
                    password: None,
                    exclude_junk: true,
                    task_id: None,
                },
                TaskController::noop(),
                &mut noop,
            )
            .expect("压缩应成功");
            assert_eq!(comp.input_files, 2, "应排除 .DS_Store 并压缩 2 个有效文件");

            let info = inspect_archive(&out_arc, None, None).expect("解析应成功");
            assert!(info.file_count >= 2);
            assert!(
                !info.entries.iter().any(|e| e.name == ".DS_Store"),
                "不应包含被过滤的 .DS_Store"
            );

            let extract_dir = work.join(format!("ext_{}", filename));
            let res = extract_archive(
                &ExtractOptions {
                    archive_path: out_arc.to_string_lossy().into_owned(),
                    dest_dir: extract_dir.to_string_lossy().into_owned(),
                    smart_extract: true,
                    password: None,
                    encoding: None,
                    selected_paths: None,
                    overwrite: true,
                    extract_mode: Some(ExtractMode::Smart),
                    task_id: None,
                },
                TaskController::noop(),
                &mut noop,
            )
            .expect("解压应成功");

            assert!(res.extracted_files >= 2);
            assert!(res.primary_target.is_some());
            assert!(res.primary_target_is_dir);
            let content = fs::read_to_string(extract_dir.join("source/hello.txt")).unwrap();
            assert_eq!(content, "你好 QPack!");
        }

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn test_7z_aes256_encryption_and_wrong_password() {
        let work = temp_workspace("7z_aes");
        let src = work.join("secret.txt");
        fs::write(&src, "绝密数据 QPack 7z AES-256").unwrap();

        let out_7z = work.join("secret.7z");
        let mut noop = |_: &ProgressSnapshot| {};
        compress_files(
            &CompressOptions {
                input_paths: vec![src.to_string_lossy().into_owned()],
                output_path: out_7z.to_string_lossy().into_owned(),
                format: CompressFormat::SevenZ,
                level: 5,
                password: Some("qpack-pass-123".into()),
                exclude_junk: true,
                task_id: None,
            },
            TaskController::noop(),
            &mut noop,
        )
        .expect("加密 7z 压缩应成功");

        // 加密 7z 可浏览目录并标记为已加密，但无密码或错密码解压应返回 InvalidPassword
        let info = inspect_archive(&out_7z, None, None).expect("应能读取加密 7z 的条目信息");
        assert!(info.is_encrypted, "加密 7z 应标记 is_encrypted = true");

        let ext_bad = work.join("out_bad");
        let err_no_pwd = extract_archive(
            &ExtractOptions {
                archive_path: out_7z.to_string_lossy().into_owned(),
                dest_dir: ext_bad.to_string_lossy().into_owned(),
                smart_extract: false,
                password: None,
                encoding: None,
                selected_paths: None,
                overwrite: true,
                extract_mode: Some(ExtractMode::Here),
                task_id: None,
            },
            TaskController::noop(),
            &mut noop,
        )
        .unwrap_err();
        assert!(
            matches!(err_no_pwd, QPackError::InvalidPassword),
            "无密码解压加密 7z 应返回 InvalidPassword，实际: {:?}",
            err_no_pwd
        );

        let err_bad_pwd = extract_archive(
            &ExtractOptions {
                archive_path: out_7z.to_string_lossy().into_owned(),
                dest_dir: ext_bad.to_string_lossy().into_owned(),
                smart_extract: false,
                password: Some("wrong".into()),
                encoding: None,
                selected_paths: None,
                overwrite: true,
                extract_mode: Some(ExtractMode::Here),
                task_id: None,
            },
            TaskController::noop(),
            &mut noop,
        )
        .unwrap_err();
        assert!(
            matches!(err_bad_pwd, QPackError::InvalidPassword),
            "错密码解压加密 7z 应返回 InvalidPassword，实际: {:?}",
            err_bad_pwd
        );

        // 正确密码可正常解析与解压
        let ext_dir = work.join("out");
        let res = extract_archive(
            &ExtractOptions {
                archive_path: out_7z.to_string_lossy().into_owned(),
                dest_dir: ext_dir.to_string_lossy().into_owned(),
                smart_extract: false,
                password: Some("qpack-pass-123".into()),
                encoding: None,
                selected_paths: None,
                overwrite: true,
                extract_mode: Some(ExtractMode::Here),
                task_id: None,
            },
            TaskController::noop(),
            &mut noop,
        )
        .expect("正确密码解压 7z 应成功");

        assert_eq!(res.extracted_files, 1);
        assert!(!res.primary_target_is_dir);
        let content = fs::read_to_string(ext_dir.join("secret.txt")).unwrap();
        assert_eq!(content, "绝密数据 QPack 7z AES-256");

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn test_cancel_compress_cleans_partial_output() {
        let work = temp_workspace("cancel_clean");
        let src = work.join("big.bin");
        fs::write(&src, vec![42u8; 256 * 1024]).unwrap();

        let out_zip = work.join("aborted.zip");
        let control = TaskController::new();
        control.cancel();

        let mut noop = |_: &ProgressSnapshot| {};
        let err = compress_files(
            &CompressOptions {
                input_paths: vec![src.to_string_lossy().into_owned()],
                output_path: out_zip.to_string_lossy().into_owned(),
                format: CompressFormat::Zip,
                level: 6,
                password: None,
                exclude_junk: true,
                task_id: None,
            },
            control,
            &mut noop,
        )
        .unwrap_err();

        assert!(matches!(err, QPackError::Cancelled));
        assert!(!out_zip.exists(), "取消压缩后应删除半成品文件");

        let _ = fs::remove_dir_all(&work);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_native_dmg_roundtrip() {
        let work = temp_workspace("dmg_roundtrip");
        let src_dir = work.join("demo_app");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("readme.txt"), "hello dmg native!").unwrap();

        let out_dmg = work.join("demo.dmg");
        let mut noop = |_: &ProgressSnapshot| {};
        let comp = compress_files(
            &CompressOptions {
                input_paths: vec![src_dir.to_string_lossy().into_owned()],
                output_path: out_dmg.to_string_lossy().into_owned(),
                format: CompressFormat::Dmg,
                level: 6,
                password: None,
                exclude_junk: true,
                task_id: None,
            },
            TaskController::noop(),
            &mut noop,
        )
        .expect("macOS 原生 DMG 打包应成功");
        assert_eq!(comp.input_files, 1);
        assert!(out_dmg.exists());

        let info = inspect_archive(&out_dmg, None, None).expect("macOS 原生 DMG 解析应成功");
        assert!(info.engine.contains("macOS 原生 DMG"));
        assert!(info.entries.iter().any(|e| e.name == "readme.txt"));

        let ext_dir = work.join("extracted");
        let res = extract_archive(
            &ExtractOptions {
                archive_path: out_dmg.to_string_lossy().into_owned(),
                dest_dir: ext_dir.to_string_lossy().into_owned(),
                smart_extract: true,
                password: None,
                encoding: None,
                selected_paths: None,
                overwrite: true,
                extract_mode: Some(ExtractMode::Smart),
                task_id: None,
            },
            TaskController::noop(),
            &mut noop,
        )
        .expect("macOS 原生 DMG 解压应成功");
        assert_eq!(res.extracted_files, 1);
        let content = fs::read_to_string(ext_dir.join("demo_app/readme.txt")).unwrap();
        assert_eq!(content, "hello dmg native!");

        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn test_bundled_sevenz_engine_ready() {
        let status = get_engine_status();
        assert!(status.sevenz_cli_ready, "内置 7-Zip 引擎应开箱即用");
        assert!(status.supported_read_formats.iter().any(|f| f == "rar"));
        assert!(status.supported_read_formats.iter().any(|f| f == "iso"));
        assert!(status.supported_read_formats.iter().any(|f| f == "dmg"));
    }
}
