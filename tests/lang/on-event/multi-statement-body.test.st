/// A multi-statement `@on` body runs EVERY statement.
///
/// This was broken for every `@on` body with more than one statement — two
/// plain writes included — and nothing in the suite covered it.
///
/// The compiler joins a driver body with `"; "` (drivers.rs `mutations.join`).
/// `ST.runMutations` normalized a plain-string body as `js_statements: [body]`
/// — ONE statement — so the joined text met the `$x <- expr` regex, whose `.+`
/// is greedy, and the entire remainder was swallowed as the FIRST statement's
/// right-hand side. `$a <- 1; $b <- 2;` assigned `1; $b <- 2` to `$a` and
/// never touched `$b`.
///
/// Nothing errored. The handler fired, the first write appeared to work, and
/// every statement after it vanished — exactly the silent-drop class the
/// string-normalization comment above it was written to prevent.
///
/// Fix: `ST._splitStatements` splits at TOP-LEVEL `;` only, so a semicolon
/// inside a string literal or a bracketed group stays with its statement.

@import "stdlib/testing/test"

@test "two writes in one body both land" {
  @fixture { <button class="ms-two">go</button> }
  @given .ms-two { @on &.click { $msA <- 1; $msB <- 2; } }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.msA = 0)
  @eval (window.SpacetimeLocal.msB = 0)

  @when .ms-two click
  @wait 50ms
  @assert (window.SpacetimeLocal.msA === 1)
    "the first write must land as 1, not as the rest of the body";
  @assert (window.SpacetimeLocal.msB === 2)
    "the second write must not be swallowed by the first";
}

@test "three writes all land" {
  @fixture { <button class="ms-three">go</button> }
  @given .ms-three { @on &.click { $msX <- 1; $msY <- 2; $msZ <- 3; } }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.msX = 0)
  @eval (window.SpacetimeLocal.msY = 0)
  @eval (window.SpacetimeLocal.msZ = 0)

  @when .ms-three click
  @wait 50ms
  @assert (window.SpacetimeLocal.msX === 1) "first";
  @assert (window.SpacetimeLocal.msY === 2) "second";
  @assert (window.SpacetimeLocal.msZ === 3) "third — the tail must not be lost";
}

/// The splitter must not cut a `;` that belongs to a string literal.
@test "a semicolon inside a string stays inside it" {
  @fixture { <button class="ms-str">go</button> }
  @given .ms-str { @on &.click { $msMsg <- "a; b"; $msAfter <- 9; } }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.msMsg = "")
  @eval (window.SpacetimeLocal.msAfter = 0)

  @when .ms-str click
  @wait 50ms
  @assert (window.SpacetimeLocal.msMsg === "a; b")
    "the quoted semicolon must survive — not split the statement in half";
  @assert (window.SpacetimeLocal.msAfter === 9)
    "the statement after a quoted semicolon must still run";
}
