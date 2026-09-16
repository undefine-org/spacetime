//! Deploy Protocol v2 per-blob wire compression policy.

/// Maximum size sent through the orchestrator rather than a presigned PUT.
pub const SMALL_BLOB_THRESHOLD_BYTES: u64 = 4 * 1024 * 1024;
const ZSTD_LEVEL: i32 = 3;

/// Formats whose bytes are already compressed and must use identity framing.
pub const ALREADY_COMPRESSED_EXTENSIONS: &[&str] = &[
    "mp4", "webm", "ogg", "ogv", "mov", "mkv", "avi", "mp3", "wav", "flac", "aac", "oga", "m4a",
    "png", "jpg", "jpeg", "gif", "webp", "avif", "svgz", "woff2", "woff", "zip", "gz", "bz2", "xz",
    "7z", "br",
];

pub fn should_compress(path: &str) -> bool {
    path.rsplit_once('.')
        .map(|(_, ext)| !ALREADY_COMPRESSED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(true)
}

pub fn compress(bytes: &[u8]) -> Result<Vec<u8>, String> {
    zstd::bulk::compress(bytes, ZSTD_LEVEL).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_and_archives_use_identity_case_insensitively() {
        for path in [
            "clip.MP4",
            "audio.ogg",
            "photo.png",
            "font.woff2",
            "archive.gz",
        ] {
            assert!(!should_compress(path), "{path}");
        }
        assert!(should_compress("index.html"));
        assert!(should_compress("Makefile"));
    }

    #[test]
    fn zstd_level_three_output_roundtrips() {
        let source = b"protocol v2 text ".repeat(256);
        let framed = compress(&source).unwrap();
        assert_eq!(
            zstd::bulk::decompress(&framed, source.len()).unwrap(),
            source
        );
    }
}

/// Decompresses a zstd frame, refusing output larger than `max_bytes`
/// (zip-bomb guard — mirrors the host's bounded decompress).
pub fn decompress(bytes: &[u8], max_bytes: usize) -> Result<Vec<u8>, String> {
    let mut decoder = zstd::Decoder::new(bytes).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let mut take = std::io::Read::take(&mut decoder, max_bytes as u64 + 1);
    std::io::Read::read_to_end(&mut take, &mut out).map_err(|e| e.to_string())?;
    if out.len() > max_bytes {
        return Err("decompressed blob exceeds maximum allowed size".to_string());
    }
    Ok(out)
}
