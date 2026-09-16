//! LSP Performance Benchmarks
//!
//! Run with: cargo bench --features lsp --bench lsp_benchmark

use std::time::Instant;

fn main() {
    // Load a representative large file
    let large_content = std::fs::read_to_string("stdlib/testing/test.st")
        .unwrap_or_else(|_| include_str!("../stdlib/primitives/state.st").to_string());

    let small_content = r#"
@scope(".hero") {
    @scroll {
        opacity: 0 -> 1;
        transform: translateY(20px) -> translateY(0);
    }
}
"#;

    // Also create a medium-sized synthetic document
    let medium_content = generate_medium_doc();

    println!("=== LSP Performance Benchmark ===\n");
    println!("File sizes:");
    println!(
        "  Small:  {} bytes ({} lines)",
        small_content.len(),
        small_content.lines().count()
    );
    println!(
        "  Medium: {} bytes ({} lines)",
        medium_content.len(),
        medium_content.lines().count()
    );
    println!(
        "  Large:  {} bytes ({} lines)",
        large_content.len(),
        large_content.lines().count()
    );
    println!();

    // Benchmark parsing
    benchmark_parsing(&small_content, &medium_content, &large_content);

    // Benchmark position mapping
    benchmark_position_mapping(&small_content, &medium_content, &large_content);

    // Benchmark semantic tokens
    benchmark_semantic_tokens(&small_content, &medium_content, &large_content);

    // Benchmark document open (full pipeline)
    benchmark_document_open(&small_content, &medium_content, &large_content);

    // Benchmark validation/diagnostics
    benchmark_validation(&small_content, &medium_content, &large_content);

    // Benchmark colors
    benchmark_colors(&small_content, &medium_content, &large_content);
}

fn generate_medium_doc() -> String {
    let mut doc = String::new();
    for i in 0..50 {
        doc.push_str(&format!(
            r#"
@scope(".section-{}") {{
    @scroll {{
        opacity: 0 -> 1;
        transform: translateY(20px) -> translateY(0);
    }}
    @on("hover") {{
        background-color: #ff0088 -> #00ff88;
    }}
}}
"#,
            i
        ));
    }
    doc
}

fn benchmark_parsing(small: &str, medium: &str, large: &str) {
    println!("--- Parsing (pest) ---");

    // Warm up
    let _ = spacetime::parser::parse(small);

    // Small
    let iterations = 1000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = spacetime::parser::parse(small);
    }
    let small_time = start.elapsed() / iterations;
    println!("  Small:  {:?} per parse", small_time);

    // Medium
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = spacetime::parser::parse(medium);
    }
    let medium_time = start.elapsed() / iterations;
    println!("  Medium: {:?} per parse", medium_time);

    // Large
    let iterations = 50;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = spacetime::parser::parse(large);
    }
    let large_time = start.elapsed() / iterations;
    println!("  Large:  {:?} per parse", large_time);

    println!();
}

fn benchmark_position_mapping(small: &str, medium: &str, large: &str) {
    println!("--- Position Mapping ---");

    use spacetime::lsp::PositionMapper;

    // Small
    let iterations = 10000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = PositionMapper::new(small);
    }
    let small_time = start.elapsed() / iterations;
    println!("  Small:  {:?} per mapping", small_time);

    // Medium
    let iterations = 1000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = PositionMapper::new(medium);
    }
    let medium_time = start.elapsed() / iterations;
    println!("  Medium: {:?} per mapping", medium_time);

    // Large
    let iterations = 500;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = PositionMapper::new(large);
    }
    let large_time = start.elapsed() / iterations;
    println!("  Large:  {:?} per mapping", large_time);

    println!();
}

fn benchmark_semantic_tokens(small: &str, medium: &str, large: &str) {
    println!("--- Semantic Tokens ---");

    use spacetime::lsp::{DocumentState, provide_semantic_tokens};

    // Small
    let doc = DocumentState::new(small.to_string(), 1);
    let iterations = 1000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = provide_semantic_tokens(&doc);
    }
    let small_time = start.elapsed() / iterations;
    println!("  Small:  {:?} per request", small_time);

    // Medium
    let doc = DocumentState::new(medium.to_string(), 1);
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = provide_semantic_tokens(&doc);
    }
    let medium_time = start.elapsed() / iterations;
    println!("  Medium: {:?} per request", medium_time);

    // Large
    let doc = DocumentState::new(large.to_string(), 1);
    let iterations = 50;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = provide_semantic_tokens(&doc);
    }
    let large_time = start.elapsed() / iterations;
    println!("  Large:  {:?} per request", large_time);

    println!();
}

fn benchmark_document_open(small: &str, medium: &str, large: &str) {
    println!("--- Document Open (full pipeline) ---");

    use spacetime::lsp::DocumentState;

    // Small
    let iterations = 500;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = DocumentState::new(small.to_string(), 1);
    }
    let small_time = start.elapsed() / iterations;
    println!("  Small:  {:?} per open", small_time);

    // Medium
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = DocumentState::new(medium.to_string(), 1);
    }
    let medium_time = start.elapsed() / iterations;
    println!("  Medium: {:?} per open", medium_time);

    // Large
    let iterations = 30;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = DocumentState::new(large.to_string(), 1);
    }
    let large_time = start.elapsed() / iterations;
    println!("  Large:  {:?} per open", large_time);

    println!();
}

fn benchmark_validation(small: &str, medium: &str, large: &str) {
    println!("--- Validation/Diagnostics ---");

    use spacetime::lsp::{DocumentState, FormRegistry, validate_document};

    let registry = FormRegistry::new();

    // Small
    let doc = DocumentState::new(small.to_string(), 1);
    let iterations = 1000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = validate_document(&doc, &registry);
    }
    let small_time = start.elapsed() / iterations;
    println!("  Small:  {:?} per validation", small_time);

    // Medium
    let doc = DocumentState::new(medium.to_string(), 1);
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = validate_document(&doc, &registry);
    }
    let medium_time = start.elapsed() / iterations;
    println!("  Medium: {:?} per validation", medium_time);

    // Large
    let doc = DocumentState::new(large.to_string(), 1);
    let iterations = 30;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = validate_document(&doc, &registry);
    }
    let large_time = start.elapsed() / iterations;
    println!("  Large:  {:?} per validation", large_time);

    println!();
}

fn benchmark_colors(small: &str, medium: &str, large: &str) {
    println!("--- Color Detection ---");

    use spacetime::lsp::{DocumentState, provide_document_colors};

    // Small
    let doc = DocumentState::new(small.to_string(), 1);
    let iterations = 5000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = provide_document_colors(&doc);
    }
    let small_time = start.elapsed() / iterations;
    println!("  Small:  {:?} per request", small_time);

    // Medium (has colors in @on hover blocks)
    let doc = DocumentState::new(medium.to_string(), 1);
    let iterations = 500;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = provide_document_colors(&doc);
    }
    let medium_time = start.elapsed() / iterations;
    println!("  Medium: {:?} per request", medium_time);

    // Large
    let doc = DocumentState::new(large.to_string(), 1);
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = provide_document_colors(&doc);
    }
    let large_time = start.elapsed() / iterations;
    println!("  Large:  {:?} per request", large_time);

    println!();
}
