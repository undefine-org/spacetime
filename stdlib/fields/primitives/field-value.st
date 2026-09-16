/// Pure field value helpers plus field-local control adapters.
/// No helper addresses, persists, or reads application globals.
///
/// GUARD STYLE — BUG-212 (FIXED 2026-07-25). At the top level of an `%emit js`
/// body, a bodyless `if (cond) return;` followed by an arrow-function declaration
/// used to be mis-emitted: the guard became a block that swallowed the arrow's
/// body, the `return` was deleted, and the `const f = () =>` binding was dropped —
/// silently. It cost this module its entire range control (sliders rendered,
/// composed nothing) until the emitted bundle was read directly.
///
/// The emitter's `extract_cond_and_body` now refuses a non-brace token after the
/// condition, so an early return round-trips. The positive-condition guards below
/// are kept because they read well, NOT because the alternative is unsafe — the
/// previous "do not tidy these back into early returns" instruction no longer
/// applies.

%primitive field-range-control(&field) {
  %emit js {
    const slider = %&field.querySelector('.st-fld-range');
    const input = () => {
      const value = String(slider.value) + String(slider.dataset.unit || '');
      const twin = %&field.querySelector('[data-range-twin]'), readout = %&field.querySelector('.st-fld-range-value');
      if (readout) readout.textContent = value;
      if (twin) { twin.value = value; const ev = new Event('input', { bubbles: true }); ev.stFieldCommit = true; twin.dispatchEvent(ev); }
    };
    if (slider) { slider.addEventListener('input', input); }
    %cleanup { if (slider) { slider.removeEventListener('input', input); } }
  }
}

%primitive field-color-control(&field) {
  %emit js {
    const text = %&field.querySelector('.st-fld-color-text'), swatch = %&field.querySelector('.st-fld-color-swatch');
    const rgbToHsv = value => {
      const hex = String(value || '#000').replace('#', ''); const raw = hex.length === 3 ? hex.split('').map(x => x + x).join('') : hex;
      const r = parseInt(raw.slice(0, 2), 16) / 255 || 0, g = parseInt(raw.slice(2, 4), 16) / 255 || 0, b = parseInt(raw.slice(4, 6), 16) / 255 || 0;
      const max = Math.max(r, g, b), min = Math.min(r, g, b), d = max - min; let h = 0;
      if (d) h = max === r ? 60 * (((g - b) / d) % 6) : max === g ? 60 * ((b - r) / d + 2) : 60 * ((r - g) / d + 4);
      return { h: (h + 360) % 360, s: max ? d / max * 100 : 0, v: max * 100, a: raw.length >= 8 ? parseInt(raw.slice(6, 8), 16) / 255 : 1 };
    };
    const hsvToHex = hsv => { const h = ((hsv.h % 360) + 360) % 360, s = Math.max(0, Math.min(100, hsv.s)) / 100, v = Math.max(0, Math.min(100, hsv.v)) / 100, c = v * s, x = c * (1 - Math.abs((h / 60) % 2 - 1)), m = v - c; let r=0,g=0,b=0; if(h<60){r=c;g=x}else if(h<120){r=x;g=c}else if(h<180){g=c;b=x}else if(h<240){g=x;b=c}else if(h<300){r=x;b=c}else{r=c;b=x}; const p = n => Math.round((n+m)*255).toString(16).padStart(2,'0').toUpperCase(); const a = Math.max(0,Math.min(1,hsv.a)); return '#' + p(r)+p(g)+p(b)+(a<1?Math.round(a*255).toString(16).padStart(2,'0').toUpperCase():''); };
    const paint = hsv => { const q = s => %&field.querySelector(s); const sat=q('.st-fld-picker-s'), val=q('.st-fld-picker-v'), alpha=q('.st-fld-picker-a'); if(sat) sat.style.background=`linear-gradient(to right, ${hsvToHex({h:hsv.h,s:0,v:hsv.v,a:1})}, ${hsvToHex({h:hsv.h,s:100,v:hsv.v,a:1})})`; if(val) val.style.background=`linear-gradient(to right, #000, ${hsvToHex({h:hsv.h,s:hsv.s,v:100,a:1})})`; if(alpha) alpha.style.background=`linear-gradient(to right, rgba(0,0,0,0), ${hsvToHex({h:hsv.h,s:hsv.s,v:hsv.v,a:1})})`; };
    const sync = () => { const hsv = rgbToHsv(text && text.value); %&field.__stHsv = hsv; [['.st-fld-picker-h',hsv.h],['.st-fld-picker-s',hsv.s],['.st-fld-picker-v',hsv.v],['.st-fld-picker-a',hsv.a]].forEach(([sel,val]) => { const el=%&field.querySelector(sel); if(el) el.value=val; }); paint(hsv); };
    const slide = event => { const el=event.target, cached=%&field.__stHsv || rgbToHsv(text && text.value), hsv={...cached}; if(el.classList.contains('st-fld-picker-h'))hsv.h=Number(el.value);else if(el.classList.contains('st-fld-picker-s'))hsv.s=Number(el.value);else if(el.classList.contains('st-fld-picker-v'))hsv.v=Number(el.value);else if(el.classList.contains('st-fld-picker-a'))hsv.a=Number(el.value); %&field.__stHsv=hsv; const value=hsvToHex(hsv); if(text){text.value=value;const ev=new Event('input',{bubbles:true});ev.stFieldCommit=true;text.dispatchEvent(ev);} if(swatch && /^#[0-9A-F]{6}$/.test(value))swatch.value=value; paint(hsv); };
    const open = %&field.querySelector('.st-fld-picker-open'); if(open) open.addEventListener('change', sync); %&field.querySelectorAll('.st-fld-picker').forEach(el => el.addEventListener('input', slide)); sync();
    %cleanup { if(open) open.removeEventListener('change', sync); %&field.querySelectorAll('.st-fld-picker').forEach(el => el.removeEventListener('input', slide)); }
  }
}

/// Hydrate a checkbox from its descriptor value.
///
/// `checked` is DOM STATE, not an attribute a hole can render, so a shared
/// toggle mounted with `value: true` still appeared OFF and the user's first
/// click turned it ON — inverting the meaning of every boolean it edits. The
/// admin got away without this because its own `adHydrateControls` fills
/// `.ad-form` checkboxes; nothing does that for the pill or the workbench.
%primitive field-toggle-control(&field) {
  %emit js {
    const input = %&field.querySelector('input[type="checkbox"]');
    if (input) {
      const raw = %&field.dataset ? %&field.dataset.checked : '';
      const on = raw === 'true' || raw === '1' || raw === 'on' || raw === 'yes';
      input.checked = on;
    }
  }
}
