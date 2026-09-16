/// Data Binding Primitives
///
/// Reactive bindings between data and DOM elements.


/// Text Binding Primitive
///
/// Binds data to element's text content.
///
/// @param &el - Element to bind
/// @param value - Value expression to display
/// @param format - Optional format function

// TODO: bind-text is one-shot — it sets textContent once but never re-evaluates
// when signals change. Making it reactive (listen for `local:*:updated` events
// from compute-fn) is a separate task. For now, use @cycle for interval-driven
// text updates, or @value-change for mutation-triggered animations.

%primitive bind-text(
  &el,
  value: expr,
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
///
/// @param &el - Element to bind
/// @param attr - Attribute name
/// @param value - Value expression

%primitive bind-attr(
  &el,
  attr: string,
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
///
/// @param &el - Element to bind
/// @param className - Class name to toggle
/// @param condition - Boolean condition

%primitive bind-class(
  &el,
  className: string,
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
///
/// @param &el - Element to bind
/// @param property - CSS property name
/// @param value - Value expression

%primitive bind-style(
  &el,
  property: string,
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
///
/// @param &el - Input element to bind
/// @param value - Reference to data value
/// @param event - Event to listen for changes
/// @param debounce - Debounce delay in ms
///
/// @yields $value - Current input value

%primitive bind-input(
  &el,
  value: any,
  event: string = "input",
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
    $value: any
  }
}


/// Visibility Binding Primitive
///
/// Shows/hides element based on condition.
///
/// @param &el - Element to show/hide
/// @param condition - Boolean condition
/// @param mode - "display" or "visibility"

%primitive bind-visible(
  &el,
  condition: expr,
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
///
/// @param &el - Element to conditionally render
/// @param condition - Boolean condition
///
/// @yields $mounted - Whether element is currently in DOM

%primitive bind-if(
  &el,
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
    $mounted: bool
  }
}
