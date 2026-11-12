//! Facts read from files (PLAN-156).
//!
//! A value that names something inside a file — a model's animation clip —
//! has one source of truth: the file. A `%scalar_type` row says so with
//! `%facts <sibling> <reader>`; the compiler checks the value against what the
//! reader finds in the file the sibling param names, and the studio offers
//! those as the field's choices. three.js reads the same file at runtime.
//!
//! These are file-FORMAT readers (the glTF container), not readers of any
//! Spacetime construct.

use std::path::Path;

/// The readers a `%facts` clause may name.
pub const READERS: &[&str] = &["gltf-clips"];

/// What `reader` finds in the file at `path`.
pub fn read(reader: &str, path: &Path) -> Result<Vec<String>, String> {
    match reader {
        "gltf-clips" => gltf_clips(path),
        other => Err(format!("unknown fact reader `{other}` (known: {})", READERS.join(", "))),
    }
}

/// The names of a glTF model's animations (`.glb` binary or `.gltf` JSON).
pub fn gltf_clips(path: &Path) -> Result<Vec<String>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let json: serde_json::Value = if bytes.starts_with(b"glTF") {
        // header (12 bytes), then the first chunk: length, type "JSON", data
        if bytes.len() < 20 || &bytes[16..20] != b"JSON" {
            return Err(format!("{}: not a glTF binary with a JSON chunk", path.display()));
        }
        let len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let chunk = bytes.get(20..20 + len).ok_or_else(|| format!("{}: truncated JSON chunk", path.display()))?;
        serde_json::from_slice(chunk).map_err(|e| format!("{}: {e}", path.display()))?
    } else {
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?
    };
    Ok(json["animations"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x["name"].as_str().map(str::to_string)).collect())
        .unwrap_or_default())
}

/// The file a sibling value names, relative to the `.st` file that wrote it.
/// A URL, a `$signal` reference or an empty value names no file.
pub fn resolve(source_file: Option<&str>, value: &str) -> Option<std::path::PathBuf> {
    let v = value.trim().trim_matches('"');
    if v.is_empty() || v.starts_with('$') || v.contains("://") || v.starts_with("data:") {
        return None;
    }
    let base = source_file.map(Path::new).and_then(Path::parent).unwrap_or(Path::new("."));
    let p = if let Some(abs) = v.strip_prefix('/') { base.join(abs) } else { base.join(v) };
    p.is_file().then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glb_lists_its_clips() {
        let clips = gltf_clips(Path::new("demos/film-robot-day/models/robot.glb")).expect("reads");
        assert_eq!(clips.len(), 14);
        for c in ["Idle", "Walking", "Wave", "Death", "ThumbsUp"] {
            assert!(clips.iter().any(|x| x == c), "{c} in {clips:?}");
        }
    }

    #[test]
    fn a_value_resolves_next_to_its_file() {
        let p = resolve(Some("demos/film-robot-day/index.st"), "\"models/robot.glb\"").expect("found");
        assert!(p.ends_with("models/robot.glb"));
        assert!(resolve(Some("demos/film-robot-day/index.st"), "$model.src").is_none());
        assert!(resolve(None, "https://x/y.glb").is_none());
    }
}
