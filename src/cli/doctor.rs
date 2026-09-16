//! CLI doctor command - one-shot project audit
//!
//! Runs Spacetime's compile-time checks over the project, then (in a
//! later wave) drives a headless browser through each route, evaluates
//! `Spacetime.review()`, and aggregates the result into a single
//! `DoctorReport`.
//!
//! Mirrors the structural shape of [`crate::cli::inspect`]: typed
//! [`OutputFormat`], layer-output structs deriving `serde::Serialize`,
//! and a dispatcher (`run_doctor`) that builds the report and renders
//! it.
//!
//! ## Status (PLAN-018 wave-3)
//!
//! - Compile-time checks: implemented.
//! - Headless route loop: deferred to [[id:FUP-016]] (see plan completion
//!   report). The doctor still runs and exits with a useful report based
//!   on the check pass alone.
//! - Output modes: pretty + JSON.

use serde::Serialize;
use std::path::{Path, PathBuf};

use super::inspect::OutputFormat;
use crate::analysis::CompileAnalysis;
use crate::diagnostics::DiagnosticCollector;
use crate::parser::{parse, resolve_imports};
use crate::test_runner::{DiscoverOptions, discover_tests_with_options};
use crate::type_system::TypeRegistry;

// =============================================================================
// Public API
// =============================================================================

/// Arguments for the doctor command (mirrors the CLI flags in `main.rs`).
#[derive(Debug, Clone)]
pub struct DoctorArgs {
    /// Project root containing `.st` source files.
    pub site: PathBuf,
    /// Routes to navigate; empty list defaults to `/`.
    pub routes: Vec<String>,
    /// Output format (pretty or json).
    pub format: OutputFormat,
    /// Optional directory for per-route screenshots.
    pub screenshot_dir: Option<PathBuf>,
    /// Port for the headless dev server.
    pub port: u16,
    /// Per-route timeout for `Spacetime.review()` evaluation in milliseconds.
    /// Defaults to 30_000 (30s); large pages can use --route-timeout-ms to extend.
    pub route_timeout_ms: u64,
}

// =============================================================================
// Report types
// =============================================================================

/// Severity of a single review or check finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warn,
    Info,
}

/// A single review finding produced by `Spacetime.review()` or by check.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// Review dimension this finding belongs to (idiomatic / animation /
    /// brand / a11y / compile).
    pub dim: String,
    pub severity: Severity,
    pub message: String,
    /// CSS selector or element identifier where applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    /// Optional value (e.g., the offending hex color literal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Anchor into the spacetime-design skill, e.g.
    /// `spacetime-design#anti-slop-inline-handlers`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_ref: Option<String>,
}

/// Per-dimension score + findings.
#[derive(Debug, Clone, Serialize)]
pub struct DimReport {
    /// `idiomatic` | `animation` | `brand` | `a11y` | `compile`.
    pub name: String,
    /// 0..10.
    pub score: f64,
    pub findings: Vec<Finding>,
}

/// Result for one route in the doctor pass.
#[derive(Debug, Clone, Serialize)]
pub struct RouteReport {
    pub route: String,
    /// `min(dims[*].score)`. May be 0 when the page failed to load.
    pub score: f64,
    pub dims: Vec<DimReport>,
    /// Console errors and unhandled rejections captured during navigation.
    pub console_errors: Vec<String>,
    /// Path to the screenshot saved for this route, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<PathBuf>,
    /// Set when the page did not load or `Spacetime.review()` was missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Set when the route was not navigated (e.g., headless mode disabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
}

/// `cargo run -- check` diagnostics rolled up into one entry.
#[derive(Debug, Clone, Serialize)]
pub struct CheckDiagnostic {
    pub file: PathBuf,
    pub severity: Severity,
    pub message: String,
    /// Stable rule slug for opt-in checks (e.g. `"brand-asset-missing"`).
    /// `None` for the existing parse/import/validate diagnostics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// Top-level doctor report.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    /// Project that was audited (canonical path).
    pub project: PathBuf,
    pub started_at: String,
    pub finished_at: String,
    pub check_diagnostics: Vec<CheckDiagnostic>,
    pub routes: Vec<RouteReport>,
    /// Aggregate pass/fail.
    pub passed: bool,
}

