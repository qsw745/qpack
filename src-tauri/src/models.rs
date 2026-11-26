use serde::{Deserialize, Serialize};
use std::path::Path;

/// 支持的压缩包格式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveFormat {
    Zip,
    SevenZ,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    TarZst,
    Gz,
    Bz2,
    Xz,
    Zst,
    Rar,
    Iso,
    Cab,
    Dmg,
    Unknown,
}

impl ArchiveFormat {
    /// 根据文件路径后缀识别压缩包格式
    pub fn from_path(path: &Path) -> Self {
        let lower = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
            return Self::TarGz;
        }
        if lower.ends_with(".tar.bz2") || lower.ends_with(".tbz2") || lower.ends_with(".tbz") {
            return Self::TarBz2;
        }
        if lower.ends_with(".tar.xz") || lower.ends_with(".txz") {
            return Self::TarXz;
        }
        if lower.ends_with(".tar.zst") || lower.ends_with(".tzst") {
            return Self::TarZst;
        }

        match path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "zip" | "jar" | "apk" | "ipa" | "epub" | "cbz" => Self::Zip,
            "7z" => Self::SevenZ,
            "tar" => Self::Tar,
            "gz" => Self::Gz,
            "bz2" => Self::Bz2,
            "xz" => Self::Xz,
            "zst" | "zstd" => Self::Zst,
            "rar" => Self::Rar,
            "iso" => Self::Iso,
            "cab" => Self::Cab,
            "dmg" => Self::Dmg,
            _ => Self::Unknown,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Zip => "ZIP",
            Self::SevenZ => "7Z",
            Self::Tar => "TAR",
            Self::TarGz => "TAR.GZ",
            Self::TarBz2 => "TAR.BZ2",
            Self::TarXz => "TAR.XZ",
            Self::TarZst => "TAR.ZST",
            Self::Gz => "GZIP",
            Self::Bz2 => "BZIP2",
            Self::Xz => "XZ",
            Self::Zst => "ZSTD",
            Self::Rar => "RAR",
            Self::Iso => "ISO",
            Self::Cab => "CAB",
            Self::Dmg => "DMG",
            Self::Unknown => "未知格式",
        }
    }

    /// 去除压缩包扩展名，生成默认解压目录名
    pub fn strip_archive_extension(file_name: &str) -> String {
        let lower = file_name.to_lowercase();
        for suffix in &[
            ".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst", ".tgz", ".tbz2", ".tbz", ".txz",
            ".tzst", ".zip", ".7z", ".rar", ".tar", ".gz", ".bz2", ".xz", ".zst", ".zstd", ".iso",
            ".cab", ".dmg",
        ] {
            if lower.ends_with(suffix) && file_name.len() > suffix.len() {
                return file_name[..file_name.len() - suffix.len()].to_string();
            }
        }
        format!("{}_extracted", file_name)
    }
}

/// 压缩包内的单个条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    /// 标准化后的相对路径（使用 `/` 分隔）
    pub path: String,
    /// 文件或文件夹名
    pub name: String,
    /// 是否为目录
    pub is_dir: bool,
    /// 未压缩大小（字节）
    pub size: u64,
    /// 压缩后大小（字节）
    pub compressed_size: u64,
    /// 修改时间（ISO 8601 或可读格式）
    pub modified: Option<String>,
    /// 是否加密
    pub encrypted: bool,
    /// 条目索引（用于 ZIP 精确按索引提取，避免编码转换导致路径不匹配）
    pub index: usize,
}

/// 压缩包元数据与内容列表
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveInfo {
    pub path: String,
    pub file_name: String,
    pub format: ArchiveFormat,
    pub format_label: String,
    pub archive_size: u64,
    pub total_uncompressed_size: u64,
    pub file_count: usize,
    pub dir_count: usize,
    pub is_encrypted: bool,
    pub detected_encoding: String,
    pub engine: String,
    /// 智能解压是否会自动创建同名文件夹
    pub smart_extract_will_create_folder: bool,
    pub suggested_folder_name: String,
    pub entries: Vec<ArchiveEntry>,
}

/// 解压请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractOptions {
    pub archive_path: String,
    pub dest_dir: String,
    /// 是否启用智能解压（避免顶层散落多个文件弄乱目录）
    pub smart_extract: bool,
    /// 解压密码（可选）
    pub password: Option<String>,
    /// 手动指定文件名编码（例如 "auto" / "utf-8" / "gbk" / "shift_jis" / "big5"）
    pub encoding: Option<String>,
    /// 指定只解压部分条目（为空则全部解压）
    pub selected_paths: Option<Vec<String>>,
    /// 是否覆盖已存在文件
    pub overwrite: bool,
}

/// 解压结果摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractResult {
    pub output_dir: String,
    pub extracted_files: usize,
    pub total_bytes: u64,
    pub elapsed_ms: u64,
}

/// 压缩目标格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompressFormat {
    Zip,
    SevenZ,
    TarGz,
    TarZst,
    Tar,
}

/// 压缩请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressOptions {
    pub input_paths: Vec<String>,
    pub output_path: String,
    pub format: CompressFormat,
    /// 压缩等级 0（仅存储）~ 9（极限压缩），默认 6
    pub level: u8,
    /// 加密密码（ZIP/7Z 可选）
    pub password: Option<String>,
    /// 是否自动排除 .DS_Store / __MACOSX / Thumbs.db
    pub exclude_junk: bool,
}

/// 压缩结果摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressResult {
    pub output_path: String,
    pub input_files: usize,
    pub original_bytes: u64,
    pub archive_bytes: u64,
    pub elapsed_ms: u64,
}

/// 实时进度事件载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub task_id: String,
    pub kind: String,
    pub current_file: String,
    pub processed_items: usize,
    pub total_items: usize,
    pub percent: f64,
    pub done: bool,
}

/// 引擎状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub native_ready: bool,
    pub sevenz_cli_ready: bool,
    pub sevenz_cli_path: Option<String>,
    pub supported_read_formats: Vec<String>,
    pub supported_write_formats: Vec<String>,
}
