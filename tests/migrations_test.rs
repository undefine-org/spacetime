//! PLAN-076 W2 — syntax migrations, end-to-end through `Compiler::from_file`.
//!
//! The compat shim (src/migrate.rs) rewrites old syntax IN MEMORY so it keeps
//! compiling (W0715 per applied match); hint-kind and unsound multi-shape
//! calls still hard-error (E0910) with the hint coming from the registry
//! entry, not Rust. `@version` scopes which waves participate.

use spacetime::compiler::Compiler;

/// Compile inline `.st` source the way `Compiler::from_file` does (its public
/// API is file-based): write to a temp file, compile, return the output.
fn compile_source(source: &str) -> spacetime::compiler::CompiledSpacetime {
    let temp = tempfile::tempdir().expect("tempdir");
    let file = temp.path().join("index.st");
    std::fs::write(&file, source).expect("write probe file");
    Compiler::from_file(&file, temp.path())
        .expect("probe should parse")
        .compile()
}

/// Compile source INSIDE the migration window for the reactive-surface wave.
///
/// Since the hard cutover an absent `@version` means CURRENT, so retired
/// syntax is refused by default. A test that exercises the WINDOW must
/// therefore say which wave it is migrating FROM — the same thing a real
/// project does. `2026-06-08` is the day before the reactive-surface wave, so
/// every migration at or after it is still pending and the shim participates.
///
/// NB this is not a test-only escape hatch: it is exactly the `@version` fact
/// `spacetime migrate` writes into a project before bumping it forward.
fn compile_in_window(source: &str) -> spacetime::compiler::CompiledSpacetime {
    compile_source(&format!("@version 2026-06-08;\n{source}"))
}

#[test]
fn bind_text_compiles_through_shim_to_identical_js() {
    let old = "$user object: { name: \"Ada\" };\n.card { @bind(text: $user.name) }\n";
    let new = "$user object: { name: \"Ada\" };\n.card { text <- $user.name; }\n";

    let old_out = compile_in_window(old);
    let new_out = compile_in_window(new);

    assert!(
        old_out.pipeline_errors.is_empty(),
        "shim must keep old syntax compiling: {:?}",
        old_out.pipeline_errors
    );
    assert!(
        new_out.pipeline_errors.is_empty(),
        "reference must compile: {:?}",
        new_out.pipeline_errors
    );
    // RED→GREEN core: the shimmed compile emits THE SAME JS as the
    // hand-migrated reference.
    assert_eq!(
        old_out.js, new_out.js,
        "shimmed output must equal the reference output"
    );

    // Exactly one W0715 warning + one pending entry carrying the rewrite.
    let w: Vec<_> = old_out
        .migration_warnings
        .iter()
        .filter(|w| w.code == "W0715")
        .collect();
    assert_eq!(w.len(), 1, "warnings: {:?}", old_out.migration_warnings);
    assert_eq!(old_out.pending_migrations.len(), 1);
    let p = &old_out.pending_migrations[0];
    assert_eq!(p.migration_id, "reactive-surface");
    assert_eq!(p.rule.as_deref(), Some("bind-text"));
    assert_eq!(p.date, "2026-06-09");
    assert_eq!(p.old_text, "@bind(text: $user.name)");
    assert_eq!(p.new_text.as_deref(), Some("text <- $user.name;"));
    assert!(p.is_automatic());

    // The hand-migrated reference has NOTHING pending and no warnings.
    assert!(new_out.pending_migrations.is_empty());
    assert!(new_out.migration_warnings.is_empty());
}

/// PLAN-079 (capsules): hint-kind directives COMPILE THROUGH the window —
/// the embedded retired %macro's %binds still route to their (retained)
/// primitives, so `@show` emits the same visibility toggle it always did,
/// with a W0715 + the entry's %hint instead of a hard error. The window
/// closes at `@version 2026-06-09` (E0911, tested elsewhere).
#[test]
fn show_compiles_through_window_with_registry_hint() {
    let out = compile_in_window("$flag bool: true;\n.card { @show(when: $flag) }\n");
    assert!(
        !out.pipeline_errors.iter().any(|e| e.code == "E0910"),
        "hint-kind must not hard-error inside the window: {:?}",
        out.pipeline_errors
    );
    let w: Vec<_> = out
        .migration_warnings
        .iter()
        .filter(|w| w.code == "W0715")
        .collect();
    assert_eq!(w.len(), 1, "warnings: {:?}", out.migration_warnings);
    assert!(
        w[0].message.contains("@show") && w[0].message.contains("2026-06-09"),
        "message: {}",
        w[0].message
    );
    // The hint comes from the %migration entry's %hint — not a Rust match-arm.
    assert!(
        w[0].hint
            .as_deref()
            .unwrap_or_default()
            .contains(".hidden:"),
        "hint: {:?}",
        w[0].hint
    );
    // And the retired semantics actually emit: bind-visible's display toggle.
    assert!(
        out.js.contains("display"),
        "retired @show must still emit its visibility toggle: {}",
        &out.js[..out.js.len().min(400)]
    );
}

