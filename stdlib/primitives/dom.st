/// DOM Primitives
///
/// Low-level primitives for DOM interaction that don't fit in other categories.


/// Focus Primitive
///
/// Tracks focus state of an element.

%primitive focus(&el) {
  %emit js {
    %yield document.activeElement === %&el -> $focused;

    const onFocus = () => %yield true -> $focused;
    const onBlur = () => %yield false -> $focused;

    %&el.addEventListener('focus', onFocus);
    %&el.addEventListener('blur', onBlur);

    %cleanup {
      %&el.removeEventListener('focus', onFocus);
      %&el.removeEventListener('blur', onBlur);
    }
  }

  %exports {
    /// True when element has focus
    $focused: bool
  }
}


/// Element Reference Primitive
///
/// Gets a reference to an element by selector and tracks its rect.

%primitive element-ref(
  /// CSS selector to find element(s)
  selector: string
) {
  %emit js {
    const el = %selector.startsWith('.')
      ? document.querySelector(%selector)
      : document.querySelectorAll(%selector);

    %yield el -> $ref;

    if (el instanceof Element) {
      const updateRect = () => %yield el.getBoundingClientRect() -> $rect;
      const resizeObs = new ResizeObserver(updateRect);
      resizeObs.observe(el);
      updateRect();

      %cleanup {
        resizeObs.disconnect();
      }
    }
  }

  %exports {
    /// The element or NodeList
    $ref: Element
    /// Bounding client rect (for single elements)
    $rect: DOMRect?
  }
}


/// DOM Action Primitive
///
/// Executes a DOM action and yields completion signal.
/// Used for DOM mutations that should be tracked.

%primitive dom-action(
  /// The action to execute
  action: expr
) {
  %emit js {
    %yield false -> $done;
    %action;
    %yield true -> $done;
  }

  %exports {
    /// True when action completes
    $done: bool
  }
}


/// Persist Primitive
///
/// Persists data to a URL endpoint.

%primitive persist-remote(
  /// URL to persist to
  url: string,
  /// Data to persist
  data: expr
) {
  %emit js {
    %yield false -> $pending;
    %yield false -> $ok;
    %yield null -> $error;

    const run = async () => {
      %yield true -> $pending;
      %yield false -> $ok;
      %yield null -> $error;

      try {
        await fetch(%url, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(%data)
        });
        %yield false -> $pending;
        %yield true -> $ok;
      } catch (e) {
        %yield false -> $pending;
        %yield e -> $error;
      }
    };

    %yield run -> $run;
  }

  %exports {
    /// Function to trigger persistence
    $run: fn() ~> effect
    /// True while persisting
    $pending: bool
    /// True on success
    $ok: bool
    /// Error if failed
    $error: any?
  }
}
