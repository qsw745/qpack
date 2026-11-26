use serde::Serialize;
use thiserror::Error;

/// QPack 统一错误类型
#[derive(Debug, Error)]
pub enum QPackError {
    #[error("文件或目录不存在: {0}")]
    NotFound(String),

    #[error("不支持的压缩包格式: {0}")]
    UnsupportedFormat(String),

    #[error("密码错误或压缩包已加密，请输入正确密码")]
    InvalidPassword,

    #[error("检测到不安全的压缩包路径（路径穿越）: {0}")]
    UnsafePath(String),

    #[error("此格式需要内置或系统安装 7zz 引擎支持: {0}")]
    EngineUnavailable(String),

    #[error("压缩/解压处理失败: {0}")]
    ArchiveError(String),

    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
}

impl Serialize for QPackError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type QResult<T> = Result<T, QPackError>;
