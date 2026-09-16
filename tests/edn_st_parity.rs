//! **The parity test** (PLAN-148 W3, SPEC §2 obligation P3).
//!
//! The whole zero-change guarantee rests on one claim: an EDN-origin program and
//! its `.st` twin reach the SAME `Vec<FormMatch>`, and therefore compile through
//! the IDENTICAL backend — no second pipeline, no divergence possible.
//!
//! This test attacks that claim at the only place it can be settled honestly:
//! the compiler's own output. Not "equivalent", not "looks the same" —
//! **byte-identical** `js` / `css` / `html`.
//!
//! If this passes, "EDN is a second concrete syntax over the same waist" is a
//! measured fact rather than a design intention.

use spacetime::edn::read_forms;
use spacetime::syntax::STDLIB_REGISTRY;

/// Compile a `.st` source and an EDN source, and assert their emitted artifacts
/// match byte for byte.
fn assert_parity(label: &str, st_src: &str, edn_src: &str) {
    // `.st` path — completely untouched by the EDN work.
    let st_ast = spacetime::parse(st_src)
        .unwrap_or_else(|e| panic!("[{label}] .st did not parse: {e:?}"));
    let st_out = spacetime::Compiler::from_ast(&st_ast).without_runtime().compile();

    // EDN path — reader → FormMatch, then the SAME compiler.
    let edn_matches = read_forms(edn_src, &STDLIB_REGISTRY)
        .unwrap_or_else(|e| panic!("[{label}] .edn did not read: {e}"));

    // Swap the EDN-derived matches into an otherwise-identical AST so the only
    // difference between the two runs is WHERE THE MATCHES CAME FROM.
    let mut edn_ast = st_ast.clone();
    edn_ast.matches = edn_matches;
    let edn_out = spacetime::Compiler::from_ast(&edn_ast).without_runtime().compile();

    assert_eq!(
        st_out.js, edn_out.js,
        "[{label}] JS differs between .st and .edn origins"
    );
    assert_eq!(
        st_out.css, edn_out.css,
        "[{label}] CSS differs between .st and .edn origins"
    );
    assert_eq!(
        st_out.html, edn_out.html,
        "[{label}] HTML differs between .st and .edn origins"
    );
}

#[test]
fn data_inline_parity() {
    // NB `:value` is an `Expr` capture holding the SOURCE TEXT of the
    // initialiser — for `: 0;` that text is `0`, carried byte-verbatim in both
    // directions (SPEC §8.2). A bare EDN string would decode to `String`, not
    // `Expr`, and emit different JS; the parity check caught exactly that in the
    // first version of this test.
    assert_parity(
        "data-inline",
        r#"@data inline $mcpPrompt : 0;"#,
        r#"(data-inline :name $mcpPrompt :value [:st/expr "0"])"#,
    );
}

#[test]
fn selector_scoped_parity() {
    // The `sel` wrapper must reconstruct the same scope binding a `.st` scope
    // block produces — `selector` is a FormMatch field, not a capture.
    assert_parity(
        "mcp-action in a selector scope",
        r#".kit-btn-confirm { @mcp-action(action: "kit-confirm", value: "confirm") }"#,
        r#"(sel ".kit-btn-confirm"
             (mcp-action :action "kit-confirm" :value "confirm"
                         :target "" :targetAttr "" :valueAttr "" :on "click"))"#,
    );
}

#[test]
fn positional_sugar_parity() {
    // Sugar must be a pure notation difference: same bytes out.
    assert_parity(
        "positional sugar",
        r#"@data inline $count : 0;"#,
        r#"(data-inline $count [:st/expr "0"])"#,
    );
}

/// The printer and the reader must agree: `.st` → FormMatch → `.st` → FormMatch
/// is stable, and the EDN spelling lands on the same value.
#[test]
fn print_read_round_trip_is_stable() {
    let st_src = r#"@data inline $mcpPrompt : "";"#;
    let ast = spacetime::parse(st_src).expect("parse");

    let printed = spacetime::edn::print_forms(&ast.matches, &STDLIB_REGISTRY).expect("print");
    let reparsed = spacetime::parse(&printed).expect("printed text must re-parse");

    assert_eq!(ast.matches.len(), reparsed.matches.len());
    assert_eq!(
        ast.matches[0].matched_macro,
        reparsed.matches[0].matched_macro
    );
    assert_eq!(ast.matches[0].captures, reparsed.matches[0].captures);
}

// ── W5: ingress parity ───────────────────────────────────────────────────────
//
// Every source-bearing MCP path converges on `put_source_with_entry`, which
// normalises EDN to `.st` BEFORE compiling. The guarantee to prove is twofold:
// `.st` is untouched, and EDN reaches the same program.

#[test]
fn st_ingress_is_byte_identical() {
    // The zero-change guarantee as an assertion: normalisation is the IDENTITY
    // on `.st`, so no existing user can be affected by EDN support existing.
    for src in [
        "@data inline $x : 0;\n",
        ".card { color: red; }\n",
        "%macro foo {\n  %form { @foo }\n}\n",
        "// leading comment\n@mcp-input;\n",
        "",
    ] {
        assert_eq!(
            spacetime::edn::normalize_source(src, &STDLIB_REGISTRY).unwrap(),
            src,
            "`.st` source must pass through normalisation unchanged"
        );
    }
}

