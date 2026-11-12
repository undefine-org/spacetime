/// Data Binding Primitives
///
/// Reactive bindings between data and DOM elements.


/// Text Binding Primitive
///
/// Binds data to element's text content.

// TODO: bind-text is one-shot — it sets textContent once but never re-evaluates
// when signals change. Making it reactive (listen for `local:*:updated` events
// from compute-fn) is a separate task. For now, use @cycle for interval-driven
// text updates, or @value-change for mutation-triggered animations.

%primitive bind-text(
  /// Element to bind
  &el,
  /// Value expression to display
  value: expr,
  /// Optional format function
  format: fn?
) {
  %emit js {
    let _lastVal;
    const update = () => {
      let val = %value;
      if (%format && typeof %format === 'function') {
        val = %format(val);
      }
      const finalVal = val ?? '';
      if (finalVal === _lastVal) return;
      _lastVal = finalVal;
      %&el.textContent = finalVal;
    };

    update();

    // Re-evaluate when any local state changes
    document.addEventListener('local:updated', update);
  }

  %exports {}
}


/// Attribute Binding Primitive
///
/// Binds data to an element attribute.

%primitive bind-attr(
  /// Element to bind
  &el,
  /// Attribute name
  attr: string,
  /// Value expression
  value: expr
) {
  %emit js {
    const attrName = %attr;
    let _lastVal;

    const update = () => {
      const val = %value;
      if (val === _lastVal) return;
      _lastVal = val;
      if (val === null || val === undefined || val === false) {
        %&el.removeAttribute(attrName);
      } else if (val === true) {
        %&el.setAttribute(attrName, '');
      } else {
        %&el.setAttribute(attrName, String(val));
      }
    };

    update();

    // Re-evaluate when any local state changes
    document.addEventListener('local:updated', update);
  }

  %exports {}
}


/// Class Binding Primitive
///
/// Conditionally adds/removes a CSS class.

%primitive bind-class(
  /// Element to bind
  &el,
  /// Class name to toggle
  className: string,
  /// Boolean condition
  condition: expr
) {
  %emit js {
    const cls = %className;
    let _lastCond;

    const update = () => {
      if (!%&el) return;
      const cond = !!(%condition);
      if (cond === _lastCond) return;
      _lastCond = cond;
      if (cond) {
        %&el.classList.add(cls);
      } else {
        %&el.classList.remove(cls);
      }
    };

    update();

    // Re-evaluate when any local state changes
    document.addEventListener('local:updated', update);
  }

  %exports {}
}


/// Style Binding Primitive
///
/// Binds data to a CSS style property.

%primitive bind-style(
  /// Element to bind
  &el,
  /// CSS property name
  property: string,
  /// Value expression
  value: expr
) {
  %emit js {
    const prop = %property;

    const update = () => {
      const val = %value;
      if (val === null || val === undefined) {
        %&el.style.removeProperty(prop);
      } else {
        %&el.style.setProperty(prop, String(val));
      }
    };

    update();
  }

  %exports {}
}


/// Two-Way Input Binding Primitive
///
/// Creates two-way binding between input element and data.

%primitive bind-input(
  /// Input element to bind
  &el,
  /// Reference to data value
  value: any,
  /// Event to listen for changes
  event: string = "input",
  /// Debounce delay in ms
  debounce: number = 0
) {
  %emit js {
    const eventName = %event;
    const debounceMs = %debounce;
    let timeoutId = null;
    let currentValue = %value;

    // Set initial value
    if (el.type === 'checkbox') {
      el.checked = !!currentValue;
    } else if (el.type === 'radio') {
      el.checked = el.value === currentValue;
    } else {
      el.value = currentValue ?? '';
    }

    const getValue = () => {
      if (el.type === 'checkbox') return el.checked;
      if (el.type === 'number') return el.valueAsNumber;
      return el.value;
    };

    const handleChange = () => {
      const newValue = getValue();
      currentValue = newValue;
      %yield newValue -> $value;
    };

    const debouncedHandler = debounceMs > 0
      ? () => {
          if (timeoutId) clearTimeout(timeoutId);
          timeoutId = setTimeout(handleChange, debounceMs);
        }
      : handleChange;

    el.addEventListener(eventName, debouncedHandler);

    %cleanup {
      el.removeEventListener(eventName, debouncedHandler);
      if (timeoutId) clearTimeout(timeoutId);
    }
  }

  %exports {
    /// Current input value
    $value: any
  }
}


/// Visibility Binding Primitive
///
/// Shows/hides element based on condition.

%primitive bind-visible(
  /// Element to show/hide
  &el,
  /// Boolean condition
  condition: expr,
  /// "display" or "visibility"
  mode: string = "display"
) {
  %emit js {
    const displayMode = %mode;
    let _lastVisible;

    const update = () => {
      const visible = !!(%condition);
      if (visible === _lastVisible) return;
      _lastVisible = visible;

      if (displayMode === 'visibility') {
        %&el.style.visibility = visible ? '' : 'hidden';
      } else {
        %&el.style.display = visible ? '' : 'none';
      }
    };

    update();

    // Re-evaluate when any local state changes
    document.addEventListener('local:updated', update);
  }

  %exports {}
}


/// Conditional Render Primitive
///
/// Inserts/removes element from DOM based on condition.
/// More efficient than visibility for rarely-shown content.

%primitive bind-if(
  /// Element to conditionally render
  &el,
  /// Boolean condition
  condition: expr
) {
  %emit js {
    const placeholder = document.createComment('st-if');
    const parent = el.parentNode;
    let mounted = true;

    // Insert placeholder before element
    parent.insertBefore(placeholder, el);

    const update = () => {
      const shouldMount = !!(%condition);
      if (shouldMount === mounted) return;

      if (shouldMount && !mounted) {
        placeholder.after(el);
        mounted = true;
        %yield true -> $mounted;
      } else if (!shouldMount && mounted) {
        el.remove();
        mounted = false;
        %yield false -> $mounted;
      }
    };

    update();

    // Re-evaluate when any local state changes
    document.addEventListener('local:updated', update);

    %cleanup {
      placeholder.remove();
    }
  }

  %exports {
    /// Whether element is currently in DOM
    $mounted: bool
  }
}
