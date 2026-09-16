mod form_extractor;
mod grammar_gen;
mod highlight_gen;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "treesitter-gen")]
#[command(about = "Generate Tree-sitter grammar from Spacetime %form declarations")]
struct Args {
    /// Path to stdlib directory containing .st files
    #[arg(long, default_value = "./stdlib")]
    stdlib: PathBuf,

    /// Output directory for tree-sitter-spacetime
    #[arg(long, default_value = "./tree-sitter-spacetime")]
    out: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Extracting %form patterns from {:?}...", args.stdlib);
    let forms = form_extractor::extract_all_forms(&args.stdlib)?;
    println!("Found {} form patterns", forms.len());

    // Create output directories
    std::fs::create_dir_all(&args.out)?;
    std::fs::create_dir_all(args.out.join("src"))?;
    std::fs::create_dir_all(args.out.join("queries"))?;

    // Generate grammar.js
    println!("Generating grammar.js...");
    let grammar = grammar_gen::generate_grammar(&forms);
    std::fs::write(args.out.join("grammar.js"), grammar)?;

    // Generate highlights.scm
    println!("Generating queries/highlights.scm...");
    let highlights = highlight_gen::generate_highlights(&forms);
    std::fs::write(args.out.join("queries/highlights.scm"), highlights)?;

    // Generate injections.scm
    println!("Generating queries/injections.scm...");
    let injections = highlight_gen::generate_injections();
    std::fs::write(args.out.join("queries/injections.scm"), injections)?;

    // Generate package.json
    println!("Generating package.json...");
    let package_json = generate_package_json();
    std::fs::write(args.out.join("package.json"), package_json)?;

    println!("Done! Run 'cd {:?} && npx tree-sitter generate' to build.", args.out);
    Ok(())
}

fn generate_package_json() -> String {
    serde_json::json!({
        "name": "tree-sitter-spacetime",
        "version": "0.1.0",
        "description": "Tree-sitter grammar for Spacetime DSL",
        "main": "bindings/node",
        "keywords": ["tree-sitter", "parser", "spacetime"],
        "repository": {
            "type": "git",
            "url": "https://github.com/user/tree-sitter-spacetime"
        },
        "license": "MIT",
        "dependencies": {
            "nan": "^2.17.0"
        },
        "devDependencies": {
            "tree-sitter-cli": "^0.22.0"
        },
        "tree-sitter": [{
            "scope": "source.spacetime",
            "file-types": ["st"],
            "injection-regex": "^spacetime$"
        }]
    }).to_string()
}
