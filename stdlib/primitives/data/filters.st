/// Data Filter Primitives
///
/// Transform and filter data in pipelines.
/// Used with the pipe syntax: $data | filter(args)


/// Filter Primitive
///
/// Filters array based on predicate.

%primitive filter(
  /// Array to filter
  data: array,
  /// Filter function or property name
  predicate: any,
  /// Value to match (for property filter)
  value: any?
) {
  %emit js {
    const arr = %data;
    const pred = %predicate;
    const matchValue = %value;

    let result;

    if (!Array.isArray(arr)) {
      result = [];
    } else if (typeof pred === 'function') {
      result = arr.filter(pred);
    } else if (typeof pred === 'string') {
      // Property-based filter
      result = arr.filter(item => {
        const itemValue = item?.[pred];
        return matchValue !== undefined
          ? itemValue === matchValue
          : !!itemValue;
      });
    } else {
      result = arr;
    }

    %yield result -> $result;
  }

  %exports {
    /// Filtered array
    $result: array
  }
}


/// Map Primitive
///
/// Transforms each item in array.

%primitive map(
  /// Array to map
  data: array,
  /// Transform function or property name
  transform: any
) {
  %emit js {
    const arr = %data;
    const fn = %transform;

    let result;

    if (!Array.isArray(arr)) {
      result = [];
    } else if (typeof fn === 'function') {
      result = arr.map(fn);
    } else if (typeof fn === 'string') {
      // Extract property
      result = arr.map(item => item?.[fn]);
    } else {
      result = arr;
    }

    %yield result -> $result;
  }

  %exports {
    /// Mapped array
    $result: array
  }
}


/// Sort Primitive
///
/// Sorts array by property or comparator.

%primitive sort(
  /// Array to sort
  data: array,
  /// Property name or comparator function
  by: any?,
  /// "asc" or "desc"
  order: string = "asc"
) {
  %emit js {
    const arr = %data;
    const sortBy = %by;
    const sortOrder = %order;

    if (!Array.isArray(arr)) {
      %yield [] -> $result;
      return;
    }

    const result = [...arr];
    const multiplier = sortOrder === 'desc' ? -1 : 1;

    if (typeof sortBy === 'function') {
      result.sort((a, b) => multiplier * sortBy(a, b));
    } else if (typeof sortBy === 'string') {
      result.sort((a, b) => {
        const aVal = a?.[sortBy];
        const bVal = b?.[sortBy];
        if (aVal < bVal) return -1 * multiplier;
        if (aVal > bVal) return 1 * multiplier;
        return 0;
      });
    } else {
      result.sort((a, b) => {
        if (a < b) return -1 * multiplier;
        if (a > b) return 1 * multiplier;
        return 0;
      });
    }

    %yield result -> $result;
  }

  %exports {
    /// Sorted array
    $result: array
  }
}


/// Slice Primitive
///
/// Takes a portion of an array.

%primitive slice(
  /// Array to slice
  data: array,
  /// Start index
  start: number = 0,
  /// End index (exclusive)
  end: number?
) {
  %emit js {
    const arr = %data;
    const startIdx = %start;
    const endIdx = %end;

    if (!Array.isArray(arr)) {
      %yield [] -> $result;
      return;
    }

    const result = endIdx !== undefined
      ? arr.slice(startIdx, endIdx)
      : arr.slice(startIdx);

    %yield result -> $result;
  }

  %exports {
    /// Sliced array
    $result: array
  }
}


/// First/Last Primitive
///
/// Gets first or last N items from array.

%primitive take(
  /// Source array
  data: array,
  /// Number of items
  count: number = 1,
  /// "first" or "last"
  from: string = "first"
) {
  %emit js {
    const arr = %data;
    const n = %count;
    const direction = %from;

    if (!Array.isArray(arr)) {
      %yield [] -> $result;
      return;
    }

    const result = direction === 'last'
      ? arr.slice(-n)
      : arr.slice(0, n);

    %yield result -> $result;
  }

  %exports {
    /// Subset array
    $result: array
  }
}


/// Reduce Primitive
///
/// Reduces array to single value.