#[test]
fn multi_shape_bind_degrades_to_e0910_with_split_hint() {
    let out = compile_in_window(
        "$user object: { name: \"Ada\" };\n$flag bool: true;\n.card { @bind(text: $user.name, class: \"open\", when: $flag) }\n",
    );
    let e: Vec<_> = out
        .pipeline_errors
        .iter()
        .filter(|e| e.code == "E0910")
        .collect();
    assert_eq!(
        e.len(),
        1,
        "multi-shape call must error, not silently truncate: {:?}",
        out.pipeline_errors
    );
    // And the pending entry explains WHY: whichever single-shape form
    // claimed the call, the REMAINING args are the unsound-to-drop extras.
    assert_eq!(out.pending_migrations.len(), 1);
    let p = &out.pending_migrations[0];
    assert!(!p.is_automatic());
    let hint = p.hint.as_deref().unwrap_or_default();
    assert!(
        hint.contains("text"),
        "pending hint should name the leftover args: {hint}"
    );
}

#[test]
fn version_at_wave_makes_old_syntax_a_version_error_not_a_migration() {
    // Project claims it crossed the 2026-06-09 wave; a lingering @bind is a
    // version inconsistency (E0911), not a pending migration.
    let out = compile_source(
        "@version 2026-06-09;\n$user object: { name: \"Ada\" };\n.card { @bind(text: $user.name) }\n",
    );
    assert!(
        out.pending_migrations.is_empty(),
        "inert wave must not pend: {:?}",
        out.pending_migrations
    );
    assert!(out.migration_warnings.is_empty());
    let e: Vec<_> = out
        .pipeline_errors
        .iter()
        .filter(|e| e.code == "E0911")
        .collect();
    assert_eq!(e.len(), 1, "errors: {:?}", out.pipeline_errors);
    assert!(
        e[0].message.contains("2026-06-09"),
        "message: {}",
        e[0].message
    );
}

#[test]
fn version_at_wave_with_clean_source_compiles_quietly() {
    let out = compile_source(
        "@version 2026-06-09;\n$user object: { name: \"Ada\" };\n.card { text <- $user.name; }\n",
    );
    assert!(
        out.pipeline_errors.is_empty(),
        "errors: {:?}",
        out.pipeline_errors
    );
    assert!(out.pending_migrations.is_empty());
    assert!(out.migration_warnings.is_empty());
}

#[test]
fn remaining_rewrite_seeds_end_to_end() {
    // Table-driven: every rewrite-kind seed must shim to its documented
    // surface with the exact pending rewrite text.
    for (old, reference, want_new_text) in [
        (
            ".card { @bind(attr: \"src\", value: $user.avatar) }",
            ".card { src <- $user.avatar; }",
            "src <- $user.avatar;",
        ),
        (
            ".card { @bind(style: \"color\", value: $theme.fg) }",
            ".card { color: $theme.fg; }",
            "color: $theme.fg;",
        ),
        (
            ".card { @bind(visible: $open) }",
            ".card { .shown: $open; }",
            ".shown: $open;",
        ),
    ] {
        let old_out = compile_in_window(old);
        let new_out = compile_in_window(reference);
        assert!(
            old_out.pipeline_errors.is_empty(),
            "{old} must compile via shim: {:?}",
            old_out.pipeline_errors
        );
        assert!(
            new_out.pipeline_errors.is_empty(),
            "{reference} must compile: {:?}",
            new_out.pipeline_errors
        );
        assert_eq!(old_out.js, new_out.js, "{old} vs {reference}");
        assert_eq!(
            old_out.pending_migrations[0].new_text.as_deref(),
            Some(want_new_text),
            "{old}"
        );
    }
}

