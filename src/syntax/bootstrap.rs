//! Bootstrap Stdlib - Load .st Files into SyntaxRegistry
//!
//! This module loads all %macro definitions from stdlib .st files
//! and builds a SyntaxRegistry for universal form-based syntax parsing.
//!
//! # Architecture
//!
//! ```text
//! stdlib/*.st files → parse() → MacroDefAst → SyntaxRegistry
//!                                                    ↓
//!                                          prefix-indexed %forms
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use spacetime::syntax::bootstrap::bootstrap_stdlib;
//!
//! let registry = bootstrap_stdlib(Path::new("./stdlib"))?;
//! println!("Loaded {} forms", registry.form_count());
//! ```

use std::path::Path;
use thiserror::Error;

use crate::parser::meta_ast::{MacroDefAst, MetaDef};
use crate::syntax::SyntaxRegistry;

/// Error during stdlib bootstrap
#[derive(Debug, Error)]
pub enum BootstrapError {
    /// IO error reading stdlib files
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error in a stdlib file
    #[error("Parse error in {file}: {message}")]
    Parse { file: String, message: String },

    /// Registry error (duplicate macro, etc.)
    #[error("Registry error: {0}")]
    Registry(String),
}

/// Bootstrap the stdlib and build a SyntaxRegistry.
///
/// This function:
/// 1. Recursively loads all .st files from `stdlib_path`
/// 2. Parses %macro definitions from each file
/// 3. Registers %form patterns in a SyntaxRegistry
/// 4. Returns the populated registry
///
/// # Skipped Files
///
/// - Files in `examples/` directories (user-facing code, not metasystem defs)
/// - Files ending in `.test.st` (test files that use macros, don't define them)
///
/// # Error Handling
///
/// Parse errors are logged but don't stop the bootstrap process.
/// This allows stdlib loading to succeed even with experimental files.
///
/// # Example
///
/// ```ignore
/// let registry = bootstrap_stdlib(Path::new("./stdlib"))?;
/// assert!(registry.form_count() > 0);
/// ```
pub fn bootstrap_stdlib(stdlib_path: &Path) -> Result<SyntaxRegistry, BootstrapError> {
    let mut registry = SyntaxRegistry::new();
    let macros = load_macros_from_dir(stdlib_path)?;

    for macro_def in macros {
        registry.register(&macro_def);
    }

    // Load %capture_type definitions so stdlib grammar productions drive matching
    // (PLAN-023 W2). Compiled into custom extractors at match time.
    let capture_types = load_capture_types_from_dir(stdlib_path)?;
    for def in capture_types {
        registry.register_capture_type(def);
    }

    // The `%scalar_type` rows travel the same road: a scalar's `%capture` names
    // the production whose value flattens to its source text, and the extractor
    // that consults it runs before the meta registry finishes loading.
    for capture in load_scalar_captures_from_dir(stdlib_path)? {
        registry.register_scalar_capture(capture);
    }

    Ok(registry)
}

/// Load the `%capture` production named by each `%scalar_type` row.
pub fn load_scalar_captures_from_dir(dir: &Path) -> Result<Vec<String>, BootstrapError> {
    let mut out = Vec::new();
    load_scalar_captures_recursive(dir, &mut out)?;
    Ok(out)
}

fn load_scalar_captures_recursive(dir: &Path, out: &mut Vec<String>) -> Result<(), BootstrapError> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(BootstrapError::Io)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_scalar_captures_recursive(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("st") {
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(parsed) = crate::parser::parse_for_bootstrap(&content) else {
                continue;
            };
            for meta_def in parsed.meta_defs {
                if let MetaDef::ScalarType(s) = meta_def {
                    out.push(s.capture);
                }
            }
        }
    }
    Ok(())
}

/// Load all `%capture_type` definitions from a directory tree (PLAN-023 W2).
pub fn load_capture_types_from_dir(
    dir: &Path,
) -> Result<Vec<crate::parser::meta_ast::CaptureTypeDefAst>, BootstrapError> {
    let mut out = Vec::new();
    load_capture_types_recursive(dir, &mut out)?;
    Ok(out)
}