%primitive reduce(
  /// Array to reduce
  data: array,
  /// Reduce function
  reducer: fn,
  /// Initial value
  initial: any = 0
) {
  %emit js {
    const arr = %data;
    const fn = %reducer;
    const init = %initial;

    if (!Array.isArray(arr)) {
      %yield init -> $result;
      return;
    }

    const result = arr.reduce(fn, init);
    %yield result -> $result;
  }

  %exports {
    /// Reduced value
    $result: any
  }
}


/// Unique Primitive
///
/// Removes duplicate values from array.

%primitive unique(
  /// Source array
  data: array,
  /// Property to check for uniqueness
  by: string?
) {
  %emit js {
    const arr = %data;
    const prop = %by;

    if (!Array.isArray(arr)) {
      %yield [] -> $result;
      return;
    }

    let result;
    if (prop) {
      const seen = new Set();
      result = arr.filter(item => {
        const key = item?.[prop];
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
    } else {
      result = [...new Set(arr)];
    }

    %yield result -> $result;
  }

  %exports {
    /// Array with unique values
    $result: array
  }
}


/// Group By Primitive
///
/// Groups array items by property value.

%primitive group-by(
  /// Source array
  data: array,
  /// Property to group by
  by: string
) {
  %emit js {
    const arr = %data;
    const prop = %by;

    if (!Array.isArray(arr)) {
      %yield {} -> $result;
      return;
    }

    const result = {};
    for (const item of arr) {
      const key = String(item?.[prop] ?? 'undefined');
      if (!result[key]) result[key] = [];
      result[key].push(item);
    }

    %yield result -> $result;
  }

  %exports {
    /// Object with groups
    $result: object
  }
}


/// Format Primitives
///
/// Common formatting utilities.

%primitive uppercase(data: string) {
  %emit js {
    %yield String(%data).toUpperCase() -> $result;
  }
  %exports { $result: string }
}

%primitive lowercase(data: string) {
  %emit js {
    %yield String(%data).toLowerCase() -> $result;
  }
  %exports { $result: string }
}

%primitive capitalize(data: string) {
  %emit js {
    const str = String(%data);
    %yield str.charAt(0).toUpperCase() + str.slice(1) -> $result;
  }
  %exports { $result: string }
}

%primitive truncate(data: string, length: number = 100, suffix: string = "...") {
  %emit js {
    const str = String(%data);
    const len = %length;
    const suf = %suffix;
    %yield str.length > len ? str.slice(0, len) + suf : str -> $result;
  }
  %exports { $result: string }
}

%primitive currency(data: number, locale: string = "en-US", currency: string = "USD") {
  %emit js {
    const num = Number(%data);
    const formatter = new Intl.NumberFormat(%locale, {
      style: 'currency',
      currency: %currency
    });
    %yield formatter.format(num) -> $result;
  }
  %exports { $result: string }
}

%primitive number-format(data: number, locale: string = "en-US", decimals: number?) {
  %emit js {
    const num = Number(%data);
    const opts = %decimals !== undefined
      ? { minimumFractionDigits: %decimals, maximumFractionDigits: %decimals }
      : {};
    const formatter = new Intl.NumberFormat(%locale, opts);
    %yield formatter.format(num) -> $result;
  }
  %exports { $result: string }
}

%primitive date-format(data: any, format: string = "short", locale: string = "en-US") {
  %emit js {
    const date = new Date(%data);
    const fmt = %format;
    const loc = %locale;

    let result;
    if (fmt === 'iso') {
      result = date.toISOString();
    } else if (fmt === 'relative') {
      const rtf = new Intl.RelativeTimeFormat(loc, { numeric: 'auto' });
      const diff = date.getTime() - Date.now();
      const days = Math.round(diff / (1000 * 60 * 60 * 24));
      result = rtf.format(days, 'day');
    } else {
      const options = fmt === 'long'
        ? { dateStyle: 'long', timeStyle: 'short' }
        : fmt === 'medium'
        ? { dateStyle: 'medium' }
        : { dateStyle: 'short' };
      result = new Intl.DateTimeFormat(loc, options).format(date);
    }

    %yield result -> $result;
  }
  %exports { $result: string }
}
