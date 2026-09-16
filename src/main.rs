//! Spacetime - Declarative Animation DSL
//!
//! Timelines are the fundamental abstraction. All dynamic behavior is a function
//! of some timeline's progress, whether that timeline is driven by:
//! - Scroll position
//! - Absolute time
//! - Cyclical loops
//! - Mouse position
//! - Discrete events (hover, click)

use axum::extract::State;
use axum::routing::get;
use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use spacetime::analysis::CompileAnalysis;
use spacetime::compiler::{CompileCache, Compiler};
use spacetime::dev_server::{CompilationResult, TestServerState, create_test_runner_routes};
use spacetime::diagnostics::DiagnosticCollector;
use spacetime::html::{RecommendationConfig, detect_html_context_for_st, validate_with_html};
use spacetime::lsp::run_lsp;
use spacetime::parser::parse;
use spacetime::profiler::BundleProfile;
use spacetime::server::{AppMode, AppState, DevCompileCache, create_router};
use spacetime::test_runner::{
    CompileOptions as TestCompileOptions, DiscoverOptions, compile_test_html_with_options,
    discover_tests, discover_tests_with_options,
};
use spacetime::type_system::TypeRegistry;
use spacetime::utils::generate_diff;
use spacetime::watcher::TestWatcher;

#[cfg(feature = "headless")]
use rustyscript::{Runtime, RuntimeOptions};

