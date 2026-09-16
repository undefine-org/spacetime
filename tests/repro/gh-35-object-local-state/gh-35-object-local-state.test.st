/// GH-35 / BUG-342: an object-typed local-state initializer must WORK.
///
///   body { $user object: { name: "Ada" }; }
///   .card { text <- $user.name; }
///
/// History, because this gate's polarity flipped twice and the reason matters:
///
///  1. Originally the object initializer was SILENTLY ELIDED — the `$value`
///     capture stopped before the `{ … }`, the leftover body was discarded, and
///     the consumer still emitted a read that produced `undefined` at runtime
///     with zero diagnostics.
///  2. That was then made a REFUSAL (E0946) rather than a silent wrong answer —
///     the right interim move, and this file's older text plus a
///     `check-contract.sh` asserted exactly that diagnostic.
///  3. BUG-342 (`a0e3aa0d`) fixed it for real: `variable_ref` now uses the same
///     `starts_object_literal` guard `directive()` already had, so a `{` in
///     VALUE position is an object literal instead of a body.
///
/// After step 3 the E0946 contract became FALSE — the script asserting it was
/// failing against CORRECT code, which is the exact failure mode a refusal-gate
/// has when the feature lands: it pins the workaround, not the contract. So the
/// gate is now behavioral (does "Ada" RENDER), which is true in step 3 and
/// false in both step 1 and step 2.
///
/// The scalar control stays: it proves the mount + binding + assert harness
/// works, so a red object gate means the object shape, not the harness.
@import "stdlib/testing/test"

@test "object-typed local state renders a field" {
  @mount {
    <div class="card"></div>
    body {
      $user object: { name: "Ada" };
    }
    .card { text <- $user.name; }
  }
  @flush
  @then .card should have_text "Ada"
}

@test "a quoted-key object literal renders the same" {
  // `{ "name": … }` (JSON-shaped) and `{ name: … }` (CSS-native) are two
  // spellings of one value; they must not disagree.
  @mount {
    <div class="qcard"></div>
    body {
      $quoted object: { "name": "Ada" };
    }
    .qcard { text <- $quoted.name; }
  }
  @flush
  @then .qcard should have_text "Ada"
}

@test "scalar local-state control renders true (harness works)" {
  @mount {
    <div class="flag"></div>
    body {
      $flag bool: true;
    }
    .flag { text <- $flag; }
  }
  @flush
  @then .flag should have_text "true"
}

@test "a css declaration body is still a body, not an object value" {
  // The guard that made the object literal work must not have turned every
  // `{ ident: value; … }` into a value — that shape is a scope body and is
  // by far the more common one. This is the over-widening companion.
  @mount {
    <div class="scoped"></div>
    .scoped { color: red; padding: 4px; }
  }
  @flush
  @then .scoped should exist
}