#[test]
fn hint_kind_with_inert_version_is_e0911_not_pending() {
    // @version at the seed wave + a hint-kind use (@show): inert wave, so
    // it's a version inconsistency — no pending, no W0715.
    let out =
        compile_source("@version 2026-06-09;\n$flag bool: true;\n.card { @show(when: $flag) }\n");
    assert!(out.pending_migrations.is_empty());
    assert!(out.migration_warnings.is_empty());
    let e: Vec<_> = out
        .pipeline_errors
        .iter()
        .filter(|e| e.code == "E0911")
        .collect();
    assert_eq!(e.len(), 1, "errors: {:?}", out.pipeline_errors);
}

#[test]
fn bind_in_an_imported_file_degrades_to_e0910() {
    // The shim is root-only (v1): old syntax in an @import-ed file surfaces
    // as E0910 with the degrade hint, not a pending row.
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("helpers.st"),
        ".card { @bind(text: $user.name) }\n",
    )
    .expect("write import");
    std::fs::write(
        temp.path().join("index.st"),
        "@import \"./helpers.st\";\n$user object: { name: \"Ada\" };\n",
    )
    .expect("write index");
    let out = Compiler::from_file(&temp.path().join("index.st"), temp.path())
        .expect("parse")
        .compile();
    // The contract is that imported retired syntax ERRORS rather than silently
    // passing. Since the hard cutover it errors HARDER: an imported file gets
    // no `@version` fact of its own, so it defaults to CURRENT and the retired
    // `@bind` is E0911 ("you already crossed that wave") rather than E0910
    // ("the shim couldn't rewrite this"). Either code satisfies the contract;
    // asserting only E0910 would pin this test to the pre-cutover default and
    // fail on a strictly better diagnostic.
    let e: Vec<_> = out
        .pipeline_errors
        .iter()
        .filter(|e| e.code == "E0910" || e.code == "E0911")
        .collect();
    assert_eq!(
        e.len(),
        1,
        "imported old syntax must error, not silently pass: {:?}",
        out.pipeline_errors
    );
}

#[test]
fn string_args_with_commas_and_parens_are_not_misread() {
    // R2 P1: the multi-shape soundness scanner must be string-aware.
    // A comma inside a string arg must NOT fabricate a positional extra arg
    // (false degrade), and a paren inside a string arg must NOT truncate the
    // arg list (silent-truncation rewrite).
    let out = compile_in_window(".card { @bind(text: \"Hello, world (hi)\") }\n");
    assert!(
        out.pipeline_errors.is_empty(),
        "a sound single-shape call with punctuation in the string must shim: {:?}",
        out.pipeline_errors
    );
    assert_eq!(out.pending_migrations.len(), 1);
    assert_eq!(
        out.pending_migrations[0].new_text.as_deref(),
        Some("text <- \"Hello, world (hi)\";")
    );

    // Same string WITH an extra arg: the extra must still be detected
    // (degrade, never silently truncate `class` away).
    let out = compile_in_window(".card { @bind(text: \"a)b\", class: \"open\") }\n");
    let e: Vec<_> = out
        .pipeline_errors
        .iter()
        .filter(|e| e.code == "E0910")
        .collect();
    assert_eq!(
        e.len(),
        1,
        "extra args past a paren-in-string must still degrade: {:?}",
        out.pipeline_errors
    );
}

#[test]
fn migrations_pill_compiles_cleanly() {
    // The dev-dock widget (stdlib/migrations/__dev__/pill.st) is compiled
    // server-side by the same rail as the host widget — a compile break in
    // it must fail loudly here, not silently in the browser.
    let path = std::path::Path::new("stdlib/migrations/__dev__/pill.st");
    let compiled = Compiler::from_file(path, std::path::Path::new("."))
        .expect("pill parses")
        .compile();
    assert!(
        compiled.pipeline_errors.is_empty(),
        "pill must compile: {:?}",
        compiled.pipeline_errors
    );
    assert!(compiled.html.contains("st-migrations-widget"));
    assert!(compiled.css.contains(".st-migrations-widget__pill"));
    assert!(compiled.js.contains("/status.json"));
}

#[test]
fn bind_class_compiles_through_shim_unquoting_the_class_name() {
    // String-literal captures splice their semantic (unquoted) value:
    // @bind(class: "active") -> `.active:` — not `."active":`.
    let old = "$isActive bool: true;\n.card { @bind(class: \"active\", when: $isActive) }\n";
    let new = "$isActive bool: true;\n.card { .active: $isActive; }\n";

    let old_out = compile_in_window(old);
    let new_out = compile_in_window(new);

    assert!(
        old_out.pipeline_errors.is_empty(),
        "shim must keep old syntax compiling: {:?}",
        old_out.pipeline_errors
    );
    assert!(
        new_out.pipeline_errors.is_empty(),
        "reference must compile: {:?}",
        new_out.pipeline_errors
    );
    assert_eq!(
        old_out.js, new_out.js,
        "shimmed output must equal the reference output"
    );
    assert_eq!(
        old_out.pending_migrations[0].new_text.as_deref(),
        Some(".active: $isActive;")
    );
}

