//! Static asset types shared by the dev server and native render stage.
//! Source pages still go through their existing compilation routes.

use std::path::Path;

const TYPES: &[(&str, &str)] = &[
    ("html", "text/html; charset=utf-8"),
    ("css", "text/css"),
    ("js", "application/javascript"),
    ("mjs", "application/javascript"),
    ("json", "application/json"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("png", "image/png"),
    ("webp", "image/webp"),
    ("svg", "image/svg+xml"),
    ("mp4", "video/mp4"),
    ("webm", "video/webm"),
    ("m4a", "audio/mp4"),
    ("aac", "audio/aac"),
    ("flac", "audio/flac"),
    ("wav", "audio/wav"),
    ("mp3", "audio/mpeg"),
    ("ogg", "audio/ogg"),
    ("glb", "model/gltf-binary"),
    ("gltf", "model/gltf+json"),
    ("bin", "application/octet-stream"),
    ("ktx2", "application/octet-stream"),
    ("hdr", "application/octet-stream"),
    ("exr", "application/octet-stream"),
    ("draco", "application/octet-stream"),
    ("otf", "font/otf"),
    ("ttf", "font/ttf"),
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
];

pub(crate) fn content_type(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?;
    TYPES
        .iter()
        .find_map(|&(ext, mime)| (ext == extension).then_some(mime))
}
