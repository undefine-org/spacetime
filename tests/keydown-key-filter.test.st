/// Behavior gate: a keyboard driver answers ONE key (BUG-375)
///
/// `@on &.keydown("Enter") { … }` must run its body on Enter and MUST NOT run
/// it on any other key. Before the fix there was no filter at all: the qualifier
/// authors reached for — `@on &.keydown:Enter` — has no slot in `driver_expr`'s
/// grammar, so it was parsed and DISCARDED, and the handler fired on every
/// keystroke. A chat composer bound that way sent a message per character, and
/// the compiler reported no problem at any stage.
///
/// THE NEGATIVE IS THE LOAD-BEARING ASSERTION, for the reason
/// tests/event-driver-triggers.test.st states about its own rows: a test that
/// presses Enter and sees the body run passes just as happily against a handler
/// with NO filter — i.e. against the bug. Only "a different key changed
/// nothing" tells the two apart.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"

// One element per case: page state persists across tests in a file, so a shared
// node would let one test's keypress satisfy another's assertion.
.kf-enter { @on &.keydown("Enter")       { $fired <- "yes"; } }
.kf-named { @on &.keydown(key: "Escape") { $fired <- "yes"; } }
.kf-any   { @on &.keydown                { $fired <- "yes"; } }

@test "a keyed driver runs on its own key" {
  @fixture {
    <input class="kf-enter" type="text">
  }
  @eval (ST.set(document.querySelector('.kf-enter'), 'fired', ''))
  @when .kf-enter keydown "Enter"
  @then $fired on .kf-enter should equal "yes"
}

@test "a keyed driver does NOT run on another key" {
  // The bug, stated as a test: without the filter this body ran on every
  // keystroke. This is the assertion that fails against the old compiler.
  @fixture {
    <input class="kf-enter" type="text">
  }
  @eval (ST.set(document.querySelector('.kf-enter'), 'fired', ''))
  @when .kf-enter keydown "a"
  @then $fired on .kf-enter should equal ""
}

@test "the named spelling filters the same way" {
  // `("Escape")` and `(key: "Escape")` are one binding written two ways: the
  // positional slot and the named param must reach the same place, or one of
  // two documented spellings silently does nothing.
  @fixture {
    <input class="kf-named" type="text">
  }
  @eval (ST.set(document.querySelector('.kf-named'), 'fired', ''))
  @when .kf-named keydown "x"
  @then $fired on .kf-named should equal ""
}

@test "the named spelling runs on its key" {
  @fixture {
    <input class="kf-named" type="text">
  }
  @eval (ST.set(document.querySelector('.kf-named'), 'fired', ''))
  @when .kf-named keydown "Escape"
  @then $fired on .kf-named should equal "yes"
}

@test "an unkeyed driver still answers every key" {
  // The regression guard for the fix itself. `keyed()` wraps EVERY listener in
  // the event-driver primitive, including the non-keyboard rows that share it
  // (hover/click/focus/submit), so an empty key must leave a handler exactly as
  // it was.
  @fixture {
    <input class="kf-any" type="text">
  }
  @eval (ST.set(document.querySelector('.kf-any'), 'fired', ''))
  @when .kf-any keydown "q"
  @then $fired on .kf-any should equal "yes"
}
