//! Recursive `Block` printing — the `@stage` case (PLAN-148, item #2).
//!
//! `CapturedValue::Block(Vec<FormMatch>)` holds nested forms: a `@stage` body is
//! `@particles`, `@scroll-3d`, `@post` — each a `FormMatch` in its own right.
//! The printer refused these rather than emit a stage with an empty body, which
//! would have been a DIFFERENT program (all the children silently deleted).
//!
//! This was genuinely blocked until the EDN reader existed, because printing a
//! nested form is the same problem as printing a top-level one and the two must
//! not drift into separate implementations.
//!
//! NB `@stage` is no longer blocking EDN AUTHORSHIP — ingress lifts EDN straight
//! to `FormMatch` and never consults the printer (see
//! `edn_st_parity::a_printer_gap_no_longer_blocks_edn_authorship`). What is at
//! stake here is the `.st` ⟷ EDN round trip and the corpus thesis test, i.e.
//! whether the registry can really print everything it can match.

use spacetime::syntax::{CapturedValue, STDLIB_REGISTRY};

const NESTED: &str = r#"
@import "stdlib/3d";

canvas {
    @stage(camZ: 6.5, tone: "aces", exposure: 0.9) {
        @particles(count: 30000, size: 0.018, radius: 2.2, spin: 0.05)
        @scroll-3d(drive: "morph", to: 3, ease: 0.06)
    }
}
"#;

fn stage_match(ast: &spacetime::parser::StFile) -> &spacetime::syntax::FormMatch {
    ast.matches
        .iter()
        .chain(ast.scopes.iter().flat_map(|s| s.matches.iter()))
        .find(|m| m.matched_macro.as_deref() == Some("stage"))
        .expect("the fixture declares a @stage")
}

#[test]
fn the_fixture_really_nests_forms() {
    // Precondition: if `children` were empty this test would prove nothing.
    let ast = spacetime::parse(NESTED).expect("fixture must parse");
    let m = stage_match(&ast);

    match m.captures.get("children") {
        Some(CapturedValue::Block(children)) => {
            assert!(
                !children.is_empty(),
                "the @stage body must carry its child forms, found an empty Block"
            );
        }
        other => panic!("expected a Block body, found {other:?}"),
    }
}

#[test]
fn a_nested_block_prints_its_children() {
    let ast = spacetime::parse(NESTED).expect("fixture must parse");
    let m = stage_match(&ast);

    let printed = spacetime::edn::print_form(m, &STDLIB_REGISTRY)
        .expect("a form with a nested Block body must print");

    assert!(printed.contains("@stage"), "{printed}");
    assert!(
        printed.contains("particles"),
        "the children must appear in the printed body, not be silently dropped:\n{printed}"
    );
    assert!(printed.contains("scroll-3d"), "{printed}");
}