#[derive(Parser, Debug)]
#[command(name = "spacetime")]
#[command(about = "Declarative animation DSL - timelines as first-class constructs")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Port to listen on
    #[arg(short, long, default_value = "3333")]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Input .st file to parse (uses demo if not provided)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Enable dev mode with hot-reload (reads files on each request)
    #[arg(long)]
    dev: bool,

    /// Site directory containing index.html, animations.st, styles.css
    #[arg(long)]
    site: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Convert a `.st` file to EDN (PLAN-148). EDN is a second concrete syntax
    /// over the same `%form` registry — `.st` remains the reference.
    Edn {
        /// `.st` file to render as EDN
        #[arg(required = true)]
        file: PathBuf,
    },

    /// Convert an EDN file to `.st` (PLAN-148) — the inverse of `edn`.
    St {
        /// `.edn` file to render as Spacetime source
        #[arg(required = true)]
        file: PathBuf,
    },

    /// Check .st files for syntax errors without starting server
    Check {
        /// .st files or directories to check
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Show verbose output including AST stats
        #[arg(short, long)]
        verbose: bool,

        /// Behave as if the project's @version were this date (YYYY-MM-DD):
        /// retired syntax from waves AT/BEFORE it is reported as an error
        /// (what bumping would break); later waves report as pending
        #[arg(long)]
        at_version: Option<String>,

        /// Suppress non-error output: no warning text, no success lines, no
        /// summary. Errors still print. GH-34: warnings are visible by default;
        /// this is the opt-out for scripts that only care about the exit code.
        #[arg(long)]
        quiet: bool,

        /// Treat any warning as a failure: exit non-zero when a file warned.
        /// For CI — the moment warnings are visible, `--deny-warnings` must
        /// exist so a warning-only file cannot pass silently.
        #[arg(long)]
        deny_warnings: bool,
    },
    /// Apply pending syntax migrations (PLAN-076): rewrite old .st syntax
    /// on disk, wave by wave, and bump the project's @version fact
    Migrate {
        /// Project directory (or a single .st file) to migrate
        path: PathBuf,

        /// Apply only this wave (YYYY-MM-DD); older pending waves must be
        /// applied first
        #[arg(long)]
        wave: Option<String>,

        /// Print what would change (per-hunk) without writing anything
        #[arg(long)]
        dry_run: bool,

        /// Explain a migration entry (its retired macros, rewrite rules,
        /// hints) + how many sites in this project it still claims
        #[arg(long)]
        explain: Option<String>,

        /// Scaffold a NEW migration capsule at
        /// stdlib/migrations/entries/<today>-<id>.st (stdlib-authoring tool)
        #[arg(long)]
        scaffold: Option<String>,
    },
    /// Inspect compilation layers (parse, registry, keyframes, expansion, ir, emit, scopes, state)
    Inspect {
        /// .st file to inspect (optional for 'registry' layer)
        file: Option<PathBuf>,

        /// Layer to inspect: parse, registry, keyframes, expansion, ir, emit, scopes, state
        #[arg(long, required = true)]
        layer: String,

        /// Filter to specific CSS selector (or primitive/macro name for registry)
        #[arg(long)]
        selector: Option<String>,

        /// Filter to specific timeline name
        #[arg(long)]
        timeline: Option<String>,

        /// Dispatch layer: a call to resolve, e.g. `@data fetch $api : "/url"`
        #[arg(long)]
        probe: Option<String>,

        /// Output format: pretty (default) or json
        #[arg(long, default_value = "pretty")]
        format: String,
    },
    /// Start the development server (default command)
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3333")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Site directory containing index.html, animations.st, styles.css
        #[arg(required = true)]
        site: PathBuf,

        /// Enable trace logging in generated runtime.js
        #[arg(long)]
        trace: bool,

        /// Enable visual debugger panel for runtime inspection
        #[arg(long)]
        debug: bool,

        /// Run as Android app using Tauri (requires src-tauri/ directory)
        #[arg(long)]
        android: bool,

        /// Run as iOS app using Tauri (requires src-tauri/ directory)
        #[arg(long)]
        ios: bool,

        /// Force fresh stdlib parsing on every request (slower; useful when editing stdlib files)
        #[arg(long)]
        fresh_stdlib: bool,

        /// Profile compilation with Chrome Trace.
        ///
        /// Writes spacetime-trace.json to the working directory.
        /// Open the trace in chrome://tracing or https://ui.perfetto.dev
        ///
        /// Requires building with: cargo build --features perf-trace
        ///
        /// Usage:
        ///   cargo run --features perf-trace -- serve projects/<name>/ --perf-trace
        #[arg(long)]
        perf_trace: bool,
    },
    /// Profile bundle size and show optimization opportunities
    Profile {
        /// Site directory to profile
        #[arg(required = true)]
        site: PathBuf,

        /// Output as JSON instead of human-readable text
        #[arg(long)]
        json: bool,
    },
    /// Compile .st files to JS/CSS output
    Build {
        /// .st file to compile
        #[arg(required = true)]
        file: PathBuf,

        /// Output directory (default: current directory)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Generate source maps for debugging
        #[arg(long)]
        sourcemap: bool,

        /// Minify output
        #[arg(long)]
        minify: bool,

        /// Enable trace logging in generated runtime.js
        #[arg(long)]
        trace: bool,
    },
    /// Render a page to an mp4 under Chromium's virtual clock — the SAME page
    /// `test --cdp` compiles, driven by virtual time instead of the wall clock
    /// (PLAN-132 W5). No author-facing syntax: the page does not know it is
    /// being rendered. Requires the `cdp` feature, a discoverable Chrome, and
    /// `ffmpeg`/`ffprobe` on PATH.
    Render {
        /// Project directory or page (.st / .st.md) to render
        #[arg(required = true)]
        entry: PathBuf,

        /// Output video path (e.g. film.mp4); overwrites loudly
        #[arg(short, long)]
        out: PathBuf,

        /// Frames per second (default 60)
        #[arg(long, default_value_t = 60)]
        fps: u32,

        /// Output width in pixels (default 1920)
        #[arg(long, default_value_t = 1920)]
        width: u32,

        /// Output height in pixels (default 1080)
        #[arg(long, default_value_t = 1080)]
        height: u32,

        /// PLAN-150 W11: device pixel ratio — render at width*dpr × height*dpr
        /// (`--dpr 2` gives a 4K render of a 1080p film). Default 1.
        #[arg(long, default_value_t = 1.0)]
        dpr: f64,

        /// PLAN-150 W11: capture only these timestamps (seconds, comma-separated)
        /// as PNG stills instead of a video — the fast iteration loop
        /// (`--stills 1.5,9,24`). Writes `still-<ms>.png` into `--out`'s dir.
        #[arg(long, value_delimiter = ',')]
        stills: Option<Vec<f64>>,

        /// PLAN-150 W11: render only from this time (seconds).
        #[arg(long)]
        from: Option<f64>,

        /// PLAN-150 W11: render only up to this time (seconds).
        #[arg(long)]
        to: Option<f64>,
    },
    /// Execute a Spacetime script for source transformations
    Script {

        /// Script file (.st) defining the transformation
        #[arg(required = true)]
        script: PathBuf,

        /// Target files or directories to transform
        #[arg(required = true)]
        targets: Vec<PathBuf>,

        /// Preview changes without writing (dry-run mode)
        #[arg(long)]
        dry_run: bool,

        /// Write changes directly to input files
        #[arg(short, long)]
        write: bool,

        /// Output format (text, json, diff)
        #[arg(long, default_value = "text")]
        format: String,

        /// Interactive mode - review and approve each file's changes
        #[arg(short, long)]
        interactive: bool,
    },
    /// Compile and run tests in browser or headlessly via V8
    Test {
        /// .st test files or directories containing .test.st files
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Output HTML file (opens in browser if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Filter tests by name pattern
        #[arg(short, long)]
        filter: Option<String>,

        /// Serve the test page on this port instead of writing to file
        #[arg(long)]
        serve: Option<u16>,

        /// Run tests headlessly using V8 JS engine (no browser required)
        #[arg(long)]
        headless: bool,

        /// Run tests in a warm Chromium over CDP (real layout/timing/paint rungs).
        /// Requires the `cdp` feature + a discoverable Chrome.
        #[arg(long)]
        cdp: bool,

        /// DEPRECATED (W2): multi-engine Playwright parity was removed. Use --cdp.
        #[arg(long, value_delimiter = ',', hide = true)]
        browser: Option<Vec<String>>,

        /// Output format for headless mode (text, json, junit)
        #[arg(long, default_value = "text")]
        format: String,

        /// Backend fidelity ceiling (pure|logic|layout|timing|paint). The V8
        /// headless backend tops out at `logic`; tests needing more REFUSE to run
        /// here rather than fake layout/timing. Default: logic.
        #[arg(long, default_value = "logic")]
        rung: String,

        /// Report macro-expansion (directive) coverage: which registered macro
        /// forms the suite exercised, and which it never touched (the moat).
        #[arg(long)]
        coverage: bool,

        /// Fail if directive coverage is below this percent (with --coverage).
        #[arg(long)]
        min_directive: Option<f64>,

        /// Run with live 3-pane UI in browser (like Cypress)
        #[arg(long)]
        live: bool,

        /// Watch for file changes and auto-reload tests
        #[arg(long)]
        watch: bool,

        /// Enable mobile testing mode (imports stdlib/mobile/testing)
        #[arg(long)]
        mobile: bool,

        /// Target device profile for mobile testing (e.g., "iPhone 15 Pro", "Pixel 8")
        #[arg(long)]
        device: Option<String>,
    },
    /// Start the Language Server Protocol server (for editor integration)
    Lsp,
    /// Start the Model Context Protocol server over stdio (PLAN-037).
    ///
    /// Exposes Spacetime as a live-coding environment to an MCP client such as
    /// Spell. Speaks newline-delimited JSON-RPC 2.0 on stdin/stdout; all logs
    /// go to stderr. Register in spell.kdl:
    ///
    ///   mcp { server "spacetime" type=stdio { command "cargo" args "run" "--quiet" "--" "mcp" } }
    Mcp {
        /// Workspace root for discovering environments (default: current dir).
        #[arg(long)]
        root: Option<PathBuf>,
        /// Override the stdlib root for the dev-mode `__mcp__` filesystem overlay
        /// (FUP-071). When set, the workbench shell `env.st` is read from
        /// `<dir>/__mcp__/env.st` instead of the embedded bytes, so edits are
        /// live on the next mount with no rebuild. Defaults to auto-detecting
        /// `<root>/stdlib`; production (no on-disk stdlib) uses embedded.
        #[arg(long = "stdlib-dir")]
        stdlib_dir: Option<PathBuf>,
    },
    /// Serve the Spacetime MCP workbench as a plain HTTP page — no MCP stdio
    /// transport, no harness process to race, no `inst-N` id to hunt for
    /// (FUP-110). Binds a FIXED port (stable across runs) and opens the
    /// browser straight at the workbench, ready to compose/inspect/edit.
    /// Useful for: driving the workbench yourself with a browser or an
    /// automation tool (puppeteer/@drive) without going through an MCP
    /// client at all.
    Workbench {
        /// Workspace root for discovering environments (default: current dir).
        #[arg(long)]
        root: Option<PathBuf>,
        /// Port to bind (fixed — stable across runs, unlike the MCP stdio
        /// path's ephemeral port-per-launch).
        #[arg(long, default_value = "4949")]
        port: u16,
        /// Override the stdlib root for the dev-mode `__mcp__` filesystem overlay
        /// (same semantics as `spacetime mcp --stdlib-dir`).
        #[arg(long = "stdlib-dir")]
        stdlib_dir: Option<PathBuf>,
        /// Skip auto-opening a browser tab.
        #[arg(long)]
        no_open: bool,
    },
    /// Initialize a new Spacetime project
    Init {
        /// Project name (creates directory with this name)
        #[arg(required = true)]
        name: String,

        /// Include Tauri mobile scaffolding for iOS/Android builds
        #[arg(long)]
        mobile: bool,
    },
    /// Generate tree-sitter grammar from stdlib + extensions
    GenerateGrammar {
        /// Additional .st files to include (glob patterns)
        #[arg(long, short)]
        include: Vec<String>,

        /// Output path (default: tree-sitter-spacetime/grammar.js)
        #[arg(long, short)]
        output: Option<PathBuf>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Export a Spell `LanguageProfile` JSON from the live registry (FEAT-118).
    /// Bridges Spacetime → the Spell harness so `.st` gets outline/`::§`/edit
    /// intelligence from a data profile, not a hand-written Rust profile.
    ExportSpellProfile {
        /// Output path (default: stdout)
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Manage vendored stdlib dependencies (git-submodule-backed %vendor blobs)
    Vendor {
        #[command(subcommand)]
        action: VendorAction,
    },
    /// Run a one-shot project audit (check + headless review + screenshots)
    Doctor {
        /// Project directory to audit
        #[arg(required = true)]
        site: PathBuf,

        /// Routes to navigate (comma-separated, default: "/")
        #[arg(long, value_delimiter = ',', default_value = "/")]
        routes: Vec<String>,

        /// Output format: pretty (default) or json
        #[arg(long, default_value = "pretty")]
        format: String,

        /// Directory to save per-route screenshots
        #[arg(long)]
        screenshot_dir: Option<PathBuf>,

        /// Port for headless server (default: 4444)
        #[arg(long, default_value = "4444")]
        port: u16,

        /// Per-route timeout for `Spacetime.review()` evaluation (ms, default: 30000)
        #[arg(long, default_value = "30000")]
        route_timeout_ms: u64,
    },
    /// Spacetime Host commands (login, push, publish, domain, billing) --
    /// a thin PATH shim to the separate `spacetime-host` CLI binary
    /// (commercial/host, a private repo, not distributed with this OSS
    /// compiler). Mirrors `run_tauri_dev`'s external-process pattern
    /// (spawn + inherit stdio + propagate exit code) but simpler: no
    /// stdout/stderr line-forwarding needed since we inherit the child's
    /// stdio directly rather than interleaving it with our own output.
    #[command(disable_help_flag = true, disable_help_subcommand = true)]
    Host {
        /// Everything after `spacetime host` -- forwarded verbatim to the
        /// real `spacetime-host` binary (e.g. `login`, `push`, `--help`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum VendorAction {
    /// Rebuild a module's vendored bundle from its pinned git submodule.
    ///
    /// Regenerates `<module>/vendor/<dep>.bundle.js` on the filesystem; a running
    /// `serve` picks it up via stdlib hot-reload. Maintainer-time only.
    Build {
        /// Module name under `stdlib/` (e.g. `text`).
        #[arg(required = true)]
        module: String,

        /// Force rebuild even when the bundle matches the submodule commit.
        #[arg(long)]
        force: bool,
    },
}

/// The path this invocation was asked to operate on, if any.
///
/// Used once at startup to anchor stdlib resolution to the FILE rather than to
/// the shell's working directory (BUG-326). Only commands that actually compile
/// something are listed — the rest have no meaningful anchor and correctly fall
/// back to the cwd walk.
fn primary_input_path(command: &Command) -> Option<PathBuf> {
    match command {
        Command::Check { paths, .. } | Command::Test { paths, .. } => paths.first().cloned(),
        Command::Build { file, .. } => Some(file.clone()),
        Command::Serve { site, .. }
        | Command::Profile { site, .. }
        | Command::Doctor { site, .. } => Some(site.clone()),
        Command::Migrate { path, .. } => Some(path.clone()),
        Command::Inspect { file, .. } => file.clone(),
        Command::Render { entry, .. } => Some(entry.clone()),
        _ => None,
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // BUG-326 — anchor the stdlib to what we were ASKED to compile, before any
    // registry loads.
    //
    // The metasystem registry is a process-global `LazyLock`, so it has no file
    // to ask about and every one of its loaders fell back to the CWD. That made
    // the compiler's answer depend on the shell's location: the same file, same
    // binary, checked from a different directory, loaded a different stdlib (or
    // none, failing with a confusing internal-primitive error). The CLI is the
    // one place that knows the real answer, and it knows it here — first, before
    // anything can touch the registry.
    if let Some(path) = args.command.as_ref().and_then(primary_input_path) {
        spacetime::toolchain::set_compile_anchor(&path);
    }

    // Conditionally initialize logging/tracing
    #[cfg(feature = "perf-trace")]
    let _chrome_guard = {
        let perf_trace = matches!(
            &args.command,
            Some(Command::Serve {
                perf_trace: true,
                ..
            })
        );
        if perf_trace {
            use tracing_chrome::ChromeLayerBuilder;
            use tracing_subscriber::prelude::*;
            let (chrome_layer, guard) = ChromeLayerBuilder::new()
                .file("spacetime-trace.json")
                .include_args(true)
                .build();
            let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);
            let filter = tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
            tracing_subscriber::registry()
                .with(chrome_layer)
                .with(fmt_layer)
                .with(filter)
                .init();
            let _ = tracing_log::LogTracer::init();
            eprintln!("\x1b[36mperf-trace:\x1b[0m recording to spacetime-trace.json");
            eprintln!("  Trigger reloads, then Ctrl+C to flush the trace.");
            eprintln!("  View in chrome://tracing or https://ui.perfetto.dev");
            Some(guard)
        } else {
            env_logger::init();
            None
        }
    };

    #[cfg(not(feature = "perf-trace"))]
    {
        if matches!(
            &args.command,
            Some(Command::Serve {
                perf_trace: true,
                ..
            })
        ) {
            eprintln!(
                "\x1b[33mwarning:\x1b[0m --perf-trace has no effect without the perf-trace feature."
            );
            eprintln!("  Rebuild with: cargo build --features perf-trace");
            eprintln!(
                "  Then run:     cargo run --features perf-trace -- serve projects/<name>/ --perf-trace"
            );
        }
        env_logger::init();
    }

    // Handle subcommands
    if let Some(command) = args.command {
        match command {
            // PLAN-148 W5: make the translator inspectable from a shell, with no
            // MCP client in the loop.
            Command::Edn { file } => {
                let source = std::fs::read_to_string(&file)
                    .unwrap_or_else(|e| { eprintln!("cannot read {}: {e}", file.display()); std::process::exit(1) });
                let ast = spacetime::parse(&source)
                    .unwrap_or_else(|e| { eprintln!("{}: {e:?}", file.display()); std::process::exit(1) });
                let doc = spacetime::edn::to_document(&ast);
                println!(
                    "{}",
                    spacetime::edn::write_document(&doc, &spacetime::syntax::STDLIB_REGISTRY)
                );
                return;
            }

            Command::St { file } => {
                let source = std::fs::read_to_string(&file)
                    .unwrap_or_else(|e| { eprintln!("cannot read {}: {e}", file.display()); std::process::exit(1) });
                let st = spacetime::edn::normalize_source(&source, &spacetime::syntax::STDLIB_REGISTRY)
                    .unwrap_or_else(|e| { eprintln!("{}: {e}", file.display()); std::process::exit(1) });
                print!("{st}");
                return;
            }

            Command::Check {
                paths,
                verbose,
                at_version,
                quiet,
                deny_warnings,
            } => {
                run_check(paths, verbose, at_version, quiet, deny_warnings);
                return;
            }
            Command::Migrate {
                path,
                wave,
                dry_run,
                explain,
                scaffold,
            } => {
                run_migrate(path, wave, dry_run, explain, scaffold);
                return;
            }
            Command::Inspect {
                file,
                layer,
                selector,
                timeline,
                probe,
                format,
            } => {
                run_inspect(file, layer, selector, timeline, probe, format);
                return;
            }
            Command::Serve {
                port,
                host,
                site,
                trace,
                debug,
                android,
                ios,
                fresh_stdlib,
                perf_trace,
            } => {
                run_serve(
                    host,
                    port,
                    site,
                    trace,
                    debug,
                    android,
                    ios,
                    !fresh_stdlib,
                    perf_trace,
                )
                .await;
                return;
            }
            Command::Profile { site, json } => {
                run_profile(site, json);
                return;
            }
            Command::Build {
                file,
                output,
                sourcemap,
                minify,
                trace,
            } => {
                run_build(file, output, sourcemap, minify, trace);
                return;
            }
            Command::Render {
                entry,
                out,
                fps,
                width,
                height,
                dpr,
                stills,
                from,
                to,
            } => {
                #[cfg(feature = "cdp")]
                {
                    if !spacetime::cdp::is_available() {
                        eprintln!(
                            "\x1b[31m✗\x1b[0m render needs Chromium, but none was found. Set SPACETIME_CHROME or install one."
                        );
                        std::process::exit(1);
                    }

                    // PLAN-150 W11: --stills is a distinct, faster path — grab
                    // specific timestamps as PNGs, no video mux.
                    if let Some(times_s) = stills {
                        let out_dir = out
                            .parent()
                            .map(|p| p.to_path_buf())
                            .filter(|p| !p.as_os_str().is_empty())
                            .unwrap_or_else(|| std::path::PathBuf::from("."));
                        let sopts = spacetime::render::StillsOptions {
                            times_s,
                            out_dir,
                            width,
                            height,
                            dpr,
                        };
                        let entry_owned = entry.clone();
                        let result = std::thread::spawn(move || {
                            let r = spacetime::render::render_stills(&entry_owned, &sopts);
                            spacetime::cdp::shutdown();
                            r
                        })
                        .join()
                        .expect("stills thread panicked");
                        match result {
                            Ok(paths) => {
                                println!(
                                    "\x1b[32m✓\x1b[0m {} still(s) written:",
                                    paths.len()
                                );
                                for p in &paths {
                                    println!("  {}", p.display());
                                }
                                return;
                            }
                            Err(e) => {
                                eprintln!("\x1b[31m✗\x1b[0m stills failed: {e}");
                                std::process::exit(1);
                            }
                        }
                    }

                    let opts = spacetime::render::RenderOptions {
                        out,
                        fps,
                        width,
                        height,
                        dpr,
                        from_ms: from.map(|s| (s * 1000.0).round() as u64),
                        to_ms: to.map(|s| (s * 1000.0).round() as u64),
                    };
                    // render_page uses its own Tokio runtime (block_on); run it on a
                    // dedicated thread so it is not nested inside the CLI's async
                    // runtime (mirrors the `--cdp` test path).
                    let entry_owned = entry;
                    let result = std::thread::spawn(move || {
                        let r = spacetime::render::render_page(&entry_owned, &opts);
                        spacetime::cdp::shutdown();
                        r
                    })
                    .join()
                    .expect("render thread panicked");
                    match result {
                        Ok(rep) => {
                            println!(
                                "\x1b[32m✓\x1b[0m rendered {} frame(s) of {}ms @ {}fps -> {}",
                                rep.frames,
                                rep.duration_ms,
                                rep.fps,
                                rep.out.display()
                            );
                            return;
                        }
                        Err(e) => {
                            eprintln!("\x1b[31m✗\x1b[0m render failed: {e}");
                            std::process::exit(1);
                        }
                    }
                }
                #[cfg(not(feature = "cdp"))]
                {
                    eprintln!("\x1b[31m✗\x1b[0m render requires the 'cdp' feature");
                    eprintln!("  Rebuild with: cargo build --features cdp");
                    std::process::exit(1);
                }
            }
            Command::Script {
                script,
                targets,
                dry_run,
                write,
                format,
                interactive,
            } => {
                run_script(script, targets, dry_run, write, &format, interactive);
                return;
            }
            Command::Test {
                paths,
                output,
                filter,
                serve,
                headless,
                cdp,
                browser,
                format,
                rung,
                coverage,
                min_directive,
                live,
                watch,
                mobile,
                device,
            } => {
                run_test(
                    paths,
                    output,
                    filter,
                    serve,
                    headless,
                    cdp,
                    browser,
                    format,
                    rung,
                    coverage,
                    min_directive,
                    live,
                    watch,
                    mobile,
                    device,
                )
                .await;
                return;
            }
            Command::Lsp => {
                run_lsp().await;
                return;
            }
            Command::Mcp { root, stdlib_dir } => {
                let workspace_root = root
                    .or_else(|| std::env::current_dir().ok())
                    .unwrap_or_else(|| PathBuf::from("."));
                // FUP-071: an explicit --stdlib-dir overrides the auto-detected
                // `<root>/stdlib` overlay root. Bridged via env var so the
                // resolver (mcp::stdlib_source) needs no plumbed-through config.
                if let Some(dir) = stdlib_dir {
                    // Safe: set once at startup, before the server spawns any
                    // threads (single-threaded env mutation).
                    unsafe { std::env::set_var("SPACETIME_STDLIB_DIR", dir) };
                }
                if let Err(e) = spacetime::mcp::run(workspace_root) {
                    eprintln!("\x1b[31m✗\x1b[0m MCP server error: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Command::Workbench {
                root,
                port,
                stdlib_dir,
                no_open,
            } => {
                let workspace_root = root
                    .or_else(|| std::env::current_dir().ok())
                    .unwrap_or_else(|| PathBuf::from("."));
                if let Some(dir) = stdlib_dir {
                    // Safe: set once at startup, before the server spawns any
                    // threads (single-threaded env mutation) — mirrors `mcp`.
                    unsafe { std::env::set_var("SPACETIME_STDLIB_DIR", dir) };
                }
                match spacetime::mcp::run_http_only(workspace_root, port).await {
                    Ok(bound_port) => {
                        let url = format!("http://127.0.0.1:{bound_port}/__spacetime/workbench");
                        println!(
                            "\n\x1b[32m\u{2713}\x1b[0m Spacetime workbench running at \x1b[36m{url}\x1b[0m"
                        );
                        println!(
                            "  (no MCP stdio transport \u{2014} plain HTTP, drive it with a browser or automation directly)"
                        );
                        println!("\x1b[2m  Press Ctrl+C to stop\x1b[0m\n");
                        if !no_open {
                            #[cfg(target_os = "linux")]
                            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                            #[cfg(target_os = "macos")]
                            let _ = std::process::Command::new("open").arg(&url).spawn();
                            #[cfg(target_os = "windows")]
                            let _ = std::process::Command::new("start")
                                .arg("")
                                .arg(&url)
                                .spawn();
                        }
                        // The live surface runs on its own thread/runtime (see
                        // LiveServer::start_on); park this task until Ctrl+C.
                        let _ = tokio::signal::ctrl_c().await;
                        println!("\nShutting down.");
                    }
                    Err(e) => {
                        eprintln!("\x1b[31m\u{2717}\x1b[0m workbench server error: {e}");
                        std::process::exit(1);
                    }
                }
                return;
            }
            Command::Init { name, mobile } => {
                run_init(name, mobile);
                return;
            }
            Command::GenerateGrammar {
                include,
                output,
                verbose,
            } => {
                run_generate_grammar(include, output, verbose);
                return;
            }
            Command::ExportSpellProfile { output } => {
                run_export_spell_profile(output);
                return;
            }
            Command::Doctor {
                site,
                routes,
                format,
                screenshot_dir,
                port,
                route_timeout_ms,
            } => {
                run_doctor(site, routes, format, screenshot_dir, port, route_timeout_ms).await;
                return;
            }
            Command::Vendor { action } => {
                run_vendor(action);
                return;
            }
            Command::Host { args } => {
                run_host(args).await;
                return;
            }
        }
    }

    // Legacy behavior for backwards compatibility
    run_legacy(args).await;
}

/// Inspect compilation layers
/// Rebuild a module's vendored bundles from its pinned git submodules.
///
/// Loads `stdlib/<module>` to discover its `%vendor` declarations, then runs the
/// shared bundler for each. This is stdlib tooling, NOT a site build: it only
/// regenerates the `out:` artifact on disk (a running `serve` hot-reloads it).
fn run_vendor(action: VendorAction) {
    use spacetime::metasystem::MetaRegistry;
    use spacetime::vendor::build_vendor;
    use std::path::Path;

    match action {
        VendorAction::Build { module, force } => {
            let module_dir = Path::new("stdlib").join(&module);
            if !module_dir.exists() {
                eprintln!(
                    "\x1b[31m✗\x1b[0m stdlib module not found: {}",
                    module_dir.display()
                );
                std::process::exit(1);
            }

            // Load the module's meta defs to find %vendor declarations.
            let mut registry = MetaRegistry::new();
            let result = registry.load_stdlib_collecting_errors(&module_dir);
            for e in &result.errors {
                eprintln!("\x1b[33m⚠\x1b[0m {}: {}", e.file_path.display(), e.message);
            }

            let vendors: Vec<_> = registry.vendors().cloned().collect();
            if vendors.is_empty() {
                eprintln!(
                    "\x1b[33m⚠\x1b[0m no %vendor declarations found in {}",
                    module_dir.display()
                );
                return;
            }

            let mut had_error = false;
            for def in &vendors {
                // %vendor paths are relative to the file that declared them; that
                // file lives within the module dir, so resolve against module_dir.
                match build_vendor(&module_dir, def, force) {
                    Ok(res) => {
                        let verb = if res.skipped { "up-to-date" } else { "built" };
                        println!(
                            "  \x1b[32m✓\x1b[0m {} {} → {} ({} bytes, blake3 {})",
                            def.name,
                            verb,
                            res.out_path.display(),
                            res.bytes,
                            &res.content_hash[..16.min(res.content_hash.len())]
                        );
                    }
                    Err(e) => {
                        eprintln!("  \x1b[31m✗\x1b[0m {}: {}", def.name, e);
                        had_error = true;
                    }
                }
            }
            if had_error {
                std::process::exit(1);
            }
        }
    }
}
fn run_inspect(
    file: Option<PathBuf>,
    layer: String,
    selector: Option<String>,
    timeline: Option<String>,
    probe: Option<String>,
    format: String,
) {
    use spacetime::cli::{InspectFilter, InspectLayer, OutputFormat, run_inspect};

    // Parse layer argument
    let layer = match InspectLayer::from_str(&layer) {
        Some(l) => l,
        None => {
            let valid = InspectLayer::all_names().join(", ");
            eprintln!(
                "\x1b[31m✗\x1b[0m Invalid layer '{}'. Must be one of: {}",
                layer, valid
            );
            std::process::exit(1);
        }
    };

    // Parse format argument
    let output_format = match OutputFormat::from_str(&format) {
        Some(f) => f,
        None => {
            eprintln!(
                "\x1b[31m✗\x1b[0m Invalid format '{}'. Must be 'pretty' or 'json'",
                format
            );
            std::process::exit(1);
        }
    };

    // Registry and dispatch layers don't need a file (they introspect the stdlib).
    if layer == InspectLayer::Registry || layer == InspectLayer::Dispatch {
        let filter = InspectFilter {
            selector,
            timeline,
            probe,
        };
        match run_inspect(None, layer, filter, output_format) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("\x1b[31m✗\x1b[0m {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Other layers require a file
    let file = match file {
        Some(f) => f,
        None => {
            eprintln!(
                "\x1b[31m✗\x1b[0m Layer '{}' requires a .st file argument",
                layer.as_str()
            );
            std::process::exit(1);
        }
    };

    // Create filter
    let filter = InspectFilter {
        selector,
        timeline,
        probe,
    };

    // Run inspection
    match run_inspect(Some(file), layer, filter, output_format) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m {}", e);
            std::process::exit(1);
        }
    }
}

/// One-shot project audit: check + headless review + screenshots
async fn run_doctor(
    site: PathBuf,
    routes: Vec<String>,
    format: String,
    screenshot_dir: Option<PathBuf>,
    port: u16,
    route_timeout_ms: u64,
) {
    use spacetime::cli::{DoctorArgs, OutputFormat, run_doctor as run};

    let output_format = match OutputFormat::from_str(&format) {
        Some(f) => f,
        None => {
            eprintln!(
                "\x1b[31m\u{2717}\x1b[0m Invalid format '{}'. Must be 'pretty' or 'json'",
                format
            );
            std::process::exit(1);
        }
    };

    let args = DoctorArgs {
        site,
        routes,
        format: output_format,
        screenshot_dir,
        port,
        route_timeout_ms,
    };

    // CDP backend readiness (PLAN-027 W2): report whether a Chrome/Chromium is
    // discoverable for the layout/timing/paint test rungs.
    #[cfg(feature = "cdp")]
    {
        match spacetime::cdp::find_chrome() {
            Some(path) => println!(
                "\x1b[32m✓\x1b[0m CDP backend: Chrome found at {}",
                path.display()
            ),
            None => println!(
                "\x1b[33m⚠\x1b[0m CDP backend: no Chrome found (set SPACETIME_CHROME, or use \
                 the kernel-images Docker for CI). Layout/timing/paint tests will be \
                 unavailable; the V8 `logic` backend still works."
            ),
        }
    }

    match run(args).await {
        Ok(report) => {
            if !report.passed {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("\x1b[31m\u{2717}\x1b[0m doctor: {}", e);
            std::process::exit(1);
        }
    }
}

/// Check .st files for syntax errors
/// `spacetime migrate` (PLAN-076 W3): persist pending syntax migrations to
/// disk, wave by wave, then bump the project's `@version` fact. Uses the
/// SAME rewrite engine as the compile-time shim (`migrate::plan_file_apply`) —
/// what this writes is exactly what the shim already compiles.
/// `migrate --explain <id>` — the capsule's full retirement contract: wave
/// date, docs, the embedded retired macros (old grammar ownership), every
/// %rewrite rule (old shape → new template), every %hint, and how many sites
/// in THIS project the migration still claims (pending apply).
fn run_migrate_explain(path: &PathBuf, id: &str) {
    use spacetime::compiler;
    use spacetime::migrate;

    let (registry, stdlib_errors) = compiler::load_stdlib_registry();
    // A capsule that fails to LOAD must not masquerade as an unknown id
    // (R3): surface entry-file errors first (same guard as the apply path).
    for e in &stdlib_errors {
        if e.file_path.components().any(|c| c.as_os_str() == "entries") {
            eprintln!(
                "\x1b[31m✗\x1b[0m migration entry: {} ({})",
                e.message,
                e.file_path.display()
            );
        }
    }
    let Some(mig) = registry.get_migration(id) else {
        let known: Vec<&str> = registry.all_migrations().map(|m| m.id.as_str()).collect();
        eprintln!(
            "\x1b[31m✗\x1b[0m no migration `{id}` (known: {})",
            known.join(", ")
        );
        std::process::exit(2);
    };

    println!("\x1b[1mmigration `{}`\x1b[0m — wave {}", mig.id, mig.date);
    println!("  {}", mig.docs);
    println!();
    println!("  retired definitions (embedded, still executable through the window):");
    for mac in &mig.macros {
        let directive = mac
            .form
            .as_ref()
            .map(|f| f.directive_name.clone())
            .unwrap_or_default();
        println!("    %macro {} — {}", mac.name, directive);
    }
    if !mig.rewrites.is_empty() {
        println!();
        println!("  rewrite rules (automatic):");
        for rule in &mig.rewrites {
            println!("    {}", rule.id);
            for line in rule.match_source.trim().lines() {
                println!("      - {}", line.trim());
            }
            for line in rule.template.trim().lines() {
                println!("      + {}", line.trim());
            }
        }
    }
    if !mig.hints.is_empty() {
        println!();
        println!("  hints (manual):");
        for hint in &mig.hints {
            println!("    @{}: {}", hint.directive, hint.text);
        }
    }

    // Sites this migration still claims in the project (post-@version waves
    // only — an inert wave's leftovers are E0911, not pending).
    let files = migrate::collect_project_st_files(path);
    let root = if path.is_dir() {
        path.join("index.st")
    } else {
        path.clone()
    };
    let version = std::fs::read_to_string(&root)
        .ok()
        .and_then(|c| spacetime::parser::parse(&c).ok())
        .and_then(|ast| migrate::read_syntax_version(&ast.matches).version);
    let mut sites = 0;
    for file in &files {
        let Ok(content) = std::fs::read_to_string(file) else {
            continue;
        };
        let Ok(ast) = spacetime::parser::parse(&content) else {
            continue;
        };
        for fm in &ast.matches {
            let def_name = fm.matched_macro.as_deref().unwrap_or(&fm.macro_name);
            if let Some((mig_id, _)) = registry.retired_by_macro(def_name)
                && mig_id == &mig.id
            {
                sites += 1;
            }
        }
    }
    println!();
    match (sites, version.as_deref()) {
        (0, _) => println!("  no sites in {} — nothing to apply", path.display()),
        (n, Some(v)) if mig.date.as_str() <= v => println!(
            "  \x1b[33m⚠\x1b[0m {n} site(s) but @version is {v} — this wave is inert;              lingering uses are version errors (E0911)"
        ),
        (n, _) => println!(
            "  {n} site(s) pending — `spacetime migrate {}`",
            path.display()
        ),
    }
}

/// `migrate --scaffold <id>` — write a NEW capsule skeleton at
/// `stdlib/migrations/entries/<today>-<id>.st`. Stdlib-authoring tool: the
/// wave date is the authoring date (waves sort lexicographically).
fn run_migrate_scaffold(id: &str) {
    let valid = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid {
        eprintln!("\x1b[31m✗\x1b[0m id `{id}` must be kebab-case ([a-z0-9-])");
        std::process::exit(2);
    }
    let today = today_iso_date();
    let dir = PathBuf::from("stdlib/migrations/entries");
    let file = dir.join(format!("{today}-{id}.st"));
    if file.exists() {
        eprintln!("\x1b[31m✗\x1b[0m {} already exists", file.display());
        std::process::exit(1);
    }
    let skeleton = format!(
        r#"/// {id} — what retired, and what replaces it.
///
/// A %migration is a self-contained CAPSULE (PLAN-079): it embeds the retired
/// %macro definitions VERBATIM (old grammar + %binds semantics), so the old
/// syntax keeps compiling through the window while the rewrite rules /
/// hints below tell users (and `spacetime migrate`) how to cross it. Delete
/// this file and the old syntax ceases to exist.
%migration {id} {{
  %date {today}
  %docs "TODO: one line — what was retired and what replaces it."

  // The retired definition, VERBATIM from where it lived before (one %macro
  // per retired directive this migration owns).
  %macro old-name {{
    %form {{ @old-name(value: $value:expr) }}
    %binds {{ TODO-primitive(&self, value: $value) }}
  }}

  // Automatic rewrite: old shape → new syntax (holes `$x` splice captures).
  %rewrite old-to-new {{
    %match {{
      @old-name(value: $value:expr)
    }}
    %into {{
      TODO-new-syntax `$value`;
    }}
  }}

  // …or, when no mechanical rewrite is sound, guidance for the manual path:
  // %hint for @old-name "Replace with … See docs/MIGRATION.md."
}}
"#,
        id = id,
        today = today
    );
    if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, &skeleton)) {
        eprintln!("\x1b[31m✗\x1b[0m writing {}: {e}", file.display());
        std::process::exit(1);
    }
    println!("\x1b[32m✓\x1b[0m scaffolded {}", file.display());
    println!("  next: fill in the retired %macro verbatim + rewrite rules/hints,");
    println!("  then `spacetime migrate <project> --explain {id}` to see it live.");
}

/// Today's date as YYYY-MM-DD (civil-from-days; no date crate in the deps).
fn today_iso_date() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.as_secs() / 86_400) as i64)
        .unwrap_or(0);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn run_migrate(
    path: PathBuf,
    wave: Option<String>,
    dry_run: bool,
    explain: Option<String>,
    scaffold: Option<String>,
) {
    use spacetime::compiler;
    use spacetime::migrate;

    if let Some(id) = &scaffold {
        run_migrate_scaffold(id);
        return;
    }
    if let Some(id) = &explain {
        run_migrate_explain(&path, id);
        return;
    }

    // Validate the --wave date early.
    if let Some(w) = &wave
        && !migrate::is_iso_wave_date(w)
    {
        eprintln!("\x1b[31m✗\x1b[0m --wave `{w}` is not an ISO date (YYYY-MM-DD)");
        std::process::exit(2);
    }

    let (registry, stdlib_errors) = compiler::load_stdlib_registry();
    // The loader is error-collecting by design: an unrelated broken stdlib
    // file (e.g. one being edited right now) must not block a migration run.
    // A broken MIGRATION ENTRY is different — the registry's migration set
    // may be incomplete, and applying a partial wave is unsafe: hard-fail.
    let mut migration_entry_errors = 0;
    for e in &stdlib_errors {
        let in_entries = e.file_path.components().any(|c| c.as_os_str() == "entries");
        if in_entries {
            migration_entry_errors += 1;
            eprintln!(
                "\x1b[31m✗\x1b[0m migration entry: {} ({})",
                e.message,
                e.file_path.display()
            );
        } else {
            eprintln!(
                "\x1b[33m⚠\x1b[0m stdlib (non-migration, continuing): {} ({})",
                e.message,
                e.file_path.display()
            );
        }
    }
    if migration_entry_errors > 0 {
        eprintln!("\x1b[31m✗\x1b[0m aborting: fix the migration entries first");
        std::process::exit(1);
    }

    let files = migrate::collect_project_st_files(&path);
    if files.is_empty() {
        println!("No .st files under {}", path.display());
        return;
    }

    // The project's @version lives in the ROOT entry (index.st for a dir,
    // the file itself for a single-file run).
    let root = if path.is_dir() {
        path.join("index.st")
    } else {
        path.clone()
    };
    let root_version = std::fs::read_to_string(&root)
        .ok()
        .and_then(|c| spacetime::parser::parse(&c).ok())
        .map(|ast| migrate::read_syntax_version(&ast.matches).version)
        .flatten();

    // Unfiltered scan (also the --wave ordering check).
    let mut unfiltered: Vec<migrate::FileApplyPlan> = Vec::new();
    let mut parse_failures = 0;
    for file in &files {
        match migrate::plan_file_apply(file, &registry, root_version.as_deref(), None) {
            Some(plan) => unfiltered.push(plan),
            None => {
                parse_failures += 1;
                eprintln!(
                    "\x1b[33m⚠\x1b[0m {} does not parse — skipped",
                    file.display()
                );
            }
        }
    }

    if let Some(w) = &wave {
        let older: Vec<&str> = unfiltered
            .iter()
            .flat_map(|p| p.pending.iter().map(|e| e.date.as_str()))
            .filter(|d| *d < w.as_str())
            .collect();
        if !older.is_empty() {
            eprintln!(
                "\x1b[31m✗\x1b[0m wave {w} cannot be applied yet: {} pending entr{} belong to older wave(s) — apply {} first",
                older.len(),
                if older.len() == 1 { "y" } else { "ies" },
                older.iter().min().unwrap()
            );
            std::process::exit(1);
        }
    }

    // The plans to APPLY (wave-filtered when requested).
    let plans: Vec<migrate::FileApplyPlan> = if wave.is_some() {
        files
            .iter()
            .filter_map(|f| {
                migrate::plan_file_apply(f, &registry, root_version.as_deref(), wave.as_deref())
            })
            .collect()
    } else {
        unfiltered
    };

    let changed: Vec<&migrate::FileApplyPlan> = plans.iter().filter(|p| p.is_changed()).collect();
    let automatic: usize = plans
        .iter()
        .flat_map(|p| p.pending.iter())
        .filter(|e| e.is_automatic())
        .count();
    let manual: Vec<&migrate::PendingMigration> = plans
        .iter()
        .flat_map(|p| p.pending.iter())
        .filter(|e| !e.is_automatic())
        .collect();

    // AUD-010 — files this run could not read are NOT evidence of being current.
    //
    // An unparseable file silently left the plan list, so `migrate` printed "the
    // project is current" over files it never inspected. It cannot know that,
    // and the file holding retired syntax is exactly the one most likely not to
    // parse — so the reassuring message was most wrong precisely when it
    // mattered.
    //
    // Reported rather than refused: the migration still runs on every file it
    // CAN read, because blocking a whole tree over one bad file would also block
    // the migration that might fix it. What changes is that the skipped files
    // are named and the exit is non-zero, so no script mistakes a partial pass
    // for a complete one.
    let unreadable: Vec<std::path::PathBuf> = migrate::UNREADABLE_FILES
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default();

    if changed.is_empty() && manual.is_empty() {
        if !unreadable.is_empty() {
            eprintln!(
                "\x1b[31m✗\x1b[0m {} file(s) could not be parsed and were NOT migrated:",
                unreadable.len()
            );
            for p in &unreadable {
                eprintln!("  - {}", p.display());
            }
            eprintln!(
                "  = hint: these files were never inspected, so nothing can be concluded\n\
                 \x20        about whether they still hold retired syntax. Run `spacetime check`\n\
                 \x20        on each to see why it does not parse, fix it, then migrate again."
            );
            std::process::exit(1);
        }
        match &wave {
            Some(w) => println!("No pending entries for wave {w} — nothing to do."),
            None => println!("No pending migrations — the project is current."),
        }
        return;
    }

    if !unreadable.is_empty() {
        eprintln!(
            "\x1b[33mWarning:\x1b[0m {} file(s) could not be parsed and were NOT migrated:",
            unreadable.len()
        );
        for p in &unreadable {
            eprintln!("  - {}", p.display());
        }
    }

    // The wave date a successful apply crosses (newest AUTOMATIC entry date).
    let applied_wave = plans
        .iter()
        .flat_map(|p| p.pending.iter())
        .filter(|e| e.is_automatic())
        .map(|e| e.date.clone())
        .max();

    if dry_run {
        for plan in &changed {
            println!("\x1b[1m{}\x1b[0m", plan.path.display());
            for entry in plan.pending.iter().filter(|e| e.is_automatic()) {
                // Render against the SCANNED source (spans index it; a
                // re-read could have changed underneath — R3 P3).
                print!("{}", migrate::render_pending_hunk(&plan.old_source, entry));
            }
        }
        for entry in &manual {
            println!(
                "  ! manual: {} ({}) — {}",
                entry.migration_id,
                entry.docs,
                entry.hint.as_deref().unwrap_or_default()
            );
        }
        if let Some(d) = &applied_wave {
            let from = root_version.as_deref().unwrap_or("(none)");
            println!("@version: {from} -> {d} (in {})", root.display());
        }
        println!(
            "\x1b[36mdry run\x1b[0m: {} file(s), {} entr{} would change{}",
            changed.len(),
            automatic,
            if automatic == 1 { "y" } else { "ies" },
            if parse_failures > 0 {
                format!("; {parse_failures} file(s) skipped (parse errors)")
            } else {
                String::new()
            }
        );
        return;
    }

    // Apply: write changed files atomically (temp + rename), then bump
    // @version in the root entry — the ONE shared write path.
    let changed_refs: Vec<&migrate::FileApplyPlan> = changed.clone();
    let summary = match migrate::write_apply_plans(&changed_refs, &root, applied_wave.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m {e}");
            std::process::exit(1);
        }
    };
    for plan in &changed {
        println!("\x1b[32m✓\x1b[0m {}", plan.path.display());
    }
    match (&summary.version, &applied_wave) {
        (Some(d), _) => {
            let from = root_version.as_deref().unwrap_or("(none)");
            println!(
                "\x1b[32m✓\x1b[0m @version: {from} -> {d} ({})",
                root.display()
            );
        }
        (None, Some(d)) => {
            eprintln!(
                "\x1b[33m⚠\x1b[0m no root entry at {} — @version bump skipped (add it by hand: `@version {d};`)",
                root.display()
            );
        }
        _ => {}
    }

    println!(
        "\x1b[32m✓\x1b[0m applied {} entr{} across {} file(s){}",
        summary.entries_applied,
        if summary.entries_applied == 1 {
            "y"
        } else {
            "ies"
        },
        summary.files_changed,
        if !manual.is_empty() {
            format!("; {} manual migration(s) remain", manual.len())
        } else {
            String::new()
        }
    );
}

fn truncate_comment_text(text: &str, limit: usize) -> String {
    let single_line = text.replace('\n', " ");
    let mut chars = single_line.chars();
    let truncated: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn report_check_comments(
    scanned: Vec<spacetime::comments::ScannedComment>,
    scanned_files: &[String],
    roster: &BTreeMap<String, spacetime::parser::meta_ast::CommentTypeDefAst>,
    mut diagnostics: Vec<spacetime::comments::CommentDiagnostic>,
    project_root: &Path,
    quiet: bool,
) {
    // GH-34: --quiet suppresses even the informational Comments block, so a
    // clean script run prints truly nothing on a passing check.
    if quiet {
        return;
    }

    let (sidecar, sidecar_diagnostics) = spacetime::comments::read_sidecar(project_root);
    diagnostics.extend(sidecar_diagnostics);
    let index = spacetime::comments::merge(scanned, sidecar, scanned_files, &today_iso_date());
    diagnostics.extend(index.diagnostics);

    let mut counts = BTreeMap::<(String, String), usize>::new();
    for record in &index.records {
        *counts
            .entry((record.status.as_str().to_string(), record.type_id.clone()))
            .or_default() += 1;
    }

    println!("\n\x1b[36mℹ\x1b[0m Comments");
    if counts.is_empty() {
        println!("  none");
    } else {
        for ((status, type_id), count) in counts {
            println!("  {count} {status} {type_id}");
        }
        for record in index
            .records
            .iter()
            .filter(|record| record.status.is_open())
        {
            let hint = roster
                .get(&record.type_id)
                .and_then(|comment_type| comment_type.agent_hint.as_deref())
                .map(|hint| format!(" — hint: {hint}"))
                .unwrap_or_default();
            println!(
                "  open {} [{}] {} — {}{}",
                record.id,
                record.type_id,
                record.anchor.display(),
                truncate_comment_text(&record.text, 96),
                hint
            );
        }
    }
    // An element anchor with no content fingerprint predates FUP-171 W-b. It
    // cannot be verified to still point where it did, and after any edit above
    // it, it may describe a DIFFERENT element than its author selected. `check`
    // has no live DOM to resolve against, so it cannot say which — but staying
    // silent is what let the defect hide in the first place. Name them and say
    // what to do.
    let unverified: Vec<_> = index
        .records
        .iter()
        .filter(|record| {
            matches!(
                &record.anchor,
                spacetime::comments::Anchor::Element {
                    fingerprint: None,
                    ..
                }
            )
        })
        .collect();
    if !unverified.is_empty() {
        println!(
            "  \x1b[33m⚠\x1b[0m {} element comment(s) have no content fingerprint and may point \
             at the wrong element after an edit — re-anchor them from the comments pill to fix",
            unverified.len()
        );
        for record in unverified {
            println!(
                "    {} [{}] {}",
                record.id,
                record.type_id,
                record.anchor.display()
            );
        }
    }

    for diagnostic in diagnostics {
        let location = if diagnostic.line == 0 {
            diagnostic.file
        } else {
            format!("{}:{}", diagnostic.file, diagnostic.line)
        };
        println!("  \x1b[33m⚠\x1b[0m {location} — {}", diagnostic.message);
    }
}

fn run_check(paths: Vec<PathBuf>, verbose: bool, at_version: Option<String>, quiet: bool, deny_warnings: bool) {
    // Validate --at-version early.
    if let Some(v) = &at_version
        && !spacetime::migrate::is_iso_wave_date(v)
    {
        eprintln!("\x1b[31m✗\x1b[0m --at-version `{v}` is not an ISO date (YYYY-MM-DD)");
        std::process::exit(2);
    }
    if let Some(v) = &at_version {
        println!("checking as if @version were {v} (waves at/before it hard-error)");
    }

    // Now check user files

    let mut total_files = 0;
    let mut errors = 0;
    let mut files_with_errors: Vec<(PathBuf, String)> = Vec::new();
    // Comment scans are trivia: collect and report them without touching errors.
    let mut scanned_comments = Vec::new();
    let mut comment_diagnostics = Vec::new();
    let mut scanned_comment_files = Vec::new();
    // THE PROJECT being checked, not the shell's cwd. `check projects/foo/`
    // must read `projects/foo/.comments` and `projects/foo/_prelude.st` —
    // reading `./.comments` instead meant every saved status, reply, and
    // project-declared type was invisible here while the pill and MCP saw
    // them, and the same comment carried a different file identity in each
    // surface. Derive the root the way `serve` does: the directory the
    // checked path names, or its parent when a single file was named —
    // then climb to the nearest `_prelude.st`, so `check <project>/missions/`
    // sees the same brand forms `check <project>/` does.
    let workspace_root = paths
        .first()
        .map(|first| {
            if first.is_dir() {
                spacetime::compiler::project_root_for(&first.join("index.st"))
            } else {
                spacetime::compiler::project_root_for(first)
            }
        })
        .filter(|root| !root.as_os_str().is_empty())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    // Comments use the same stdlib + project overlay roster as MCP and the pill.
    // A malformed overlay remains informational: the scanner reports work without
    // changing check's exit status.
    let (mut comment_registry, comment_roster_load_errors) =
        spacetime::compiler::load_stdlib_registry();
    // PLAN-135 W1: the overlay's `@form` DECLARATIONS ride along too. `build`
    // merges them into every page before `validate_form_refs` runs
    // (src/compiler.rs, the `needs_forms` seam); `check` did not, so every
    // brand form a project declared in `_prelude.st` reported E0947 "unknown
    // form" from the authoring loop while building perfectly — the exact
    // check/build disagreement PLAN-139 W2 removed for imported modules.
    let project_overlay =
        spacetime::compiler::load_project_overlay(&mut comment_registry, Some(&workspace_root));
    let comment_overlay_errors = project_overlay.errors;
    let overlay_form_matches = project_overlay.form_matches;
    let mut comment_roster: BTreeMap<_, _> = comment_registry
        .comment_types()
        .map(|comment_type| (comment_type.id.clone(), comment_type.clone()))
        .collect();
    for error in comment_roster_load_errors
        .into_iter()
        .chain(comment_overlay_errors)
    {
        comment_diagnostics.push(spacetime::comments::CommentDiagnostic {
            file: error.file_path.to_string_lossy().to_string(),
            line: 0,
            message: format!("could not load comment type roster: {}", error.message),
        });
    }
    // PLAN-076: ONE registry clone for the per-file migration scans
    // (cached_stdlib_registry deep-clones per call — hoisting avoids an
    // N-files × registry-clone hot loop).
    let (mig_registry, _) = spacetime::compiler::cached_stdlib_registry();

    for path in &paths {
        if path.is_dir() {
            // Find all .st files in directory recursively. SIP-002: literate
            // documents (`*.st.md`) are Spacetime source too — a second pass
            // collects them (the discovery matcher is suffix-based, and
            // `.st.md` does not end in `.st`).
            let options = DiscoverOptions::new().with_pattern("*.st");
            let mut st_files = discover_tests_with_options(path, &options);
            let lit_options = DiscoverOptions::new().with_pattern("*.st.md");
            st_files.extend(discover_tests_with_options(path, &lit_options));
            // PLAN-148: EDN pages are peers — a third pass collects them.
            let edn_options = DiscoverOptions::new().with_pattern("*.edn");
            st_files.extend(discover_tests_with_options(path, &edn_options));
            // `_prelude.st` is the PROJECT OVERLAY — metasystem declarations the
            // compiler auto-loads before any page, never a page itself. Checking
            // it as one reports a prelude-only problem twice (once as a page
            // error, once as an overlay load error) and tells the author their
            // declarations are a broken page, which is not what they wrote.
            st_files.retain(|candidate| {
                candidate.file_name().and_then(|name| name.to_str())
                    != Some(spacetime::compiler::PROJECT_PRELUDE)
            });
            st_files.sort();
            for file_path in &st_files {
                check_file(
                    file_path,
                    verbose,
                    quiet,
                    deny_warnings,
                    at_version.as_deref(),
                    &mut total_files,
                    &mut errors,
                    &mut files_with_errors,
                    &workspace_root,
                    &mig_registry,
                    &mut scanned_comments,
                    &mut comment_diagnostics,
                    &mut scanned_comment_files,
                    &mut comment_roster,
                    &overlay_form_matches,
                );
            }
        } else if spacetime::literate::is_spacetime_source(path) {
            check_file(
                path,
                verbose,
                quiet,
                deny_warnings,
                at_version.as_deref(),
                &mut total_files,
                &mut errors,
                &mut files_with_errors,
                &workspace_root,
                &mig_registry,
                &mut scanned_comments,
                &mut comment_diagnostics,
                &mut scanned_comment_files,
                &mut comment_roster,
                &overlay_form_matches,
            );
        } else {
            eprintln!("Skipping non-.st file: {}", path.display());
        }
    }

    report_check_comments(
        scanned_comments,
        &scanned_comment_files,
        &comment_roster,
        comment_diagnostics,
        &workspace_root,
        quiet,
    );

    // Summary. `--quiet` suppresses the success summary; the error list still
    // prints (a failing file is never silent), and the exit code is unchanged.
    // A scan that examined NOTHING is not a pass.
    //
    // `✓ All 0 file(s) passed` is byte-identical in shape to a real clean run,
    // so an empty or aborted scan silently masquerades as evidence of
    // correctness. That is how a "zero fallout across 576 files" measurement got
    // reported from a run that scanned nothing at all. Zero units examined is a
    // distinct, loud outcome — never success.
    if total_files == 0 && errors == 0 {
        if !quiet {
            println!();
            println!(
                "\x1b[31m✗\x1b[0m No source files found — nothing was checked."
            );
            println!(
                "  = hint: this is reported as a failure on purpose. A scan that \
                 examined nothing cannot\n         evidence that anything passed, \
                 and an empty result must never read like a clean one."
            );
        }
        std::process::exit(1);
    }

    if !quiet {
        println!();
        if errors == 0 {
            println!("\x1b[32m✓\x1b[0m All {} file(s) passed", total_files);
        } else {
            if errors > 0 {
                println!(
                    "\x1b[31m✗\x1b[0m {} of {} file(s) had errors:",
                    errors, total_files
                );
                for (path, _) in &files_with_errors {
                    println!("  - {}", path.display());
                }
            } else if total_files > 0 {
                println!("\x1b[32m✓\x1b[0m All {} user file(s) passed", total_files);
            }
        }
    }
    if errors > 0 {
        std::process::exit(1);
    }
}

fn check_file(
    path: &PathBuf,
    verbose: bool,
    quiet: bool,
    deny_warnings: bool,
    at_version: Option<&str>,
    total: &mut usize,
    errors: &mut usize,
    files_with_errors: &mut Vec<(PathBuf, String)>,
    workspace_root: &Path,
    mig_registry: &spacetime::metasystem::MetaRegistry,
    scanned_comments: &mut Vec<spacetime::comments::ScannedComment>,
    comment_diagnostics: &mut Vec<spacetime::comments::CommentDiagnostic>,
    scanned_comment_files: &mut Vec<String>,
    comment_roster: &mut BTreeMap<String, spacetime::parser::meta_ast::CommentTypeDefAst>,
    overlay_form_matches: &[spacetime::syntax::FormMatch],
) {
    *total += 1;

    // SIP-002: literate documents tangle to `.st` source first, so `check`
    // parses exactly what the compiler will. The line map rides along so a
    // parse error can be reported against the author's `.st.md` (W4).
    let (content, line_map) = match spacetime::literate::read_source(path) {
        Ok(c) => c,
        Err(e) => {
            *errors += 1;
            let msg = format!("Failed to read: {}", e);
            eprintln!("\x1b[31m✗\x1b[0m {} - {}", path.display(), msg);
            files_with_errors.push((path.clone(), msg));
            return;
        }
    };

    // EDN pages (PLAN-148) lift to an StFile through the same extension-gated
    // ingress the compiler uses; `check` then sees exactly what a compile
    // would. The parse error is pre-rendered to a string so both syntaxes
    // share the one match below.
    let parsed: Result<spacetime::parser::StFile, String> =
        match Compiler::edn_ingress(path, &content) {
            Some(r) => r,
            None => parse(&content).map_err(|e| match &line_map {
                Some(map) => {
                    let original = std::fs::read_to_string(path).unwrap_or_default();
                    spacetime::literate::remap_parse_errors(&e, Some(map), &content, &original)
                        .render_all_plain(&original, &path.to_string_lossy())
                }
                None => e.render_all_plain(&content, &path.to_string_lossy()),
            }),
        };
    match parsed {
        Ok(main_ast) => {
            // The parser knows which byte ranges are literal markup, where `//@`
            // is page text rather than a private comment marker.
            let markup_ranges: Vec<(usize, usize)> = main_ast
                .html_blocks
                .iter()
                .map(|block| (block.span.start, block.span.end))
                .collect();
            let scanned_file = path.to_string_lossy().to_string();
            let (file_comments, scan_diagnostics) =
                spacetime::comments::scan_source_excluding(&scanned_file, &content, &markup_ranges);
            scanned_comment_files.push(scanned_file);
            scanned_comments.extend(file_comments.iter().cloned());
            comment_diagnostics.extend(scan_diagnostics);

            // PLAN-076: surface pending syntax migrations. Cheap hit-scan
            // FIRST — the shim's clones only happen when a migration match
            // is actually present. (Capsule naming: matched_macro is
            // `<mig>#<rule>` / `<mig>#@<macro>` — resolve via retired_by_macro,
            // never get_migration.)
            let has_migration_hit = main_ast.matches.iter().any(|fm| {
                let def_name = fm.matched_macro.as_deref().unwrap_or(&fm.macro_name);
                mig_registry.retired_by_macro(def_name).is_some()
            });
            // Run the shim BEFORE everything downstream: the compile below
            // must see the SHIMMED AST (auto-migrated rewrites consumed), or
            // the pipeline's degrade branch reports E0910 for shapes the shim
            // actually handles (a false error). --at-version analysis runs on
            // the PRE-shim matches (the on-disk reality).
            let shim_outcome = if has_migration_hit {
                let fact = spacetime::migrate::read_syntax_version(&main_ast.matches);
                // HARD CUTOVER: an absent `@version` means CURRENT, so retired
                // syntax is refused here exactly as it is in `compile()` —
                // otherwise `check` would auto-migrate (W0715) a file that
                // `build`/`serve` refuses, and the two would disagree about
                // whether the same source is valid.
                let effective_version = spacetime::migrate::effective_syntax_version(
                    at_version.map(|v| v.to_string()).or(fact.version.clone()),
                    mig_registry,
                );
                let shim = spacetime::migrate::apply_migration_shim(
                    content.clone(),
                    main_ast.clone(),
                    mig_registry,
                    effective_version.as_deref(),
                );
                if !shim.pending.is_empty() {
                    println!(
                        "\x1b[36mℹ\x1b[0m {} — {} syntax migration(s) pending — run `spacetime migrate`",
                        path.display(),
                        shim.pending.len()
                    );
                }
                Some(shim)
            } else {
                None
            };
            // When the shim rewrote, its AST's spans index the SHIMMED
            // source — downstream diagnostics (collector, HTML class
            // injection, pipeline errors) must render against THAT text or
            // every location drifts by the rewrite deltas (R3). The shim's
            // own iteration-0 warnings stay correct: they're emitted below
            // with the collector already swapped, and their spans index the
            // pre-shim text… which only differs where a rewrite landed, and
            // a rewrite site never carries a shim warning span of its own
            // (the warning's span IS the rewritten site — the text at that
            // offset in shim.source is the NEW text, the right anchor).
            let (main_ast, diag_source) = match &shim_outcome {
                Some(shim) => (shim.ast.clone(), shim.source.clone()),
                None => (main_ast, content.clone()),
            };
            let mut diagnostics = DiagnosticCollector::new(&diag_source)
                .with_path(path.to_string_lossy().to_string());

            // Shim W0715s (auto-migrated + hint-kind) surface as warnings.
            // Their spans index the PRE-shim text (iteration-0 coords), so
            // they ride a collector over `content` — emitting them into the
            // post-shim collector would drift them by earlier rewrite deltas
            // (R4). Pipeline diagnostics below use the post-shim collector.
            if let Some(shim) = &shim_outcome
                && !shim.diagnostics.is_empty()
            {
                let mut pre_shim_diags = DiagnosticCollector::new(&content)
                    .with_path(path.to_string_lossy().to_string());
                for diag in &shim.diagnostics {
                    pre_shim_diags.emit(diag.clone());
                }
                println!("{}", pre_shim_diags.render());
            }

            // Resolve and merge imports
            let ast = if !main_ast.imports.is_empty() {
                match spacetime::parser::resolve_imports(&main_ast, path, workspace_root) {
                    Ok(a) => a,
                    Err(e) => {
                        *errors += 1;
                        let msg = format!("Import resolution failed: {}", e);
                        eprintln!("\x1b[31m✗\x1b[0m {} - {}", path.display(), msg);
                        files_with_errors.push((path.clone(), msg));
                        return;
                    }
                }
            } else {
                // PLAN-117 W3: mirror the compiler's no-import branch. A page
                // with no imports never enters import resolution, so the
                // qualified-reference pass must run here too — otherwise
                // `check` (the primary authoring loop) stays silent on an
                // ambiguous `1/$denominator` that `build` correctly refuses.
                let mut solo = main_ast;
                // BUG-229: same omission, same branch. A single-file page that
                // declares its OWN `%macro`/`%capture_type` never entered the
                // user-macro rematch here, so its directives were only ever matched
                // against the stdlib registry — which has never heard of that
                // grammar. The directive silently produced no match, and `check`
                // called the file clean. The rematch is what makes a user grammar
                // enforceable at all, so it must run wherever the compiler runs it.
                // NB rematch against `diag_source`, NOT `content`. When a migration
                // shim rewrote the file, `solo` IS the post-shim AST, and its spans
                // index the shimmed text — rematching against the pre-shim source
                // would assign matches by byte spans drawn from a different string.
                // `Compiler::from_file` already rebinds `content = shimmed.source`
                // before its rematch; this keeps `check` byte-identical to `build`,
                // which for the primary authoring loop is not optional.
                if !solo.meta_defs.is_empty() {
                    spacetime::parser::rematch_with_user_macros(&mut solo, &diag_source);
                }
                let diags = spacetime::pipeline::qualified_refs::resolve_qualified_refs(&mut solo);
                solo.diagnostics.extend(diags);
                solo
            };

            // PLAN-139 W2: form refs are judged against the ASSEMBLED page, the
            // same seam `Compiler` runs (`needs_forms` in src/compiler.rs).
            // Imports are parsed unvalidated — a module's `--name;` is declared
            // by a SIBLING file it cannot see — so the parse-time E0947s they
            // carry are meaningless until the merge. `build` cleared them here
            // and `check` did not, which is how every `stdlib/showcases`
            // pattern came to report four "unknown form" errors from `check`
            // while building perfectly. A diagnostic tool that disagrees with
            // the compiler is worse than no diagnostic tool.
            let mut ast = ast;
            if !overlay_form_matches.is_empty() || spacetime::parser::ast_has_form_refs(&ast) {
                // Same seam as `Compiler::compile`: the project's `_prelude.st`
                // forms join the assembled page BEFORE refs are judged.
                ast.matches.extend(overlay_form_matches.iter().cloned());
                spacetime::parser::validate_form_refs(&mut ast);
                spacetime::parser::expand_style_form_splices(&mut ast);
            }
            let ast = ast;

            // Run validation on the AST
            // Surface PARSER-produced diagnostics (e.g. W0712 inline-@each,
            // W0714 unknown top-level @directive) — they live on `ast.diagnostics`
            // and were previously never shown by `check` (only validate/analysis
            // diagnostics were). Without this an authored typo'd directive passed
            // silently. (BUG-133.)
            // Collapse identical diagnostics before emitting. A construct can be
            // reachable from more than one traversal (the flat `matches` list and
            // the `scopes` tree both carry it), so a whole-page pass can report
            // ONE authoring mistake several times. Reporting a mistake twice
            // trains authors to skim errors, which is how real errors get missed.
            let mut emitted = std::collections::HashSet::new();
            for diag in &ast.diagnostics {
                if emitted.insert(format!("{}|{}", diag.code.as_str(), diag.message)) {
                    diagnostics.emit(diag.clone());
                }
            }

            // Build compile-time analysis using FormMatches
            let mut analysis = CompileAnalysis::new();

            // Use ast.matches for all analysis (FormMatch-based)
            let matches = &ast.matches;

            // Build type registry from FormMatches (Phase 1.1)
            let registry = TypeRegistry::from_form_matches(matches);

            // Analyze using FormMatches
            analysis.analyze_types(matches, &registry);
            analysis.analyze_data_sources(matches, &registry);
            analysis.analyze_locals(matches);
            analysis.analyze_computed(matches);
            analysis.analyze_functions(matches);
            analysis.analyze_scopes(&ast.scopes);
            analysis.analyze_read_usages(&ast.scopes, matches);
            analysis.analyze_timelines(&ast.scopes);

            // Ambiguous-dispatch lint (FEAT-088): needs the meta registry's forms.
            // BUG-155-adjacent: `cached_stdlib_registry()` only loads the 6 CORE
            // stdlib dirs (capture-types/runtime/primitives/syntax/macros/testing)
            // -- it never sees macros from an `@import`ed non-core module (e.g.
            // `stdlib/__mcp__`, `stdlib/3d`). The real compile pipeline registers
            // `ast.meta_defs` (this file's OWN + all its resolved `@import`s) into a
            // fresh registry seeded from the cached one -- mirror that here, so a
            // macro like `@mcp-input` (defined in stdlib/__mcp__, never in the core
            // dirs) is visible to `analyze_dispatch`/`register_macro_yield_signals`
            // below, exactly as it is to the actual compiler. Without this, ANY
            // macro-yielded signal from a non-core module falsely triggers E0405
            // ("references '<name>', which does not exist") on a perfectly valid
            // binding.
            let (mut meta_registry, _) = spacetime::compiler::cached_stdlib_registry();
            for def in ast.meta_defs.clone() {
                let _ = meta_registry.register(def);
            }
            for comment in &file_comments {
                comment_diagnostics.extend(spacetime::comments::validate_against_roster(
                    comment,
                    &*comment_roster,
                ));
            }
            analysis.analyze_dispatch(matches, &meta_registry);

            // Visual consistency lints (W201-W205)
            // Read paired HTML file for HTML-aware lints
            let html_content_for_lint = {
                let st_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("index");
                let html_path = path.parent().map(|d| d.join(format!("{}.html", st_stem)));
                html_path.and_then(|p| std::fs::read_to_string(p).ok())
            };
            analysis.analyze_visual(&ast.scopes, matches, html_content_for_lint.as_deref());

            // Register macro %binds yields as signals from the registry (BUG-126)
            // BEFORE validating binding sources, so @realtime/@presence/@data
            // collection/signal yields resolve instead of raising E0405.
            analysis.register_macro_yield_signals(matches, &meta_registry);

            // GH-26: E0408 ("signal used but not defined") — wired AFTER the
            // yields above so it merges (never clobbers) the registered signal
            // set and suppresses E0408 for any name the analysis already knows.
            // Previously this diagnostic existed with no production caller: an
            // undefined `$signal` compiled to a runtime lookup that rendered
            // empty with zero diagnostics. (Class fix: see the analysis-coverage
            // test asserting every pub fn analyze_* is reachable from here.)
            analysis.analyze_signals_from_ast(&ast);

            // Detect orphans and validate dependencies
            analysis.detect_orphans();
            analysis.validate_computed_dependencies();
            analysis.validate_binding_sources();
            // FUP-150 (PLAN-119 W2): field-check typed dotted reads against their
            // binding type (E0401). Additive: untyped bases are skipped.
            analysis.validate_typed_reads(&registry, &ast.scopes);

            // Validate data source files exist
            if let Some(site_dir) = path.parent() {
                analysis.validate_data_source_files(site_dir);

                // Wave B (FEAT-135): self-aware LiveView contract check. If a
                // `<page>.contract.json` sidecar exists, every `@host … live(…)`
                // page intent (`send emit "X"`) is checked ⊆ the server module's
                // declared events; a mismatch is E0928. Opt-in: no sidecar, no check.
                for diag in spacetime::analysis::liveview_contract::check_liveview_contracts(
                    site_dir, matches,
                ) {
                    analysis.diagnostics.push(diag);
                }
            }

            // Merge analysis diagnostics
            for diag in &analysis.diagnostics {
                diagnostics.emit(diag.clone());
            }

            // GH-12 / PLAN-137 W6 (I6): surface the W0963 warning half of the
            // `%scope element(<tag>)` check. The E0963 error half already FAILS the
            // compile via compile_pipeline (pipeline_errors) below — this pass emits
            // ONLY the warning (element unknowable, e.g. `.hero { @stage }`), so it
            // is visible and never silent but does not fail the file. Same function,
            // filtered by severity per channel (no duplicated detection logic).
            for d in spacetime::validation::element_scope::scope_element_diagnostics(
                &ast,
                &meta_registry,
            ) {
                if d.severity == spacetime::diagnostics::Severity::Warning {
                    diagnostics.emit(d);
                }
            }

            // HTML validation: check selectors against paired HTML file
            // Uses convention-based naming: foo.st -> foo.html, falls back to index.html
            if let Ok(Some(mut html_ctx)) = detect_html_context_for_st(path) {
                // Register classes from @template body HTML so scope selectors
                // targeting template-created elements pass E0602 validation
                spacetime::html::inject_template_classes_from_ast(
                    &mut html_ctx,
                    &ast,
                    &diag_source,
                );
                spacetime::html::inject_bind_classes_from_ast(&mut html_ctx, &ast);

                validate_with_html(
                    &ast,
                    &html_ctx,
                    &mut diagnostics,
                    Some(RecommendationConfig::all()),
                );
            }

            // Run full compilation to catch pipeline resolution errors
            // (e.g., missing required bindings for nested body directives)
            let compiled = Compiler::from_ast(&ast)
                .with_version_override(at_version.map(|v| v.to_string()))
                // The checked file's directory is its project — `_prelude.st`
                // participates in `check` exactly as under `serve` (PLAN-135).
                .with_site_dir(path.parent().map(|p| p.to_path_buf()))
                .compile();
            // Hint-kind per-site warnings (span + the entry's %hint) ride a
            // dedicated channel out of the pipeline — surface them (R4).
            for w in &compiled.migration_warnings {
                use spacetime::diagnostics::{DiagnosticCode, SourceSpan as DiagSpan};
                let mut d = spacetime::diagnostics::Diagnostic::warning(
                    DiagnosticCode::W0715,
                    w.message.clone(),
                );
                if let Some(s) = w.span {
                    d = d.with_span(DiagSpan::new(s.start, s.end));
                }
                if let Some(h) = &w.hint {
                    d = d.with_hint(h.clone());
                }
                diagnostics.emit(d);
            }
            for pe in &compiled.pipeline_errors {
                use spacetime::diagnostics::{DiagnosticCode, SourceSpan as DiagSpan};
                // FUP-148 (structural fix): derive the variant from the SAME
                // table that produced the string. This used to be an
                // arm-per-code match whose `_ => E0806` fallback silently
                // RELABELLED any unlisted code as "unknown directive" — a
                // correct diagnostic reported under a wrong name, with no
                // symptom but a confusing message. It caught E0900, E0904,
                // E0906, E0938 and E0946 in turn, each "fixed" by appending
                // one more line to the same doomed table. E0951 was next.
                //
                // An unknown code now stays E0806 only when it is genuinely
                // unrecognizable, and `DiagnosticCode::from_code_str` is
                // proven total by `every_code_round_trips`.
                let code = DiagnosticCode::from_code_str(&pe.code).unwrap_or(DiagnosticCode::E0806);
                let span = pe.span.map(|s| DiagSpan::new(s.start, s.end));
                if let Some(hint) = &pe.hint {
                    diagnostics.error_with_hint(code, &pe.message, span, hint);
                } else {
                    diagnostics.error(code, &pe.message, span);
                }
            }

            // Check if there are any diagnostics to report. GH-34: warnings are
            // VISIBLE BY DEFAULT — a warning-only file used to print green
            // "All files passed" with zero warning text (the gate was
            // `verbose || errors`). `--quiet` suppresses the block (errors still
            // print, so a failing file is never silent); `--deny-warnings`
            // promotes any warning to a non-zero exit for CI.
            let has_diags = diagnostics.error_count() > 0 || diagnostics.warning_count() > 0;
            if has_diags && (diagnostics.error_count() > 0 || !quiet) {
                println!("\x1b[33m⚠\x1b[0m {}", path.display());
                println!("{}", diagnostics.render());
            }

            if diagnostics.error_count() > 0 {
                *errors += 1;
                files_with_errors.push((path.clone(), "Validation errors".to_string()));
                return;
            }
            // Warning-only files stay exit 0 unless --deny-warnings is set.
            if deny_warnings && diagnostics.warning_count() > 0 {
                *errors += 1;
                files_with_errors.push((path.clone(), "Warnings denied".to_string()));
                return;
            }

            if verbose && !quiet {
                // Show detailed analysis report
                println!("\x1b[32m✓\x1b[0m {}", path.display());
                let project_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                println!("{}", analysis.generate_report(project_name));
            } else if !quiet {
                println!("\x1b[32m✓\x1b[0m {}", path.display());
            }
        }
        Err(msg) => {
            *errors += 1;
            eprintln!("\x1b[31m✗\x1b[0m {}\n{}", path.display(), msg);
            files_with_errors.push((path.clone(), msg));
        }
    }
}

/// Profile bundle size for a site directory
fn run_profile(site: PathBuf, json_output: bool) {
    let st_path = site.join("animations.st");
    let compiled = match Compiler::from_file(&st_path, &site) {
        Ok(c) => c.compile(),
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m {}", e);
            std::process::exit(1);
        }
    };
    let profile = BundleProfile::from_compiled(&compiled).with_images(&site);

    if json_output {
        println!("{}", serde_json::to_string_pretty(&profile).unwrap());
    } else {
        print!("{}", profile.to_display_string());
    }
}

/// Build/compile a .st file to JS/CSS output
fn run_build(file: PathBuf, output: Option<PathBuf>, sourcemap: bool, minify: bool, trace: bool) {
    use spacetime::emit::SourceMapBuilder;

    // Site-level export: if the path is a directory, run full site export
    if file.is_dir() {
        let output_dir = output.unwrap_or_else(|| file.join("dist"));
        let config = spacetime::export::ExportConfig {
            site_dir: file.clone(),
            output_dir: output_dir.clone(),
            minify,
        };
        match spacetime::export::export_site(&config) {
            Ok(report) if report.pages_exported == 0 => {
                // BUG-351 — zero pages is a failed build, not a quiet success.
                //
                // This printed `✓ Export complete: Pages: 0` and exited 0, so a
                // deploy step reading the exit code ships an empty dist/ and
                // reports green. "Nothing went wrong" is not "something went
                // right"; only the second is what a `✓` promises.
                //
                // Same defect class as `check` reporting `✓ All 0 file(s)
                // passed` — an instrument emitting its success output while
                // having done no work.
                eprintln!("\x1b[31m\u{2717}\x1b[0m Export produced no pages — nothing was written.");
                eprintln!(
                    "  = hint: every source file compiled, but none of them emitted a page,\n\
                     \x20        so dist/ is empty. A page needs markup to render; a file that is\n\
                     \x20        only styles, only imports, or whose markup is commented out\n\
                     \x20        exports nothing."
                );
                for w in &report.warnings {
                    eprintln!("  \x1b[33mWarning:\x1b[0m {}", w);
                }
                std::process::exit(1);
            }
            Ok(report) => {
                println!("\x1b[32m\u{2713}\x1b[0m Export complete:");
                println!("  Pages: {}", report.pages_exported);
                println!("  Files copied: {}", report.files_copied);
                println!("  Total size: {} bytes", report.total_size);
                for w in &report.warnings {
                    eprintln!("  \x1b[33mWarning:\x1b[0m {}", w);
                }
            }
            Err(e) => {
                eprintln!("\x1b[31m\u{2717}\x1b[0m Export failed: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Read and parse the input file
    let content = match std::fs::read_to_string(&file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m Failed to read {}: {}", file.display(), e);
            std::process::exit(1);
        }
    };

    let workspace_root = spacetime::compiler::project_root_for(&file);
    let workspace_root = workspace_root.as_path();
    let mut compiled = match Compiler::from_file(&file, workspace_root) {
        // The nearest `_prelude.st` names the project: it participates in
        // single-file builds exactly as it does under `serve` (PLAN-135 W1).
        Ok(c) => c
            .trace(trace)
            .with_site_dir(Some(workspace_root.to_path_buf()))
            .compile(),
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m {}", e);
            std::process::exit(1);
        }
    };

    // Check for pipeline errors (e.g., stdlib parse errors)
    if !compiled.pipeline_errors.is_empty() {
        use spacetime::pipeline::make_error_friendly;

        eprintln!(
            "\x1b[31m✗\x1b[0m Compilation failed with {} error(s):",
            compiled.pipeline_errors.len()
        );
        for err in &compiled.pipeline_errors {
            eprintln!();

            // Get source line for friendly error generation
            let src_line = if let Some(macro_file) = &err.macro_file {
                if let Some(span) = &err.bind_span {
                    std::fs::read_to_string(macro_file).ok().map(|content| {
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
                }
            } else {
                None
            };

            // Generate friendly error message
            // Prefer the structured hint from the error if available
            let friendly = make_error_friendly(&err.message, src_line.as_deref());

            // Print error summary
            eprintln!("  \x1b[31merror[{}]\x1b[0m: {}", err.code, friendly.summary);

            // Print our structured hint if available (takes precedence over generated hint)
            if let Some(hint) = &err.hint {
                eprintln!(
                    "  \x1b[33m= help:\x1b[0m {}",
                    hint.replace("\n\n", "\n  = help: ")
                );
            }

            // Print file location with source context
            if let Some(macro_file) = &err.macro_file {
                if let Some(span) = &err.bind_span {
                    if let Ok(content) = std::fs::read_to_string(macro_file) {
                        let before = &content[..span.start.min(content.len())];
                        let line = before.matches('\n').count() + 1;
                        let last_newline = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
                        let col = span.start - last_newline + 1;

                        // Extract the source line
                        let line_end = content[span.start.min(content.len())..]
                            .find('\n')
                            .map(|p| span.start + p)
                            .unwrap_or(content.len());
                        let src_line = &content[last_newline..line_end.min(content.len())];

                        eprintln!("  \x1b[36m-->\x1b[0m {}:{}:{}", macro_file, line, col);
                        eprintln!("     |");
                        eprintln!("  {:>3} | {}", line, src_line);
                        eprintln!(
                            "     | {}\x1b[31m^\x1b[0m",
                            " ".repeat(col.saturating_sub(1))
                        );
                    } else {
                        eprintln!("  \x1b[36m-->\x1b[0m {}", macro_file);
                    }
                } else {
                    eprintln!("  \x1b[36m-->\x1b[0m {}", macro_file);
                }
            }

            // Print explanation (note)
            if let Some(explanation) = &friendly.explanation {
                eprintln!("  \x1b[36m= note:\x1b[0m {}", explanation);
            }

            // Print suggestion (help) - only if we didn't already print a structured hint
            if err.hint.is_none()
                && let Some(suggestion) = &friendly.suggestion
            {
                eprintln!("  \x1b[33m= help:\x1b[0m {}", suggestion);
            }

            // Print general help - only if we didn't already print a structured hint
            if err.hint.is_none()
                && let Some(help) = &friendly.help
            {
                eprintln!("  \x1b[90m= info:\x1b[0m {}", help);
            }
        }
        eprintln!();
        std::process::exit(1);
    }

    // Generate source maps if requested
    if sourcemap {
        let source_name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("input.st");

        // Build JS source map
        let mut js_map = SourceMapBuilder::new("spacetime.js");
        let js_src_idx = js_map.add_source(source_name, Some(&content));
        // Add a mapping for the beginning of the output
        // Full span propagation will be enhanced incrementally
        js_map.add_mapping(0, 0, js_src_idx, 0, 0);
        compiled.js_source_map = Some(js_map.build());

        // Build CSS source map
        let mut css_map = SourceMapBuilder::new("spacetime.css");
        let css_src_idx = css_map.add_source(source_name, Some(&content));
        css_map.add_mapping(0, 0, css_src_idx, 0, 0);
        compiled.css_source_map = Some(css_map.build());
    }

    // Determine output directory
    // BUG-189: when --output is omitted, default the output dir to the ENTRY
    // FILE's own directory — never the process CWD. The old default
    // (`std::env::current_dir()`) made the post-build asset-link injection's
    // RECURSIVE collect_html_files walk the entire CWD subtree, rewriting
    // every .html under it (vendored submodule docs included) when building
    // e.g. `project/main.st` from the repo root. Artifacts now land next to
    // the source entry, and injection is scoped to that directory.
    let out_dir = output.unwrap_or_else(|| {
        file.parent()
            .map(|p| p.to_path_buf())
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| std::env::current_dir().unwrap())
    });

    // Create output directory if it doesn't exist
    if !out_dir.exists()
        && let Err(e) = std::fs::create_dir_all(&out_dir)
    {
        eprintln!("\x1b[31m✗\x1b[0m Failed to create output directory: {}", e);
        std::process::exit(1);
    }

    // Write JS output
    let js_path = out_dir.join("spacetime.js");
    let js_content = if minify {
        // Basic minification: remove extra whitespace and comments
        compiled
            .js
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .collect::<Vec<_>>()
            .join("")
    } else {
        compiled.js.clone()
    };

    // Append source map reference if generated
    let js_output = if sourcemap {
        format!("{}\n//# sourceMappingURL=spacetime.js.map", js_content)
    } else {
        js_content
    };

    if let Err(e) = std::fs::write(&js_path, &js_output) {
        eprintln!(
            "\x1b[31m✗\x1b[0m Failed to write {}: {}",
            js_path.display(),
            e
        );
        std::process::exit(1);
    }

    // Write CSS output
    let css_path = out_dir.join("spacetime.css");
    let css_content = if minify {
        compiled
            .css
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("")
    } else {
        compiled.css.clone()
    };

    // Append source map reference if generated
    let css_output = if sourcemap {
        format!("{}\n/*# sourceMappingURL=spacetime.css.map */", css_content)
    } else {
        css_content
    };

    if let Err(e) = std::fs::write(&css_path, &css_output) {
        eprintln!(
            "\x1b[31m✗\x1b[0m Failed to write {}: {}",
            css_path.display(),
            e
        );
        std::process::exit(1);
    }

    // Write source maps if generated
    if sourcemap {
        if let Some(ref js_map) = compiled.js_source_map {
            let map_path = out_dir.join("spacetime.js.map");
            if let Err(e) = std::fs::write(&map_path, js_map) {
                eprintln!(
                    "\x1b[31m✗\x1b[0m Failed to write {}: {}",
                    map_path.display(),
                    e
                );
                std::process::exit(1);
            }
        }

        if let Some(ref css_map) = compiled.css_source_map {
            let map_path = out_dir.join("spacetime.css.map");
            if let Err(e) = std::fs::write(&map_path, css_map) {
                eprintln!(
                    "\x1b[31m✗\x1b[0m Failed to write {}: {}",
                    map_path.display(),
                    e
                );
                std::process::exit(1);
            }
        }
    }

    // Execute build scripts if present
    if !compiled.build_scripts.is_empty() {
        println!(
            "\x1b[36m⚡\x1b[0m Running {} build script(s)...",
            compiled.build_scripts.len()
        );

        // Find paired HTML file
        let html_content = if file.extension().is_some_and(|e| e == "st") {
            // Convention: foo.st pairs with foo.html
            let html_path = file.with_extension("html");
            if html_path.exists() {
                match std::fs::read_to_string(&html_path) {
                    Ok(content) => content,
                    Err(e) => {
                        eprintln!(
                            "\x1b[33m⚠\x1b[0m Could not read paired HTML {}: {}",
                            html_path.display(),
                            e
                        );
                        String::new()
                    }
                }
            } else {
                // Try index.html in the same directory
                let index_path = workspace_root.join("index.html");
                if index_path.exists() {
                    std::fs::read_to_string(&index_path).unwrap_or_default()
                } else {
                    eprintln!("\x1b[33m⚠\x1b[0m No paired HTML found for build scripts");
                    String::new()
                }
            }
        } else {
            String::new()
        };

        // Run build scripts in a separate thread to avoid nesting inside
        // the Tokio runtime (`execute_build_scripts` → rustyscript → block_on
        // panics when a Tokio runtime is already active on the current thread).
        let scripts = compiled.build_scripts.clone();
        let ws = workspace_root.to_path_buf();
        let html = html_content.clone();
        let od = out_dir.clone();

        let build_result = std::thread::spawn(move || {
            spacetime::build_runtime::execute_build_scripts(&scripts, &ws, &html, &od)
        })
        .join()
        .expect("build-script thread panicked");

        match build_result {
            Ok(result) => {
                for msg in &result.log_messages {
                    match msg.level {
                        spacetime::build_runtime::LogLevel::Info => {
                            println!("  \x1b[36m•\x1b[0m {}", msg.message);
                        }
                        spacetime::build_runtime::LogLevel::Warn => {
                            println!("  \x1b[33m⚠\x1b[0m {}", msg.message);
                        }
                    }
                }
                if !result.written_files.is_empty() {
                    println!(
                        "\x1b[32m✓\x1b[0m Build scripts wrote {} file(s)",
                        result.written_files.len()
                    );
                }
            }
            Err(errors) => {
                eprintln!("\x1b[31m✗\x1b[0m Build script execution failed:");
                for err in &errors {
                    eprintln!("  {}", err);
                }
                std::process::exit(1);
            }
        }
    }

    // Copy static assets (fonts, images, data files, etc.) from workspace to output
    {
        const ASSET_EXTENSIONS: &[&str] = &[
            // Fonts
            "woff",
            "woff2",
            "ttf",
            "otf",
            "eot",
            // Images
            "png",
            "jpg",
            "jpeg",
            "gif",
            "svg",
            "webp",
            "ico",
            "avif",
            "bmp",
            // Data
            "json",
            "xml",
            "csv",
            "tsv",
            // Audio/Video
            "mp3",
            "mp4",
            "webm",
            "ogg",
            "wav",
            // Documents
            "pdf",
            // Web
            "webmanifest",
        ];

        fn copy_assets(
            dir: &std::path::Path,
            workspace: &std::path::Path,
            out_dir: &std::path::Path,
            extensions: &[&str],
            copied: &mut u32,
        ) {
            let entries = match std::fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => return,
            };

            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = entry.file_name();
                let name_str = name.to_string_lossy();

                // Skip output dir, hidden dirs, and module dirs
                if path.starts_with(out_dir)
                    || name_str.starts_with('.')
                    || name_str == "modules"
                    || name_str == "node_modules"
                {
                    continue;
                }

                if path.is_dir() {
                    copy_assets(&path, workspace, out_dir, extensions, copied);
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
                    && extensions.contains(&ext.to_lowercase().as_str())
                    && let Ok(rel) = path.strip_prefix(workspace)
                {
                    let dest = out_dir.join(rel);
                    if let Some(parent) = dest.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::copy(&path, &dest).is_ok() {
                        *copied += 1;
                    }
                }
            }
        }

        let mut copied = 0u32;
        copy_assets(
            workspace_root,
            workspace_root,
            &out_dir,
            ASSET_EXTENSIONS,
            &mut copied,
        );
        if copied > 0 {
            println!("\x1b[32m✓\x1b[0m Copied {} static asset(s)", copied);
        }
    }

    // Post-process HTML files: inject spacetime.css/js links
    {
        fn collect_html_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        collect_html_files(&path, out);
                    } else if path.extension().is_some_and(|ext| ext == "html") {
                        out.push(path);
                    }
                }
            }
        }

        let mut html_files = Vec::new();
        collect_html_files(&out_dir, &mut html_files);

        let mut injected_count = 0u32;
        for path in &html_files {
            let mut content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Skip if already injected or no </body>
            if content.contains("spacetime.css") || !content.contains("</body>") {
                continue;
            }

            // Compute relative path from this HTML's dir to out_dir root
            let rel = path
                .parent()
                .unwrap()
                .strip_prefix(&out_dir)
                .unwrap_or(std::path::Path::new(""));
            let depth = rel.components().count();
            let prefix = if depth == 0 {
                ".".to_string()
            } else {
                (0..depth).map(|_| "..").collect::<Vec<_>>().join("/")
            };

            // Inject FOUC prevention in </head>
            if let Some(pos) = content.find("</head>") {
                let fouc = "    <style>:not(:defined){visibility:hidden}</style>\n";
                content.insert_str(pos, fouc);
            }

            // Inject CSS + JS before </body>
            if let Some(pos) = content.rfind("</body>") {
                let inject = format!(
                    "    <link rel=\"stylesheet\" href=\"{}/spacetime.css\" />\n    \
                     <script src=\"{}/spacetime.js\"></script>\n",
                    prefix, prefix
                );
                content.insert_str(pos, &inject);
            }

            if std::fs::write(path, &content).is_ok() {
                injected_count += 1;
            }
        }

        if injected_count > 0 {
            println!(
                "\x1b[32m✓\x1b[0m Injected asset links into {} HTML file(s)",
                injected_count
            );
        }
    }

    // Print summary
    println!("\x1b[32m✓\x1b[0m Build complete:");
    println!("  {} ({} bytes)", js_path.display(), js_output.len());
    println!("  {} ({} bytes)", css_path.display(), css_output.len());
    if sourcemap {
        println!(
            "  {} (source map)",
            out_dir.join("spacetime.js.map").display()
        );
        println!(
            "  {} (source map)",
            out_dir.join("spacetime.css.map").display()
        );
    }
}

