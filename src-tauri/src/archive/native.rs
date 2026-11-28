use crate::archive::encoding::{decode_entry_path, detect_best_encoding};
use crate::archive::smart::{build_selection_set, is_system_junk, matches_selection, safe_join};
use crate::archive::task_control::{copy_with_progress, ProgressReader, ProgressTracker};
use crate::error::{QPackError, QResult};
use crate::models::{
    ArchiveEntry, ArchiveFormat, CompressFormat, CompressOptions, CompressResult, ExtractOptions,
    ExtractResult,
};
use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;
use xz2::read::XzDecoder;
use xz2::write::XzEncoder;
use zip::write::SimpleFileOptions;
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter};

/// 提取路径最后一段作为展示文件名
fn entry_display_name(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    trimmed
        .rsplit('/')
        .next()
        .unwrap_or(trimmed)
        .to_string()
}

/// 将 zip::DateTime 格式化为可读时间字符串
fn format_zip_time(dt: Option<zip::DateTime>) -> Option<String> {
    let dt = dt?;
    Some(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        dt.year(),
        dt.month(),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    ))
}

/// 将 Unix 时间戳格式化为可读时间字符串
fn format_unix_time(ts: u64) -> Option<String> {
    let dt = DateTime::<Utc>::from_timestamp(ts as i64, 0)?;
    Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

/// 判断 7z 错误是否由缺少密码或密码错误导致
fn map_sevenz_error(err: sevenz_rust::Error) -> QPackError {
    match &err {
        sevenz_rust::Error::PasswordRequired | sevenz_rust::Error::MaybeBadPassword(_) => {
            return QPackError::InvalidPassword;
        }
        _ => {}
    }
    let msg = err.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("password")
        || lower.contains("aes")
        || lower.contains("decrypt")
        || lower.contains("crc")
        || lower.contains("checksum")
    {
        QPackError::InvalidPassword
    } else {
        QPackError::ArchiveError(msg)
    }
}

/// 将条目流写入目标文件，并在失败/取消时清理未完成的半成品文件
fn extract_stream_to_file<R: Read + ?Sized>(
    dest_path: &Path,
    reader: &mut R,
    tracker: &mut ProgressTracker,
) -> QResult<u64> {
    let mut out_file = File::create(dest_path)?;
    match copy_with_progress(reader, &mut out_file, tracker) {
        Ok(copied) => Ok(copied),
        Err(err) => {
            drop(out_file);
            let _ = fs::remove_file(dest_path);
            Err(err)
        }
    }
}

/// 读取 ZIP 压缩包条目（支持自动编码检测，且加密包无需密码即可浏览目录）
pub fn inspect_zip(
    path: &Path,
    override_encoding: Option<&str>,
) -> QResult<(Vec<ArchiveEntry>, String)> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file).map_err(|e| QPackError::ArchiveError(e.to_string()))?;

    let mut raw_names: Vec<Vec<u8>> = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = archive
            .by_index_raw(i)
            .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
        raw_names.push(entry.name_raw().to_vec());
    }

    let encoding = detect_best_encoding(&raw_names, override_encoding);
    let mut entries = Vec::with_capacity(archive.len());

    for i in 0..archive.len() {
        let entry = archive
            .by_index_raw(i)
            .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
        let decoded_path = decode_entry_path(entry.name_raw(), encoding);
        let is_dir = entry.is_dir() || decoded_path.ends_with('/');
        let name = entry_display_name(&decoded_path);

        entries.push(ArchiveEntry {
            path: decoded_path,
            name,
            is_dir,
            size: entry.size(),
            compressed_size: entry.compressed_size(),
            modified: format_zip_time(entry.last_modified()),
            encrypted: entry.encrypted(),
            index: i,
        });
    }

    Ok((entries, encoding.name().to_string()))
}

