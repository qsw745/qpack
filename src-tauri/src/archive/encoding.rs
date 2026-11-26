use chardetng::EncodingDetector;
use encoding_rs::{Encoding, BIG5, EUC_KR, GB18030, SHIFT_JIS, UTF_8};

/// 根据用户选择的编码或自动检测结果，将原始文件名字节流解码为 UTF-8 字符串
pub fn detect_best_encoding(raw_samples: &[Vec<u8>], override_encoding: Option<&str>) -> &'static Encoding {
    if let Some(enc_name) = override_encoding {
        let normalized = enc_name.trim().to_lowercase();
        if !normalized.is_empty() && normalized != "auto" {
            return match normalized.as_str() {
                "utf-8" | "utf8" => UTF_8,
                "gbk" | "gb18030" | "gb2312" => GB18030,
                "big5" => BIG5,
                "shift_jis" | "shift-jis" | "sjis" => SHIFT_JIS,
                "euc-kr" | "euckr" => EUC_KR,
                other => Encoding::for_label(other.as_bytes()).unwrap_or(UTF_8),
            };
        }
    }

    // 检查是否全部为合法 UTF-8
    let mut non_utf8_samples: Vec<&[u8]> = Vec::new();
    for sample in raw_samples {
        if std::str::from_utf8(sample).is_err() {
            non_utf8_samples.push(sample.as_slice());
        }
    }

    if non_utf8_samples.is_empty() {
        return UTF_8;
    }

    // 对非 UTF-8 字节样本使用 chardetng 自动识别（优先兼容中文 GB18030/GBK）
    let mut detector = EncodingDetector::new();
    for sample in &non_utf8_samples {
        detector.feed(sample, false);
    }
    detector.feed(&[], true);
    let guessed = detector.guess(None, true);

    // 常见 Windows 中文压缩包经常被误判为 windows-1252，若全是合法 GB18030 则优先 GB18030
    if guessed.name() == "windows-1252" {
        let all_gbk_clean = non_utf8_samples.iter().all(|s| {
            let (_, _, had_errors) = GB18030.decode(s);
            !had_errors
        });
        if all_gbk_clean {
            return GB18030;
        }
    }

    guessed
}

/// 使用指定编码解码单条路径，并统一路径分隔符为 `/`
pub fn decode_entry_path(raw: &[u8], encoding: &'static Encoding) -> String {
    let decoded = if encoding == UTF_8 {
        String::from_utf8_lossy(raw).into_owned()
    } else {
        let (cow, _, _) = encoding.decode(raw);
        cow.into_owned()
    };

    decoded.replace('\\', "/")
}