/// Run the dev server with a site directory
async fn run_serve(
    host: String,
    port: u16,
    site: PathBuf,
    trace: bool,
    debug: bool,
    android: bool,
    ios: bool,
    cache_stdlib: bool,
    _perf_trace: bool,
) {
    use spacetime::debugger::DebuggerConfig;
    use tokio::sync::broadcast;
    use tower_http::services::ServeDir;

    // Check if this is a Tauri project
    let tauri_dir = site.join("src-tauri");
    let is_tauri_project = tauri_dir.exists();

    // If --android or --ios flag is set, run Tauri mobile dev
    if android || ios {
        if !is_tauri_project {
            eprintln!(
                "\x1b[31m✗\x1b[0m Tauri project not found at {}",
                tauri_dir.display()
            );
            eprintln!(
                "  Run 'spacetime init {} --mobile' to create a Tauri project",
                site.file_name().unwrap_or_default().to_string_lossy()
            );
            std::process::exit(1);
        }

        run_tauri_dev(&site, android, ios).await;
        return;
    }

    // If Tauri project detected but no mobile flags, inform user
    if is_tauri_project {
        println!(
            "\x1b[36m📱\x1b[0m Tauri project detected at {}",
            tauri_dir.display()
        );
        println!("  Use --android or --ios flags for mobile development");
        println!();
    }

    println!("Dev mode enabled - hot-reload from: {}", site.display());
    println!("  - Compiles {{name}}.st for {{name}}.html (convention-based)");
    println!("  - Live reload: watching for .st changes");
    if trace {
        println!("  - Trace logging enabled in runtime.js");
    }
    if debug {
        println!("  - Debug panel enabled at /__spacetime/debugger");
    }
    if cache_stdlib {
        println!(
            "  - Stdlib: cached with incremental hot-reload (use --fresh-stdlib for full reload)"
        );
    } else {
        println!("  - Stdlib: fresh per request (slower, for stdlib development)");
    }

    // Create sync state for WebSocket support
    let (tx, _rx) = broadcast::channel(100);
    let compilation_cache = Arc::new(std::sync::RwLock::new(None));
    let sync_state =
        Arc::new(TestServerState::new(tx, compilation_cache).with_site_dir(site.clone()));

    // Quick-access workbench (FUP-110/friction fix): `spacetime serve` also boots
    // the MCP live surface on its own ephemeral loopback port and exposes
    // `/__spacetime/workbench` as a one-click redirect into it — no separate
    // `spacetime mcp`/`spacetime workbench` invocation needed just to poke at the
    // workbench while iterating on a project. Best-effort: a failure to boot it
    // (e.g. port exhaustion) only disables this convenience route, never the dev
    // server itself.
    let workbench_port: Option<u16> = {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match spacetime::mcp::run_http_only(root, 0).await {
            Ok(p) => {
                println!("  - Workbench: http://{host}:{port}/__spacetime/workbench");
                Some(p)
            }
            Err(e) => {
                eprintln!("\x1b[33mwarning:\x1b[0m workbench live surface failed to start: {e}");
                None
            }
        }
    };

    // Compile caches are created HERE (not inline in AppState) so the watch
    // loop below can warm them proactively on every file change.
    let compile_cache = std::sync::Arc::new(CompileCache::new());
    let dev_compile_cache = std::sync::Arc::new(DevCompileCache::new());

    // Start file watcher for live reload. Also watch `stdlib/` when present so a
    // `vendor build` rewriting a `*.bundle.js` triggers reload (PLAN-024 W2).
    {
        let mut watch_paths = vec![site.clone()];
        let stdlib_dir = PathBuf::from("stdlib");
        if stdlib_dir.exists() {
            watch_paths.push(stdlib_dir);
        }
        let (mut watcher, mut file_rx) = TestWatcher::new(watch_paths);
        let changes_tx = sync_state.file_changes_tx.clone();
        let watch_site = site.clone();
        let watch_compile_cache = compile_cache.clone();
        let watch_dev_cache = dev_compile_cache.clone();
        tokio::spawn(async move {
            if let Err(e) = watcher.watch().await {
                eprintln!("\x1b[33mwarning:\x1b[0m File watcher error: {}", e);
            }
        });
        tokio::spawn(async move {
            // ONE task owns every proactive compile, in order: the boot compile
            // runs first, then each watched change. A detached boot compile
            // could finish AFTER a change-driven one and overwrite a fresher
            // build-status.json with a stale verdict — serialization is the
            // whole protocol.
            {
                let site = watch_site.clone();
                let cc = watch_compile_cache.clone();
                let dc = watch_dev_cache.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    spacetime::server::compile_default_entry_with_status(
                        &site, false, cache_stdlib, &cc, &dc,
                    );
                })
                .await;
            }
            while let Ok(change) = file_rx.recv().await {
                // Compile BEFORE the reload is announced: the status file and
                // the warm cache exist whether or not a browser is connected,
                // and the reload lands on fresh output rather than triggering
                // the compile that produces it. A failure still reloads (the
                // page shows the diagnostics overlay) — the status file is the
                // machine-readable record of the same verdict.
                {
                    let site = watch_site.clone();
                    let cc = watch_compile_cache.clone();
                    let dc = watch_dev_cache.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        spacetime::server::compile_default_entry_with_status(
                            &site, false, cache_stdlib, &cc, &dc,
                        );
                    })
                    .await;
                }
                let _ = changes_tx.send(change);
            }
        });
    }

    // Create test runner routes (includes /ws WebSocket endpoint)
    let ws_routes = create_test_runner_routes(sync_state);

    // Create debugger config if debug mode is enabled
    let debugger_config = if debug {
        Some(DebuggerConfig::new())
    } else {
        None
    };

    // Create Spacetime server state for compilation
    let state = AppState {
        mode: AppMode::Dev {
            site_dir: site.clone(),
            trace,
            debug,
        },
        static_dir: None,
        debugger_config,
        cache_stdlib,
        compile_cache,
        dev_compile_cache,
    };

    // Trigger stdlib cache initialization eagerly (before TCP bind)
    // This overlaps stdlib loading/deserialization with browser connection setup
    if cache_stdlib {
        std::thread::spawn(|| {
            // Pre-load both registries in background thread to overlap with browser connection setup
            let _meta_registry = spacetime::compiler::cached_stdlib_registry();
            let _syntax_registry = &*spacetime::syntax::STDLIB_REGISTRY;
        });
    }

    // Create main router with Spacetime handlers (compilation, index injection, etc.)
    let spacetime_router = create_router(state);

    // Quick-access workbench redirect (FUP-110/friction fix): the workbench live
    // surface runs on its OWN ephemeral loopback port (a separate axum
    // Router/AppState from the project's dev router), so `/__spacetime/workbench`
    // here is a thin 302 to that port's `/__mcp/workbench` — which itself
    // idempotently mounts/reuses the ONE workbench instance and redirects again to
    // its `inst-N` page. Net effect for the human: ONE stable bookmark
    // (`http://<dev-host>/__spacetime/workbench`) always lands on a live,
    // ready-to-use workbench, regardless of which ephemeral instance id is
    // current this run.
    let app = if let Some(wb_port) = workbench_port {
        let target = format!("http://127.0.0.1:{wb_port}/__mcp/workbench");
        ws_routes.merge(spacetime_router).route(
            "/__spacetime/workbench",
            get(move || {
                let target = target.clone();
                async move { axum::response::Redirect::to(&target) }
            }),
        )
    } else {
        ws_routes.merge(spacetime_router)
    }
    .fallback_service(ServeDir::new(&site));

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid address");

    println!("\nSpacetime server running at http://{}", addr);
    if debug {
        println!("Debug panel: http://{}/__spacetime/debugger", addr);
    }
    println!("Press Ctrl+C to stop\n");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // A supervising process (beam-lisp's `Spell.Serve` Port, an editor
    // plugin) holds this server's lifetime in its own: when our stdin PIPE
    // closes, the owner is gone and a serve that keeps running is an orphan
    // still holding the port and the file watcher — the next session then
    // silently talks to a stale binary on a stale site. Scoped to pipes: a
    // terminal's stdin never EOFs (Ctrl+C is the gesture there), and a
    // redirected /dev/null under nohup must not read as abandonment.
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        let is_pipe = std::fs::metadata("/dev/stdin")
            .map(|m| m.file_type().is_fifo())
            .unwrap_or(false);
        if is_pipe {
            tokio::task::spawn_blocking(|| {
                use std::io::Read;
                let mut buf = [0u8; 1];
                // Blocks until the owner writes (ignored) or closes (exit).
                while std::io::stdin().read(&mut buf).unwrap_or(0) > 0 {}
                eprintln!("\nserve: stdin closed — the supervising process is gone, exiting");
                std::process::exit(0);
            });
        }
    }

    axum::serve(listener, app).await.unwrap();
}