impl DoctorReport {
    /// Pass = no error-severity check diagnostics, every navigated route
    /// score >= 6, and zero console errors per route. Routes with a benign
    /// `skipped_reason` (see `BENIGN_SKIP_REASONS`) are still ignored; routes
    /// with any other `skipped_reason` cause failure (FEAT-033).
    pub fn compute_pass(&self) -> bool {
        let no_check_errors = !self
            .check_diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error);
        let all_routes_ok = self.routes.iter().all(|r| {
            if let Some(reason) = &r.skipped_reason {
                return is_benign_skip_reason(reason);
            }
            r.error.is_none() && r.score >= 6.0 && r.console_errors.is_empty()
        });
        no_check_errors && all_routes_ok
    }
}

// =============================================================================
// Slug helper
// =============================================================================

/// Turn a URL path into a filesystem-safe slug.
///
/// `/` -> `index`, `/about` -> `about`, `/fr/contact` -> `fr-contact`.
pub fn route_slug(route: &str) -> String {
    let trimmed = route.trim_matches('/');
    if trimmed.is_empty() {
        return "index".to_string();
    }
    trimmed.replace('/', "-")
}

// =============================================================================
// Check phase
// =============================================================================

/// Discover .st entry files in the project and run validation over them.
///
/// Returns `(diagnostics, files_checked)`. Files with parse/resolve
/// failures contribute one `CheckDiagnostic` per error.
fn run_check_phase(site: &Path) -> (Vec<CheckDiagnostic>, usize) {
    let mut diagnostics = Vec::new();

    // Walk the site for .st files, excluding stdlib mirrors and node_modules.
    let options = DiscoverOptions::new()
        .with_pattern("*.st")
        .exclude("dist")
        .exclude("scratch")
        .exclude("__dev__");

    let st_files = discover_tests_with_options(&site.to_path_buf(), &options);
    if st_files.is_empty() {
        diagnostics.push(CheckDiagnostic {
            file: site.to_path_buf(),
            severity: Severity::Warn,
            message: "no .st files found in project".to_string(),
            rule: None,
        });
        return (diagnostics, 0);
    }

    // Skip test fixtures so doctor doesn't fail on intentionally-broken
    // .st files used by integration tests.
    let st_files: Vec<PathBuf> = st_files
        .into_iter()
        .filter(|p| {
            let s = p.to_string_lossy();
            !s.contains("/tests/") && !s.contains("/fixtures/")
        })
        .collect();

    for path in &st_files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                diagnostics.push(CheckDiagnostic {
                    file: path.clone(),
                    severity: Severity::Error,
                    message: format!("read failed: {}", e),
                    rule: None,
                });
                continue;
            }
        };

        let main_ast = match parse(&content) {
            Ok(ast) => ast,
            Err(e) => {
                diagnostics.push(CheckDiagnostic {
                    file: path.clone(),
                    severity: Severity::Error,
                    message: format!("parse failed: {}", e),
                    rule: None,
                });
                continue;
            }
        };

        let ast = if !main_ast.imports.is_empty() {
            match resolve_imports(&main_ast, path, site) {
                Ok(a) => a,
                Err(e) => {
                    diagnostics.push(CheckDiagnostic {
                        file: path.clone(),
                        severity: Severity::Error,
                        message: format!("import resolution failed: {}", e),
                        rule: None,
                    });
                    continue;
                }
            }
        } else {
            main_ast
        };

        // Diagnostic collector (mirrors `cargo run -- check`).
        let mut collector =
            DiagnosticCollector::new(&content).with_path(path.to_string_lossy().to_string());

        let mut analysis = CompileAnalysis::new();
        let matches = &ast.matches;
        let registry = TypeRegistry::from_form_matches(matches);
        analysis.analyze_types(matches, &registry);
        analysis.analyze_data_sources(matches, &registry);
        analysis.analyze_locals(matches);
        analysis.analyze_computed(matches);
        analysis.analyze_functions(matches);
        analysis.analyze_scopes(&ast.scopes);
        analysis.analyze_read_usages(&ast.scopes, matches);
        analysis.analyze_timelines(&ast.scopes);
        // Register macro %binds yields as signals from the registry (BUG-126).
        let (meta_registry, _) = crate::compiler::cached_stdlib_registry();
        analysis.register_macro_yield_signals(matches, &meta_registry);
        analysis.detect_orphans();
        analysis.validate_computed_dependencies();
        analysis.validate_binding_sources();

        // Wave B (FEAT-135): self-aware LiveView contract check (opt-in via a
        // `<page>.contract.json` sidecar). Mirrors the `check` path in main.rs.
        if let Some(site_dir) = path.parent() {
            for diag in
                crate::analysis::liveview_contract::check_liveview_contracts(site_dir, matches)
            {
                analysis.diagnostics.push(diag);
            }
        }

        for diag in &analysis.diagnostics {
            collector.emit(diag.clone());
        }

        for diag in collector.diagnostics() {
            let sev = match diag.severity {
                crate::diagnostics::Severity::Error => Severity::Error,
                crate::diagnostics::Severity::Warning => Severity::Warn,
            };
            diagnostics.push(CheckDiagnostic {
                file: path.clone(),
                severity: sev,
                message: diag.message.clone(),
                rule: None,
            });
        }
    }

    (diagnostics, st_files.len())
}