#[test]
fn edn_ingress_compiles_to_the_same_bundle() {
    // W5's proof: the same function submitted as `.st` and as EDN produces
    // byte-identical artifacts, because EDN normalises to `.st` before the
    // compile step rather than taking a parallel path.
    let st_src = "@data inline $count : 0;\n";
    let edn_src = r#"(data-inline :name $count :value [:st/expr "0"])"#;

    let normalized =
        spacetime::edn::normalize_source(edn_src, &STDLIB_REGISTRY).expect("EDN must normalise");

    let st_out = spacetime::Compiler::from_ast(&spacetime::parse(st_src).unwrap())
        .without_runtime()
        .compile();
    let edn_out = spacetime::Compiler::from_ast(&spacetime::parse(&normalized).unwrap())
        .without_runtime()
        .compile();

    assert_eq!(st_out.js, edn_out.js, "JS differs across ingress paths");
    assert_eq!(st_out.css, edn_out.css, "CSS differs across ingress paths");
    assert_eq!(st_out.html, edn_out.html, "HTML differs across ingress paths");
}

#[test]
fn edn_detection_never_misfires_on_st() {
    // Detection is structural: `.st` starts with a directive, selector, `%` form
    // or comment — never with `(`, `[` or `{`. A misfire here would route real
    // `.st` through the EDN reader and break it, so this is the load-bearing
    // half of the zero-change guarantee.
    use spacetime::edn::SourceLang;
    for src in [
        "@import \"x\";",
        ".a { }",
        "#hero { }",
        "[data-x] { }",
        "> child { }",
        "%capture_type t { }",
        "/* c */ .a { }",
    ] {
        assert_eq!(
            spacetime::edn::detect(src),
            SourceLang::St,
            "misdetected as EDN: {src:?}"
        );
    }
}

// ── The printer must NOT be on the ingress path ──────────────────────────────
//
// W5 originally routed EDN through the printer: EDN → FormMatch → `.st` text →
// parse → FormMatch → compile. That is a DETOUR, and it put the printer's
// coverage on the critical path: a form the reader accepts but the printer
// cannot render became inexpressible in EDN, for a reason unrelated to meaning.
//
// It also contradicted the thesis. `pipeline::compile` consumes `Vec<FormMatch>`
// and that is precisely why EDN cannot diverge from `.st` — so EDN must reach
// the compiler as `FormMatch`, not as text.

#[test]
fn edn_ingress_does_not_depend_on_the_printer() {
    // `@stage` is the live case: the reader handles it, the printer cannot
    // (nested `Block` bodies), so under the text detour this EDN was rejected
    // with "could not be rendered as .st" — nonsense to an author who never
    // wrote `.st`.
    let edn = r#"(sel "canvas" (stage :fov 50))"#;

    let matches = spacetime::edn::read_forms(edn, &STDLIB_REGISTRY)
        .expect("the reader accepts this form");
    assert_eq!(matches.len(), 1);

    let file = spacetime::edn::to_st_file(edn, &STDLIB_REGISTRY).unwrap_or_else(|e| {
        panic!("EDN ingress must not require the printer, but failed: {e}")
    });

    assert_eq!(file.matches.len(), 1);
    assert_eq!(
        file.matches[0].matched_macro.as_deref(),
        Some("stage"),
        "the form must survive ingress with its identity intact"
    );
}

#[test]
fn edn_ingress_produces_the_same_matches_as_the_reader() {
    // Ingress must be a pure lift of the reader's output into a document — no
    // re-parse, no round trip, nothing that could change a capture on the way.
    let edn = r#"(data-inline :name $x :value [:st/expr "0"])"#;

    let from_reader = spacetime::edn::read_forms(edn, &STDLIB_REGISTRY).expect("read");
    let from_ingress = spacetime::edn::to_st_file(edn, &STDLIB_REGISTRY).expect("ingress");

    assert_eq!(from_ingress.matches.len(), from_reader.len());
    assert_eq!(from_ingress.matches[0].captures, from_reader[0].captures);
    assert_eq!(
        from_ingress.matches[0].matched_macro,
        from_reader[0].matched_macro
    );
}

#[test]
fn edn_reaches_the_compiler_through_the_bundle_path() {
    // The end-to-end assertion for the ingress fix: EDN submitted as SOURCE (the
    // shape every MCP path carries) compiles, and compiles to the same artifacts
    // as its `.st` twin. If the printer were still on this path, a `@stage` form
    // would fail here while the reader accepted it.
    let st_src = "@data inline $count : 0;\n";
    let edn_src = r#"(data-inline :name $count :value [:st/expr "0"])"#;

    let st_file = spacetime::parse(st_src).expect("parse .st");
    let edn_file = spacetime::edn::to_st_file(edn_src, &STDLIB_REGISTRY).expect("lift EDN");

    let st_out = spacetime::Compiler::from_ast(&st_file).without_runtime().compile();
    let edn_out = spacetime::Compiler::from_ast(&edn_file).without_runtime().compile();

    assert_eq!(st_out.js, edn_out.js);
    assert_eq!(st_out.css, edn_out.css);
    assert_eq!(st_out.html, edn_out.html);
}

// `a_printer_gap_no_longer_blocks_edn_authorship` lived here.
//
// It asserted that a form the PRINTER could not render was still authorable in
// EDN — the guarantee the ingress fix exists to provide. It selected a
// still-unprintable form dynamically and panicked when every candidate printed,
// with instructions to retire it rather than weaken it into a tautology.
//
// That moment arrived: the corpus reports 1491/1491 forms printed, 0 failing
// macros. There is no unprintable form left to build the fixture from, so the
// guarantee is now carried by `edn_ingress_does_not_depend_on_the_printer`
// (ingress never calls the printer, by construction) and by the corpus gate
// itself (`EDN_CORPUS_STRICT=1`), which fails if any form stops printing.