/// Run Tauri dev server for mobile development
async fn run_tauri_dev(site: &Path, android: bool, ios: bool) {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command as TokioCommand;

    // First, build the .st files
    println!("\x1b[36m▶\x1b[0m Building Spacetime files...");

    // Find all .st files and compile them
    let st_files: Vec<_> = std::fs::read_dir(site)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().map(|ext| ext == "st").unwrap_or(false))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();

    for st_file in &st_files {
        println!("  Compiling {}...", st_file.display());
        let compiled = match Compiler::from_file(st_file, site) {
            Ok(c) => c.compile(),
            Err(e) => {
                eprintln!("\x1b[31m✗\x1b[0m {}", e);
                std::process::exit(1);
            }
        };

        // Write compiled output
        let js_path = site.join("spacetime.js");
        let css_path = site.join("spacetime.css");

        if let Err(e) = std::fs::write(&js_path, &compiled.js) {
            eprintln!(
                "\x1b[31m✗\x1b[0m Failed to write {}: {}",
                js_path.display(),
                e
            );
            std::process::exit(1);
        }

        if let Err(e) = std::fs::write(&css_path, &compiled.css) {
            eprintln!(
                "\x1b[31m✗\x1b[0m Failed to write {}: {}",
                css_path.display(),
                e
            );
            std::process::exit(1);
        }
    }

    println!("\x1b[32m✓\x1b[0m Spacetime build complete\n");

    // Determine which Tauri command to run
    let tauri_command = if android {
        vec!["tauri", "android", "dev"]
    } else if ios {
        vec!["tauri", "ios", "dev"]
    } else {
        vec!["tauri", "dev"]
    };

    let platform_name = if android { "Android" } else { "iOS" };
    println!(
        "\x1b[36m📱\x1b[0m Starting Tauri {} development...",
        platform_name
    );
    println!("  Running: cargo {}", tauri_command.join(" "));
    println!();

    // Spawn cargo tauri dev
    let mut child = match TokioCommand::new("cargo")
        .args(&tauri_command)
        .current_dir(site)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m Failed to start Tauri: {}", e);
            eprintln!("  Make sure you have Tauri CLI installed:");
            eprintln!("  cargo install tauri-cli");
            if android {
                eprintln!("\n  For Android development, also ensure:");
                eprintln!("  - Android SDK is installed");
                eprintln!("  - ANDROID_HOME environment variable is set");
            } else if ios {
                eprintln!("\n  For iOS development, also ensure:");
                eprintln!("  - Xcode is installed");
                eprintln!("  - iOS simulators are available");
            }
            std::process::exit(1);
        }
    };

    // Forward stdout
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                println!("{}", line);
            }
        });
    }

    // Forward stderr
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                eprintln!("{}", line);
            }
        });
    }

    // Wait for the process to complete
    match child.wait().await {
        Ok(status) => {
            if !status.success() {
                eprintln!("\x1b[31m✗\x1b[0m Tauri exited with status: {}", status);
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m Error waiting for Tauri: {}", e);
            std::process::exit(1);
        }
    }
}
/// Runs `spacetime host <args>` by spawning the separate `spacetime-host`
/// CLI binary (commercial/host, a private repo, not distributed with
/// this OSS compiler) and forwarding stdio directly.
///
/// Mirrors `run_tauri_dev`'s external-process SHAPE (spawn + wait +
/// propagate exit code) per `docs/architecture/V1_IMPLEMENTATION_PLAN.org`
/// §5's citation of that function as the reference implementation --
/// but simpler: `run_tauri_dev` interleaves the CHILD's stdout/stderr
/// with our own printed lines (hence its separate line-forwarding
/// tasks), whereas `spacetime host` has nothing else to print
/// concurrently, so `.stdout(Stdio::inherit())`/`.stderr(Stdio::inherit())`
/// hands the child the SAME file descriptors directly -- no buffering,
/// no interleaving logic needed, and interactive prompts (a real
/// concern for `spacetime host login`'s device-flow UX) pass through
/// correctly, which piped stdio would complicate.
async fn run_host(args: Vec<String>) {
    use std::process::Stdio;
    use tokio::process::Command as TokioCommand;

    let binary_path = match which::which("spacetime-host") {
        Ok(path) => path,
        Err(_) => {
            eprintln!("\x1b[31m\u{2717}\x1b[0m `spacetime-host` not found on PATH.");
            eprintln!(
                "  Spacetime Host is a separate binary -- install it and ensure it's on your PATH."
            );
            eprintln!(
                "  (commercial/host is a private repo; see its own README for build/install instructions.)"
            );
            std::process::exit(1);
        }
    };

    let status = match TokioCommand::new(binary_path)
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .status()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\x1b[31m\u{2717}\x1b[0m Failed to run spacetime-host: {e}");
            std::process::exit(1);
        }
    };

    std::process::exit(status.code().unwrap_or(1));
}
/// Resolve the init target directory.
///
/// When `cwd` contains a `projects/` subdirectory, the new site is created at
/// `cwd/projects/<name>/`. Otherwise it is created at `cwd/<name>/`.
/// `metadata().is_dir()` is used so that a symlink pointing at a directory is
/// honored just like a regular directory.
fn resolve_init_target(name: &str, cwd: &Path) -> PathBuf {
    let projects_dir = cwd.join("projects");
    if std::fs::metadata(&projects_dir)
        .map(|m| m.is_dir())
        .unwrap_or(false)
    {
        projects_dir.join(name)
    } else {
        cwd.join(name)
    }
}