// =============================================================================
// Brand asset check (FEAT-028)
// =============================================================================

/// Scan `<site>/brand-spec.md` for asset paths and emit `brand-asset-missing`
/// warns when referenced files are absent.
///
/// Opt-in: returns an empty vec if `brand-spec.md` does not exist. The file
/// is parsed line-by-line: any heading matching `## Logo`, `## Hero imagery`,
/// or `## UI screenshots` opens an asset section, which closes at the next
/// `## ` heading. Within a section, bullet lines of the form
/// `- <Label>: <path>` contribute one path; `(n/a ...)` markers are skipped.
pub fn run_brand_asset_check(site: &Path) -> Vec<CheckDiagnostic> {
    let spec_path = site.join("brand-spec.md");
    let content = match std::fs::read_to_string(&spec_path) {
        Ok(s) => s,
        Err(_) => return Vec::new(), // opt-in: no spec, no findings
    };
    let asset_headings = ["Logo", "Hero imagery", "UI screenshots"];
    let mut findings = Vec::new();
    let mut in_asset_section = false;
    for raw in content.lines() {
        let line = raw.trim_end();
        if let Some(heading) = line.strip_prefix("## ") {
            in_asset_section = asset_headings
                .iter()
                .any(|h| heading.eq_ignore_ascii_case(h));
            continue;
        }
        if !in_asset_section {
            continue;
        }
        let trimmed = line.trim_start();
        let after_dash = match trimmed.strip_prefix("- ") {
            Some(rest) => rest,
            None => continue,
        };
        let (_label, path_str) = match after_dash.split_once(':') {
            Some((l, p)) => (l.trim(), p.trim()),
            None => continue,
        };
        if path_str.is_empty() || path_str.starts_with('(') {
            continue;
        }
        let path_str = path_str
            .trim_matches('`')
            .trim_start_matches('[')
            .trim_end_matches(']');
        let candidate = site.join(path_str);
        if !candidate.exists() {
            findings.push(CheckDiagnostic {
                file: spec_path.clone(),
                severity: Severity::Warn,
                message: format!("brand-spec.md references missing asset: {}", path_str),
                rule: Some("brand-asset-missing".to_string()),
            });
        }
    }
    findings
}

// =============================================================================
// Pretty + JSON renderers
// =============================================================================

