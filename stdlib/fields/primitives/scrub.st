/// Scrub a number by dragging its label (PLAN-156), as in Blender, After
/// Effects and Figma: drag right to raise it, left to lower it, one `step` per
/// few pixels; Shift for a tenth of a step, Ctrl/Cmd for ten.
///
/// The label sits in a field with a number `<input>`. While dragging, the input
/// shows the value and fires `input` (a preview a page may show); on release it
/// fires `change` once, which is when a field writes. A click without a drag
/// focuses the input instead.
%primitive scrub(&el) {
  %emit js {
    const label = %&el;
    const field = label.closest('.w-field');
    const input = field && field.querySelector('input[type=number], input.w-number');
    if (!input || label._stScrub) { return; }
    label._stScrub = true;
    label.style.cursor = 'ew-resize';
    label.style.userSelect = 'none';
    let drag = null;
    const decimals = (s) => { const t = String(s); const i = t.indexOf('.'); return i < 0 ? 0 : t.length - i - 1; };
    label.addEventListener('pointerdown', (ev) => {
      if (ev.button !== 0) { return; }
      const step = parseFloat(input.step) || 0.1;
      drag = { x: ev.clientX, v: parseFloat(input.value) || 0, step, moved: false, places: Math.max(decimals(step), decimals(input.value)) };
      label.setPointerCapture(ev.pointerId);
      ev.preventDefault();
    });
    label.addEventListener('pointermove', (ev) => {
      if (!drag) { return; }
      const dx = ev.clientX - drag.x;
      if (!drag.moved && Math.abs(dx) < 3) { return; }
      drag.moved = true;
      const mult = ev.shiftKey ? 0.1 : (ev.ctrlKey || ev.metaKey) ? 10 : 1;
      let v = drag.v + Math.round(dx / 4) * drag.step * mult;
      const min = input.min === '' ? -Infinity : parseFloat(input.min);
      const max = input.max === '' ? Infinity : parseFloat(input.max);
      v = Math.min(max, Math.max(min, v));
      input.value = v.toFixed(Math.min(4, drag.places + (ev.shiftKey ? 1 : 0)));
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });
    const end = () => {
      if (!drag) { return; }
      const moved = drag.moved;
      drag = null;
      if (moved) { input.dispatchEvent(new Event('change', { bubbles: true })); } else { input.focus(); input.select(); }
    };
    label.addEventListener('pointerup', end);
    label.addEventListener('pointercancel', end);
  }
}

%macro scrub {
  %scope selector
  %form { @scrub }
  %binds { scrub(&self) }
}