/// Initialize a new Spacetime project
fn run_init(name: String, mobile: bool) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project_dir = resolve_init_target(&name, &cwd);

    // Check if directory already exists
    if project_dir.exists() {
        eprintln!(
            "\x1b[31m✗\x1b[0m Directory '{}' already exists",
            project_dir.display()
        );
        std::process::exit(1);
    }

    println!(
        "\x1b[36m▶\x1b[0m Creating Spacetime project: {}",
        project_dir.display()
    );

    // Create project directory
    if let Err(e) = std::fs::create_dir_all(&project_dir) {
        eprintln!("\x1b[31m✗\x1b[0m Failed to create directory: {}", e);
        std::process::exit(1);
    }

    // Generate index.html
    let index_html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{}</title>
  <link rel="stylesheet" href="styles.css">
  <link rel="stylesheet" href="spacetime.css">
</head>
<body>
  <main class="app">
    <h1 class="title">Welcome to {}</h1>
    <p class="subtitle">Built with Spacetime</p>
    <button class="cta-button">Get Started</button>
  </main>

  <script src="spacetime.js"></script>
</body>
</html>
"#,
        name, name
    );

    if let Err(e) = std::fs::write(project_dir.join("index.html"), &index_html) {
        eprintln!("\x1b[31m✗\x1b[0m Failed to write index.html: {}", e);
        std::process::exit(1);
    }
    println!("  \x1b[32m✓\x1b[0m index.html");

    // Generate index.st
    let index_st = r#"// Spacetime Animation Definition
// Learn more at https://spacetime.dev

// Title fade-in on load
.title {
  @on visible title-entrance(600ms, immediate: true) {
    & {
      opacity: 0 -> 1;
      translate-y: 20px -> 0;
      easing: &ease-out-quad;
    }
  }
}

// Subtitle follows title
.subtitle {
  @on visible subtitle-entrance(500ms, delay: 200ms, immediate: true) {
    & {
      opacity: 0 -> 1;
      translate-y: 15px -> 0;
      easing: &ease-out-quad;
    }
  }
}

// Button entrance
.cta-button {
  @on visible button-entrance(400ms, delay: 400ms, immediate: true) {
    & {
      opacity: 0 -> 1;
      scale: 0.9 -> 1;
      easing: &ease-out-back;
    }
  }

  // Button hover effect
  @on hover button-hover(200ms) {
    & {
      scale: 1 -> 1.05;
      box-shadow: 0 4px 12px rgba(0,0,0,0.15) -> 0 8px 24px rgba(0,0,0,0.2);
      easing: &ease-out-quad;
    }
  }
}
"#;

    if let Err(e) = std::fs::write(project_dir.join("index.st"), index_st) {
        eprintln!("\x1b[31m✗\x1b[0m Failed to write index.st: {}", e);
        std::process::exit(1);
    }
    println!("  \x1b[32m✓\x1b[0m index.st");

    // Generate styles.css
    let styles_css = r#"/* Base styles */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.app {
  text-align: center;
  padding: 2rem;
}

.title {
  font-size: 3rem;
  font-weight: 700;
  margin-bottom: 1rem;
  text-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.subtitle {
  font-size: 1.25rem;
  opacity: 0.9;
  margin-bottom: 2rem;
}

.cta-button {
  background: white;
  color: #667eea;
  border: none;
  padding: 1rem 2rem;
  font-size: 1rem;
  font-weight: 600;
  border-radius: 50px;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
  transition: background-color 0.2s;
}

.cta-button:hover {
  background: #f8f9fa;
}

/* Mobile-first responsive */
@media (max-width: 640px) {
  .title {
    font-size: 2rem;
  }

  .subtitle {
    font-size: 1rem;
  }
}
"#;

    if let Err(e) = std::fs::write(project_dir.join("styles.css"), styles_css) {
        eprintln!("\x1b[31m✗\x1b[0m Failed to write styles.css: {}", e);
        std::process::exit(1);
    }
    println!("  \x1b[32m✓\x1b[0m styles.css");

    // If mobile flag is set, create Tauri scaffolding
    if mobile {
        create_tauri_scaffolding(&project_dir, &name);
    }

    println!();
    println!("\x1b[32m✓\x1b[0m Project created successfully!");
    println!();
    println!("Next steps:");
    let cd_target = project_dir
        .strip_prefix(&cwd)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| project_dir.display().to_string());
    println!("  cd {}", cd_target);
    println!("  spacetime serve .");
    if cd_target.starts_with("projects/") {
        println!("  # Or from the Spacetime repo root:");
        println!("  cargo run -- serve {}", cd_target);
    }
    if mobile {
        println!();
        println!("For mobile development:");
        println!("  spacetime serve . --android");
        println!("  spacetime serve . --ios");
    }
}

/// Create Tauri scaffolding for mobile development
fn create_tauri_scaffolding(project_dir: &Path, name: &str) {
    println!();
    println!("\x1b[36m▶\x1b[0m Creating Tauri mobile scaffolding...");

    let tauri_dir = project_dir.join("src-tauri");
    let tauri_src_dir = tauri_dir.join("src");
    let capabilities_dir = tauri_dir.join("capabilities");

    // Create directories
    for dir in &[&tauri_dir, &tauri_src_dir, &capabilities_dir] {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("\x1b[31m✗\x1b[0m Failed to create {}: {}", dir.display(), e);
            std::process::exit(1);
        }
    }

    // Generate Cargo.toml for Tauri
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
description = "A Spacetime app with Tauri"
authors = [""]
edition = "2021"

[build-dependencies]
tauri-build = {{ version = "2", features = [] }}

[dependencies]
tauri = {{ version = "2", features = [] }}
tauri-plugin-haptics = "2"
tauri-plugin-biometric = "2"
tauri-plugin-notification = "2"
tauri-plugin-geolocation = "2"
tauri-plugin-barcode-scanner = "2"
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]
"#,
        name.replace('-', "_")
    );

    if let Err(e) = std::fs::write(tauri_dir.join("Cargo.toml"), &cargo_toml) {
        eprintln!("\x1b[31m✗\x1b[0m Failed to write Cargo.toml: {}", e);
        std::process::exit(1);
    }
    println!("  \x1b[32m✓\x1b[0m src-tauri/Cargo.toml");

    // Generate tauri.conf.json
    let tauri_conf = format!(
        r#"{{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "{}",
  "version": "0.1.0",
  "identifier": "com.spacetime.{}",
  "build": {{
    "frontendDist": ".."
  }},
  "app": {{
    "withGlobalTauri": true,
    "windows": [
      {{
        "title": "{}",
        "width": 800,
        "height": 600,
        "resizable": true,
        "fullscreen": false
      }}
    ],
    "security": {{
      "csp": null
    }}
  }},
  "bundle": {{
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }},
  "plugins": {{
    "haptics": {{}},
    "biometric": {{}},
    "notification": {{}},
    "geolocation": {{}},
    "barcode-scanner": {{}}
  }}
}}
"#,
        name,
        name.replace('-', "_"),
        name
    );

    if let Err(e) = std::fs::write(tauri_dir.join("tauri.conf.json"), &tauri_conf) {
        eprintln!("\x1b[31m✗\x1b[0m Failed to write tauri.conf.json: {}", e);
        std::process::exit(1);
    }
    println!("  \x1b[32m✓\x1b[0m src-tauri/tauri.conf.json");

    // Generate build.rs
    let build_rs = r#"fn main() {
    tauri_build::build()
}
"#;

    if let Err(e) = std::fs::write(tauri_dir.join("build.rs"), build_rs) {
        eprintln!("\x1b[31m✗\x1b[0m Failed to write build.rs: {}", e);
        std::process::exit(1);
    }
    println!("  \x1b[32m✓\x1b[0m src-tauri/build.rs");

    // Generate main.rs
    let main_rs = r#"// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_haptics::init())
        .plugin(tauri_plugin_biometric::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_geolocation::init())
        .plugin(tauri_plugin_barcode_scanner::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
"#;

    if let Err(e) = std::fs::write(tauri_src_dir.join("main.rs"), main_rs) {
        eprintln!("\x1b[31m✗\x1b[0m Failed to write main.rs: {}", e);
        std::process::exit(1);
    }
    println!("  \x1b[32m✓\x1b[0m src-tauri/src/main.rs");

    // Generate capabilities/default.json
    let capabilities_json = r#"{
  "$schema": "https://schema.tauri.app/config/2",
  "identifier": "default",
  "description": "Default capability for the app",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "haptics:allow-vibrate",
    "haptics:allow-impact-feedback",
    "haptics:allow-notification-feedback",
    "haptics:allow-selection-feedback",
    "biometric:allow-authenticate",
    "biometric:allow-status",
    "notification:default",
    "notification:allow-is-permission-granted",
    "notification:allow-request-permission",
    "notification:allow-notify",
    "geolocation:allow-get-current-position",
    "geolocation:allow-watch-position",
    "geolocation:allow-clear-watch",
    "barcode-scanner:allow-scan",
    "barcode-scanner:allow-cancel",
    "barcode-scanner:allow-check-permissions",
    "barcode-scanner:allow-request-permissions"
  ]
}
"#;

    if let Err(e) = std::fs::write(capabilities_dir.join("default.json"), capabilities_json) {
        eprintln!("\x1b[31m✗\x1b[0m Failed to write default.json: {}", e);
        std::process::exit(1);
    }
    println!("  \x1b[32m✓\x1b[0m src-tauri/capabilities/default.json");

    // Copy mobile adapters from stdlib if they exist
    let stdlib_adapters = PathBuf::from("stdlib/mobile/adapters");
    if stdlib_adapters.exists() {
        let adapters_dest = project_dir.join("adapters");
        if let Err(e) = std::fs::create_dir_all(&adapters_dest) {
            eprintln!(
                "\x1b[33m⚠\x1b[0m Could not create adapters directory: {}",
                e
            );
        } else {
            for adapter in &["api.js", "tauri.js", "browser.js"] {
                let src = stdlib_adapters.join(adapter);
                let dest = adapters_dest.join(adapter);
                if src.exists() {
                    if let Err(e) = std::fs::copy(&src, &dest) {
                        eprintln!("\x1b[33m⚠\x1b[0m Could not copy {}: {}", adapter, e);
                    } else {
                        println!("  \x1b[32m✓\x1b[0m adapters/{}", adapter);
                    }
                }
            }
        }
    }

    println!();
    println!("\x1b[36m📱\x1b[0m Tauri mobile scaffolding created");
    println!("  Plugins included: haptics, biometric, notification, geolocation, barcode-scanner");
}