#[test]
fn a_printed_nested_block_re_parses_to_the_same_children() {
    // The round-trip bar: printing then re-parsing must recover the same nested
    // forms, or the body is being reshaped rather than reproduced.
    let ast = spacetime::parse(NESTED).expect("fixture must parse");
    let before = stage_match(&ast);

    let printed = spacetime::edn::print_file(&ast, &STDLIB_REGISTRY).expect("print the file");
    let reparsed = spacetime::parse(&printed)
        .unwrap_or_else(|e| panic!("printed text must re-parse:\n{printed}\n{e:?}"));

    let after = stage_match(&reparsed);

    let names = |m: &spacetime::syntax::FormMatch| match m.captures.get("children") {
        Some(CapturedValue::Block(cs)) => cs
            .iter()
            .map(|c| c.matched_macro.clone().unwrap_or_else(|| c.macro_name.clone()))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    assert_eq!(
        names(before),
        names(after),
        "the nested children changed across the round trip\n{printed}"
    );
}

#[test]
fn an_empty_block_body_still_prints() {
    // A stage with no children is legal and must not be confused with a stage
    // whose children failed to render.
    let src = "canvas { @stage(fov: 50) { } }\n";
    let ast = spacetime::parse(src).expect("fixture must parse");
    let m = stage_match(&ast);

    let printed = spacetime::edn::print_form(m, &STDLIB_REGISTRY)
        .expect("an empty Block body must print");
    assert!(printed.contains("@stage"), "{printed}");
}

/// A zero-parameter `@template` must print its `()`.
///
/// `%form { @template &$name:ident($params:param_list) { … } }` spells the
/// parens as LITERALS, and `param_list` documents `() -> []` — so a template
/// with no parameters still needs them. The printer used to suppress an empty
/// param render as "noise the author never wrote", which is right for a form
/// whose params are all defaulted-and-absent and wrong here.
///
/// The failure was quiet, which is what makes it worth a test: the printed
/// `@template &shell { … }` still PARSES, it just yields zero matches. The
/// declaration disappears while the file looks fine, and the loss only surfaces
/// later as a template nothing declared. Found by round-tripping a generated
/// page and counting the forms that came back.
#[test]
fn a_zero_param_template_keeps_its_parens() {
    let src = "@template &shell() { <main class=\"chat\"></main> }\n";
    let ast = spacetime::parse(src).expect("fixture must parse");
    assert_eq!(ast.matches.len(), 1, "fixture must yield the template form");

    let printed = spacetime::edn::print_file(&ast, &STDLIB_REGISTRY).expect("print the file");
    assert!(
        printed.contains("&shell()"),
        "the empty parens are structural, not noise:\n{printed}"
    );

    // The property that actually matters: it survives a round trip.
    let back = spacetime::parse(&printed).expect("printed output must re-parse");
    assert_eq!(
        back.matches.len(),
        1,
        "the template declaration vanished across the round trip:\n{printed}"
    );
}

/// A `@data signal` must print its send/receive body.
///
/// The `%form` names `$send:send_clause` and `$receive:receive_block`, but
/// parsing FLATTENS those: `send_clause`'s captures (`verb`, `target`) and
/// `receive_block`'s (`sum`, `arms`) land at the top level of the match. A
/// printer that only looks up `send` by name finds nothing and emits
///
///     @data signal $inc() to $counter;
///
/// which is not the same program — it sends nothing and decodes nothing. It
/// also does not re-parse: the form's own grammar rejects it with E0946
/// ("expected body block") and E0804 ("missing required parameter 'sum'").
///
/// Found by rendering a GENERATED page and asking the compiler to check it —
/// the errors pointed at the emitter, but the reference page `counter.st`
/// round-tripped to the same empty signal, which located the defect here.
#[test]
fn a_signal_keeps_its_send_and_receive_body() {
    let src = concat!(
        "@import \"stdlib/macros/data-kind\"\n",
        "@import \"stdlib/macros/host\"\n",
        "@host $counter : live(\"App.CounterLive\")\n",
        "@data signal $inc() to $counter {\n",
        "  send emit \"inc\"\n",
        "  receive to IncResult { \"ok\" => Bumped($.reply as number); _ => Failed($.reply); }\n",
        "  policy latest\n",
        "}\n",
    );

    let ast = spacetime::parse(src).expect("fixture must parse");
    let printed = spacetime::edn::print_file(&ast, &STDLIB_REGISTRY).expect("print the file");

    assert!(
        printed.contains("send emit"),
        "the signal lost its send clause:\n{printed}"
    );
    assert!(
        printed.contains("IncResult"),
        "the signal lost its receive block:\n{printed}"
    );
    assert!(
        printed.contains("Bumped"),
        "the signal lost its decode arms:\n{printed}"
    );

    // The property that matters: the printed signal is still a signal.
    let back = spacetime::parse(&printed).expect("printed output must re-parse");
    let signals = back
        .matches
        .iter()
        .filter(|m| m.matched_macro.as_deref() == Some("data-signal"))
        .count();
    assert_eq!(signals, 1, "the signal did not survive the round trip:\n{printed}");
}

/// The flattened-capture reading must not INVENT syntax.
///
/// `on_mutation` is `"$" $target:ident "<-" $expr:expr`. Descending into it
/// whenever `$mut` is absent rendered its literals against unrelated data and
/// printed a bare `$<-;` — a mutation with no target and no expression — into
/// every `@on` body that held only a motion line. Five corpus files regressed
/// on exactly this before the presence check went in.
#[test]
fn an_on_body_with_only_a_motion_line_prints_that_line() {
    let src = concat!(
        "@import \"stdlib/macros/on\"\n",
        ".hero { @on &.visible(420ms) { opacity: 0 -> 1; } }\n",
    );

    let ast = spacetime::parse(src).expect("fixture must parse");
    let printed = spacetime::edn::print_file(&ast, &STDLIB_REGISTRY).expect("print the file");

    assert!(
        printed.contains("opacity"),
        "the motion line was dropped:\n{printed}"
    );
    assert!(
        !printed.contains("$<-"),
        "a mutation was invented from an absent capture:\n{printed}"
    );

    spacetime::parse(&printed).expect("printed output must re-parse");
}

/// A template argument that names a binding must print as the binding.
///
/// `TemplateInvocation` stores its args as strings, so `&bubble($m)` arrives as
/// `String("$m")`. The general rule for a string — quote it — is right
/// everywhere else and wrong here: it printed `&bubble("$m")`, which re-parses
/// happily as a string LITERAL. The template then received the two characters
/// `$m` instead of the loop item, `$m.role` resolved to nothing, and `@each`
/// rendered zero rows.
///
/// Nothing failed. The page compiled, the bundle built, and the screenshot came
/// back a plausible 76 KB with an empty conversation. Found by requiring a
/// rendered selector in the DOM rather than trusting the image.
#[test]
fn a_template_argument_binding_is_not_quoted() {
    let src = concat!(
        "@import \"stdlib/macros/each\"\n",
        "@data inline $rows : [1, 2];\n",
        "@template &row($m) { <li>`$m.text`</li> }\n",
        ".log { @each($rows as $m) { &row($m); } }\n",
    );

    let ast = spacetime::parse(src).expect("fixture must parse");
    let printed = spacetime::edn::print_file(&ast, &STDLIB_REGISTRY).expect("print the file");

    assert!(
        printed.contains("&row($m)"),
        "the loop item was passed as a string literal:\n{printed}"
    );
    assert!(
        !printed.contains("&row(\"$m\")"),
        "the binding was quoted into a literal:\n{printed}"
    );

    spacetime::parse(&printed).expect("printed output must re-parse");
}
