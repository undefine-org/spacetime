//! PLAN-124 W3.3a — `@on $signal { pattern => --form(args); … }`: signal arms.
//!
//! Arms discriminate on `->` (a bare pattern means "arrived from anywhere";
//! N states never force N² arms). Each arm's form resolves through the form
//! registry at COMPILE time — the emitted runtime is pure dispatch (the
//! signal-arms primitive watches the signal and plays the matching arm's
//! WAAPI keyframes). Call-site args (positional or `name: value`) override
//! the form's declared defaults; `6f` parses as frames (100ms at 60fps).

use std::process::Command;

fn write_temp(source: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "w33a-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    std::fs::write(&file, source).expect("write source");
    file
}

fn run_cli(source: &str, args: &[&str]) -> (bool, String) {
    let file = write_temp(source);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_spacetime"));
    cmd.arg(args[0]).arg(&file).args(&args[1..]);
    let out = cmd.output().expect("run spacetime CLI");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    (out.status.success(), combined)
}

fn build_js(source: &str) -> String {
    let file = write_temp(source);
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&file)
        .output()
        .expect("run spacetime build");
    assert!(
        out.status.success(),
        "build failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let js = std::fs::read_to_string(file.parent().unwrap().join("spacetime.js"))
        .expect("read spacetime.js");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    js
}

const SLIDES: &str = "@form motion --slide-open($duration = 280ms) { height: 0 -> 4.5rem; opacity: 0 -> 1; }\n@form motion --slide-shut($duration = 200ms) { height: 4.5rem -> 0; opacity: 1 -> 0; }\n";

/// THE GATE — the launch article's `.drawer` fence: two-state arms (a Gate).
/// Both forms resolve, positional call args override the duration, and the
/// runtime watches the signal.
#[test]
fn the_drawer_fence_emits_resolved_arms() {
    let src = format!(
        "{SLIDES}\n.drawer {{\n  $open bool: false;\n  @on $open {{\n    true  => --slide-open(280ms);\n    false => --slide-shut(200ms);\n  }}\n}}\n"
    );
    let js = build_js(&src);
    assert!(
        js.contains("match: \"true\"") && js.contains("match: \"false\""),
        "both gate arms emit with their match values:\n{js}"
    );
    assert!(
        js.contains("\"4.5rem\""),
        "the DECLARED forms' keyframes are resolved into the arms:\n{js}"
    );
    assert!(
        js.contains("duration: 280") && js.contains("duration: 200"),
        "the positional call args reach the WAAPI play:\n{js}"
    );
    assert!(
        js.contains("ST.watch"),
        "the runtime watches the signal:\n{js}"
    );
}

/// String-valued signal arms (the article's `$view` fence) and the
/// transition + wildcard shapes (`$ready`): every match spec emits.
#[test]
fn string_transition_and_wildcard_arms_emit() {
    let src = "@form motion --grid-in($duration = 360ms) { opacity: 0 -> 1; }\n@form motion --list-in($duration = 360ms) { opacity: 0 -> 1; }\n@form motion --cut { opacity: 1 -> 1; }\n@form motion --dissolve($duration = 300ms) { opacity: 0 -> 1; }\n\n.grid {\n  $view string: \"grid\";\n  @on $view {\n    \"grid\" => --grid-in(360ms);\n    \"list\" => --list-in(360ms);\n  }\n}\n\n.ready {\n  $ready string: \"a\";\n  @on $ready {\n    a -> b => --cut;\n    _      => --dissolve(6f);\n  }\n}\n";
    let js = build_js(src);
    assert!(
        js.contains("match: \"grid\"") && js.contains("match: \"list\""),
        "string arms:\n{js}"
    );
    assert!(
        js.contains("match: [\"a\", \"b\"]"),
        "a transition arm emits [from, to]:\n{js}"
    );
    assert!(js.contains("match: \"*\""), "the wildcard arm emits:\n{js}");
    assert!(
        js.contains("duration: 100"),
        "6f parses as frames (100ms at 60fps):\n{js}"
    );
}

/// An arm naming an undeclared form is a loud error, not a silent skip.
#[test]
fn an_arm_with_an_undeclared_form_errors() {
    let src = ".a {\n  $x bool: false;\n  @on $x {\n    true => --ghost;\n  }\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(!passed, "an undeclared arm form must fail:\n{out}");
    assert!(
        out.contains("E0948") && out.contains("--ghost"),
        "must name the form:\n{out}"
    );
}

/// A non-arm body still routes to the mutation dispatcher (permissive
/// sibling): the arms form is tried first, rejects, falls through.
#[test]
fn a_mutation_body_still_dispatches() {
    let src = ".toggle {\n  $open bool: false;\n  @on click t { $open <- !$open; }\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(
        passed,
        "the mutation-dispatcher surface must survive:\n{out}"
    );
}

/// W3 review P1: an arm that OMITS a declared arg must keep the form's
/// default — an empty override used to win and emit an empty value.
#[test]
fn an_arm_with_omitted_args_keeps_the_form_default() {
    let src = "@form motion --rise($distance = 24px) { translate-y: $distance -> 0; }\n\n.a {\n  $x bool: false;\n  @on $x {\n    true => --rise;\n  }\n}\n";
    let js = build_js(src);
    assert!(
        js.contains("\"24px\""),
        "the declared default survives an omitted call arg:\n{js}"
    );
    assert!(
        !js.contains("value: \"\""),
        "never an empty value (the P1 corruption):\n{js}"
    );
}

/// W3 review P2: substring substitution — `$x` must not rewrite `$x2`.
#[test]
fn substitution_is_token_bounded() {
    let src = "@form motion --grow($x = 10px, $x2 = 20px) {\n  padding: $x2 -> 0;\n  margin: $x -> 0;\n}\n\n.a {\n  $go bool: false;\n  @on $go {\n    true => --grow;\n  }\n}\n";
    let js = build_js(src);
    assert!(
        js.contains("\"20px\""),
        "$x2 keeps its own value (would have been corrupted to `10px2`):\n{js}"
    );
    assert!(js.contains("\"10px\""), "$x substitutes correctly:\n{js}");
    assert!(!js.contains("10px2"), "never the prefix corruption:\n{js}");
}

/// W3 review P2: a valid arm followed by a non-arm statement must not fall
/// through to the legacy mutation dispatcher (a `$`-headed @on is the
/// signal-arms surface, full stop).
#[test]
fn a_mixed_arms_body_is_a_hard_error() {
    let src = "@form motion --x { opacity: 0 -> 1; }\n\n.a {\n  $sig bool: false;\n  $y number: 0;\n  @on $sig {\n    true => --x;\n    $y <- 1;\n  }\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(
        !passed,
        "a mixed body must fail, not compile through the wrong dispatcher:\n{out}"
    );
}

/// W3 review P2: two forms after the driver head are extra input, not a
/// silent drop.
#[test]
fn two_forms_after_the_driver_are_rejected() {
    let src = "@form motion --a { opacity: 0 -> 1; }\n@form motion --b { opacity: 0 -> 1; }\n\n.a {\n  @on &.visible: --a --b;\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(!passed, "two forms must fail loudly:\n{out}");
    assert!(
        out.contains("extra inline token"),
        "named as extra input:\n{out}"
    );
}
