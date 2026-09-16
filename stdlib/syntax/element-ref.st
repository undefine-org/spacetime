/// Element Reference Syntax
///
/// Creates named references to DOM elements with CSS variable bindings
/// for element dimensions (width, height, etc.).
/// Syntax: &name selector;
///
/// Examples:
///   &header .header;
///   &main #main-content;
///   &sidebar [data-sidebar];

%primitive element-ref-impl(
  &self,
  name: ident,
  selector: selector
) %uses element-refs {
  %emit css {
    :root {
      --st-%name-width: 0px;
      --st-%name-height: 0px;
      --st-%name-top: 0px;
      --st-%name-left: 0px;
    }
  }

  %emit js {
    // Set up element reference with ResizeObserver
    (function() {
      const el = document.querySelector(%selector);
      if (!el) {
        console.warn('Element ref "%name" not found:', %selector);
        return;
      }

      // Register the resolved element under its name so derives/bounds can
      // find it. The registry init is guaranteed by the element-refs module
      // prelude (`%uses element-refs`) — the inline re-init here was the
      // drift shape (a second defensive copy).
      if (typeof window !== 'undefined') {
        window.__stRefs['%name'] = el;
      }

      const updateVars = () => {
        const rect = el.getBoundingClientRect();
        const root = document.documentElement;
        root.style.setProperty('--st-%name-width', rect.width + 'px');
        root.style.setProperty('--st-%name-height', rect.height + 'px');
        root.style.setProperty('--st-%name-top', rect.top + 'px');
        root.style.setProperty('--st-%name-left', rect.left + 'px');
      };

      // Observe size changes
      new ResizeObserver(updateVars).observe(el);

      // Initial update
      updateVars();

      // Re-update on scroll for position values
      window.addEventListener('scroll', updateVars, { passive: true });
    })();
  }

  %exports {
    $width: length
    $height: length
    $top: length
    $left: length
  }
}

%macro element-ref {
  // Prefix '&' extracted from %form pattern
  %form { &name:ident $selector:selector ; }

  %binds {
    element-ref-impl(&self, name: $name, selector: $selector)
  }
}