fn load_capture_types_recursive(
    dir: &Path,
    out: &mut Vec<crate::parser::meta_ast::CaptureTypeDefAst>,
) -> Result<(), BootstrapError> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            load_capture_types_recursive(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("st") {
            // Tolerate per-file parse errors (mirrors load_macros_recursive): a file that
            // fails to parse simply contributes no capture types.
            let _ = load_capture_types_from_file(&path, out);
        }
    }
    Ok(())
}

fn load_capture_types_from_file(
    path: &Path,
    out: &mut Vec<crate::parser::meta_ast::CaptureTypeDefAst>,
) -> Result<(), BootstrapError> {
    use crate::parser::meta_ast::MetaDef;
    let content = std::fs::read_to_string(path)?;
    let parsed =
        crate::parser::parse_for_bootstrap(&content).map_err(|e| BootstrapError::Parse {
            file: path.display().to_string(),
            message: e.to_string(),
        })?;
    let source_file = path.to_string_lossy().to_string();
    for meta_def in parsed.meta_defs {
        if let MetaDef::CaptureType(mut ct) = meta_def {
            ct.source_file = Some(source_file.clone());
            out.push(ct);
        }
    }
    Ok(())
}

/// Bootstrap the SyntaxRegistry from embedded stdlib files.
///
/// This is the fallback path when the `stdlib/` directory doesn't exist on disk
/// (e.g., when running push-online from a site directory without a local stdlib).
/// It iterates over all embedded stdlib files, parses them with `parse_for_bootstrap`,
/// and extracts %macro definitions to register %form patterns.
pub fn bootstrap_stdlib_from_embedded() -> SyntaxRegistry {
    use crate::stdlib_embedded::{self, EmbeddedFile};

    let mut registry = SyntaxRegistry::new();

    for (category, files) in stdlib_embedded::all_embedded_files() {
        for EmbeddedFile { path, content } in files.iter() {
            let virtual_path = format!("stdlib/{}/{}", category, path);

            // Parse without STDLIB_REGISTRY to avoid circular dependency
            let parsed = match crate::parser::parse_for_bootstrap(content) {
                Ok(ast) => ast,
                Err(_) => continue,
            };

            // Extract %macro definitions and %capture_type productions (PLAN-023 W2).
            for meta_def in parsed.meta_defs {
                match meta_def {
                    MetaDef::Macro(mut macro_def) => {
                        macro_def.source_file = Some(virtual_path.clone());
                        registry.register(&macro_def);
                    }
                    MetaDef::Migration(mut mig) => {
                        // PLAN-079 capsule: register the migration's OWN
                        // macros (embedded retired defs + match-only rule
                        // defs) — the retired grammar lives IN the entry.
                        mig.source_file = Some(virtual_path.clone());
                        for mac in mig.registration_macros() {
                            registry.register(&mac);
                        }
                    }
                    MetaDef::ScalarType(scalar) => {
                        registry.register_scalar_capture(scalar.capture);
                    }
                    MetaDef::CaptureType(mut ct) => {
                        ct.source_file = Some(virtual_path.clone());
                        registry.register_capture_type(ct);
                    }
                    _ => {}
                }
            }
        }
    }

    registry
}

/// Load all %macro definitions from a directory (recursive).
///
/// This function:
/// 1. Recursively walks the directory tree
/// 2. Parses each .st file
/// 3. Extracts %macro definitions
/// 4. Returns all macros found
///
/// # Skipped Files
///
/// - `examples/` directories - contain user-facing code, not metasystem defs
/// - `.test.st` files - test files that use macros, don't define them
///
/// # Error Handling
///
/// Parse errors in individual files are logged but don't stop the process.
/// Returns Ok with all successfully parsed macros.
pub fn load_macros_from_dir(dir: &Path) -> Result<Vec<MacroDefAst>, BootstrapError> {
    let mut macros = Vec::new();
    load_macros_recursive(dir, &mut macros)?;
    Ok(macros)
}

