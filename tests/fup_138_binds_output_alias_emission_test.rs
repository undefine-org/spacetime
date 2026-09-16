//! FUP-138 — a `%binds` output alias must reach the emitted `ST.set` key.
//!
//! `%yield expr -> $export` inside a primitive writes an element signal. The name
//! it writes is the ALIAS declared in the consuming `%binds`
//! (`$status as $insWriteStatus`), because that is the name the page reads. The
//! remap is threaded resolve → PrimitiveArgs.outputs → `ctx.with_output` (BUG-154).
//!
//! That chain was intact; the ENTRY POINT was not. `parse_bind_outputs` split its
//! list on `,` alone, and the bind-line accumulator upstream joins physical lines
//! with a space — so a one-per-line output block arrived as a single run and
//! parsed as ONE output whose alias was the rest of the text. A non-plain-ident
//! alias fails resolve's gate, no remap is recorded, and `%yield` falls back to
//! the RAW export name. Every export after the first was silently dead.
//!
//! Unit tests at the parser and resolver layers each passed while this was broken,
//! because neither ran the whole path. This test asserts the OBSERVABLE end: what
//! key actually appears in the emitted JS.

use spacetime::compiler::Compiler;

/// Compile a source string and return the emitted JS.
fn emit_js(src: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, src).expect("write entry");
    Compiler::from_file(&entry, dir.path())
        .unwrap_or_else(|e| panic!("compile failed: {e}"))
        .compile()
        .js
}

const PRIMITIVE: &str = r#"
%primitive two-exports(&element) {
  %emit js {
    (function() {
      const el = %&element;
      let statusText = 'Saved';
      let errorText = 'nope';
      %yield statusText -> $status;
      %yield errorText -> $error;
    })();
  }
  %exports {
    $status: string
    $error: string
  }
}
"#;

#[test]
fn one_per_line_binds_outputs_emit_both_aliases() {
    let src = format!(
        r#"{PRIMITIVE}
%macro two-exports-macro {{
  %form {{ @two-exports }}
  %binds {{
    two-exports(&self) -> {{
      $status as $myStatus
      $error as $myError
    }}
  }}
}}

.host {{ @two-exports }}
<div class="host"></div>
"#
    );
    let js = emit_js(&src);

    assert!(
        js.contains(r#""myStatus""#),
        "the FIRST alias must reach the emitted ST.set key"
    );
    assert!(
        js.contains(r#""myError""#),
        "the SECOND alias must too — dropping it is the FUP-138 defect: the output \
         list was split on ',' only, so a one-per-line block parsed as ONE entry \
         and every export after the first wrote its RAW name"
    );
    assert!(
        !js.contains(r#"ST.set(el, "status""#),
        "the raw export name must NOT be written once an alias is declared — a page \
         reading the alias would see nothing"
    );
}

#[test]
fn comma_and_newline_binds_output_forms_emit_identically() {
    let forms = [
        "$status as $myStatus, $error as $myError",
        "$status as $myStatus,\n      $error as $myError",
        "$status as $myStatus\n      $error as $myError",
    ];
    for form in forms {
        let src = format!(
            r#"{PRIMITIVE}
%macro two-exports-macro {{
  %form {{ @two-exports }}
  %binds {{
    two-exports(&self) -> {{ {form} }}
  }}
}}

.host {{ @two-exports }}
<div class="host"></div>
"#
        );
        let js = emit_js(&src);
        assert!(
            js.contains(r#""myStatus""#) && js.contains(r#""myError""#),
            "separator form must not change the emitted keys, but this one did:\n{form}"
        );
    }
}
