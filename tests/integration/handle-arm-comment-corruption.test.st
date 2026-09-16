/// `@handle` receive-arm-body comment corruption (BUG-183 / BUG-100) regression tests.
///
/// Found via a live user report + bisection: a comment placed INSIDE an
/// `@handle` receive arm's `{ ... }` body could silently corrupt or null the
/// WHOLE arm's compiled output -- no diagnostic, no compile error, the arm's
/// effect statements simply never ran at runtime (or ran truncated/garbled).
/// Two independent triggers: a bare apostrophe alone inside such a comment
/// nulled the arm entirely; a bare double-colon alone spliced the comment
/// text into the arm's own JSON-like intermediate representation as bogus
/// object keys/values, truncating the real statement body.
///
/// FIXED (BUG-100): root-caused to TWO independent bugs, both in
/// `src/syntax/events/extractors/custom.rs`:
///   1. `BalancedExtractor`'s byte-scanner (the machinery behind `balanced(')`
///      captures like `handle_arm`'s `$body:balanced('}')`) had zero
///      comment-awareness -- a `'` or `::` inside a `//`/`/* */` comment was
///      scanned as if it could open a string literal or matter structurally,
///      desyncing the scan. Fixed by skipping whole `//` and `/* */` comments
///      before any quote/bracket/delimiter char is considered.
///   2. Even with (1) fixed, `handle_arm`'s captured `body`/`stmt` field
///      stayed a raw `CapturedValue::Expr` -- codegen's Expr branch splices
///      `Expr` text UNQUOTED into the generated JS (assuming it's already
///      valid inline JS), which is fine for typical arm text but corrupts the
///      surrounding object-literal codegen once that text contains a genuine
///      `//`/`/* */` comment (the comment eats the rest of the emitted line).
///      Fixed via a `handle_arm` reifier that retags `body`/`stmt` as
///      `CapturedValue::String`, so codegen quotes+escapes it as opaque
///      runtime text (which the `signal-handler` primitive splits on `;` and
///      interprets at runtime anyway -- it was never meant to be raw JS).
///
/// The first two `@test`s below pin the pre-fix WORKAROUND (comments ABOVE
/// the `@handle` block) as still correct. The third `@test` is the actual fix
/// pin: a comment INSIDE an arm body, containing BOTH trigger characters,
/// with a trailing statement after it -- this must fire the WHOLE arm.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"
@import "stdlib/macros/host"
@import "stdlib/macros/data-kind"
@import "stdlib/macros/handle"

@host $testApi : http("https://handle-arm-comment-test.invalid")

@data signal $withApostropheComment() to $testApi {
  send GET "/a"
  receive to ResultA {
    200 => Ok($.body);
    _   => Failed($.statusText);
  }
  policy latest
}

@data signal $withDoubleColonComment() to $testApi {
  send GET "/b"
  receive to ResultB {
    200 => Ok($.body);
    _   => Failed($.statusText);
  }
  policy latest
}

@data signal $withCommentInsideArm() to $testApi {
  send GET "/c"
  receive to ResultC {
    200 => Ok($.body);
    _   => Failed($.statusText);
  }
  policy latest
}

// Regression pin for BUG-183's apostrophe trigger: a comment containing a
// bare apostrophe, if placed INSIDE the receive arm body below (the way an
// earlier version of stdlib/__host__/index.st's own $startLogin handler
// did), would null this whole arm. The comment lives ABOVE the block
// instead -- the verified-safe workaround this session established.
@handle $withApostropheComment {
  receive {
    Ok(body) => { $apostropheArmRan <- true; }
    Failed(_) => { }
  }
}

// Regression pin for BUG-183's double-colon trigger (a SEPARATE corruption
// shape from the apostrophe one -- confirmed via live bisection this
// session that each reproduces independently). Comment kept above, not
// inside, for the same reason.
@handle $withDoubleColonComment {
  receive {
    Ok(body) => { $doubleColonArmRan <- true; }
    Failed(_) => { }
  }
}

// FIX pin (BUG-183 / BUG-100): the comment lives INSIDE the receive arm body
// (both trigger characters together -- apostrophe AND `::`) with a trailing
// statement AFTER it. Before the fix this either nulled the whole arm
// (apostrophe) or spliced the comment text into the arm's own captured body
// as bogus structure, truncating the trailing statement (double-colon). Both
// $insideArmFirstRan and $insideArmSecondRan must be observed true -- proving
// the WHOLE arm (not just a prefix) survives a comment with either trigger
// character inside its body.
@handle $withCommentInsideArm {
  receive {
    Ok(body) => {
      $insideArmFirstRan <- true;
      // serde's default, std::time::Duration
      $insideArmSecondRan <- true;
    }
    Failed(_) => { }
  }
}

@data inline $apostropheArmRan bool : false;
@data inline $doubleColonArmRan bool : false;
@data inline $insideArmFirstRan bool : false;
@data inline $insideArmSecondRan bool : false;

@test "a handle arm with NO comment inside its body compiles and fires correctly (apostrophe-trigger sibling)" {
  @fixture {
    <div class="hacc-a">x</div>
  }

  @let (el = document.querySelector('.hacc-a'))

  @eval (window.__realFetch = window.__realFetch || window.fetch)
  @eval (window.fetch = function(url, opts) { return Promise.resolve({ status: 200, ok: true, statusText: 'OK', headers: {}, text: function() { return Promise.resolve('{}'); } }); })

  @eval ((ST.getSignalCall('withApostropheComment'))())

  @wait 200ms

  @eval (window.fetch = window.__realFetch)

  @assert (ST.resolve(document.body, 'apostropheArmRan') === true)
    "apostrophe-trigger handle arm must set $apostropheArmRan true"
}

@test "a handle arm with NO comment inside its body compiles and fires correctly (double-colon-trigger sibling)" {
  @fixture {
    <div class="hacc-b">x</div>
  }

  @let (el = document.querySelector('.hacc-b'))

  @eval (window.__realFetch = window.__realFetch || window.fetch)
  @eval (window.fetch = function(url, opts) { return Promise.resolve({ status: 200, ok: true, statusText: 'OK', headers: {}, text: function() { return Promise.resolve('{}'); } }); })

  @eval ((ST.getSignalCall('withDoubleColonComment'))())

  @wait 200ms

  @eval (window.fetch = window.__realFetch)

  @assert (ST.resolve(document.body, 'doubleColonArmRan') === true)
    "double-colon-trigger handle arm must set $doubleColonArmRan true"
}

@test "a handle arm WITH a comment (containing both an apostrophe AND ::) inside its body still fires the WHOLE arm correctly (BUG-183/BUG-100 fix pin)" {
  @fixture {
    <div class="hacc-c">x</div>
  }

  @let (el = document.querySelector('.hacc-c'))

  @eval (window.__realFetch = window.__realFetch || window.fetch)
  @eval (window.fetch = function(url, opts) { return Promise.resolve({ status: 200, ok: true, statusText: 'OK', headers: {}, text: function() { return Promise.resolve('{}'); } }); })

  @eval ((ST.getSignalCall('withCommentInsideArm'))())

  @wait 200ms

  @eval (window.fetch = window.__realFetch)

  @assert (ST.resolve(document.body, 'insideArmFirstRan') === true)
    "comment-containing handle arm must set $insideArmFirstRan true"
  @assert (ST.resolve(document.body, 'insideArmSecondRan') === true)
    "comment-containing handle arm must set $insideArmSecondRan true"
}
