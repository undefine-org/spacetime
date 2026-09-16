//! Stdlib syntax checker for CI/CD
//!
//! This binary checks all stdlib files for parse errors and reports them.
//! Exit code 0 = all files valid, exit code 1 = errors found.
//!
//! Usage:
//!   cargo run --bin check-stdlib
//!   cargo run --bin check-stdlib -- --verbose

use std::path::Path;
use std::process::ExitCode;

use spacetime::metasystem::MetaRegistry;
use spacetime::pipeline::{StdlibLoadError, make_error_friendly};

fn main() -> ExitCode {
    let verbose = std::env::args().any(|arg| arg == "--verbose" || arg == "-v");

    println!("Checking stdlib for parse errors...\n");

    let mut registry = MetaRegistry::new();
    let mut all_errors: Vec<StdlibLoadError> = Vec::new();

    // Check all stdlib directories (canonical list — PLAN-076)
    for dir in spacetime::metasystem::STDLIB_DIRS {
        let path = Path::new(dir);
        if !path.exists() {
            eprintln!("Warning: stdlib directory '{}' not found", dir);
            continue;
        }

        let result = registry.load_stdlib_collecting_errors(path);

        if verbose && result.loaded_count > 0 {
            println!("  {} files loaded from {}", result.loaded_count, dir);
        }

        all_errors.extend(result.errors);
    }

    if all_errors.is_empty() {
        println!("\x1b[32m✓\x1b[0m All stdlib files are valid!");
        return ExitCode::SUCCESS;
    }

    // Report errors
    eprintln!(
        "\x1b[31m✗\x1b[0m Found {} stdlib error(s):\n",
        all_errors.len()
    );

    for err in &all_errors {
        // Get source line for friendly error
        let source_line = if let Some(span) = &err.span {
            std::fs::read_to_string(&err.file_path).ok().map(|content| {
                let before = &content[..span.start.min(content.len())];
                let last_newline = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
                let line_end = content[span.start.min(content.len())..]
                    .find('\n')
                    .map(|p| span.start + p)
                    .unwrap_or(content.len());
                content[last_newline..line_end.min(content.len())].to_string()
            })
        } else {
            None
        };

        let friendly = make_error_friendly(&err.message, source_line.as_deref());

        // Print error
        eprintln!("  \x1b[31merror\x1b[0m: {}", friendly.summary);

        // Print location
        if let Some(span) = &err.span {
            if let Ok(content) = std::fs::read_to_string(&err.file_path) {
                let before = &content[..span.start.min(content.len())];
                let line = before.matches('\n').count() + 1;
                let last_newline = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
                let col = span.start - last_newline + 1;

                eprintln!(
                    "  \x1b[36m-->\x1b[0m {}:{}:{}",
                    err.file_path.display(),
                    line,
                    col
                );

                if verbose {
                    // Show source line
                    let line_end = content[span.start.min(content.len())..]
                        .find('\n')
                        .map(|p| span.start + p)
                        .unwrap_or(content.len());
                    let src_line = &content[last_newline..line_end.min(content.len())];
                    eprintln!("     |");
                    eprintln!("  {:>3} | {}", line, src_line);
                    eprintln!(
                        "     | {}\x1b[31m^\x1b[0m",
                        " ".repeat(col.saturating_sub(1))
                    );
                }
            } else {
                eprintln!("  \x1b[36m-->\x1b[0m {}", err.file_path.display());
            }
        } else {
            eprintln!("  \x1b[36m-->\x1b[0m {}", err.file_path.display());
        }

        if verbose {
            if let Some(note) = &friendly.explanation {
                eprintln!("  \x1b[36m= note:\x1b[0m {}", note);
            }
            if let Some(help) = &friendly.suggestion {
                eprintln!("  \x1b[33m= help:\x1b[0m {}", help);
            }
        }

        eprintln!();
    }

    eprintln!("Found {} error(s) in stdlib files.", all_errors.len());
    ExitCode::FAILURE
}
