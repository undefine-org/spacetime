//! CLI command for generating tree-sitter grammar from stdlib + extensions

use std::path::PathBuf;

use crate::metasystem::MetaRegistry;
use crate::treesitter_gen::{extract_directives, generate_grammar};

/// Arguments for the generate-grammar command
#[derive(Debug, Default)]
pub struct GenerateGrammarArgs {
    /// Additional .st files to include (glob patterns)
    pub include: Vec<String>,
    /// Output path (default: tree-sitter-spacetime/grammar.js)
    pub output: Option<PathBuf>,
    /// Verbose output
    pub verbose: bool,
}

pub fn run(args: GenerateGrammarArgs) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load stdlib
    let mut registry = MetaRegistry::new();

    let stdlib_dirs: Vec<&str> = crate::metasystem::STDLIB_DIRS
        .iter()
        .copied()
        .chain(["stdlib/scripting", "stdlib/mobile"])
        .collect();

    let mut total_loaded = 0;
    let mut total_errors = 0;

    for dir in &stdlib_dirs {
        let path = std::path::Path::new(dir);
        if path.exists() {
            let result = registry.load_stdlib_collecting_errors(path);
            total_loaded += result.loaded_count;
            total_errors += result.errors.len();
            if args.verbose {
                if result.errors.is_empty() {
                    println!("  Loaded: {} ({} files)", dir, result.loaded_count);
                } else {
                    println!(
                        "  Loaded: {} ({} files, {} errors)",
                        dir,
                        result.loaded_count,
                        result.errors.len()
                    );
                    for err in &result.errors {
                        eprintln!("    - {}: {}", err.file_path.display(), err.message);
                    }
                }
            }
        } else if args.verbose {
            println!("  Skipped (not found): {}", dir);
        }
    }

    // 2. Load user extension directories
    for pattern in &args.include {
        let path = std::path::Path::new(pattern);
        if path.exists() {
            if path.is_dir() {
                let result = registry.load_stdlib_collecting_errors(path);
                total_loaded += result.loaded_count;
                if args.verbose {
                    println!(
                        "  Loaded extension dir: {} ({} files)",
                        path.display(),
                        result.loaded_count
                    );
                }
                for err in &result.errors {
                    eprintln!(
                        "\x1b[33m⚠\x1b[0m Failed to load {}: {}",
                        err.file_path.display(),
                        err.message
                    );
                }
            } else {
                // Single file - use load_dir_flat on parent with a filter
                eprintln!(
                    "\x1b[33m⚠\x1b[0m Single file loading not supported yet, use directory patterns: {}",
                    path.display()
                );
            }
        } else if args.verbose {
            eprintln!("  Extension not found: {}", pattern);
        }
    }

    if args.verbose {
        println!(
            "\nLoaded {} stdlib files with {} errors",
            total_loaded, total_errors
        );
    }

    // 3. Extract directives from meta-AST
    let directives = extract_directives(&registry);
    println!(
        "\x1b[32m✓\x1b[0m Extracted {} directive rules from {} macros",
        directives.len(),
        registry.macro_count()
    );

    if args.verbose {
        println!("\nDirectives:");
        for d in &directives {
            let params: Vec<_> = d.params.iter().map(|p| p.name.as_str()).collect();
            let body_indicator = if d.has_body { " { ... }" } else { "" };
            println!("  @{}({}){}", d.name, params.join(", "), body_indicator);
        }
    }

    // 4. Generate grammar
    let grammar = generate_grammar(&directives);

    // 5. Write output
    let output = args
        .output
        .unwrap_or_else(|| PathBuf::from("tree-sitter-spacetime/grammar.js"));

    // Create parent directory if needed
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&output, &grammar)?;
    println!("\x1b[32m✓\x1b[0m Wrote grammar to {}", output.display());

    // Print next steps
    println!("\nNext steps:");
    println!("  cd tree-sitter-spacetime && npx tree-sitter generate");
    println!("  npx tree-sitter parse ../projects/jeton/index.st");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = GenerateGrammarArgs::default();
        assert!(args.include.is_empty());
        assert!(args.output.is_none());
        assert!(!args.verbose);
    }
}