/// Generate tree-sitter grammar from stdlib + extensions
fn run_generate_grammar(include: Vec<String>, output: Option<PathBuf>, verbose: bool) {
    use spacetime::cli::{GenerateGrammarArgs, run_generate_grammar};

    let args = GenerateGrammarArgs {
        include,
        output,
        verbose,
    };

    match run_generate_grammar(args) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m {}", e);
            std::process::exit(1);
        }
    }
}

/// Export a Spell `LanguageProfile` JSON from the live stdlib registry (FEAT-118).
fn run_export_spell_profile(output: Option<PathBuf>) {
    let (registry, _errors) = spacetime::compiler::cached_stdlib_registry();
    match spacetime::cli::export_profile::run_export_spell_profile(output, &registry) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m {}", e);
            std::process::exit(1);
        }
    }
}

/// Legacy CLI behavior for backwards compatibility
async fn run_legacy(args: Args) {
    // Create server state based on mode
    // --site implies using the site's index.html (no need for --dev flag)
    let state = if let Some(site_dir) = args.site.clone() {
        println!("Serving site from: {}", site_dir.display());
        println!("  - Compiles {{name}}.st for {{name}}.html (convention-based)");
        AppState {
            mode: AppMode::Dev {
                site_dir: site_dir.clone(),
                trace: false,
                debug: false,
            },
            static_dir: Some(site_dir.join("static")),
            debugger_config: None,
            cache_stdlib: false,
            compile_cache: std::sync::Arc::new(CompileCache::new()),
            dev_compile_cache: std::sync::Arc::new(DevCompileCache::new()),
        }
    } else {
        // No --site: compile and generate demo HTML
        let ast = if let Some(input_path) = &args.input {
            println!("Loading from: {}", input_path.display());
            let content = std::fs::read_to_string(input_path).expect("Failed to read input file");

            let ast = parse(&content).expect("Failed to parse .st file");

            println!("Parsed:");
            println!("  - {} imports", ast.imports.len());
            println!("  - {} presets", ast.presets.len());
            println!("  - {} scopes", ast.scopes.len());
            ast
        } else {
            spacetime::parser::StFile::default()
        };

        // Compile to JS/CSS
        let compiled = Compiler::from_ast(&ast).compile();
        println!("Spacetime compiled:");
        println!("  - {} bytes CSS", compiled.css.len());
        println!("  - {} bytes JS runtime", compiled.js.len());

        AppState {
            mode: AppMode::Compiled(compiled),
            static_dir: None,
            debugger_config: None,
            cache_stdlib: false,
            compile_cache: std::sync::Arc::new(CompileCache::new()),
            dev_compile_cache: std::sync::Arc::new(DevCompileCache::new()),
        }
    };

    // Build router
    let app = create_router(state);

    // Start server
    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .expect("Invalid address");

    println!("\nSpacetime server running at http://{}", addr);
    println!("Press Ctrl+C to stop\n");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Execute a Spacetime script for source transformations
fn run_script(
    script: PathBuf,
    targets: Vec<PathBuf>,
    dry_run: bool,
    write: bool,
    format: &str,
    interactive: bool,
) {
    #[cfg(feature = "headless")]
    {
        run_script_headless(script, targets, dry_run, write, format, interactive);
    }
    #[cfg(not(feature = "headless"))]
    {
        let _ = interactive; // Suppress unused warning
        eprintln!("\x1b[31m✗\x1b[0m Script mode requires the 'headless' feature");
        eprintln!("  Rebuild with: cargo build --features headless");
        std::process::exit(1);
    }
}

/// Compile and run tests in browser or headlessly via V8
/// Device profile configuration for mobile testing
struct DeviceProfile {
    name: &'static str,
    width: u32,
    height: u32,
    pixel_ratio: f64,
    safe_area_top: u32,
    safe_area_bottom: u32,
    platform: &'static str,
}

const DEVICE_PROFILES: &[DeviceProfile] = &[
    DeviceProfile {
        name: "iPhone SE",
        width: 375,
        height: 667,
        pixel_ratio: 2.0,
        safe_area_top: 20,
        safe_area_bottom: 0,
        platform: "ios",
    },
    DeviceProfile {
        name: "iPhone 15 Pro",
        width: 393,
        height: 852,
        pixel_ratio: 3.0,
        safe_area_top: 59,
        safe_area_bottom: 34,
        platform: "ios",
    },
    DeviceProfile {
        name: "iPhone 15 Pro Max",
        width: 430,
        height: 932,
        pixel_ratio: 3.0,
        safe_area_top: 59,
        safe_area_bottom: 34,
        platform: "ios",
    },
    DeviceProfile {
        name: "Pixel 8",
        width: 412,
        height: 915,
        pixel_ratio: 2.625,
        safe_area_top: 24,
        safe_area_bottom: 0,
        platform: "android",
    },
    DeviceProfile {
        name: "Galaxy S24",
        width: 360,
        height: 780,
        pixel_ratio: 3.0,
        safe_area_top: 25,
        safe_area_bottom: 0,
        platform: "android",
    },
];

fn get_device_profile(name: &str) -> Option<&'static DeviceProfile> {
    DEVICE_PROFILES
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
}

