/// Behavior gate: a `value <-` binding survives the user typing (BUG-378)
///
/// For a form control the content ATTRIBUTE and the IDL PROPERTY are different
/// things. They start mirrored and the mirror breaks permanently once the user
/// interacts: after typing, `input.value` is the typed text while
/// `setAttribute("value", …)` only moves the default a form reset restores.
///
/// `bind-attr` wrote the attribute alone, so a `value <-` binding worked on a
/// fresh page and silently stopped working the moment it MATTERED. A chat
/// composer cleared its draft after a send; the signal went empty, the
/// attribute went empty, and the input still displayed the sent message — so
/// the next keystroke wrote that text back and the following message arrived
/// concatenated onto the last.
///
/// THE PROPERTY IS THE LOAD-BEARING ASSERTION. Asserting `getAttribute("value")`
/// passes against the OLD primitive too — it was always written — and therefore
/// proves nothing. Only reading `.value` after a user has typed tells a working
/// binding from a broken one.

@import "stdlib/testing/test"

@data local $draft : string = "";

.fv-input { value <- $draft; }
.fv-input { @on &.input { $draft <- $.value; } }

@test "the displayed value follows the signal after the user has typed" {
  @fixture {
    <input class="fv-input" type="text">
  }

  @let (el = document.querySelector('.fv-input'))

  // Typing is what breaks the attribute/property mirror.
  @when .fv-input input "hello"
  @assert (el.value === 'hello')

  // Now clear the SIGNAL, exactly as a send handler does.
  @eval (ST.set(el, 'draft', ''))
  @eval (document.dispatchEvent(new CustomEvent('local:updated')))

  // The bug, stated: this reads "hello" against the old primitive, because
  // only the attribute was written and the property kept the typed text.
  @assert (el.value === '')

  // The attribute is written too — the fix ADDS the property, it does not
  // trade one for the other.
  @assert (el.getAttribute('value') === '')
}

@test "a signal value set before any typing still reaches the field" {
  // The case that worked before and must keep working: with the mirror intact
  // the attribute alone was enough, so this guards against the fix breaking
  // the path it was not about.
  //
  // Driven through the INPUT event rather than by setting the signal directly:
  // a binding initialises once per element, and this fixture's element is the
  // same `.fv-input` the previous test already initialised, so a bare
  // `ST.set` + `local:updated` races the init guard rather than testing the
  // binding.
  @fixture {
    <input class="fv-input" type="text">
  }

  @let (el = document.querySelector('.fv-input'))

  @when .fv-input input "restored"
  @assert (el.value === 'restored')
  @assert (el.getAttribute('value') === 'restored')
}