fn render_pretty(report: &DoctorReport) {
    let bold = "\x1b[1m";
    let dim = "\x1b[2m";
    let red = "\x1b[31m";
    let yellow = "\x1b[33m";
    let green = "\x1b[32m";
    let reset = "\x1b[0m";

    println!(
        "{}Spacetime Doctor{}  {}{}{}",
        bold,
        reset,
        dim,
        report.project.display(),
        reset
    );
    println!();

    let err_count = report
        .check_diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warn_count = report
        .check_diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warn)
        .count();

    let check_color = if err_count > 0 {
        red
    } else if warn_count > 0 {
        yellow
    } else {
        green
    };
    println!(
        "  {bold}check{reset}   {color}{err}{reset} error(s), {warn} warning(s) over {n} diagnostic(s)",
        bold = bold,
        reset = reset,
        color = check_color,
        err = err_count,
        warn = warn_count,
        n = report.check_diagnostics.len(),
    );

    if err_count > 0 {
        let max_show = 10;
        for (idx, diag) in report
            .check_diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .enumerate()
        {
            if idx >= max_show {
                println!(
                    "    {}\u{2026}{} ({} more)",
                    dim,
                    reset,
                    err_count - max_show
                );
                break;
            }
            println!(
                "    {red}\u{2717}{reset} {file}: {msg}",
                red = red,
                reset = reset,
                file = diag.file.display(),
                msg = diag.message,
            );
        }
    }

    println!();
    println!("  {bold}routes{reset}", bold = bold, reset = reset);
    if report.routes.is_empty() {
        println!(
            "    {dim}(no routes navigated){reset}",
            dim = dim,
            reset = reset
        );
    }
    for route in &report.routes {
        if let Some(reason) = &route.skipped_reason {
            println!(
                "    {dim}\u{2014}{reset} {route} {dim}skipped:{reset} {reason}",
                dim = dim,
                reset = reset,
                route = route.route,
                reason = reason,
            );
            continue;
        }
        let icon_color = if route.score >= 8.0 {
            green
        } else if route.score >= 5.0 {
            yellow
        } else {
            red
        };
        println!(
            "    {color}\u{25CF}{reset} {route}  score={score}",
            color = icon_color,
            reset = reset,
            route = route.route,
            score = route.score,
        );
        for dim_report in &route.dims {
            println!(
                "        {bold}{name:<10}{reset} {score}",
                bold = bold,
                reset = reset,
                name = dim_report.name,
                score = dim_report.score,
            );
        }
    }

    println!();
    let verdict_color = if report.passed { green } else { red };
    let verdict_text = if report.passed { "PASS" } else { "FAIL" };
    println!(
        "  {bold}verdict{reset} {color}{text}{reset}",
        bold = bold,
        reset = reset,
        color = verdict_color,
        text = verdict_text,
    );
}

fn render_json(report: &DoctorReport) {
    println!("===JSON===");
    match serde_json::to_string_pretty(report) {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("\x1b[31m\u{2717}\x1b[0m doctor: serialize failed: {}", e),
    }
}

// =============================================================================
// Wall-clock helper (kept simple to avoid pulling chrono).
// =============================================================================

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_else(|_| "0".to_string())
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Run the doctor command.
///
/// Wave-3 implementation: runs check, marks routes as skipped (the
/// headless loop is deferred — see [[id:FUP-016]]). Wave-5 will fill in
/// the route loop.
pub async fn run_doctor(args: DoctorArgs) -> Result<DoctorReport, String> {
    if !args.site.exists() {
        return Err(format!("site directory not found: {}", args.site.display()));
    }

    let started_at = now_iso();

    // -------------------- Check phase --------------------
    let (mut check_diagnostics, files_checked) = run_check_phase(&args.site);
    check_diagnostics.extend(run_brand_asset_check(&args.site));

    // -------------------- Route phase --------------------
    let routes: Vec<String> = if args.routes.is_empty() {
        vec!["/".to_string()]
    } else {
        args.routes.clone()
    };

    let route_reports: Vec<RouteReport> = run_headless_phase(
        &args.site,
        &routes,
        args.screenshot_dir.as_deref(),
        args.port,
        args.route_timeout_ms,
    )
    .await;

    let _ = files_checked;

    let mut report = DoctorReport {
        project: args.site.clone(),
        started_at,
        finished_at: now_iso(),
        check_diagnostics,
        routes: route_reports,
        passed: false,
    };
    report.passed = report.compute_pass();

    match args.format {
        OutputFormat::Pretty => render_pretty(&report),
        OutputFormat::Json => render_json(&report),
    }

    Ok(report)
}

// =============================================================================
// Headless route phase (FEAT-033)
// =============================================================================

