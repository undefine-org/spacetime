/// Wall-clock Now Primitive
///
/// Yields a live, reactive string of the current time-of-day, updated on a
/// configurable interval. Backed by `Date` and `Intl.DateTimeFormat`.
///
/// @example
/// .clock {
///   @now(every: 1s) {
///     text <- $now;
///   }
/// }
///
/// @param every - Update interval (default 1s)
///
/// @yields $now - Formatted current time string

%primitive now(
  &el,
  every: number = 1000
) {
  %emit js {
    const pad2 = (n) => String(n).padStart(2, '0');

    const formatTime = (date) => {
      try {
        if (typeof Intl !== 'undefined' && Intl.DateTimeFormat) {
          const f = new Intl.DateTimeFormat('en-US', {
            timeZone: 'UTC',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
            hour12: false
          });
          return f.format(date).replace(/\u202f/g, ' ');
        }
      } catch (e) {}
      const d = new Date(date.toLocaleString('en-US', { timeZone: 'UTC' }));
      return pad2(d.getUTCHours()) + ':' + pad2(d.getUTCMinutes()) + ':' + pad2(d.getUTCSeconds());
    };

    const update = () => {
      const text = formatTime(new Date());
      el.textContent = text;
      %yield text -> $now;
    };

    update();
    const timerId = setInterval(update, %every);

    %cleanup {
      clearInterval(timerId);
    }
  }

  %exports {
    $now: string  // formatted current time
  }
}