#[test]
fn duplicate_version_is_an_authoring_error() {
    let out = compile_source("@version 2026-06-09;\n@version 2027-01-01;\n");
    let e: Vec<_> = out
        .pipeline_errors
        .iter()
        .filter(|e| e.code == "E0911" && e.message.contains("duplicate"))
        .collect();
    assert_eq!(e.len(), 1, "errors: {:?}", out.pipeline_errors);
}

// ── BUG-344: a %rewrite must not claim CURRENT syntax ────────────────────────
//
// The retired named form put the name BEFORE the parens
// (`@on &.visible reveal(600ms) { … }`). The current form binds AFTER, with
// `as`:
//
//     @on &.visible(600ms) as $reveal { … }
//
// `$name:ident` is unanchored, so it happily matched the trailing `as`
// keyword: the rule fired on a shape it never meant to claim and rewrote
// working code to `@on &.load(name: as, …)`, silently DROPPING the binding.
// 31 such sites exist across 10 files, so a repo-wide `spacetime migrate .`
// would have corrupted every one of them.

/// The `as` binding is CURRENT syntax on a CURRENT driver — nothing may
/// rewrite it, and `as` must never be captured as an animation name.
#[test]
fn the_as_binding_form_is_not_claimed_by_any_migration() {
    for driver in ["visible", "hover", "click", "focus", "load"] {
        let source = format!(
            ".target {{\n  @on &.{driver}(600ms) as $reveal {{\n    opacity: 0 -> 1;\n  }}\n}}\n"
        );
        let out = compile_source(&source);
        let js = &out.js;

        assert!(
            !js.contains("name: as") && !js.contains("\"as\""),
            "`as` leaked in as the animation name for &.{driver} — the binding was \
             swallowed by an unanchored $name:ident capture (BUG-344).\njs: {js}"
        );
        assert!(
            out.pipeline_errors.is_empty(),
            "current `as` syntax must compile clean for &.{driver}: {:?}",
            out.pipeline_errors
        );
    }
}

// ── Hard cutover: an absent @version means CURRENT, not ancient ──────────────
//
// The migration window (W0715) exists so a project can cross a wave gradually:
// retired syntax keeps compiling, rewritten in memory, until the project's
// `@version` reaches the wave date and it becomes E0911.
//
// That window was open FOREVER for any file with no `@version` fact, because
// an absent version compared as "before every wave". A new file could be
// written today in syntax retired months ago and compile silently — the window
// was acting as a permanent amnesty rather than a transition.
//
// After the cutover an absent `@version` means CURRENT: retired syntax is an
// error, and a project that genuinely needs the window says so explicitly by
// declaring the older `@version` it is migrating FROM.

/// A file with no `@version` is a NEW file: retired syntax must be refused.
#[test]
fn retired_syntax_without_a_version_is_refused() {
    let out = compile_source(".a { @scroll fade(scope: cover) { opacity: 0 -> 1; } }\n");
    let codes: Vec<&str> = out
        .pipeline_errors
        .iter()
        .map(|e| e.code.as_str())
        .collect();
    assert!(
        codes.contains(&"E0911") || codes.contains(&"E0910"),
        "retired @scroll must be refused when no @version opens the window, got: {:?}",
        out.pipeline_errors
    );
}

/// The escape hatch stays: declaring the PRE-wave version reopens the window,
/// so a real in-progress migration is not blocked by the cutover.
#[test]
fn an_explicit_pre_wave_version_still_opens_the_window() {
    let out = compile_source(
        "@version 2026-07-25;\n.a { @scroll fade(scope: cover) { opacity: 0 -> 1; } }\n",
    );
    assert!(
        !out.pipeline_errors.iter().any(|e| e.code == "E0911"),
        "an explicit pre-wave @version must still permit retired syntax: {:?}",
        out.pipeline_errors
    );
}