/// Recursive helper for loading macros from a directory tree
fn load_macros_recursive(dir: &Path, macros: &mut Vec<MacroDefAst>) -> Result<(), BootstrapError> {
    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Skip examples directories - they contain user-facing code, not metasystem defs
            if path.file_name().is_some_and(|name| name == "examples") {
                continue;
            }

            // Recurse into subdirectories
            // Continue loading other directories even if one fails
            let _ = load_macros_recursive(&path, macros);
        } else if path.extension().is_some_and(|ext| ext == "st") {
            // Skip test files - they use macros, they don't define them
            let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if file_name.ends_with(".test") {
                continue;
            }

            // Load macros from this file
            // Continue loading other files even if one fails to parse
            let _ = load_macros_from_file(&path, macros);
        }
    }

    Ok(())
}

/// Load %macro definitions from a single .st file
fn load_macros_from_file(path: &Path, macros: &mut Vec<MacroDefAst>) -> Result<(), BootstrapError> {
    let content = std::fs::read_to_string(path)?;

    // Use parse_for_bootstrap to avoid circular dependency with STDLIB_REGISTRY
    let parsed =
        crate::parser::parse_for_bootstrap(&content).map_err(|e| BootstrapError::Parse {
            file: path.display().to_string(),
            message: e.to_string(),
        })?;

    // Store path as relative to stdlib (e.g., "stdlib/template.st")
    let source_file = path.to_string_lossy().to_string();

    // Extract %macro definitions from the parsed file
    for meta_def in parsed.meta_defs {
        match meta_def {
            MetaDef::Macro(mut macro_def) => {
                // Tag each macro with its source file for error traces
                macro_def.source_file = Some(source_file.clone());
                macros.push(macro_def);
            }
            MetaDef::Migration(mut mig) => {
                // PLAN-079 capsule: the migration's OWN macros (embedded
                // retired defs + match-only rule defs) register so retired
                // directives still parse into a FormMatch.
                mig.source_file = Some(source_file.clone());
                macros.extend(mig.registration_macros());
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_macro_file(content: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.st");
        fs::write(&file_path, content).unwrap();
        (dir, file_path)
    }

    #[test]
    fn test_load_simple_macro() {
        let content = r#"
%macro test-macro {
  %creates @test

  %form {
    @test($value:expr)
  }
}
"#;
        let (_dir, file_path) = create_test_macro_file(content);

        let mut macros = Vec::new();
        load_macros_from_file(&file_path, &mut macros).unwrap();

        assert_eq!(macros.len(), 1);
        assert_eq!(macros[0].name, "test-macro");
    }

    #[test]
    fn test_load_multiple_macros_from_file() {
        let content = r#"
%macro first {
  %creates @first
  %form { @first }
}

%macro second {
  %creates @second
  %form { @second }
}
"#;
        let (_dir, file_path) = create_test_macro_file(content);

        let mut macros = Vec::new();
        load_macros_from_file(&file_path, &mut macros).unwrap();

        assert_eq!(macros.len(), 2);
        assert_eq!(macros[0].name, "first");
        assert_eq!(macros[1].name, "second");
    }

    #[test]
    fn test_skip_examples_directory() {
        let dir = TempDir::new().unwrap();
        let examples_dir = dir.path().join("examples");
        fs::create_dir(&examples_dir).unwrap();

        let example_file = examples_dir.join("example.st");
        fs::write(&example_file, "%macro example { %creates @example }").unwrap();

        let mut macros = Vec::new();
        load_macros_recursive(dir.path(), &mut macros).unwrap();

        // Should not load macros from examples/
        assert_eq!(macros.len(), 0);
    }

    #[test]
    fn test_skip_test_files() {
        let dir = TempDir::new().unwrap();
        let test_file = dir.path().join("feature.test.st");
        fs::write(&test_file, "%macro test { %creates @test }").unwrap();

        let mut macros = Vec::new();
        load_macros_recursive(dir.path(), &mut macros).unwrap();

        // Should not load macros from .test.st files
        assert_eq!(macros.len(), 0);
    }

    #[test]
    fn test_load_from_subdirectories() {
        let dir = TempDir::new().unwrap();

        // Create subdirectory with macro
        let subdir = dir.path().join("macros");
        fs::create_dir(&subdir).unwrap();
        let macro_file = subdir.join("data.st");
        fs::write(
            &macro_file,
            "%macro data { %creates @data %form { @data } }",
        )
        .unwrap();

        // Create another subdirectory
        let subdir2 = dir.path().join("primitives");
        fs::create_dir(&subdir2).unwrap();
        let macro_file2 = subdir2.join("state.st");
        fs::write(
            &macro_file2,
            "%macro state { %creates @state %form { @state } }",
        )
        .unwrap();

        let mut macros = Vec::new();
        load_macros_recursive(dir.path(), &mut macros).unwrap();

        assert_eq!(macros.len(), 2);
    }

    #[test]
    fn test_bootstrap_builds_registry() {
        let dir = TempDir::new().unwrap();
        let macro_file = dir.path().join("test.st");
        fs::write(
            &macro_file,
            r#"
%macro test {
  %creates @test
  %form { @test($value:expr) }
}
"#,
        )
        .unwrap();

        let registry = bootstrap_stdlib(dir.path()).unwrap();

        assert_eq!(registry.form_count(), 1);
        assert!(registry.has_prefix('@'));
    }

    #[test]
    fn test_registry_has_correct_prefixes() {
        let dir = TempDir::new().unwrap();

        // Create @directive macro
        let at_file = dir.path().join("at.st");
        fs::write(&at_file, "%macro test-at { %form { @test } }").unwrap();

        // Create $variable macro
        let dollar_file = dir.path().join("dollar.st");
        fs::write(&dollar_file, "%macro test-dollar { %form { $name:ident } }").unwrap();

        let registry = bootstrap_stdlib(dir.path()).unwrap();

        assert_eq!(registry.form_count(), 2);
        assert!(registry.has_prefix('@'));
        assert!(registry.has_prefix('$'));
        assert!(!registry.has_prefix('&'));
    }

    #[test]
    fn test_continue_on_parse_error() {
        let dir = TempDir::new().unwrap();

        // Create valid macro file
        let valid_file = dir.path().join("valid.st");
        fs::write(&valid_file, "%macro valid { %form { @valid } }").unwrap();

        // Create invalid macro file
        let invalid_file = dir.path().join("invalid.st");
        fs::write(&invalid_file, "%macro invalid { invalid syntax !!!").unwrap();

        let mut macros = Vec::new();
        load_macros_recursive(dir.path(), &mut macros).unwrap();

        // Should still load the valid macro despite parse error
        assert_eq!(macros.len(), 1);
        assert_eq!(macros[0].name, "valid");
    }

    #[test]
    fn test_empty_directory() {
        let dir = TempDir::new().unwrap();
        let registry = bootstrap_stdlib(dir.path()).unwrap();
        assert_eq!(registry.form_count(), 0);
    }

    #[test]
    fn test_extract_primitives_ignored() {
        let dir = TempDir::new().unwrap();
        let prim_file = dir.path().join("prim.st");
        fs::write(
            &prim_file,
            r#"
%primitive test-prim(&el, name: string) {
  %emit js {
    console.log("test");
  }
}

%macro test-macro {
  %creates @test
  %form { @test }
}
"#,
        )
        .unwrap();

        let mut macros = Vec::new();
        load_macros_from_file(&prim_file, &mut macros).unwrap();

        // Should only extract macros, not primitives
        assert_eq!(macros.len(), 1);
        assert_eq!(macros[0].name, "test-macro");
    }

    #[test]
    fn test_load_real_stdlib_if_exists() {
        // This test only runs if the actual stdlib directory exists
        let stdlib_path = Path::new("./stdlib");
        if stdlib_path.exists() {
            let registry = bootstrap_stdlib(stdlib_path).unwrap();

            // Should have loaded macros from stdlib
            assert!(registry.form_count() > 0);

            // Should have @ prefixed forms (e.g., @data, @each, @animate)
            assert!(registry.has_prefix('@'));

            // Verify specific known macros exist
            let data_forms = registry.get_forms_for_directive("data");
            assert!(
                !data_forms.is_empty(),
                "Should have loaded @data macro from stdlib"
            );
        }
    }
}