/// Reasons that compute_pass treats as benign (don't fail when present).
///
/// Adding a new benign reason is an opt-in to skip-without-fail; do this only
/// when the route physically can't be evaluated (build feature gating, etc.).
const BENIGN_SKIP_REASONS: &[&str] = &["browser feature not enabled"];

/// Drive each route through Playwright + `Spacetime.review()`.
///
/// In `--features browser` builds: spawns `cargo run -- serve` as a child, polls
/// for readiness, navigates each route via Chromium, evaluates
/// `Spacetime.review()` (with the configurable per-route timeout), captures
/// console errors and an optional screenshot, then tears the server down.
///
/// In non-browser builds: returns a `RouteReport` per route with
/// `skipped_reason: Some("browser feature not enabled")`. compute_pass treats
/// this reason as benign so doctor still passes when the user opts out of
/// browser deps; building with `--features browser` flips it to a real gate.
pub async fn run_headless_phase(
    site: &Path,
    routes: &[String],
    screenshot_dir: Option<&Path>,
    port: u16,
    route_timeout_ms: u64,
) -> Vec<RouteReport> {
    // The doctor route-screenshot phase was driven by Playwright, which was
    // removed in PLAN-027 W2. Porting it to the chromiumoxide CDP backend is
    // tracked as a follow-up; until then this phase is skipped cleanly.
    let _ = (site, screenshot_dir, port, route_timeout_ms);
    routes
        .iter()
        .map(|route| RouteReport {
            route: route.clone(),
            score: 0.0,
            dims: Vec::new(),
            console_errors: Vec::new(),
            screenshot: None,
            error: None,
            skipped_reason: Some(
                "route-screenshot phase pending CDP port (PLAN-027 W2 removed Playwright)"
                    .to_string(),
            ),
        })
        .collect()
}