async fn run_test(
    paths: Vec<PathBuf>,
    output: Option<PathBuf>,
    filter: Option<String>,
    serve: Option<u16>,
    headless: bool,
    cdp: bool,
    browser: Option<Vec<String>>,
    format: String,
    rung: String,
    coverage: bool,
    min_directive: Option<f64>,
    live: bool,
    watch: bool,
    mobile: bool,
    device: Option<String>,
) {
    if coverage {
        spacetime::coverage::begin();
    }
    let mut test_files = Vec::new();

    // Collect all test files from paths
    for path in &paths {
        if path.is_dir() {
            // Discover test files in directory
            let discovered = discover_tests(path);
            test_files.extend(discovered);
        } else if path.extension().map(|e| e == "st").unwrap_or(false) {
            test_files.push(path.clone());
        } else {
            eprintln!("\x1b[33m⚠\x1b[0m Skipping non-.st file: {}", path.display());
        }
    }

    if test_files.is_empty() {
        eprintln!("\x1b[31m✗\x1b[0m No test files found");
        std::process::exit(1);
    }

    println!("Found {} test file(s):", test_files.len());
    for f in &test_files {
        println!("  - {}", f.display());
    }

    // Handle mobile testing configuration
    let device_profile = if let Some(ref device_name) = device {
        match get_device_profile(device_name) {
            Some(profile) => {
                println!("\n\x1b[36m📱\x1b[0m Mobile testing mode: {}", profile.name);
                println!(
                    "  Viewport: {}x{} @{}x",
                    profile.width, profile.height, profile.pixel_ratio
                );
                println!("  Platform: {}", profile.platform);
                println!(
                    "  Safe areas: top={}px, bottom={}px",
                    profile.safe_area_top, profile.safe_area_bottom
                );
                Some(profile)
            }
            None => {
                eprintln!("\x1b[31m✗\x1b[0m Unknown device: {}", device_name);
                eprintln!("  Available devices:");
                for p in DEVICE_PROFILES {
                    eprintln!("    - {}", p.name);
                }
                std::process::exit(1);
            }
        }
    } else if mobile {
        // Default to iPhone 15 Pro for mobile mode
        let profile = get_device_profile("iPhone 15 Pro").unwrap();
        println!(
            "\n\x1b[36m📱\x1b[0m Mobile testing mode: {} (default)",
            profile.name
        );
        println!(
            "  Viewport: {}x{} @{}x",
            profile.width, profile.height, profile.pixel_ratio
        );
        println!("  Platform: {}", profile.platform);
        println!("  Use --device to specify a different device");
        Some(profile)
    } else {
        None
    };

    // Generate mobile CSS injection if mobile mode is active
    let _mobile_css_inject = device_profile.map(|p| {
        format!(
            r#"<style>
:root {{
  --st-device-name: "{}";
  --st-viewport-width: {}px;
  --st-viewport-height: {}px;
  --st-device-pixel-ratio: {};
  --st-safe-area-top: {}px;
  --st-safe-area-bottom: {}px;
  --st-safe-area-left: 0px;
  --st-safe-area-right: 0px;
  --st-platform: {};
}}
html, body {{
  width: {}px;
  height: {}px;
  max-width: {}px;
  max-height: {}px;
  overflow: auto;
}}
</style>"#,
            p.name,
            p.width,
            p.height,
            p.pixel_ratio,
            p.safe_area_top,
            p.safe_area_bottom,
            p.platform,
            p.width,
            p.height,
            p.width,
            p.height
        )
    });

    // Live mode: run with 3-pane UI, WebSocket, and file watcher
    if live {
        println!("\n\x1b[36m▶\x1b[0m Starting live test runner...");
        run_live_test_runner(paths, test_files, serve.unwrap_or(3030), watch).await;
        return;
    }

    // CDP mode: run tests in a warm Chromium (real layout/timing/paint rungs).
    if cdp {
        #[cfg(feature = "cdp")]
        {
            if !spacetime::cdp::is_available() {
                eprintln!(
                    "\x1b[31m✗\x1b[0m No Chrome/Chromium found. Set SPACETIME_CHROME or install one."
                );
                std::process::exit(1);
            }
            let start = std::time::Instant::now();
            // cdp::run_test_files uses its own Tokio runtime (block_on); run it on a
            // dedicated thread so it is not nested inside the CLI's async runtime
            // (which would panic with "Cannot start a runtime from within a runtime").
            let files = test_files.clone();
            let filter_owned = filter.clone();
            let outcome = std::thread::spawn(move || {
                let r = spacetime::cdp::run_test_files(&files, filter_owned.as_deref());
                spacetime::cdp::shutdown(); // close the warm Chromium so it doesn't leak
                r
            })
            .join()
            .expect("CDP runner thread panicked");
            match outcome {
                Ok(mut results) => {
                    results.duration_ms = start.elapsed().as_millis() as u64;
                    println!();
                    for err in &results.errors {
                        println!("\x1b[31m✗\x1b[0m {}", err.name);
                        println!("    {}", err.message);
                    }
                    println!("\n{}", results.summary());
                    if results.failed > 0 {
                        std::process::exit(1);
                    }
                    return;
                }
                Err(e) => {
                    eprintln!("\x1b[31m✗\x1b[0m CDP test run failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        #[cfg(not(feature = "cdp"))]
        {
            eprintln!("\x1b[31m✗\x1b[0m CDP mode requires the 'cdp' feature");
            eprintln!("  Rebuild with: cargo build --features cdp");
            std::process::exit(1);
        }
    }

    // Headless mode: run tests using V8 JS engine
    if headless {
        #[cfg(feature = "headless")]
        {
            let filter_owned = filter.clone();
            let format_owned = format.clone();
            let rung_owned = rung.clone();
            std::thread::spawn(move || {
                if coverage {
                    spacetime::coverage::begin();
                }
                run_headless_tests(
                    &test_files,
                    filter_owned.as_deref(),
                    &format_owned,
                    &rung_owned,
                );
                if coverage {
                    report_directive_coverage(&test_files, min_directive);
                }
            })
            .join()
            .expect("Headless test runner thread panicked");
            return;
        }
        #[cfg(not(feature = "headless"))]
        {
            eprintln!("\x1b[31m✗\x1b[0m Headless mode requires the 'headless' feature");
            eprintln!("  Rebuild with: cargo build --features headless");
            std::process::exit(1);
        }
    }

    // Browser parity mode: run tests in real browsers via Playwright
    if let Some(ref browsers) = browser {
        println!("\n\x1b[36m▶\x1b[0m Running browser parity tests...");
        println!("  Browsers: {}", browsers.join(", "));

        // Compile tests to HTML
        let options = TestCompileOptions {
            filter: filter.clone(),
            ..Default::default()
        };

        let html = match compile_test_html_with_options(&test_files, &options) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("\x1b[31m✗\x1b[0m Compilation failed: {}", e);
                std::process::exit(1);
            }
        };

        // Start local server for Playwright
        let port = serve.unwrap_or(3031);
        let addr: SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .expect("Invalid address");

        let html_clone = html.clone();
        let app = axum::Router::new().route(
            "/",
            axum::routing::get(move || {
                let h = html_clone.clone();
                async move { axum::response::Html(h) }
            }),
        );

        // Spawn server in background
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Give server time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let test_url = format!("http://127.0.0.1:{}", port);

        // The legacy multi-browser parity runner (Playwright: firefox/chromium/
        // webkit) was removed in PLAN-027 W2 in favour of the pure-Rust CDP
        // backend (`--cdp`, Chromium-only). Cross-engine parity is tracked as a
        // follow-up; for now redirect users to `--cdp`.
        let _ = (&browsers, &test_url);
        server_handle.abort();
        eprintln!(
            "\x1b[33m⚠\x1b[0m `--browser` (multi-engine Playwright parity) was removed in W2."
        );
        eprintln!("  Use `--cdp` to run tests in a warm Chromium (real layout/timing/paint),");
        eprintln!("  or `--headless` for the fast V8 logic backend.");
        std::process::exit(1);
    }

    // Browser mode: compile to HTML
    let options = TestCompileOptions {
        filter,
        ..Default::default()
    };

    let html = match compile_test_html_with_options(&test_files, &options) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m Compilation failed: {}", e);
            std::process::exit(1);
        }
    };

    // Either serve or write to file
    if let Some(port) = serve {
        println!("\nServing test page at http://127.0.0.1:{}", port);
        println!("Press Ctrl+C to stop\n");

        let addr: SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .expect("Invalid address");

        let html_clone = html.clone();
        let app = axum::Router::new().route(
            "/",
            axum::routing::get(move || {
                let h = html_clone.clone();
                async move { axum::response::Html(h) }
            }),
        );

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    } else if let Some(out_path) = output {
        // Write to file
        match std::fs::write(&out_path, &html) {
            Ok(_) => {
                println!(
                    "\n\x1b[32m✓\x1b[0m Test HTML written to: {}",
                    out_path.display()
                );
                println!("Open in browser to run tests");
            }
            Err(e) => {
                eprintln!("\x1b[31m✗\x1b[0m Failed to write output: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Default: write to temp file and try to open
        let temp_path = std::env::temp_dir().join("spacetime-tests.html");
        match std::fs::write(&temp_path, &html) {
            Ok(_) => {
                println!(
                    "\n\x1b[32m✓\x1b[0m Test HTML written to: {}",
                    temp_path.display()
                );

                // Try to open in browser
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open")
                    .arg(&temp_path)
                    .spawn();

                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&temp_path).spawn();

                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("start")
                    .arg("")
                    .arg(&temp_path)
                    .spawn();

                println!("Opening in browser...");
            }
            Err(e) => {
                eprintln!("\x1b[31m✗\x1b[0m Failed to write temp file: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// Run the live test runner with 3-pane UI, WebSocket, and optional file watcher
async fn run_live_test_runner(
    _paths: Vec<PathBuf>,
    _test_files: Vec<PathBuf>,
    port: u16,
    watch: bool,
) {
    use axum::http::StatusCode;
    use std::sync::Arc;
    use tokio::sync::broadcast;
    use tower_http::services::ServeDir;

    // Create broadcast channel for file changes
    let (file_changes_tx, _file_changes_rx) = broadcast::channel(100);

    // Create compilation cache
    let compilation_cache = Arc::new(std::sync::RwLock::new(None));

    // Test runner directory and entry .st file (convention: index.st)
    let test_runner_dir = PathBuf::from("tests/fixtures/test-runner");
    let index_html = test_runner_dir.join("index.html");
    let index_st = test_runner_dir.join("index.st");

    // Create server state
    let state = Arc::new(
        TestServerState::new(file_changes_tx.clone(), compilation_cache)
            .with_site_dir(test_runner_dir.clone()),
    );

    if !test_runner_dir.exists() || !index_html.exists() {
        eprintln!("\x1b[31m✗\x1b[0m Test runner site not found at tests/fixtures/test-runner/");
        eprintln!("  Expected:");
        eprintln!("    - tests/fixtures/test-runner/index.html");
        eprintln!("    - tests/fixtures/test-runner/index.st");
        std::process::exit(1);
    }

    // Start file watcher if watch mode is enabled
    if watch {
        println!("\x1b[36m👁\x1b[0m  Watch mode enabled - monitoring for file changes");

        let state_clone = state.clone();
        let st_path_clone = index_st.clone();
        let workspace_clone = test_runner_dir.clone();
        let (mut watcher, mut rx) =
            TestWatcher::new(vec![PathBuf::from("tests/fixtures/test-runner")]);

        tokio::spawn(async move {
            if let Err(e) = watcher.watch().await {
                eprintln!("Watcher error: {}", e);
            }
        });

        // Listen for file changes and recompile (any .st file, since imports could change)
        tokio::spawn(async move {
            while let Ok(change) = rx.recv().await {
                if spacetime::literate::is_spacetime_source(&change.path) {
                    println!("\x1b[36m📝\x1b[0m {} changed", change.path.display());
                    recompile_and_cache(state_clone.clone(), &st_path_clone, &workspace_clone)
                        .await;
                }
            }
        });
    }

    // Perform initial compilation
    println!("\x1b[36m▶\x1b[0m  Compiling {}...", index_st.display());
    recompile_and_cache(state.clone(), &index_st, &test_runner_dir).await;

    // Build the router
    let app = axum::Router::new()
        // WebSocket endpoint
        .nest("/ws", create_test_runner_routes(state.clone()))
        // Serve test runner UI files
        .route(
            "/",
            axum::routing::get(|| async {
                match tokio::fs::read_to_string("tests/fixtures/test-runner/index.html").await {
                    Ok(html) => Ok::<_, StatusCode>(axum::response::Html(html)),
                    Err(_) => Err(StatusCode::NOT_FOUND),
                }
            }),
        )
        .nest_service("/assets", ServeDir::new(&test_runner_dir))
        // Serve compiled Spacetime output with state
        .merge(
            axum::Router::new()
                .route(
                    "/__spacetime/styles.css",
                    axum::routing::get(compile_and_serve_css),
                )
                .route(
                    "/__spacetime/runtime.js",
                    axum::routing::get(serve_runtime_js),
                )
                .with_state(state.clone()),
        );

    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("Invalid address");

    println!("\n\x1b[32m✓\x1b[0m Live test runner starting...");
    println!("\x1b[36m▶\x1b[0m  http://127.0.0.1:{}", port);
    println!("\x1b[2m  Press Ctrl+C to stop\x1b[0m\n");

    // Try to open in browser
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open")
        .arg(format!("http://127.0.0.1:{}", port))
        .spawn();

    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg(format!("http://127.0.0.1:{}", port))
        .spawn();

    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("start")
        .arg("")
        .arg(format!("http://127.0.0.1:{}", port))
        .spawn();

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Recompile .st file and update the cache
async fn recompile_and_cache(state: Arc<TestServerState>, st_path: &Path, workspace_root: &Path) {
    let result = match Compiler::from_file(st_path, workspace_root).map(|c| c.compile()) {
        Ok(compiled) => {
            println!(
                "\x1b[32m✓\x1b[0m Compiled {} successfully",
                st_path.display()
            );
            CompilationResult {
                css: compiled.css,
                js: compiled.js,
                error: None,
                timestamp: Instant::now(),
            }
        }
        Err(e) => {
            eprintln!("{}", e); // Print error ONCE here
            CompilationResult {
                css: String::new(),
                js: String::new(),
                error: Some(e),
                timestamp: Instant::now(),
            }
        }
    };

    *state.compilation_cache.write().unwrap() = Some(result);
}

/// Compile animations.st and serve as CSS
async fn compile_and_serve_css(
    State(state): State<Arc<TestServerState>>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    let cache = state.compilation_cache.read().unwrap();

    match cache.as_ref() {
        Some(result) if result.error.is_none() => (
            [(axum::http::header::CONTENT_TYPE, "text/css")],
            result.css.clone(),
        ),
        Some(result) if result.error.is_some() => {
            // Return empty CSS on error - the JS endpoint will show the error modal
            (
                [(axum::http::header::CONTENT_TYPE, "text/css")],
                "/* Compilation error - see error modal */".to_string(),
            )
        }
        _ => (
            [(axum::http::header::CONTENT_TYPE, "text/css")],
            "/* No compilation result */".to_string(),
        ),
    }
}

/// Serve Spacetime runtime JS
async fn serve_runtime_js(
    State(state): State<Arc<TestServerState>>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    let cache = state.compilation_cache.read().unwrap();

    match cache.as_ref() {
        Some(result) if result.error.is_none() => (
            [(axum::http::header::CONTENT_TYPE, "application/javascript")],
            result.js.clone(),
        ),
        Some(result) if result.error.is_some() => {
            // Return JS that shows an error modal instead of failing
            let error_msg = result.error.as_ref().unwrap();
            let escaped_error = error_msg
                .replace('\\', "\\\\")
                .replace('`', "\\`")
                .replace("${", "\\${");
            (
                [(axum::http::header::CONTENT_TYPE, "application/javascript")],
                format!(
                    r#"
// Spacetime Compilation Error
(function() {{
  const errorMsg = `{}`;

  // Create error modal
  const modal = document.createElement('div');
  modal.style.cssText = `
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.85);
    z-index: 99999;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  `;

  const content = document.createElement('div');
  content.style.cssText = `
    background: #1e1e2e;
    border-radius: 12px;
    padding: 24px;
    max-width: 800px;
    max-height: 80vh;
    overflow: auto;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
    border: 1px solid #f38ba8;
  `;

  const header = document.createElement('div');
  header.style.cssText = `
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    padding-bottom: 16px;
    border-bottom: 1px solid #313244;
  `;
  header.innerHTML = `
    <span style="font-size: 24px;">❌</span>
    <span style="color: #f38ba8; font-size: 18px; font-weight: 600;">Compilation Error</span>
  `;

  const body = document.createElement('pre');
  body.style.cssText = `
    color: #cdd6f4;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 13px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
    padding: 16px;
    background: #11111b;
    border-radius: 8px;
  `;
  body.textContent = errorMsg;

  const hint = document.createElement('div');
  hint.style.cssText = `
    color: #a6adc8;
    font-size: 12px;
    margin-top: 16px;
    text-align: center;
  `;
  hint.textContent = 'Fix the error and save to reload';

  content.appendChild(header);
  content.appendChild(body);
  content.appendChild(hint);
  modal.appendChild(content);
  document.body.appendChild(modal);
}})();
"#,
                    escaped_error
                ),
            )
        }
        _ => (
            [(axum::http::header::CONTENT_TYPE, "application/javascript")],
            "console.warn('Spacetime: No compilation result available');".to_string(),
        ),
    }
}

#[cfg(feature = "headless")]
/// LinkeDOM bundle for headless DOM simulation
const LINKEDOM_JS: &str = include_str!("../tests/v8_runtime/linkedom.bundle.js");
/// Web-platform API shims for the headless V8 + LinkeDOM `logic` backend (BUG-129).
const HEADLESS_SHIMS_JS: &str = include_str!("headless_shims.js");
/// The dev-hooks primitive source (`%emit js` body extracted at runtime). Single
/// source of truth: the debug-instrumentation globals (`__stDebugHook` /
/// `__ST_DEBUG__`) are extracted from the dev-hooks primitive by
/// `test_runner::dev_hooks_js` — the ONE shared extractor used by BOTH the V8
/// `logic` backend (here) and the Chromium `--cdp` backend (BUG-129/PLAN-056).
#[cfg(feature = "headless")]
fn dev_hooks_js() -> String {
    spacetime::test_runner::dev_hooks_js()
}

#[cfg(feature = "headless")]
/// Spacetime runtime modules (loaded in order)
const ST_CORE: &str = include_str!("../public/runtime/st.js");
#[cfg(feature = "headless")]
const ST_RAF: &str = include_str!("../public/runtime/raf-coordinator.js");
#[cfg(feature = "headless")]
const ST_EASING: &str = include_str!("../public/runtime/easing.js");
#[cfg(feature = "headless")]
const ST_COLOR: &str = include_str!("../public/runtime/color.js");
#[cfg(feature = "headless")]
const ST_INTERPOLATE: &str = include_str!("../public/runtime/interpolate.js");
#[cfg(feature = "headless")]
const ST_STAGGER: &str = include_str!("../public/runtime/stagger.js");
#[cfg(feature = "headless")]
const ST_VALUE_FUNCTIONS: &str = include_str!("../public/runtime/value-functions.js");
#[cfg(feature = "headless")]
#[cfg(feature = "headless")]
const ST_EDITABLE: &str = include_str!("../public/runtime/editable.js");
#[cfg(feature = "headless")]
const ST_TEMPLATES: &str = include_str!("../public/runtime/templates.js");

#[cfg(feature = "headless")]
/// Testing stdlib (auto-imported for test files)
const TESTING_STDLIB: &str = include_str!("../stdlib/testing/test.st");

#[cfg(feature = "headless")]
/// Generative testing macros (@property / @fuzz).
const TESTING_GENERATIVE: &str = include_str!("../stdlib/testing/generative.st");

#[cfg(feature = "headless")]
/// Animation/timing testing macros (@clock / @record-timeline).
const TESTING_TIMING: &str = include_str!("../stdlib/testing/timing.st");

#[cfg(feature = "headless")]
/// Browser automation macros (@drive / @observe / @capture / @wait-for).
const TESTING_AUTOMATION: &str = include_str!("../stdlib/testing/automation.st");

#[cfg(feature = "headless")]
/// Assertion macros (@then $sig on $sel / @wait_for_signal). The HTML/CDP test
/// path (compile_test_html_with_options) prepends these three modules; the
/// headless path must too, or every assertion written against them silently
/// does not expand here — the BUG-309 vacuity class the zero-assertion rule
/// exists to catch. Kept as separate consts so the two paths cannot drift.
const TESTING_ASSERTIONS_TIMELINE: &str = include_str!("../stdlib/testing/assertions/timeline.st");

#[cfg(feature = "headless")]
const TESTING_ASSERTIONS_STATE: &str = include_str!("../stdlib/testing/assertions/state.st");

#[cfg(feature = "headless")]
const TESTING_ASSERTIONS_BINDING: &str = include_str!("../stdlib/testing/assertions/binding.st");

#[cfg(feature = "headless")]
/// Scripting stdlib (auto-imported for script files)
const SCRIPTING_STDLIB: &str = include_str!("../stdlib/scripting/script.st");

#[cfg(feature = "headless")]
/// Combined runtime modules (concatenated once, eval'd once per context)
/// This is more efficient than 8 sequential eval() calls.
fn get_combined_runtime() -> &'static str {
    use std::sync::OnceLock;
    static COMBINED_RUNTIME: OnceLock<String> = OnceLock::new();
    COMBINED_RUNTIME.get_or_init(|| {
        [
            ST_CORE,
            ST_RAF,
            ST_EASING,
            ST_COLOR,
            ST_INTERPOLATE,
            ST_STAGGER,
            ST_VALUE_FUNCTIONS,
            ST_TEMPLATES,
            ST_EDITABLE,
        ]
        .join("\n")
    })
}

/// Report macro-expansion (directive) coverage after a coverage run (PLAN-027 W6).
/// The exercised set comes from the thread-local coverage sink (populated during
/// compilation); the denominator is the full macro list a real compile sees
/// (stdlib + testing prepend). Exits non-zero if below --min-directive.
#[cfg(feature = "headless")]
fn report_directive_coverage(test_files: &[PathBuf], min_directive: Option<f64>) {
    use spacetime::metasystem::MetaRegistry;
    use spacetime::parser::parse;

    let exercised = match spacetime::coverage::end() {
        Some(sink) => sink.macros,
        None => return,
    };

    // Denominator: every macro a test file actually has access to = the testing
    // prepend + stdlib resolved for one representative test file. Use the testing
    // stdlib + generative + timing sources (the prepend) as the macro universe.
    let mut reg = MetaRegistry::new();
    let sources = [TESTING_STDLIB, TESTING_GENERATIVE, TESTING_TIMING];
    for src in sources {
        if let Ok(ast) = parse(src) {
            let _ = reg.load_from_defs(ast.meta_defs);
        }
    }
    // Also include macros from any imported stdlib in the first test file, so the
    // universe reflects what the suite COULD exercise.
    if let Some(first) = test_files.first()
        && let Ok(text) = std::fs::read_to_string(first)
        && let Ok(ast) = parse(&text)
    {
        let _ = reg.load_from_defs(ast.meta_defs);
    }
    let all_macros: Vec<String> = reg.iter_macros().map(|(n, _)| n.to_string()).collect();

    let report = spacetime::coverage::CoverageReport::compute(&exercised, &all_macros);
    println!("\n{}", report.render_text());

    if let Some(min) = min_directive
        && report.directive_percent < min
    {
        eprintln!(
            "\x1b[31m✗\x1b[0m directive coverage {:.1}% is below --min-directive {:.1}%",
            report.directive_percent, min
        );
        std::process::exit(1);
    }
}

#[cfg(feature = "headless")]
/// Build a FRESH, fully-initialized headless V8 context (BUG-107 isolation).
///
/// Every test file gets its own context so page-global singletons — the `st.js`
/// MutationObserver, `ST._dataRegistry`, `Spacetime.templates`,
/// `ST._selectorInitializers`, and the accumulated `document.body` DOM — can NOT
/// leak across files. A leaked observer firing on a prior file's stale nodes was
/// the `Cannot read properties of null (reading 'nodeType')` class of
/// order-dependent failure. A fresh context eliminates the whole class by
/// construction.
fn new_headless_runtime(backend_rung: &str) -> Runtime {
    let mut runtime = Runtime::new(RuntimeOptions::default()).expect("Failed to create V8 runtime");

    // Console log capture + headless markers.
    let _ = runtime.eval::<serde_json::Value>(
        r#"
        const __consoleLogs = [];
        const __consoleErrors = [];
        globalThis.console = {
            log: (...args) => { __consoleLogs.push(args.join(' ')); },
            error: (...args) => { __consoleErrors.push(args.join(' ')); },
            warn: (...args) => { __consoleLogs.push('[WARN] ' + args.join(' ')); },
            info: (...args) => { __consoleLogs.push(args.join(' ')); },
            debug: () => {},
            trace: () => {},
            dir: () => {},
            table: () => {},
            group: () => {},
            groupEnd: () => {},
            time: () => {},
            timeEnd: () => {},
            clear: () => {},
        };
        globalThis.__spacetime_headless__ = true;
        globalThis.__v8__ = true;
    "#,
    );

    // Fresh LinkeDOM document (fresh `document.body` per file).
    if let Err(e) = runtime.eval::<serde_json::Value>(LINKEDOM_JS) {
        eprintln!("\x1b[31m\u{2717}\x1b[0m Failed to initialize DOM: {}", e);
        std::process::exit(1);
    }

    // DOM mocks for testing.
    let _ = runtime.eval::<serde_json::Value>(
        r#"
        globalThis.requestAnimationFrame = (cb) => { cb(Date.now()); return 0; };
        globalThis.cancelAnimationFrame = () => {};
        // getComputedStyle (I5 / PLAN-137 W5, gh-17): the logic backend can only
        // see INLINE styles. A stylesheet-derived read (a custom property or
        // defaulted value from a <style> rule) is NOT in `el.style`, so the honest
        // value is unknowable here — returning undefined/'' would be a silent
        // false-green. Refuse by rung name with the `--cdp` escape hatch; the error
        // is tagged `__stRungSkip` so the test runner counts it as a SKIP (a
        // deferral to a higher-fidelity backend), not a red.
        globalThis.getComputedStyle = (el) => {
          const style = (el && el.style) || {};
          return new Proxy(style, {
            get(target, prop) {
              if (prop === 'getPropertyValue') {
                return (name) => {
                  const v = target.getPropertyValue ? target.getPropertyValue(name) : undefined;
                  if (v === undefined || v === null) {
                    const err = new Error(
                      `getComputedStyle(${el && el.nodeName || 'el'}).getPropertyValue('${name}') ` +
                      `is stylesheet-derived — it needs 'layout' fidelity but the current backend ` +
                      `tops out at 'logic'. Run with --cdp instead of faking 'layout'`
                    );
                    err.__stRungSkip = true;
                    err.__stRungNeed = 'layout';
                    throw err;
                  }
                  return v;
                };
              }
              return target[prop];
            },
          });
        };
        if (!globalThis.performance) {
            globalThis.performance = { now: () => Date.now() };
        }

        // Mock fetch
        const __fetchMocks = new Map();
        globalThis.fetch = async (url) => {
            const data = __fetchMocks.get(url);
            if (data) return { ok: true, json: async () => data };
            throw new Error('No mock for: ' + url);
        };
    "#,
    );

    // Web-platform API shims (BUG-129): DOMParser, getSelection, IntersectionObserver,
    // Element.animate, focus/activeElement — minimal logic-level surface so tests
    // that exercise these APIs run on the fast backend instead of erroring.
    let _ = runtime.eval::<serde_json::Value>(HEADLESS_SHIMS_JS);

    // Debug-instrumentation globals (__stDebugHook / __ST_DEBUG__) — the dev tier
    // is macro-mounted and never loads in the bare test V8, so seed it from the
    // primitive's own JS (single source of truth) for the dev-runtime tests.
    let _ = runtime.eval::<serde_json::Value>(dev_hooks_js());

    // Load all ST runtime modules (st.js installs its MutationObserver here —
    // now scoped to THIS file's context only).
    if let Err(e) = runtime.eval::<serde_json::Value>(get_combined_runtime()) {
        eprintln!("\x1b[31m\u{2717}\x1b[0m Failed to load runtime: {}", e);
        std::process::exit(1);
    }

    // Fidelity ladder (PLAN-027 W1): advertise this backend's ceiling.
    {
        let backend =
            spacetime::rung::Rung::parse(backend_rung).unwrap_or(spacetime::rung::Rung::Logic);
        let _ = runtime
            .eval::<serde_json::Value>(&format!("ST._rung.setBackend('{}');", backend.as_str()));
    }

    // Generative testing helpers (@property / @fuzz).
    {
        const GENERATIVE_RUNTIME: &str = include_str!("../public/runtime/generative.js");
        let _ = runtime.eval::<serde_json::Value>(GENERATIVE_RUNTIME);
    }

    // Test registration harness.
    let _ = runtime.eval::<serde_json::Value>(
        r#"
        const __st_tests = [];
        window.__spacetime_register_test = (name, fn, options = {}) => {
            __st_tests.push({ name, fn, ...options });
        };
        window.__st_tests = __st_tests;
        // BUG-309 vacuity detection: assertion macros note their execution; a
        // test whose claims were silently dropped runs with counter at 0 and is
        // failed by the runner (see run_headless_tests' run loop).
        // Defined on globalThis, NOT on `window`: this runtime's global object is
        // globalThis, and LinkeDOM later supplies `window` as a SEPARATE object.
        // An assertion macro's emitted call is a bare `__st_noteAssertion()`, which
        // resolves against globalThis — assigning only to `window` left it
        // unreachable and every assertion threw `is not a function`.
        globalThis.__st_currentAssertions = 0;
        globalThis.__st_allowZeroAssertions = false;
        globalThis.__st_noteAssertion = () => { globalThis.__st_currentAssertions++; };
        globalThis.__st_markNoAssert = () => { globalThis.__st_allowZeroAssertions = true; };
        // The assertion macros emit the `window.`-qualified call (the CDP backend's
        // real global). LinkeDOM's `window` is a different object here, so mirror
        // the two functions onto it: one counter, reachable by both spellings, so
        // the headless and CDP backends cannot disagree about what ran.
        if (globalThis.window && globalThis.window !== globalThis) {
            globalThis.window.__st_noteAssertion = globalThis.__st_noteAssertion;
            globalThis.window.__st_markNoAssert = globalThis.__st_markNoAssert;
        }
    "#,
    );

    // Test HELPERS (state machine + __obj + __st_dispatch). These live ONLY in
    // the HTML/CDP path's `generate_test_runtime` historically, so headless tests
    // calling `__obj(...)`, `__st_register_machine`, `__st_dispatch`, or the
    // `SpacetimeStateMachine` class failed with `X is not defined` (the
    // counter/form/modal `is not defined` cluster, surfaced honestly by the
    // BUG-107 isolation fix). Inject the SAME helpers here so both backends agree.
    let _ = runtime.eval::<serde_json::Value>(
        r#"
        class SpacetimeStateMachine {
          constructor(element, config) {
            this.element = element;
            this.state = config.initial || 'idle';
            this.transitions = config.transitions || [];
            this.states = config.states || {};
            this.element.dataset.stState = this.state;
            this._boundHandlers = [];
            for (const t of this.transitions) {
              if (t.on) {
                const handler = (e) => { if (this.state === t.from) { this._transitionTo(t.to); } };
                this._boundHandlers.push({ event: t.on, handler });
                this.element.addEventListener(t.on, handler);
              }
            }
          }
          _transitionTo(newState) {
            const oldState = this.state;
            this.state = newState;
            this.element.dataset.stState = newState;
            if (this.states[newState]) { Object.assign(this.element.style, this.states[newState]); }
            this.element.dispatchEvent(new CustomEvent('st:statechange', { detail: { from: oldState, to: newState } }));
          }
          destroy() {
            for (const { event, handler } of this._boundHandlers) { this.element.removeEventListener(event, handler); }
            this._boundHandlers = [];
          }
        }
        globalThis.SpacetimeStateMachine = SpacetimeStateMachine;
        window.__st_machines = window.__st_machines || new Map();
        window.__st_register_machine = (element, config) => {
          if (window.__st_machines.has(element)) { window.__st_machines.get(element).destroy(); }
          const machine = new SpacetimeStateMachine(element, config);
          window.__st_machines.set(element, machine);
          return machine;
        };
        window.__st_dispatch = (element, eventName) => {
          element.dispatchEvent(new CustomEvent(eventName, { bubbles: true }));
        };
        window.__obj = (...args) => {
          const o = {};
          for (let i = 0; i < args.length; i += 2) { o[args[i]] = args[i + 1]; }
          return o;
        };
        globalThis.__obj = window.__obj;
    "#,
    );

    runtime
}
fn run_headless_tests(
    test_files: &[PathBuf],
    filter: Option<&str>,
    format: &str,
    backend_rung: &str,
) {
    use std::time::Instant;
    let start = Instant::now();

    println!("\n\x1b[36mRunning tests headlessly via V8...\x1b[0m\n");

    // Initialize the V8 platform once (process-global); contexts are per-file.
    spacetime::ensure_v8_initialized();

    // Vendored stdlib blobs (e.g. pretext for @value-change) are demand-injected
    // per file below — the headless path compiles each file standalone (it does
    // NOT go through compile_test_html_with_options), so without this the driver
    // code references a vendor global that never arrives ("pretext is not
    // defined" — the value-change headless failures triaged 2026-07-19).
    let vendor_registry = spacetime::vendor::stdlib_test_vendor_registry();

    // Aggregate results across every file.
    let mut total_passed: u64 = 0;
    let mut total_failed: u64 = 0;
    let mut total_skipped: u64 = 0;
    let mut total_total: u64 = 0;
    let mut all_errors: Vec<serde_json::Value> = Vec::new();
    let mut all_logs: Vec<String> = Vec::new();

    // BUG-107: run EACH file in its OWN fresh V8 context. Page-global singletons
    // (the st.js MutationObserver, ST._dataRegistry, Spacetime.templates,
    // ST._selectorInitializers, document.body DOM) therefore cannot leak across
    // files, eliminating the order-dependent `null nodeType` cross-contamination
    // by construction.
    for file in test_files {
        let mut runtime = new_headless_runtime(backend_rung);

        // Read + compile this single test file (with testing stdlib prepended).
        let test_source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "\x1b[31m\u{2717}\x1b[0m Failed to read {}: {}",
                    file.display(),
                    e
                );
                std::process::exit(1);
            }
        };

        let combined_source = format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n\n// === Test File: {} ===\n\n{}",
            TESTING_STDLIB,
            TESTING_ASSERTIONS_TIMELINE,
            TESTING_ASSERTIONS_STATE,
            TESTING_ASSERTIONS_BINDING,
            TESTING_GENERATIVE,
            TESTING_TIMING,
            TESTING_AUTOMATION,
            file.display(),
            test_source
        );

        let mut ast = match parse(&combined_source) {
            Ok(a) => a,
            Err(e) => {
                eprintln!(
                    "\x1b[31m\u{2717}\x1b[0m Parse error in {}: {:?}",
                    file.display(),
                    e
                );
                std::process::exit(1);
            }
        };

        // Resolve `@import` so a .test.st can pull in a modular stdlib MODULE
        // (e.g. `@import "stdlib/dnd"` / `"stdlib/text"`). Module defs ride the
        // import path (merge_ast -> user meta_defs), unlike the flat
        // `stdlib/macros/*` which the registry auto-scans. Without this, a module
        // macro never expands here (no prelude, no primitive emit) even though the
        // CDP/HTML path (compile_test_html) already resolves imports. The testing
        // stdlib is ALREADY prepended into combined_source, so skip re-resolving
        // `stdlib/testing/*` (that would disturb the prepend-based expansion).
        let needs_resolution = ast
            .imports
            .iter()
            .any(|i| !i.path.starts_with("stdlib/testing"));
        if needs_resolution {
            let workspace_root =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            match spacetime::parser::resolve_imports(&ast, file, &workspace_root) {
                Ok(resolved) => {
                    ast = resolved;
                    if !ast.meta_defs.is_empty() {
                        let main_sf = file.to_string_lossy().to_string();
                        spacetime::parser::rematch_with_user_macros_in(
                            &mut ast,
                            &combined_source,
                            Some(&main_sf),
                        );
                    }
                }
                Err(e) => {
                    eprintln!(
                        "\x1b[31m\u{2717}\x1b[0m Import resolution failed in {}: {}",
                        file.display(),
                        e
                    );
                    std::process::exit(1);
                }
            }
        }

        let compiled = Compiler::from_ast(&ast).compile();
        if !compiled.pipeline_errors.is_empty() {
            eprintln!(
                "\x1b[33m\u{26a0}\x1b[0m Compile warnings in {}:",
                file.display()
            );
            for err in &compiled.pipeline_errors {
                eprintln!("  - {}: {}", err.code, err.message);
            }
        }

        // Demand-inject vendored blobs this file's emitted JS references (e.g.
        // pretext), same rail as the site + CDP/HTML test paths.
        let file_js = spacetime::vendor::inject_vendor_preludes(&compiled.js, &vendor_registry);

        // Load this file's compiled JS into its fresh context.
        if !file_js.is_empty()
            && let Err(e) = runtime.eval::<serde_json::Value>(&file_js)
        {
            eprintln!(
                "\x1b[31m\u{2717}\x1b[0m JS error in {}: {}",
                file.display(),
                format!("{}", e).chars().take(200).collect::<String>()
            );
        }

        // Run the file's tests.
        let filter_str = filter
            .map(|f| format!("\"{}\"", f))
            .unwrap_or("null".to_string());
        let run_code = format!(
            r#"
        (async () => {{
            let passed = 0, failed = 0, skipped = 0;
            const errors = [];
            const filter = {};

            const tests = filter
                ? __st_tests.filter(t => t.name.includes(filter))
                : __st_tests;

            for (const test of tests) {{
                if (test.skip) {{
                    skipped++;
                    console.log('SKIP: ' + test.name);
                    continue;
                }}

                try {{
                    // Tests are async () => {{}} — AWAIT so a thrown/rejected
                    // assertion actually fails the test. Without the await the
                    // promise's rejection is dropped and every async test
                    // "passes" vacuously (the historical gate-breaker).
                    __st_currentAssertions = 0;
                    __st_allowZeroAssertions = false;
                    await test.fn();
                    // BUG-309: a test that ran but made no assertions verified
                    // nothing — fail it rather than pass it. Skipped tests never
                    // reach here; a real assertion (or the @no_assert opt-out)
                    // moves the counter/flag off their defaults.
                    if (__st_currentAssertions === 0 && !__st_allowZeroAssertions) {{
                        throw new Error('Test made no assertions — its claims did not compile to anything (e.g. a directive whose shape matched no macro was silently dropped). Every @test must execute at least one assertion; if this test is intentionally compile-only, mark its body with @no_assert.');
                    }}
                    passed++;
                    console.log('PASS: ' + test.name);
                }} catch (e) {{
                    // A fidelity refusal (ST._rung.require on a backend that
                    // can't provide the needed rung) is a DEFERRAL to a
                    // higher-fidelity backend (CDP), not a failure. Count it as
                    // skipped so a logic-rung run reports 0 failures while still
                    // surfacing the deferral; the same test runs on --cdp.
                    if (e && e.__stRungSkip) {{
                        skipped++;
                        console.log("SKIP (needs '" + e.__stRungNeed + "' fidelity): " + test.name);
                    }} else {{
                        failed++;
                        errors.push({{ name: test.name, error: e.message, stack: e.stack }});
                        console.error('FAIL: ' + test.name);
                        console.error('  ' + e.message);
                    }}
                }}
            }}

            return JSON.stringify({{
                passed: passed,
                failed: failed,
                skipped: skipped,
                total: tests.length,
                errors: errors
            }});
        }})()
    "#,
            filter_str
        );

        let result_str: String = match runtime.eval(&run_code) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "\x1b[31m\u{2717}\x1b[0m Test runner error in {}: {}",
                    file.display(),
                    e
                );
                std::process::exit(1);
            }
        };

        let results: serde_json::Value = serde_json::from_str(&result_str).unwrap_or_default();
        total_passed += results["passed"].as_u64().unwrap_or(0);
        total_failed += results["failed"].as_u64().unwrap_or(0);
        total_skipped += results["skipped"].as_u64().unwrap_or(0);
        total_total += results["total"].as_u64().unwrap_or(0);
        if let Some(errs) = results["errors"].as_array() {
            all_errors.extend(errs.iter().cloned());
        }

        // Drain this context's console buffer into the aggregate.
        let logs: Vec<String> = runtime
            .eval("JSON.stringify(__consoleLogs)")
            .ok()
            .and_then(|s: String| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        all_logs.extend(logs);
    }

    let duration = start.elapsed();

    let passed = total_passed;
    let failed = total_failed;
    let skipped = total_skipped;
    let total = total_total;
    let errors_json = serde_json::Value::Array(all_errors);
    let logs = all_logs;

    // Output based on format
    match format {
        "json" => {
            let output = serde_json::json!({
                "passed": passed,
                "failed": failed,
                "skipped": skipped,
                "total": total,
                "duration_ms": duration.as_millis(),
                "errors": errors_json,
                "logs": logs
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        "junit" => {
            println!(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
            println!(
                r#"<testsuite name="spacetime" tests="{}" failures="{}" skipped="{}" time="{:.3}">"#,
                total,
                failed,
                skipped,
                duration.as_secs_f64()
            );
            if let Some(errors) = errors_json.as_array() {
                for err in errors {
                    let name = err["name"].as_str().unwrap_or("unknown");
                    let message = err["error"].as_str().unwrap_or("");
                    println!(r#"  <testcase name="{}">"#, name);
                    println!(
                        r#"    <failure message="{}">{}</failure>"#,
                        message, message
                    );
                    println!(r#"  </testcase>"#);
                }
            }
            println!("</testsuite>");
        }
        _ => {
            println!();

            for log in &logs {
                if log.starts_with("PASS:") {
                    println!("\x1b[32m\u{2713}\x1b[0m {}", &log[6..]);
                } else if log.starts_with("FAIL:") {
                    println!("\x1b[31m\u{2717}\x1b[0m {}", &log[6..]);
                } else if log.starts_with("SKIP:") {
                    println!("\x1b[33m\u{25cb}\x1b[0m {}", &log[6..]);
                } else if !log.starts_with("  ") {
                }
            }

            if let Some(errors) = errors_json.as_array()
                && !errors.is_empty()
            {
                println!("\n\x1b[31mErrors:\x1b[0m");
                for err in errors {
                    let name = err["name"].as_str().unwrap_or("unknown");
                    let message = err["error"].as_str().unwrap_or("");
                    println!("\n  \x1b[31m{}\x1b[0m", name);
                    println!("    {}", message);
                }
            }

            println!();
            if failed > 0 {
                println!(
                    "\x1b[31m{} passed, {} failed, {} skipped ({:.2}s)\x1b[0m",
                    passed,
                    failed,
                    skipped,
                    duration.as_secs_f64()
                );
            } else {
                println!(
                    "\x1b[32m{} passed, {} failed, {} skipped ({:.2}s)\x1b[0m",
                    passed,
                    failed,
                    skipped,
                    duration.as_secs_f64()
                );
            }
        }
    }

    if failed > 0 {
        std::process::exit(1);
    }
}

#[cfg(feature = "headless")]
/// Run script headlessly using V8 via rustyscript
fn run_script_headless(
    script: PathBuf,
    targets: Vec<PathBuf>,
    dry_run: bool,
    write: bool,
    format: &str,
    interactive: bool,
) {
    use std::io::{self, Write as IoWrite};
    use std::time::Instant;
    let start = Instant::now();

    if interactive {
        println!("\n\x1b[1;35m┌──────────────────────────────────────────────────┐\x1b[0m");
        println!(
            "\x1b[1;35m│\x1b[0m  \x1b[1;36mSpacetime Script\x1b[0m - Interactive Mode             \x1b[1;35m│\x1b[0m"
        );
        println!(
            "\x1b[1;35m│\x1b[0m  Review and approve changes file by file         \x1b[1;35m│\x1b[0m"
        );
        println!("\x1b[1;35m└──────────────────────────────────────────────────┘\x1b[0m\n");
        println!("\x1b[90mScript:\x1b[0m {}", script.display());
    } else {
        println!("\n\x1b[36mRunning script: {}\x1b[0m\n", script.display());
    }

    let script_source = match std::fs::read_to_string(&script) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "\x1b[31m✗\x1b[0m Failed to read script {}: {}",
                script.display(),
                e
            );
            std::process::exit(1);
        }
    };

    let mut target_files = Vec::new();
    for target in &targets {
        if target.is_dir() {
            discover_st_files(target, &mut target_files);
        } else if target.extension().map(|e| e == "st").unwrap_or(false) {
            target_files.push(target.clone());
        } else {
            eprintln!(
                "\x1b[33m⚠\x1b[0m Skipping non-.st file: {}",
                target.display()
            );
        }
    }

    if target_files.is_empty() {
        eprintln!("\x1b[31m✗\x1b[0m No .st files found in targets");
        std::process::exit(1);
    }

    println!("Found {} target file(s):\n", target_files.len());

    let mut results = Vec::new();
    let mut total_changes = 0;

    for target_file in &target_files {
        let target_source = match std::fs::read_to_string(target_file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "\x1b[31m✗\x1b[0m Failed to read {}: {}",
                    target_file.display(),
                    e
                );
                continue;
            }
        };

        let target_ast = match parse(&target_source) {
            Ok(a) => a,
            Err(e) => {
                eprintln!(
                    "\x1b[31m✗\x1b[0m Parse error in {}: {:?}",
                    target_file.display(),
                    e
                );
                continue;
            }
        };

        let ast_json = match serde_json::to_string(&target_ast) {
            Ok(j) => j,
            Err(e) => {
                eprintln!(
                    "\x1b[31m✗\x1b[0m Failed to serialize AST for {}: {}",
                    target_file.display(),
                    e
                );
                continue;
            }
        };

        spacetime::ensure_v8_initialized();
        let mut runtime = match Runtime::new(RuntimeOptions::default()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("\x1b[31m✗\x1b[0m Failed to create V8 runtime: {}", e);
                std::process::exit(1);
            }
        };

        let _ = runtime.eval::<serde_json::Value>(
            r#"
            const __consoleLogs = [];
            const __consoleErrors = [];
            globalThis.console = {
                log: (...args) => { __consoleLogs.push(args.join(' ')); },
                error: (...args) => { __consoleErrors.push(args.join(' ')); },
                warn: (...args) => { __consoleLogs.push('[WARN] ' + args.join(' ')); },
                info: (...args) => { __consoleLogs.push(args.join(' ')); },
                debug: () => {},
                trace: () => {},
                dir: () => {},
                table: () => {},
                group: () => {},
                groupEnd: () => {},
                time: () => {},
                timeEnd: () => {},
                clear: () => {},
            };
            globalThis.__spacetime_headless__ = true;
            globalThis.__v8__ = true;
        "#,
        );

        if let Err(e) = runtime.eval::<serde_json::Value>(LINKEDOM_JS) {
            eprintln!("\x1b[31m✗\x1b[0m Failed to initialize DOM: {}", e);
            std::process::exit(1);
        }

        let _ = runtime.eval::<serde_json::Value>(
            r#"
            globalThis.requestAnimationFrame = (cb) => { cb(Date.now()); return 0; };
            globalThis.cancelAnimationFrame = () => {};
            // getComputedStyle (I5 / PLAN-137 W5, gh-17): the logic backend can
            // only see INLINE styles. A stylesheet-derived read (a custom property
            // or defaulted value coming from a <style> rule) is NOT in `el.style`,
            // so the honest value is unknowable here — returning undefined/'' would
            // be a silent false-green. Refuse by rung name with the `--cdp` escape
            // hatch; the error is tagged `__stRungSkip` so the test runner counts it
            // as a SKIP (a deferral to a higher-fidelity backend), not a red.
            globalThis.getComputedStyle = (el) => {
              const style = (el && el.style) || {};
              return new Proxy(style, {
                get(target, prop) {
                  if (prop === 'getPropertyValue') {
                    return (name) => {
                      const v = target.getPropertyValue ? target.getPropertyValue(name) : undefined;
                      if (v === undefined || v === null) {
                        const err = new Error(
                          `getComputedStyle(${el && el.nodeName || 'el'}).getPropertyValue('${name}') ` +
                          `is stylesheet-derived — it needs 'layout' fidelity but the current backend ` +
                          `tops out at 'logic'. Run with --cdp instead of faking 'layout'`
                        );
                        err.__stRungSkip = true;
                        err.__stRungNeed = 'layout';
                        throw err;
                      }
                      return v;
                    };
                  }
                  return target[prop];
                },
              });
            };
            if (!globalThis.performance) {
                globalThis.performance = { now: () => Date.now() };
            }
        "#,
        );

        // Web-platform API shims (BUG-129) — same surface as the test path.
        let _ = runtime.eval::<serde_json::Value>(HEADLESS_SHIMS_JS);
        let _ = runtime.eval::<serde_json::Value>(dev_hooks_js());

        if let Err(e) = runtime.eval::<serde_json::Value>(get_combined_runtime()) {
            eprintln!("\x1b[31m✗\x1b[0m Failed to load runtime: {}", e);
            std::process::exit(1);
        }

        let file_context = format!(
            r#"
            globalThis.__st_file_source = {};
            globalThis.__st_file_ast = {};
            globalThis.__st_file_path = {};
            "#,
            serde_json::to_string(&target_source).unwrap(),
            ast_json,
            serde_json::to_string(&target_file.display().to_string()).unwrap()
        );

        if let Err(e) = runtime.eval::<serde_json::Value>(&file_context) {
            eprintln!("\x1b[31m✗\x1b[0m Failed to set up file context: {}", e);
            continue;
        }

        let script_infra = r#"
            globalThis.__st_scripts = [];
            globalThis.__st_edits = [];
            globalThis.__spacetime_register_script = (name, fn) => {
                globalThis.__st_scripts.push({ name, fn });
            };
            globalThis.__spacetime_run_scripts = async () => {
                const results = [];
                for (const script of globalThis.__st_scripts) {
                    try {
                        const result = await script.fn();
                        results.push({ name: script.name, success: true, result });
                        console.log(`PASS: ${script.name}`);
                    } catch (e) {
                        results.push({ name: script.name, success: false, error: e.message });
                        console.error(`FAIL: ${script.name}: ${e.message}`);
                    }
                }
                return results;
            };
            globalThis.__st_add_edit = (start, end, replacement) => {
                globalThis.__st_edits.push({ start, end, replacement });
            };
            globalThis.__st_get_edits = () => [...globalThis.__st_edits];
            globalThis.__st_apply_edits = (source) => {
                const sorted = [...globalThis.__st_edits].sort((a, b) => b.start - a.start);
                let result = source;
                for (const edit of sorted) {
                    result = result.slice(0, edit.start) + edit.replacement + result.slice(edit.end);
                }
                return result;
            };
        "#;

        if let Err(e) = runtime.eval::<serde_json::Value>(script_infra) {
            eprintln!(
                "\x1b[31m✗\x1b[0m Failed to set up scripting infrastructure: {}",
                e
            );
            continue;
        }

        let combined_script = format!(
            "{}\n\n// === Script: {} ===\n\n{}",
            SCRIPTING_STDLIB,
            script.display(),
            script_source
        );

        let script_ast = match parse(&combined_script) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("\x1b[31m✗\x1b[0m Parse error in script: {:?}", e);
                std::process::exit(1);
            }
        };

        if std::env::var("ST_DEBUG").is_ok() {
            eprintln!("[debug] AST scopes count: {}", script_ast.scopes.len());
            let file_level: Vec<_> = script_ast
                .matches
                .iter()
                .filter(|m| m.selector.is_none())
                .collect();
            eprintln!("[debug] File-level matches: {}", file_level.len());
            for fm in &file_level {
                eprintln!(
                    "[debug]   file-level match: macro_name={}, captures={}",
                    fm.macro_name,
                    fm.captures.len()
                );
            }
        }

        let compiled = Compiler::from_ast(&script_ast).compile();

        if !compiled.js.is_empty()
            && let Err(e) = runtime.eval::<serde_json::Value>(&compiled.js)
        {
            eprintln!("\x1b[31m✗\x1b[0m JS error in runtime: {}", e);
            std::process::exit(1);
        }

        let run_code = r#"
            (function() {
                const results = [];
                for (const script of globalThis.__st_scripts) {
                    try {
                        const result = script.fn();
                        results.push({ name: script.name, success: true, result });
                    } catch (e) {
                        results.push({ name: script.name, success: false, error: e.message || String(e) });
                    }
                }
                const transformed = globalThis.__st_apply_edits ?
                    globalThis.__st_apply_edits(globalThis.__st_file_source) :
                    globalThis.__st_file_source;
                const edits = globalThis.__st_get_edits ? globalThis.__st_get_edits() : [];
                return JSON.stringify({
                    results: results,
                    transformed: transformed,
                    edits: edits,
                    hasChanges: edits.length > 0
                });
            })()
        "#;

        let result_str: String = match runtime.eval(run_code) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "\x1b[31m✗\x1b[0m Script execution error for {}: {}",
                    target_file.display(),
                    e
                );
                continue;
            }
        };

        if std::env::var("ST_DEBUG").is_ok() {
            eprintln!("[debug] Raw result: {}", &result_str);
        }

        let script_result: serde_json::Value =
            serde_json::from_str(&result_str).unwrap_or_default();
        let transformed = script_result["transformed"]
            .as_str()
            .unwrap_or(&target_source)
            .to_string();
        let has_changes = script_result["hasChanges"].as_bool().unwrap_or(false);

        let logs: Vec<String> = runtime
            .eval::<String>("JSON.stringify(__consoleLogs)")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        // Store result or handle interactively
        if has_changes {
            if interactive {
                println!("\n\x1b[1;33m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
                println!("\x1b[1;36m  File:\x1b[0m {}", target_file.display());
                println!("\x1b[1;33m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m\n");

                let diff = generate_diff(
                    &target_source,
                    &transformed,
                    &target_file.display().to_string(),
                );
                println!("{}", diff);

                println!();
                println!(
                    "\x1b[1;35m  [y]\x1b[0m Apply changes   \x1b[1;35m[n]\x1b[0m Skip   \x1b[1;35m[q]\x1b[0m Quit   \x1b[1;35m[d]\x1b[0m Show full diff"
                );
                print!("\n  \x1b[1;36m→\x1b[0m Your choice: ");
                io::stdout().flush().unwrap();

                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                let choice = input.trim().to_lowercase();

                match choice.as_str() {
                    "y" | "yes" | "" => {
                        total_changes += 1;
                        results.push((
                            target_file.clone(),
                            target_source.clone(),
                            transformed,
                            logs,
                        ));
                        println!(
                            "    \x1b[32m✓\x1b[0m Changes staged for {}",
                            target_file.display()
                        );
                    }
                    "n" | "no" | "s" | "skip" => {
                        println!("    \x1b[33m○\x1b[0m Skipped {}", target_file.display());
                    }
                    "q" | "quit" | "exit" => {
                        println!("\n\x1b[33mExiting interactive mode.\x1b[0m");
                        break;
                    }
                    "d" | "diff" => {
                        println!("\n\x1b[1;90m--- Original ---\x1b[0m");
                        for (i, line) in target_source.lines().enumerate() {
                            println!("\x1b[90m{:4}\x1b[0m {}", i + 1, line);
                        }
                        println!("\n\x1b[1;90m--- Transformed ---\x1b[0m");
                        for (i, line) in transformed.lines().enumerate() {
                            println!("\x1b[90m{:4}\x1b[0m {}", i + 1, line);
                        }

                        print!("\n  \x1b[1;36m→\x1b[0m Apply these changes? [y/n]: ");
                        io::stdout().flush().unwrap();
                        let mut input2 = String::new();
                        io::stdin().read_line(&mut input2).unwrap();
                        if input2.trim().to_lowercase().starts_with('y') {
                            total_changes += 1;
                            results.push((
                                target_file.clone(),
                                target_source.clone(),
                                transformed,
                                logs,
                            ));
                            println!("    \x1b[32m✓\x1b[0m Changes staged");
                        } else {
                            println!("    \x1b[33m○\x1b[0m Skipped");
                        }
                    }
                    _ => {
                        println!("    \x1b[33m○\x1b[0m Unknown input, skipping");
                    }
                }
            } else {
                total_changes += 1;
                results.push((
                    target_file.clone(),
                    target_source.clone(),
                    transformed,
                    logs,
                ));
            }
        } else if interactive {
            println!(
                "  \x1b[90m○\x1b[0m {} \x1b[90m(no changes)\x1b[0m",
                target_file.display()
            );
        } else {
            println!("  \x1b[33m○\x1b[0m {} - No changes", target_file.display());
        }
    }

    let duration = start.elapsed();

    match format {
        "json" => {
            let json_results: Vec<serde_json::Value> = results
                .iter()
                .map(|(path, original, transformed, _logs)| {
                    serde_json::json!({
                        "file": path.display().to_string(),
                        "original": original,
                        "transformed": transformed,
                        "diff": generate_diff(original, transformed, &path.display().to_string())
                    })
                })
                .collect();

            let output = serde_json::json!({
                "files_processed": target_files.len(),
                "files_changed": total_changes,
                "duration_ms": duration.as_millis(),
                "results": json_results
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        "diff" => {
            println!();
            for (path, original, transformed, _logs) in &results {
                let diff = generate_diff(original, transformed, &path.display().to_string());
                println!("{}", diff);
                println!();
            }
        }
        _ => {
            println!();
            for (path, original, transformed, logs) in &results {
                println!("\x1b[32m✓\x1b[0m {}", path.display());

                for log in logs {
                    if log.starts_with("[script]") {
                        println!("  {}", log);
                    }
                }

                if dry_run || !write {
                    let diff = generate_diff(original, transformed, &path.display().to_string());
                    println!("\n{}\n", diff);
                }

                if write && !dry_run {
                    match std::fs::write(path, transformed) {
                        Ok(_) => {
                            println!("  \x1b[32mWritten\x1b[0m");
                        }
                        Err(e) => {
                            eprintln!("  \x1b[31m✗\x1b[0m Failed to write: {}", e);
                        }
                    }
                }
            }

            println!();
            if interactive {
                println!("\x1b[1;35m┌──────────────────────────────────────────────────┐\x1b[0m");
                println!(
                    "\x1b[1;35m│\x1b[0m  \x1b[1;32m{}\x1b[0m file(s) approved for changes            \x1b[1;35m│\x1b[0m",
                    total_changes
                );
                println!(
                    "\x1b[1;35m│\x1b[0m  Time: {:.2}s                                   \x1b[1;35m│\x1b[0m",
                    duration.as_secs_f64()
                );
                println!("\x1b[1;35m└──────────────────────────────────────────────────┘\x1b[0m");

                if !results.is_empty() && !dry_run && !write {
                    println!("\n\x1b[1;36mApply all approved changes?\x1b[0m");
                    println!(
                        "\x1b[1;35m  [y]\x1b[0m Yes, write all   \x1b[1;35m[n]\x1b[0m No, exit"
                    );
                    print!("\n  \x1b[1;36m→\x1b[0m ");
                    io::stdout().flush().unwrap();

                    let mut final_input = String::new();
                    io::stdin().read_line(&mut final_input).unwrap();
                    if final_input.trim().to_lowercase().starts_with('y') {
                        for (path, _original, transformed, _logs) in &results {
                            match std::fs::write(path, transformed) {
                                Ok(_) => {
                                    println!("  \x1b[32m✓\x1b[0m Written: {}", path.display());
                                }
                                Err(e) => {
                                    eprintln!(
                                        "  \x1b[31m✗\x1b[0m Failed to write {}: {}",
                                        path.display(),
                                        e
                                    );
                                }
                            }
                        }
                        println!("\n\x1b[32mDone! {} file(s) written.\x1b[0m", results.len());
                    } else {
                        println!("\n\x1b[33mNo changes written.\x1b[0m");
                    }
                } else if write && !dry_run {
                    println!("\n\x1b[32mChanges written successfully.\x1b[0m");
                }
            } else if dry_run {
                println!(
                    "\x1b[33m{} file(s) would be changed ({:.2}s) [DRY RUN]\x1b[0m",
                    total_changes,
                    duration.as_secs_f64()
                );
            } else if write {
                println!(
                    "\x1b[32m{} file(s) changed ({:.2}s)\x1b[0m",
                    total_changes,
                    duration.as_secs_f64()
                );
            } else {
                println!(
                    "\x1b[36m{} file(s) changed ({:.2}s) [--write to apply]\x1b[0m",
                    total_changes,
                    duration.as_secs_f64()
                );
            }
        }
    }
}

#[cfg(feature = "headless")]
/// Discover all .st files in a directory recursively
fn discover_st_files(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip hidden directories and common exclusions
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            discover_st_files(&path, files);
        } else if path.extension().map(|e| e == "st").unwrap_or(false) {
            files.push(path);
        }
    }
}