/// 解压 ZIP（支持编码修复、密码、部分提取、防 Zip Slip、字节级进度、暂停/取消）
pub fn extract_zip(
    archive_path: &Path,
    output_dir: &Path,
    options: &ExtractOptions,
    tracker: &mut ProgressTracker,
) -> QResult<ExtractResult> {
    let started = Instant::now();
    let (entries, _) = inspect_zip(archive_path, options.encoding.as_deref())?;
    let selection = build_selection_set(&options.selected_paths);

    fs::create_dir_all(output_dir)?;

    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file).map_err(|e| QPackError::ArchiveError(e.to_string()))?;

    let target_entries: Vec<&ArchiveEntry> = entries
        .iter()
        .filter(|e| matches_selection(&e.path, &selection))
        .collect();

    let total_items = target_entries.len();
    let total_bytes: u64 = target_entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.size)
        .sum();
    tracker.set_totals(total_items, total_bytes);

    let mut extracted_files = 0usize;
    let mut extracted_bytes = 0u64;

    for meta in target_entries {
        tracker.checkpoint()?;
        tracker.begin_file(&meta.path, if meta.is_dir { 0 } else { meta.size });

        let dest_path = safe_join(output_dir, &meta.path)?;
        if meta.is_dir {
            fs::create_dir_all(&dest_path)?;
            tracker.finish_file();
            continue;
        }

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if dest_path.exists() && !options.overwrite {
            tracker.add_bytes(meta.size);
            tracker.finish_file();
            continue;
        }

        let mut entry = if meta.encrypted {
            let pwd = options
                .password
                .as_deref()
                .filter(|p| !p.is_empty())
                .ok_or(QPackError::InvalidPassword)?;
            archive
                .by_index_decrypt(meta.index, pwd.as_bytes())
                .map_err(|e| match e {
                    zip::result::ZipError::InvalidPassword => QPackError::InvalidPassword,
                    other => QPackError::ArchiveError(other.to_string()),
                })?
        } else {
            archive
                .by_index(meta.index)
                .map_err(|e| QPackError::ArchiveError(e.to_string()))?
        };

        let copied = extract_stream_to_file(&dest_path, &mut entry, tracker)?;
        extracted_files += 1;
        extracted_bytes += copied;
        tracker.finish_file();
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

/// 打开 TAR 系列流读取器
fn open_tar_reader(path: &Path, format: ArchiveFormat) -> QResult<Box<dyn Read>> {
    let file = File::open(path)?;
    let reader: Box<dyn Read> = match format {
        ArchiveFormat::Tar => Box::new(file),
        ArchiveFormat::TarGz => Box::new(GzDecoder::new(file)),
        ArchiveFormat::TarBz2 => Box::new(BzDecoder::new(file)),
        ArchiveFormat::TarXz => Box::new(XzDecoder::new(file)),
        ArchiveFormat::TarZst => Box::new(
            zstd::stream::read::Decoder::new(file)
                .map_err(|e| QPackError::ArchiveError(e.to_string()))?,
        ),
        _ => return Err(QPackError::UnsupportedFormat(format.display_name().into())),
    };
    Ok(reader)
}

/// 读取 TAR / TAR.GZ / TAR.BZ2 / TAR.XZ / TAR.ZST 条目
pub fn inspect_tar(
    path: &Path,
    format: ArchiveFormat,
    override_encoding: Option<&str>,
) -> QResult<(Vec<ArchiveEntry>, String)> {
    let reader = open_tar_reader(path, format)?;
    let mut archive = tar::Archive::new(reader);

    let mut raw_entries = Vec::new();
    let mut raw_samples = Vec::new();

    for (idx, item) in archive
        .entries()
        .map_err(|e| QPackError::ArchiveError(e.to_string()))?
        .enumerate()
    {
        let entry = item.map_err(|e| QPackError::ArchiveError(e.to_string()))?;
        let raw_path = entry.path_bytes().into_owned();
        let is_dir = entry.header().entry_type().is_dir();
        let size = entry.header().size().unwrap_or(0);
        let mtime = entry.header().mtime().ok().and_then(format_unix_time);

        raw_samples.push(raw_path.clone());
        raw_entries.push((idx, raw_path, is_dir, size, mtime));
    }

    let encoding = detect_best_encoding(&raw_samples, override_encoding);
    let entries = raw_entries
        .into_iter()
        .map(|(index, raw_path, is_dir, size, modified)| {
            let decoded = decode_entry_path(&raw_path, encoding);
            let name = entry_display_name(&decoded);
            ArchiveEntry {
                path: decoded,
                name,
                is_dir,
                size,
                compressed_size: size,
                modified,
                encrypted: false,
                index,
            }
        })
        .collect();

    Ok((entries, encoding.name().to_string()))
}

/// 解压 TAR 系列压缩包（支持字节级进度、暂停/取消）
pub fn extract_tar(
    archive_path: &Path,
    format: ArchiveFormat,
    output_dir: &Path,
    options: &ExtractOptions,
    tracker: &mut ProgressTracker,
) -> QResult<ExtractResult> {
    let started = Instant::now();
    let (all_entries, _) = inspect_tar(archive_path, format, options.encoding.as_deref())?;
    let selection = build_selection_set(&options.selected_paths);

    let selected_entries: Vec<&ArchiveEntry> = all_entries
        .iter()
        .filter(|e| matches_selection(&e.path, &selection))
        .collect();
    let total_items = selected_entries.len();
    let total_bytes: u64 = selected_entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.size)
        .sum();
    tracker.set_totals(total_items, total_bytes);

    fs::create_dir_all(output_dir)?;

    let reader = open_tar_reader(archive_path, format)?;
    let mut archive = tar::Archive::new(reader);

    let mut extracted_files = 0usize;
    let mut extracted_bytes = 0u64;

    for (idx, item) in archive
        .entries()
        .map_err(|e| QPackError::ArchiveError(e.to_string()))?
        .enumerate()
    {
        tracker.checkpoint()?;
        let mut entry = item.map_err(|e| QPackError::ArchiveError(e.to_string()))?;
        let meta_entry = all_entries.get(idx);
        let meta_path = meta_entry
            .map(|e| e.path.clone())
            .unwrap_or_else(|| {
                entry
                    .path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });

        if !matches_selection(&meta_path, &selection) {
            continue;
        }

        let is_dir = entry.header().entry_type().is_dir();
        let entry_size = entry.header().size().unwrap_or(0);
        tracker.begin_file(&meta_path, if is_dir { 0 } else { entry_size });

        let dest_path = safe_join(output_dir, &meta_path)?;
        if is_dir {
            fs::create_dir_all(&dest_path)?;
            tracker.finish_file();
            continue;
        }

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if dest_path.exists() && !options.overwrite {
            tracker.add_bytes(entry_size);
            tracker.finish_file();
            continue;
        }

        let copied = extract_stream_to_file(&dest_path, &mut entry, tracker)?;
        extracted_files += 1;
        extracted_bytes += copied;
        tracker.finish_file();
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

/// 读取单文件流格式（.gz / .bz2 / .xz / .zst）
pub fn inspect_single_stream(path: &Path) -> QResult<(Vec<ArchiveEntry>, String)> {
    let meta = fs::metadata(path)?;
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let inner_name = ArchiveFormat::strip_archive_extension(file_name);

    let entry = ArchiveEntry {
        path: inner_name.clone(),
        name: inner_name,
        is_dir: false,
        size: meta.len(),
        compressed_size: meta.len(),
        modified: None,
        encrypted: false,
        index: 0,
    };

    Ok((vec![entry], "UTF-8".to_string()))
}

/// 解压单文件流格式（.gz / .bz2 / .xz / .zst），按压缩输入字节追踪进度并支持暂停/取消
pub fn extract_single_stream(
    archive_path: &Path,
    format: ArchiveFormat,
    output_dir: &Path,
    options: &ExtractOptions,
    tracker: &mut ProgressTracker,
) -> QResult<ExtractResult> {
    let started = Instant::now();
    fs::create_dir_all(output_dir)?;

    let compressed_len = fs::metadata(archive_path).map(|m| m.len()).unwrap_or(0);
    let file_name = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let inner_name = ArchiveFormat::strip_archive_extension(file_name);
    let dest_path = safe_join(output_dir, &inner_name)?;

    tracker.set_totals(1, compressed_len);
    tracker.checkpoint()?;
    tracker.begin_file(&inner_name, compressed_len);

    if !dest_path.exists() || options.overwrite {
        let file = File::open(archive_path)?;
        let mut out_file = File::create(&dest_path)?;

        let copy_res: io::Result<u64> = {
            let progress_in = ProgressReader::new(file, tracker);
            match format {
                ArchiveFormat::Gz => {
                    let mut dec = GzDecoder::new(progress_in);
                    io::copy(&mut dec, &mut out_file)
                }
                ArchiveFormat::Bz2 => {
                    let mut dec = BzDecoder::new(progress_in);
                    io::copy(&mut dec, &mut out_file)
                }
                ArchiveFormat::Xz => {
                    let mut dec = XzDecoder::new(progress_in);
                    io::copy(&mut dec, &mut out_file)
                }
                ArchiveFormat::Zst => match zstd::stream::read::Decoder::new(progress_in) {
                    Ok(mut dec) => io::copy(&mut dec, &mut out_file),
                    Err(e) => Err(e),
                },
                _ => {
                    return Err(QPackError::UnsupportedFormat(format.display_name().into()));
                }
            }
        };

        match copy_res {
            Ok(copied) => {
                tracker.finish_file();
                tracker.finish();
                return Ok(ExtractResult {
                    output_dir: output_dir.to_string_lossy().into_owned(),
                    extracted_files: 1,
                    total_bytes: copied,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    primary_target: None,
                    primary_target_is_dir: false,
                });
            }
            Err(err) => {
                drop(out_file);
                let _ = fs::remove_file(&dest_path);
                if tracker.is_cancelled() {
                    return Err(QPackError::Cancelled);
                }
                return Err(err.into());
            }
        }
    }

    tracker.add_bytes(compressed_len);
    tracker.finish_file();
    tracker.finish();
    Ok(ExtractResult {
        output_dir: output_dir.to_string_lossy().into_owned(),
        extracted_files: 0,
        total_bytes: 0,
        elapsed_ms: started.elapsed().as_millis() as u64,
        primary_target: None,
        primary_target_is_dir: false,
    })
}

/// 读取 7Z 压缩包条目（纯 Rust sevenz-rust）
pub fn inspect_sevenz_native(
    path: &Path,
    password: Option<&str>,
) -> QResult<(Vec<ArchiveEntry>, String)> {
    let pwd = password
        .filter(|p| !p.is_empty())
        .map(sevenz_rust::Password::from)
        .unwrap_or_else(sevenz_rust::Password::empty);

    let reader = sevenz_rust::SevenZReader::open(path, pwd).map_err(map_sevenz_error)?;

    let has_aes_block = reader.archive().folders.iter().any(|b| {
        b.coders
            .iter()
            .any(|c| c.decompression_method_id() == sevenz_rust::SevenZMethod::ID_AES256SHA256)
    });

    let entries = reader
        .archive()
        .files
        .iter()
        .enumerate()
        .map(|(index, f)| {
            let clean_path = f.name().replace('\\', "/");
            let name = entry_display_name(&clean_path);
            ArchiveEntry {
                path: clean_path,
                name,
                is_dir: f.is_directory(),
                size: f.size(),
                compressed_size: f.size(),
                modified: None,
                encrypted: has_aes_block && !f.is_directory(),
                index,
            }
        })
        .collect();

    Ok((entries, "UTF-8".to_string()))
}

/// 解压 7Z 压缩包（纯 Rust sevenz-rust，支持部分提取、AES-256 密码、字节级进度、暂停/取消）
pub fn extract_sevenz_native(
    archive_path: &Path,
    output_dir: &Path,
    options: &ExtractOptions,
    tracker: &mut ProgressTracker,
) -> QResult<ExtractResult> {
    let started = Instant::now();
    fs::create_dir_all(output_dir)?;

    let pwd = options
        .password
        .as_deref()
        .filter(|p| !p.is_empty())
        .map(sevenz_rust::Password::from)
        .unwrap_or_else(sevenz_rust::Password::empty);

    let mut reader =
        sevenz_rust::SevenZReader::open(archive_path, pwd).map_err(map_sevenz_error)?;

    let has_aes_block = reader.archive().folders.iter().any(|b| {
        b.coders
            .iter()
            .any(|c| c.decompression_method_id() == sevenz_rust::SevenZMethod::ID_AES256SHA256)
    });

    let selection = build_selection_set(&options.selected_paths);
    let mut total_items = 0usize;
    let mut total_bytes = 0u64;
    for f in &reader.archive().files {
        let clean_path = f.name().replace('\\', "/");
        if matches_selection(&clean_path, &selection) {
            total_items += 1;
            if !f.is_directory() {
                total_bytes = total_bytes.saturating_add(f.size());
            }
        }
    }
    tracker.set_totals(total_items, total_bytes);

    let mut extracted_files = 0usize;
    let mut extracted_bytes = 0u64;
    let mut cancelled_flag = false;

    let res = reader.for_each_entries(|entry, data_reader| {
        if let Err(err) = tracker.checkpoint() {
            if matches!(err, QPackError::Cancelled) {
                cancelled_flag = true;
            }
            return Err(sevenz_rust::Error::other(err.to_string()));
        }

        let clean_path = entry.name().replace('\\', "/");
        if !matches_selection(&clean_path, &selection) {
            // 需消耗当前条目流以保持 solid block 解码同步
            let _ = io::copy(data_reader, &mut io::sink());
            return Ok(true);
        }

        let entry_size = if entry.is_directory() { 0 } else { entry.size() };
        tracker.begin_file(&clean_path, entry_size);

        let dest_path = safe_join(output_dir, &clean_path)
            .map_err(|e| sevenz_rust::Error::other(e.to_string()))?;

        if entry.is_directory() {
            fs::create_dir_all(&dest_path)?;
            tracker.finish_file();
            return Ok(true);
        }

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if dest_path.exists() && !options.overwrite {
            let _ = io::copy(data_reader, &mut io::sink());
            tracker.add_bytes(entry_size);
            tracker.finish_file();
            return Ok(true);
        }

        match extract_stream_to_file(&dest_path, data_reader, tracker) {
            Ok(copied) => {
                extracted_files += 1;
                extracted_bytes += copied;
                tracker.finish_file();
                Ok(true)
            }
            Err(QPackError::Cancelled) => {
                cancelled_flag = true;
                Err(sevenz_rust::Error::other("cancelled"))
            }
            Err(err) => Err(sevenz_rust::Error::other(err.to_string())),
        }
    });

    if cancelled_flag || tracker.is_cancelled() {
        return Err(QPackError::Cancelled);
    }

    res.map_err(|e| {
        let mapped = map_sevenz_error(e);
        if has_aes_block {
            match &mapped {
                QPackError::InvalidPassword => mapped,
                QPackError::ArchiveError(msg) => {
                    let lower = msg.to_lowercase();
                    if lower.contains("corrupted")
                        || lower.contains("lzma")
                        || lower.contains("unexpected eof")
                        || lower.contains("invalid")
                    {
                        QPackError::InvalidPassword
                    } else {
                        mapped
                    }
                }
                _ => mapped,
            }
        } else {
            mapped
        }
    })?;
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

/// 收集待压缩的文件清单：(磁盘真实路径, 压缩包内相对路径, 是否目录, 文件大小)
fn collect_input_files(
    input_paths: &[String],
    exclude_junk: bool,
) -> QResult<Vec<(PathBuf, String, bool, u64)>> {
    let mut items = Vec::new();

    for raw in input_paths {
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
            if exclude_junk && is_system_junk(&base_name) {
                continue;
            }
            let size = fs::metadata(&root)?.len();
            items.push((root, base_name, false, size));
        } else {
            for entry in WalkDir::new(&root) {
                let entry = entry.map_err(|e| QPackError::ArchiveError(e.to_string()))?;
                let path = entry.path().to_path_buf();
                let rel = path
                    .strip_prefix(&root)
                    .map_err(|e| QPackError::ArchiveError(e.to_string()))?;

                let archive_path = if rel.as_os_str().is_empty() {
                    base_name.clone()
                } else {
                    format!(
                        "{}/{}",
                        base_name,
                        rel.to_string_lossy().replace('\\', "/")
                    )
                };

                if exclude_junk && is_system_junk(&archive_path) {
                    continue;
                }

                let is_dir = path.is_dir();
                let size = if is_dir {
                    0
                } else {
                    fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
                };
                items.push((path, archive_path, is_dir, size));
            }
        }
    }

    Ok(items)
}

/// 执行压缩（支持 ZIP / 7Z / TAR.GZ / TAR.ZST / TAR，含 7z AES-256 加密、字节进度、暂停/取消清理）
pub fn compress_native(
    options: &CompressOptions,
    tracker: &mut ProgressTracker,
) -> QResult<CompressResult> {
    let output_path = PathBuf::from(&options.output_path);
    match compress_native_inner(options, &output_path, tracker) {
        Ok(res) => Ok(res),
        Err(err) => {
            let _ = fs::remove_file(&output_path);
            if tracker.is_cancelled() {
                Err(QPackError::Cancelled)
            } else {
                Err(err)
            }
        }
    }
}

fn compress_native_inner(
    options: &CompressOptions,
    output_path: &Path,
    tracker: &mut ProgressTracker,
) -> QResult<CompressResult> {
    let started = Instant::now();
    let items = collect_input_files(&options.input_paths, options.exclude_junk)?;
    if items.is_empty() {
        return Err(QPackError::ArchiveError("没有可压缩的有效文件".into()));
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let total_items = items.len();
    let total_bytes: u64 = items
        .iter()
        .filter(|(_, _, is_dir, _)| !*is_dir)
        .map(|(_, _, _, sz)| *sz)
        .sum();
    tracker.set_totals(total_items, total_bytes);

    let mut input_files = 0usize;
    let mut original_bytes = 0u64;

    match options.format {
        CompressFormat::Zip => {
            let file = File::create(output_path)?;
            let mut writer = ZipWriter::new(file);

            let method = if options.level == 0 {
                CompressionMethod::Stored
            } else {
                CompressionMethod::Deflated
            };
            let level = (options.level.min(9)) as i64;

            for (disk_path, arc_path, is_dir, size) in &items {
                tracker.checkpoint()?;
                tracker.begin_file(arc_path, if *is_dir { 0 } else { *size });

                let mut file_opts = SimpleFileOptions::default()
                    .compression_method(method)
                    .compression_level(if options.level == 0 {
                        None
                    } else {
                        Some(level)
                    });

                if let Some(pwd) = options.password.as_deref().filter(|p| !p.is_empty()) {
                    file_opts = file_opts.with_aes_encryption(AesMode::Aes256, pwd);
                }

                if *is_dir {
                    let dir_name = if arc_path.ends_with('/') {
                        arc_path.clone()
                    } else {
                        format!("{}/", arc_path)
                    };
                    writer
                        .add_directory(dir_name, file_opts)
                        .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
                } else {
                    writer
                        .start_file(arc_path, file_opts)
                        .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
                    let mut src = File::open(disk_path)?;
                    copy_with_progress(&mut src, &mut writer, tracker)?;
                    input_files += 1;
                    original_bytes += *size;
                }
                tracker.finish_file();
            }

            writer
                .finish()
                .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
        }

        CompressFormat::Tar
        | CompressFormat::TarGz
        | CompressFormat::TarBz2
        | CompressFormat::TarXz
        | CompressFormat::TarZst => {
            let file = File::create(output_path)?;
            let level = options.level.min(9) as u32;

            let encoder: Box<dyn Write> = match options.format {
                CompressFormat::Tar => Box::new(file),
                CompressFormat::TarGz => Box::new(GzEncoder::new(file, Compression::new(level))),
                CompressFormat::TarBz2 => {
                    let bz_level = bzip2::Compression::new(level.clamp(1, 9));
                    Box::new(BzEncoder::new(file, bz_level))
                }
                CompressFormat::TarXz => Box::new(XzEncoder::new(file, level.clamp(0, 9))),
                CompressFormat::TarZst => {
                    let zstd_level = (options.level.min(9) as i32 * 2).max(1);
                    let enc = zstd::stream::write::Encoder::new(file, zstd_level)
                        .map_err(|e| QPackError::ArchiveError(e.to_string()))?
                        .auto_finish();
                    Box::new(enc)
                }
                _ => unreachable!(),
            };

            let mut tar_builder = tar::Builder::new(encoder);
            for (disk_path, arc_path, is_dir, size) in &items {
                tracker.checkpoint()?;
                tracker.begin_file(arc_path, if *is_dir { 0 } else { *size });

                if *is_dir {
                    tar_builder
                        .append_dir(arc_path, disk_path)
                        .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
                } else {
                    let meta = fs::metadata(disk_path)?;
                    let mut header = tar::Header::new_gnu();
                    header.set_metadata(&meta);
                    header.set_cksum();

                    let src = File::open(disk_path)?;
                    let mut reader = ProgressReader::new(src, tracker);
                    tar_builder
                        .append_data(&mut header, arc_path, &mut reader)
                        .map_err(|e| {
                            if tracker.is_cancelled() {
                                QPackError::Cancelled
                            } else {
                                QPackError::ArchiveError(e.to_string())
                            }
                        })?;
                    input_files += 1;
                    original_bytes += *size;
                }
                tracker.finish_file();
            }

            tar_builder
                .finish()
                .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
            let mut inner = tar_builder
                .into_inner()
                .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
            inner.flush()?;
        }

        CompressFormat::SevenZ => {
            let mut writer = sevenz_rust::SevenZWriter::create(output_path)
                .map_err(|e| QPackError::ArchiveError(e.to_string()))?;

            let mut methods: Vec<sevenz_rust::SevenZMethodConfiguration> = Vec::new();
            if let Some(pwd) = options.password.as_deref().filter(|p| !p.is_empty()) {
                methods.push(
                    sevenz_rust::AesEncoderOptions::new(sevenz_rust::Password::from(pwd)).into(),
                );
                writer.set_encrypt_header(true);
            }
            let preset = options.level.min(9) as u32;
            methods.push(sevenz_rust::lzma::LZMA2Options::with_preset(preset).into());
            writer.set_content_methods(methods);

            for (disk_path, arc_path, is_dir, size) in &items {
                tracker.checkpoint()?;
                tracker.begin_file(arc_path, if *is_dir { 0 } else { *size });

                if *is_dir {
                    let entry =
                        sevenz_rust::SevenZArchiveEntry::from_path(disk_path, arc_path.clone());
                    writer
                        .push_archive_entry::<&[u8]>(entry, None)
                        .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
                } else {
                    let entry =
                        sevenz_rust::SevenZArchiveEntry::from_path(disk_path, arc_path.clone());
                    let src = File::open(disk_path)?;
                    let reader = ProgressReader::new(src, tracker);
                    writer
                        .push_archive_entry(entry, Some(reader))
                        .map_err(|e| {
                            if tracker.is_cancelled() {
                                QPackError::Cancelled
                            } else {
                                QPackError::ArchiveError(e.to_string())
                            }
                        })?;
                    input_files += 1;
                    original_bytes += *size;
                }
                tracker.finish_file();
            }

            writer
                .finish()
                .map_err(|e| QPackError::ArchiveError(e.to_string()))?;
        }

        CompressFormat::Dmg => {
            return Err(QPackError::UnsupportedFormat(
                "DMG 创建由 macOS 原生引擎处理".into(),
            ));
        }
    }

    tracker.finish();
    let archive_bytes = fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    Ok(CompressResult {
        output_path: output_path.to_string_lossy().into_owned(),
        input_files,
        original_bytes,
        archive_bytes,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}
