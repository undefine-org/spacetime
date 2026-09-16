/// BUG-225: a mutation whose string literal contains a `;` was severed
/// mid-literal, because the mutation statement list was split on `;` with no
/// string awareness.
///
/// Root cause: `CaptureType::MutationActions` in
/// `src/syntax/events/form_compiler.rs` built `js_statements` with a bare
/// `inner.split(';')`. A `;` inside a quoted value is CONTENT, but the naive
/// split treated it as a separator, so
///   `$v <- "false for nobody; every owner wants control";`
/// reached the runtime as two fragments — `$v <- "false for nobody` and
/// `every owner wants control"` — and the eval threw
/// `SyntaxError: Invalid or unexpected token`. Two sibling split sites in the
/// `@on` validator had the same flaw, plus `lower_mutation_body_text` in
/// `src/pipeline/mod.rs` (which additionally must stay byte-lossless because
/// it rejoins).
///
/// Impact: SILENT runtime failure behind a GREEN compile. `cargo run -- check`
/// passed, the handler registered, the element initialised — and the click did
/// nothing. Only a browser console revealed it. Any `.st` writing natural
/// English prose in a mutation is exposed, since semicolons and apostrophes are
/// unavoidable in real copy. Found while building `method/05-COPY.st.md`, whose
/// lint-verdict button sets exactly such a string.
///
/// Fix: `split_semicolons(text, raw)` — a quote-and-escape-aware scanner
/// tracking `"`, `'` and backtick — exposed as
/// `split_statements_outside_strings` (trimmed, empties dropped) and
/// `split_statements_outside_strings_raw` (verbatim, `join(";")` round-trips).
/// The `@exports` split was deliberately left naive: it parses identifiers, not
/// strings.
///
/// This test PINS the runtime behaviour the unit tests cannot reach: that the
/// mutation actually executes and the WHOLE string arrives intact.

@import "stdlib/testing/test"

@data inline $verdict : "idle";
@data inline $multi : "idle";
@data inline $counter number : 0;

// A semicolon inside the literal — the exact BUG-225 repro.
.b225-semi {
  @on &.click { $verdict <- "false for nobody; every owner wants control"; }
}

// An apostrophe inside a double-quoted literal: a lone `'` must not open a
// string and swallow the rest.
.b225-apos {
  @on &.click { $verdict <- "the promise that lives in nobody's record"; }
}

// Both, plus a nested single-quoted phrase containing a semicolon.
.b225-both {
  @on &.click { $verdict <- "G3 inversion: 'lose control; of your business' is absurd"; }
}

// Genuine multi-statement mutation where an EARLIER statement's string holds a
// semicolon — proves real separators still separate after the fix.
.b225-multi {
  @on &.click { $multi <- "a; b; c"; $counter <- $counter + 1; }
}

.b225-out { text <- $verdict; }
.b225-multi-out { text <- $multi; }
.b225-count { text <- $counter; }

@test "semicolon inside a mutation string literal survives intact (BUG-225)" {
    @fixture {
        <div class="b225">
          <button class="b225-semi">semi</button>
          <span class="b225-out"></span>
        </div>
    }
    @wait 60ms
    @eval (document.querySelector('.b225-semi').click())
    @wait 80ms
    @let (t = document.querySelector('.b225-out').textContent)
    @assert (t === 'false for nobody; every owner wants control')
        "the whole literal must arrive, semicolon included (got: "
}

@test "apostrophe inside a double-quoted mutation string survives (BUG-225)" {
    @fixture {
        <div class="b225">
          <button class="b225-apos">apos</button>
          <span class="b225-out"></span>
        </div>
    }
    @wait 60ms
    @eval (document.querySelector('.b225-apos').click())
    @wait 80ms
    @let (t = document.querySelector('.b225-out').textContent)
    @assert (t === "the promise that lives in nobody's record")
        "a lone apostrophe must not truncate the literal (got: "
}

@test "nested quotes plus semicolon survive together (BUG-225)" {
    @fixture {
        <div class="b225">
          <button class="b225-both">both</button>
          <span class="b225-out"></span>
        </div>
    }
    @wait 60ms
    @eval (document.querySelector('.b225-both').click())
    @wait 80ms
    @let (t = document.querySelector('.b225-out').textContent)
    @assert (t.indexOf("'lose control; of your business'") !== -1)
        "a single-quoted phrase containing a semicolon must survive (got: "
    @assert (t.indexOf('G3 inversion') === 0)
        "the literal must not be truncated at its start (got: "
}

@test "real separators still separate when an earlier string holds a semicolon (BUG-225)" {
    @fixture {
        <div class="b225">
          <button class="b225-multi">multi</button>
          <span class="b225-multi-out"></span>
          <span class="b225-count"></span>
        </div>
    }
    @wait 60ms
    @eval (document.querySelector('.b225-multi').click())
    @wait 80ms
    @let (m = document.querySelector('.b225-multi-out').textContent)
    @let (c = document.querySelector('.b225-count').textContent)
    @assert (m === 'a; b; c')
        "the string's internal semicolons are content, not separators (got: "
    @assert (c === '1')
        "the SECOND statement must still run — the real separator still separates (got: "
}