/// The refusal has to TEACH. A file with no `@version` never bumped one, so
/// the pre-cutover wording ("@version was bumped too far") would send the
/// author hunting for a declaration that does not exist. The message must name
/// the directive, the wave, the automatic fix, and the way back into the
/// window.
#[test]
fn the_cutover_refusal_explains_itself() {
    let out = compile_source(".a { @scroll fade(scope: cover) { opacity: 0 -> 1; } }\n");
    let e = out
        .pipeline_errors
        .iter()
        .find(|e| e.code == "E0911")
        .expect("retired @scroll must be refused");

    assert!(
        e.message.contains("@scroll") && e.message.contains("2026-07-26"),
        "the message must name the directive and its wave: {:?}",
        e.message
    );
    assert!(
        !e.message.contains("bumped too far"),
        "no @version was declared, so nothing was bumped — that wording misleads: {:?}",
        e.message
    );

    let hint = e.hint.as_deref().unwrap_or_default();
    assert!(
        hint.contains("spacetime migrate"),
        "the hint must name the automatic fix: {hint:?}"
    );
    assert!(
        hint.contains("@version"),
        "the hint must show how to reopen the window for an in-progress migration: {hint:?}"
    );
    assert!(
        hint.contains("@on &.scroll"),
        "the hint must carry the capsule's own replacement guidance: {hint:?}"
    );
}

/// EVERY retired directive is refused — the cutover's actual contract.
///
/// One `%rewrite` slipping through is invisible otherwise: the file compiles,
/// the shim rewrites it in memory, and the only trace is a warning. This
/// enumerates the surfaces both capsules retire and requires a hard error for
/// each, so adding a directive to a capsule without closing its window fails
/// here instead of shipping.
#[test]
fn every_retired_directive_is_refused_after_the_cutover() {
    // (source, the directive it must be refused FOR)
    let cases: Vec<(String, &str)> = vec![
        (
            ".a { @scroll fade(scope: cover) { opacity: 0 -> 1; } }".into(),
            "@scroll",
        ),
        (".a { @load in(300ms) { opacity: 0 -> 1; } }".into(), "@load"),
        (
            ".a { @hover lift(200ms) { opacity: 1 -> 0.5; } }".into(),
            "@hover",
        ),
        (
            ".a { @click pop(150ms) { opacity: 1 -> 0.5; } }".into(),
            "@click",
        ),
        (
            ".a { @mouse tilt(100ms) { opacity: 1 -> 0.8; } }".into(),
            "@mouse",
        ),
        (
            ".a { @pointer track(100ms) { opacity: 1 -> 0.8; } }".into(),
            "@pointer",
        ),
        (
            ".a { @time tick(1s) { opacity: 0 -> 1; } }".into(),
            "@time",
        ),
        (
            ".a { @loop spin(2s, mode: bounce) { opacity: 0 -> 1; } }".into(),
            "@loop",
        ),
        (
            ".a { @value-change(duration: 300ms) { :entering { opacity: 0 -> 1; } \
             :exiting { opacity: 1 -> 0; } } }"
                .into(),
            "@value-change",
        ),
        (".a { @show(when: $loading) }".into(), "@show"),
        ("input.q { @input(bind: $q) }".into(), "@input"),
    ];

    let mut escaped = Vec::new();
    for (source, directive) in &cases {
        let out = compile_source(source);
        let refused = out
            .pipeline_errors
            .iter()
            .any(|e| e.code == "E0910" || e.code == "E0911");
        if !refused {
            escaped.push(format!(
                "{directive} still compiles (errors: {:?})",
                out.pipeline_errors
                    .iter()
                    .map(|e| e.code.as_str())
                    .collect::<Vec<_>>()
            ));
        }
    }

    assert!(
        escaped.is_empty(),
        "retired directives escaped the cutover:\n  {}",
        escaped.join("\n  ")
    );
}

/// BUG-345: the `as` alias DEFINES a signal.
///
/// `@on &.visible(600ms) as $reveal` names the driver's progress signal, read
/// back as `var(--st-reveal)`. E0408 did not know that, so it called the
/// recommended surface an undefined signal — landing the false positive
/// squarely on the shape the on-cutover migration steers people INTO. It broke
/// demos/starter-pack the moment that tutorial was migrated off the retired
/// positional-name form.
#[test]
fn an_as_bound_driver_signal_is_a_definition() {
    let out = compile_source(
        ".a { @on &.visible(600ms) as $reveal { opacity: 0 -> 1; } }\n\
         .b { opacity: var(--st-reveal, 0); }\n",
    );
    let e0408: Vec<_> = out
        .pipeline_errors
        .iter()
        .filter(|e| e.code == "E0408")
        .collect();
    assert!(
        e0408.is_empty(),
        "`as $reveal` defines signal `reveal` — reading it must not be E0408: {e0408:?}"
    );
}