#[cfg(test)]
mod init_target_tests {
    use super::resolve_init_target;
    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn resolve_init_target_no_projects_dir() {
        let dir = tempdir().expect("tempdir");
        let cwd = dir.path();
        let resolved = resolve_init_target("foo", cwd);
        assert_eq!(resolved, cwd.join("foo"));
    }

    #[test]
    fn resolve_init_target_with_projects_dir() {
        let dir = tempdir().expect("tempdir");
        let cwd = dir.path();
        fs::create_dir_all(cwd.join("projects")).expect("mkdir projects");
        let resolved = resolve_init_target("foo", cwd);
        assert_eq!(resolved, cwd.join("projects").join("foo"));
    }

    #[test]
    fn resolve_init_target_projects_is_file() {
        let dir = tempdir().expect("tempdir");
        let cwd = dir.path();
        fs::write(cwd.join("projects"), b"not a dir").expect("write file");
        let resolved = resolve_init_target("foo", cwd);
        assert_eq!(resolved, cwd.join("foo"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_init_target_with_projects_symlink_to_dir() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().expect("tempdir");
        let cwd = dir.path();
        let real_projects = cwd.join("real-projects");
        fs::create_dir_all(&real_projects).expect("mkdir real-projects");
        symlink(&real_projects, cwd.join("projects")).expect("symlink");
        let resolved = resolve_init_target("foo", cwd);
        assert_eq!(resolved, cwd.join("projects").join("foo"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_init_target_with_projects_broken_symlink() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().expect("tempdir");
        let cwd = dir.path();
        let missing_target = cwd.join("missing-projects");
        symlink(&missing_target, cwd.join("projects")).expect("symlink");
        let resolved = resolve_init_target("foo", cwd);
        assert_eq!(resolved, cwd.join("foo"));
    }
}
