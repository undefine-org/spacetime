/// @portal — render an element at document.body, escaping the layout tree (FEAT-074 W3)
///
/// A drawer, modal, or preview overlay must escape its container's grid + stacking
/// context. Authored as a selector-scope directive on the element, gated by a signal:
///
/// @example
///   $menuOpen bool: false;
///   .drawer { @portal(when: $menuOpen); }      // relocates .drawer to <body> while open
///   .burger { @on click { $menuOpen <- !$menuOpen; } }
///
/// When the signal is truthy the element is moved to document.body and the modal tax
/// is applied (scroll-lock refcounted across stacked portals, focus-trap + restore,
/// Escape closes the topmost, aria-modal/role=dialog). When falsy it returns home and
/// everything is undone. `close:` names the signal to set false on Esc/close (defaults
/// to the `when:` signal).

/// The PRODUCTIVE form is declared FIRST so the clean-match ranking is what refuses
/// a bare `close:` spelling: a lenient productive match (one that drops the clause it
/// cannot bind) ranks BELOW the error sibling below, so the near-miss is refused.
%macro portal {
  %scope selector
  %order 700

  %form {
    @portal(when: $signal:binding, close: $closeSignal:binding? = "");
  }

  %binds {
    portal-mount(&self, signal: $signal, closeSignal: $closeSignal)
  }
}

/// A literal `close:` is a commitment: its value must be a `$` signal.
/// These are the two ordinary-looking wrong spellings. They are deliberately
/// a sibling form rather than an optional-capture failure, which would erase
/// the complete `@portal` directive before the pipeline can report it.
%capture_type portal_close_invalid { ( $name:element | $name:event_name ) }

%macro portal-invalid-close {
  %scope selector
  %order 701

  %form {
    @portal(when: $signal:binding, close: $close:portal_close_invalid);
  }

  %binds {
    optional-clause-error(
      message: "`close:` must name a signal",
      hint: "write `close: $close` — `$` names the signal to clear when the portal closes"
    )
  }
}
