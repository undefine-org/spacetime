//! Bundle profiling for Spacetime
//!
//! Provides size analysis of compiled outputs including gzip estimates.

use serde::Serialize;
use std::io::Write;
use std::path::Path;

use crate::compiler::CompiledSpacetime;

/// Profile of a Spacetime bundle
#[derive(Debug, Clone, Serialize)]
pub struct BundleProfile {
    /// Runtime JS size in bytes
    pub js_bytes: usize,
    /// Runtime JS size after gzip compression
    pub js_gzipped: usize,
    /// Compiled CSS size in bytes
    pub css_bytes: usize,
    /// Compiled CSS size after gzip compression
    pub css_gzipped: usize,
    /// Total uncompressed size
    pub total_bytes: usize,
    /// Total gzipped size
    pub total_gzipped: usize,
    /// Image analysis (if site_dir provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<ImageProfile>,
}

/// Profile of images in a site
#[derive(Debug, Clone, Serialize)]
pub struct ImageProfile {
    /// Number of image files found
    pub count: usize,
    /// Total size of all images in bytes
    pub total_bytes: u64,
    /// Individual image details
    pub files: Vec<ImageInfo>,
}

/// Information about a single image
#[derive(Debug, Clone, Serialize)]
pub struct ImageInfo {
    /// Relative path to the image
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// Image format (jpg, png, webp, etc.)
    pub format: String,
}

impl BundleProfile {
    /// Create a profile from compiled spacetime output
    pub fn from_compiled(compiled: &CompiledSpacetime) -> Self {
        let js_bytes = compiled.js.len();
        let css_bytes = compiled.css.len();

        let js_gzipped = estimate_gzip_size(compiled.js.as_bytes());
        let css_gzipped = estimate_gzip_size(compiled.css.as_bytes());

        Self {
            js_bytes,
            js_gzipped,
            css_bytes,
            css_gzipped,
            total_bytes: js_bytes + css_bytes,
            total_gzipped: js_gzipped + css_gzipped,
            images: None,
        }
    }

    /// Add image profile from site directory
    pub fn with_images(mut self, site_dir: &Path) -> Self {
        self.images = Some(scan_images(site_dir));
        self
    }

    /// Format as human-readable string
    pub fn to_display_string(&self) -> String {
        let mut output = String::new();

        output.push_str("Spacetime Bundle Profile\n");
        output.push_str("─────────────────────────\n");
        output.push_str(&format!(
            "Runtime JS:  {:>8} bytes ({:>6} gzipped)\n",
            format_bytes(self.js_bytes),
            format_bytes(self.js_gzipped)
        ));
        output.push_str(&format!(
            "Styles CSS:  {:>8} bytes ({:>6} gzipped)\n",
            format_bytes(self.css_bytes),
            format_bytes(self.css_gzipped)
        ));
        output.push_str("─────────────────────────\n");
        output.push_str(&format!(
            "Total:       {:>8} bytes ({:>6} gzipped)\n",
            format_bytes(self.total_bytes),
            format_bytes(self.total_gzipped)
        ));

        if let Some(images) = &self.images {
            output.push('\n');
            output.push_str(&format!(
                "Images: {} files, {} total\n",
                images.count,
                format_bytes(images.total_bytes as usize)
            ));

            // Show top 5 largest images
            if !images.files.is_empty() {
                let mut sorted = images.files.clone();
                sorted.sort_by(|a, b| b.size.cmp(&a.size));
                output.push_str("  Largest:\n");
                for img in sorted.iter().take(5) {
                    output.push_str(&format!(
                        "    {} - {} ({})\n",
                        img.path,
                        format_bytes(img.size as usize),
                        img.format
                    ));
                }
            }
        }

        output
    }
}

/// Estimate gzip compressed size
fn estimate_gzip_size(data: &[u8]) -> usize {
    use flate2::Compression;
    use flate2::write::GzEncoder;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).ok();
    encoder.finish().map(|v| v.len()).unwrap_or(data.len())
}

/// Scan a directory for image files
fn scan_images(site_dir: &Path) -> ImageProfile {
    let mut files = Vec::new();
    let mut total_bytes = 0u64;

    scan_images_recursive(site_dir, site_dir, &mut files, &mut total_bytes);

    ImageProfile {
        count: files.len(),
        total_bytes,
        files,
    }
}

fn scan_images_recursive(
    base_dir: &Path,
    current_dir: &Path,
    files: &mut Vec<ImageInfo>,
    total_bytes: &mut u64,
) {
    let Ok(entries) = std::fs::read_dir(current_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            // Skip hidden directories and common non-image directories
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if !name.starts_with('.') && name != "node_modules" {
                scan_images_recursive(base_dir, &path, files, total_bytes);
            }
            continue;
        }

        let Some(ext) = path.extension() else {
            continue;
        };
        let ext = ext.to_string_lossy().to_lowercase();

        let format = match ext.as_str() {
            "jpg" | "jpeg" => "jpeg",
            "png" => "png",
            "webp" => "webp",
            "avif" => "avif",
            "gif" => "gif",
            "svg" => "svg",
            _ => continue,
        };

        if let Ok(metadata) = path.metadata() {
            let size = metadata.len();
            let relative_path = path
                .strip_prefix(base_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            *total_bytes += size;
            files.push(ImageInfo {
                path: relative_path,
                size,
                format: format.to_string(),
            });
        }
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: usize) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1}MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1}KB", bytes as f64 / 1_000.0)
    } else {
        format!("{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B");
        assert_eq!(format_bytes(1500), "1.5KB");
        assert_eq!(format_bytes(1_500_000), "1.5MB");
    }

    #[test]
    fn test_gzip_estimation() {
        let data = b"Hello, World! This is a test string that should compress well. ".repeat(100);
        let gzipped = estimate_gzip_size(&data);
        // Gzip should compress repeated data significantly
        assert!(gzipped < data.len() / 2);
    }

    #[test]
    fn test_bundle_profile_serialization() {
        let profile = BundleProfile {
            js_bytes: 38000,
            js_gzipped: 9500,
            css_bytes: 1000,
            css_gzipped: 400,
            total_bytes: 39000,
            total_gzipped: 9900,
            images: None,
        };

        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("\"js_bytes\":38000"));
        assert!(json.contains("\"total_gzipped\":9900"));
    }
}