/// Public helper used by `compute_pass` and tests: is `reason` benign?
pub fn is_benign_skip_reason(reason: &str) -> bool {
    BENIGN_SKIP_REASONS.contains(&reason)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_report() -> DoctorReport {
        DoctorReport {
            project: PathBuf::from("test"),
            started_at: "0".into(),
            finished_at: "0".into(),
            check_diagnostics: vec![],
            routes: vec![],
            passed: false,
        }
    }

    #[test]
    fn route_slug_handles_root() {
        assert_eq!(route_slug("/"), "index");
        assert_eq!(route_slug(""), "index");
    }

    #[test]
    fn route_slug_handles_simple_paths() {
        assert_eq!(route_slug("/about"), "about");
        assert_eq!(route_slug("about/"), "about");
        assert_eq!(route_slug("/about/"), "about");
    }

    #[test]
    fn route_slug_handles_nested_paths() {
        assert_eq!(route_slug("/fr/contact"), "fr-contact");
        assert_eq!(route_slug("/a/b/c"), "a-b-c");
    }

    #[test]
    fn pass_fail_requires_no_compile_errors() {
        let mut r = empty_report();
        r.check_diagnostics.push(CheckDiagnostic {
            file: PathBuf::from("x.st"),
            severity: Severity::Error,
            message: "bang".into(),
            rule: None,
        });
        assert!(!r.compute_pass());
    }

    #[test]
    fn pass_fail_fails_when_route_score_below_six() {
        let mut r = empty_report();
        r.routes.push(RouteReport {
            route: "/".into(),
            score: 5.5,
            dims: vec![],
            console_errors: vec![],
            screenshot: None,
            error: None,
            skipped_reason: None,
        });
        assert!(!r.compute_pass());
    }

    #[test]
    fn pass_fail_passes_clean_report() {
        let mut r = empty_report();
        r.routes.push(RouteReport {
            route: "/".into(),
            score: 8.0,
            dims: vec![],
            console_errors: vec![],
            screenshot: None,
            error: None,
            skipped_reason: None,
        });
        assert!(r.compute_pass());
    }

    #[test]
    fn pass_fail_fails_on_console_errors() {
        let mut r = empty_report();
        r.routes.push(RouteReport {
            route: "/".into(),
            score: 9.0,
            dims: vec![],
            console_errors: vec!["TypeError: x".into()],
            screenshot: None,
            error: None,
            skipped_reason: None,
        });
        assert!(!r.compute_pass());
    }

    #[test]
    fn pass_fail_fails_on_non_benign_skipped_routes() {
        let mut r = empty_report();
        r.routes.push(RouteReport {
            route: "/".into(),
            score: 0.0,
            dims: vec![],
            console_errors: vec![],
            screenshot: None,
            error: None,
            skipped_reason: Some("chromium unavailable".into()),
        });
        assert!(!r.compute_pass(), "non-benign skipped routes must fail");
    }

    #[test]
    fn pass_fail_ignores_benign_skipped_routes() {
        // Benign reason: build feature-gate skip; doctor still passes.
        let mut r = empty_report();
        r.routes.push(RouteReport {
            route: "/".into(),
            score: 0.0,
            dims: vec![],
            console_errors: vec![],
            screenshot: None,
            error: None,
            skipped_reason: Some("browser feature not enabled".into()),
        });
        assert!(r.compute_pass(), "benign skipped routes must not fail");
    }

    #[test]
    fn benign_reason_helper_matches_canonical_phrases() {
        assert!(is_benign_skip_reason("browser feature not enabled"));
        assert!(!is_benign_skip_reason("chromium unavailable"));
        assert!(!is_benign_skip_reason(
            "Spacetime.review unavailable after 30000ms"
        ));
    }

    // -------------------- brand-asset-missing tests (FEAT-028) ---------------

    fn make_brand_fixture(spec: &str, files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("brand-spec.md"), spec).unwrap();
        for f in files {
            let target = dir.path().join(f);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&target, b"").unwrap();
        }
        dir
    }

    #[test]
    fn brand_asset_missing_emits_warn_when_path_referenced_but_file_absent() {
        let spec = "# Brand spec\n\n## Logo\n- Primary: assets/logo.svg\n";
        let dir = make_brand_fixture(spec, &[]);
        let findings = run_brand_asset_check(dir.path());
        assert_eq!(findings.len(), 1, "expected 1 finding, got {findings:?}");
        let f = &findings[0];
        assert_eq!(f.severity, Severity::Warn);
        assert_eq!(f.rule.as_deref(), Some("brand-asset-missing"));
        assert!(
            f.message.contains("assets/logo.svg"),
            "message: {}",
            f.message
        );
    }

    #[test]
    fn brand_asset_missing_silent_when_paths_exist() {
        let spec = "# Brand spec\n\n## Logo\n- Primary: assets/logo.svg\n\n## Hero imagery\n- Above-the-fold: assets/hero/main.jpg\n";
        let dir = make_brand_fixture(spec, &["assets/logo.svg", "assets/hero/main.jpg"]);
        let findings = run_brand_asset_check(dir.path());
        assert!(
            findings.is_empty(),
            "expected no findings, got {findings:?}"
        );
    }

    #[test]
    fn brand_asset_missing_silent_when_no_brand_spec() {
        let dir = tempfile::tempdir().expect("tempdir");
        let findings = run_brand_asset_check(dir.path());
        assert!(
            findings.is_empty(),
            "opt-in: no brand-spec.md should yield no findings"
        );
    }

    #[test]
    fn brand_asset_missing_skips_placeholder_paths() {
        let spec = "# Brand spec\n\n## Logo\n- Primary: (TBD)\n\n## UI screenshots\n- Dashboard: (n/a -- marketing site)\n";
        let dir = make_brand_fixture(spec, &[]);
        let findings = run_brand_asset_check(dir.path());
        assert!(
            findings.is_empty(),
            "placeholder markers must not warn, got {findings:?}"
        );
    }

    #[test]
    fn brand_asset_missing_ignores_paths_outside_asset_sections() {
        let spec = "# Brand spec\n\n## Tokens\n- Primary token: tokens/missing.st\n\n## Logo\n- Primary: assets/logo.svg\n";
        let dir = make_brand_fixture(spec, &["assets/logo.svg"]);
        let findings = run_brand_asset_check(dir.path());
        assert!(
            findings.is_empty(),
            "non-asset section paths must not be checked, got {findings:?}"
        );
    }
}
