//! Integration tests for CMHT-006: JS Emit Snapshot Tests
//!
//! These tests capture the JS output of the component model codegen and assert
//! on specific strings to catch regressions. Each test compiles a minimal
//! @template fixture and verifies the emitted JS contains the expected
//! runtime wiring calls (ST.set, classList.toggle, addEventListener, etc.).

use spacetime::compiler::CompileOptions;
use spacetime::{compile, parse};

/// Compile a .st snippet and return the JS output.
fn compile_st(input: &str) -> String {
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    compiled.js
}

// =============================================================================
// Test 1: State Initialization — ST.set calls
// =============================================================================

/// `$count number: 0;` inside a @template body should emit ST.set for state init
#[test]
fn test_js_emit_state_initialization() {
    let input = r#"
@template &state-init-test() {
    <div class="counter"></div>
    $count number: 0;
}
"#;

    let js = compile_st(input);

    assert!(
        js.contains("ST.set("),
        "JS should contain ST.set( for state initialization.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
    assert!(
        js.contains("count"),
        "JS should reference the state variable name 'count'.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
    assert!(
        js.contains("0"),
        "JS should contain the initial value 0.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
}

// =============================================================================
// Test 2: Class Toggle — classList.toggle wiring
// =============================================================================

/// `.active: $open;` inside a @template body should emit classList.toggle via ST.watch
#[test]
fn test_js_emit_class_toggle() {
    let input = r#"
@template &toggle-test() {
    <div class="panel"></div>
    $open bool: false;
    .active: $open;
}
"#;

    let js = compile_st(input);

    assert!(
        js.contains("classList.toggle"),
        "JS should contain classList.toggle for class toggle wiring.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
    assert!(
        js.contains("active"),
        "JS should reference the class name 'active'.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
    assert!(
        js.contains("open"),
        "JS should reference the var name 'open'.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
}

// =============================================================================
// Test 3: Event Handler — addEventListener wiring
// =============================================================================

/// `@on click { $open <- !$open; }` should emit addEventListener with 'click'
#[test]
fn test_js_emit_event_handler() {
    let input = r#"
@template &click-test() {
    <button class="btn">Click</button>
    $open bool: false;
    @on &.click { $open <- !$open; }
}
"#;

    let js = compile_st(input);

    assert!(
        js.contains("addEventListener"),
        "JS should contain addEventListener for event handler wiring.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
    assert!(
        js.contains("click"),
        "JS should reference the event name 'click'.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
}

// =============================================================================
// Test 4: Content Binding — textContent wiring
// =============================================================================

/// `.display { text <- $count; }` should emit textContent binding via ST.watch
#[test]
fn test_js_emit_content_binding() {
    let input = r#"
@template &binding-test() {
    <div class="wrapper">
        <span class="display">0</span>
    </div>
    $count number: 0;
    .display {
        text <- $count;
    }
}
"#;

    let js = compile_st(input);

    assert!(
        js.contains("textContent"),
        "JS should contain textContent for content binding.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
    assert!(
        js.contains("count"),
        "JS should reference the var name 'count'.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
}

// =============================================================================
// Test 5: Named Ref Storage — ST.set for ref
// =============================================================================

/// `&myRef &other-template()` should emit ST.set for named ref storage
#[test]
fn test_js_emit_named_ref() {
    let input = r#"
@template &child-widget() {
    <div class="child"></div>
}
@template &parent-test() {
    <div class="parent"></div>
    &myRef &child-widget();
}
"#;

    let js = compile_st(input);

    assert!(
        js.contains("ST.set("),
        "JS should contain ST.set( for named ref storage.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
    assert!(
        js.contains("myRef"),
        "JS should reference the ref name 'myRef'.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
}

// =============================================================================
// Test 6: Export Metadata — __stExports
// =============================================================================

/// `@exports { $count }` should emit __stExports metadata on the root element
#[test]
fn test_js_emit_export_metadata() {
    let input = r#"
@template &export-test() {
    <div class="exported"></div>
    $count number: 0;
    @exports { $count }
}
"#;

    let js = compile_st(input);

    assert!(
        js.contains("__stExports"),
        "JS should contain __stExports for export metadata.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
    assert!(
        js.contains("count"),
        "JS should reference the exported var name 'count'.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );
}

// =============================================================================
// BUG-135: a comment inside an @handle { } body must NOT drop its clauses
// =============================================================================

/// A `/* ... */` comment ANYWHERE inside a `@handle $sig { ... }` body must be
/// treated as trivia (parity with whitespace) - it must NOT desync the optional
/// `( $optimistic )? ( $receive )? ( $final )?` clause-group matching. Before the
/// fix, the leading comment made the `optimistic` LiteralExtractor fail at token
/// 0, the Optional group consumed nothing, and EVERY clause emitted empty, so the
/// handler silently did nothing. We assert the optimistic statement text
/// (`col:` from `{ col: $to }`) AND the final clause (`dragging`) survive when a
/// comment leads the body - and that this matches the comment-free emission.
const BUG135_SIGNAL: &str = r#"
@data signal $move(card string, to string) to $board {
  send emit "move" { card: $card, to: $to }
  receive to MoveResult {
    "ok" => Moved($.payload as Card);
    _    => Failed($.payload);
  }
}
"#;

fn bug135_handle(body: &str) -> String {
    format!(
        "{}\n.board {{\n  @handle $move {{\n{}\n  }}\n}}\n",
        BUG135_SIGNAL, body
    )
}

#[test]
fn test_js_emit_handle_comment_does_not_drop_clauses() {
    let clauses = "    optimistic { $cards.update($card, { col: $to }); }\n     receive { Moved(c) => { $cards <- c; } }\n     final { $dragging <- null; }";

    let no_comment = compile_st(&bug135_handle(clauses));
    let with_comment = compile_st(&bug135_handle(&format!(
        "    /* this comment must be ignored (BUG-135) */\n{}",
        clauses
    )));

    // The optimistic clause's object literal `{ col: $to }` must reach the JS.
    assert!(
        with_comment.contains("col:"),
        "a leading comment dropped the optimistic clause (no `col:` in emit).\nJS:\n{}",
        &with_comment[..with_comment.len().min(2000)]
    );
    // The final clause's `$dragging <- null` must reach the JS.
    assert!(
        with_comment.contains("dragging"),
        "a leading comment dropped the final clause (no `dragging` in emit).\nJS:\n{}",
        &with_comment[..with_comment.len().min(2000)]
    );
    // The receive arm `Moved(c)` must reach the JS.
    assert!(
        with_comment.contains("Moved"),
        "a leading comment dropped the receive clause (no `Moved` arm in emit).\nJS:\n{}",
        &with_comment[..with_comment.len().min(2000)]
    );

    // Comment-presence must be behavior-neutral: the three clause markers occur
    // the SAME number of times with and without the comment.
    for needle in ["col:", "dragging", "Moved"] {
        assert_eq!(
            with_comment.matches(needle).count(),
            no_comment.matches(needle).count(),
            "clause marker `{}` count differs with vs without a body comment (BUG-135)",
            needle
        );
    }
}

/// A comment INTERLEAVED between clauses (not just leading) must also be trivia.
#[test]
fn test_js_emit_handle_interleaved_comment_keeps_all_clauses() {
    let body = "    optimistic { $cards.update($card, { col: $to }); }\n     // between optimistic and receive\n     receive { Moved(c) => { $cards <- c; } }\n     /* between receive and final */\n     final { $dragging <- null; }";

    let js = compile_st(&bug135_handle(body));
    assert!(
        js.contains("col:"),
        "interleaved comment dropped optimistic clause"
    );
    assert!(
        js.contains("Moved"),
        "interleaved comment dropped receive clause"
    );
    assert!(
        js.contains("dragging"),
        "interleaved comment dropped final clause"
    );
}
