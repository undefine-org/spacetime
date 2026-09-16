/// DOM Primitives
///
/// Low-level primitives for DOM interaction that don't fit in other categories.


/// Focus Primitive
///
/// Tracks focus state of an element.
///
/// @yields $focused - True when element has focus

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
    $focused: bool
  }
}


/// Element Reference Primitive
///
/// Gets a reference to an element by selector and tracks its rect.
///
/// @param selector - CSS selector to find element(s)
/// @yields $ref - The element or NodeList
/// @yields $rect - Bounding client rect (for single elements)

%primitive element-ref(selector: string) {
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
    $ref: Element
    $rect: DOMRect?
  }
}


/// DOM Action Primitive
///
/// Executes a DOM action and yields completion signal.
/// Used for DOM mutations that should be tracked.
///
/// @param action - The action to execute
/// @yields $done - True when action completes

%primitive dom-action(action: expr) {
  %emit js {
    %yield false -> $done;
    %action;
    %yield true -> $done;
  }

  %exports {
    $done: bool
  }
}


/// Persist Primitive
///
/// Persists data to a URL endpoint.
///
/// @param url - URL to persist to
/// @param data - Data to persist
/// @yields $run - Function to trigger persistence
/// @yields $pending - True while persisting
/// @yields $ok - True on success
/// @yields $error - Error if failed

%primitive persist-remote(url: string, data: expr) {
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
    $run: fn() ~> effect
    $pending: bool
    $ok: bool
    $error: any?
  }
}
