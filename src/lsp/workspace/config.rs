//! Workspace configuration from `.local/spacetime.yaml`

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Indexing strategy for the workspace
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexingStrategy {
    /// Index all files at startup
    Eager,
    /// Index only imported files as needed
    Lazy,
    /// Index open files + imports eagerly, background scan for completions
    #[default]
    Hybrid,
}

/// Workspace configuration
#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    /// Workspace root directory
    pub root: PathBuf,
    /// Patterns to exclude from indexing
    pub exclude_patterns: Vec<String>,
    /// Indexing strategy
    pub indexing_strategy: IndexingStrategy,
    /// Import path aliases (e.g., "@lib" -> "./src/lib")
    pub import_aliases: HashMap<String, PathBuf>,
    /// Path to stdlib (None = use embedded)
    pub stdlib_path: Option<PathBuf>,
}

/// Raw YAML config structure
#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    workspace: WorkspaceSection,
    #[serde(default)]
    imports: ImportsSection,
}

#[derive(Debug, Deserialize, Default)]
struct WorkspaceSection {
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    indexing: Option<IndexingStrategy>,
    #[serde(default)]
    stdlib: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ImportsSection {
    #[serde(default)]
    aliases: HashMap<String, String>,
}

impl WorkspaceConfig {
    /// Load configuration from `.local/spacetime.yaml` in the workspace root.
    /// Returns default config if file doesn't exist or is invalid.
    pub fn load(workspace_root: &Path) -> Self {
        let config_path = workspace_root.join(".local/spacetime.yaml");

        let raw = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|content| serde_yaml::from_str::<RawConfig>(&content).ok())
                .unwrap_or_default()
        } else {
            RawConfig::default()
        };

        Self::from_raw(raw, workspace_root)
    }

    /// Create config with defaults for a workspace root
    pub fn default_for(workspace_root: &Path) -> Self {
        Self::from_raw(RawConfig::default(), workspace_root)
    }

    fn from_raw(raw: RawConfig, workspace_root: &Path) -> Self {
        let mut exclude_patterns = raw.workspace.exclude;
        if exclude_patterns.is_empty() {
            // Default excludes
            exclude_patterns = vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "dist".to_string(),
            ];
        }

        let import_aliases = raw
            .imports
            .aliases
            .into_iter()
            .map(|(alias, path)| {
                let resolved = if path.starts_with("./") || path.starts_with("../") {
                    workspace_root.join(&path)
                } else {
                    PathBuf::from(&path)
                };
                (alias, resolved)
            })
            .collect();

        let stdlib_path = raw.workspace.stdlib.map(|p| workspace_root.join(p));

        Self {
            root: workspace_root.to_path_buf(),
            exclude_patterns,
            indexing_strategy: raw.workspace.indexing.unwrap_or_default(),
            import_aliases,
            stdlib_path,
        }
    }

    /// Check if a path should be excluded from indexing
    pub fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.exclude_patterns
            .iter()
            .any(|pattern| path_str.contains(pattern))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WorkspaceConfig::default_for(Path::new("/project"));
        assert_eq!(config.indexing_strategy, IndexingStrategy::Hybrid);
        assert!(
            config
                .exclude_patterns
                .contains(&"node_modules".to_string())
        );
        assert!(config.import_aliases.is_empty());
    }

    #[test]
    fn test_should_exclude() {
        let config = WorkspaceConfig::default_for(Path::new("/project"));
        assert!(config.should_exclude(Path::new("/project/node_modules/foo")));
        assert!(config.should_exclude(Path::new("/project/.git/config")));
        assert!(!config.should_exclude(Path::new("/project/src/main.st")));
    }

    #[test]
    fn test_parse_yaml() {
        let yaml = r#"
workspace:
  exclude: [vendor, build]
  indexing: eager

imports:
  aliases:
    "@lib": ./src/lib
    "@components": ./src/components
"#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let config = WorkspaceConfig::from_raw(raw, Path::new("/project"));

        assert_eq!(config.indexing_strategy, IndexingStrategy::Eager);
        assert!(config.exclude_patterns.contains(&"vendor".to_string()));
        assert_eq!(
            config.import_aliases.get("@lib"),
            Some(&PathBuf::from("/project/src/lib"))
        );
    }
}
