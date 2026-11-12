/// Text Cycle Primitive
///
/// Cycles through items in a data source array on an interval,
/// updating the element's textContent each tick. The textContent
/// mutation triggers MutationObserver-based animations (e.g. @value-change).

%primitive text-cycle(
  /// Element whose textContent to update
  &el,
  source: ident,
  interval: number = 3000,
  field: string?
) {
  %emit js {
    const sourceName = '%source';
    const ms = %interval;
    const field = %field || null;
    let index = 0;
    let timerId = null;

    const getText = (item) => field ? item[field] : String(item);

    if (!%&el) {
      console.warn('[Spacetime] text-cycle: target element not found in DOM for source "' + sourceName + '".');
    }

    ST.afterData(sourceName, (data) => {
      if (!data || !data.length) {
        console.warn('[Spacetime] text-cycle: data source "' + sourceName + '" is empty or failed to load. Text will not cycle.');
        return;
      }

      // Start cycling after data loads
      timerId = setInterval(() => {
        index = (index + 1) % data.length;
        %&el.textContent = getText(data[index]);
      }, ms);
    });

    %cleanup {
      if (timerId) clearInterval(timerId);
    }
  }

  %exports {}
}
