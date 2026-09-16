/// Data Filter Primitives
///
/// Transform and filter data in pipelines.
/// Used with the pipe syntax: $data | filter(args)


/// Filter Primitive
///
/// Filters array based on predicate.
///
/// @param data - Array to filter
/// @param predicate - Filter function or property name
/// @param value - Value to match (for property filter)
///
/// @yields $result - Filtered array

%primitive filter(
  data: array,
  predicate: any,
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
    $result: array
  }
}


/// Map Primitive
///
/// Transforms each item in array.
///
/// @param data - Array to map
/// @param transform - Transform function or property name
///
/// @yields $result - Mapped array

%primitive map(
  data: array,
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
    $result: array
  }
}


/// Sort Primitive
///
/// Sorts array by property or comparator.
///
/// @param data - Array to sort
/// @param by - Property name or comparator function
/// @param order - "asc" or "desc"
///
/// @yields $result - Sorted array

%primitive sort(
  data: array,
  by: any?,
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
    $result: array
  }
}


/// Slice Primitive
///
/// Takes a portion of an array.
///
/// @param data - Array to slice
/// @param start - Start index
/// @param end - End index (exclusive)
///
/// @yields $result - Sliced array

%primitive slice(
  data: array,
  start: number = 0,
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
    $result: array
  }
}


/// First/Last Primitive
///
/// Gets first or last N items from array.
///
/// @param data - Source array
/// @param count - Number of items
/// @param from - "first" or "last"
///
/// @yields $result - Subset array

%primitive take(
  data: array,
  count: number = 1,
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
    $result: array
  }
}


/// Reduce Primitive
///
/// Reduces array to single value.
///
/// @param data - Array to reduce
/// @param reducer - Reduce function
/// @param initial - Initial value
///
/// @yields $result - Reduced value

%primitive reduce(
  data: array,
  reducer: fn,
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
    $result: any
  }
}


/// Unique Primitive
///
/// Removes duplicate values from array.
///
/// @param data - Source array
/// @param by - Property to check for uniqueness
///
/// @yields $result - Array with unique values

%primitive unique(
  data: array,
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
    $result: array
  }
}


/// Group By Primitive
///
/// Groups array items by property value.
///
/// @param data - Source array
/// @param by - Property to group by
///
/// @yields $result - Object with groups

%primitive group-by(
  data: array,
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
