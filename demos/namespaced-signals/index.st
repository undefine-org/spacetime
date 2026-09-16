/* ============================================================================
   Namespaced Signals — grouping related state under one object signal
   No custom JavaScript. A single $pan object signal holds related fields
   (deltaX / deltaY / scale); file-scope bindings read individual fields with
   dot-notation ($pan.deltaX) and react when the object mutates.

   Patterns:
     1. @data ... : object  — one signal grouping related fields
     2. .sel { text: $obj.field } — namespaced (dotted) field binding
     3. @on &.click { $obj <- { ...new } } — replace the object; fields re-render
   ============================================================================ */

@data inline $pan : { "deltaX": 0, "deltaY": 0, "scale": 100 };

/* Namespaced field bindings — each reads one field of the $pan object. */
.val-x     { text: $pan.deltaX; }
.val-y     { text: $pan.deltaY; }
.val-scale { text: $pan.scale; }

/* Interactions mutate the grouped object; every field binding re-renders. */
.left  { @on &.click { $pan <- { "deltaX": $pan.deltaX - 10, "deltaY": $pan.deltaY, "scale": $pan.scale }; } }
.right { @on &.click { $pan <- { "deltaX": $pan.deltaX + 10, "deltaY": $pan.deltaY, "scale": $pan.scale }; } }
.up    { @on &.click { $pan <- { "deltaX": $pan.deltaX, "deltaY": $pan.deltaY - 10, "scale": $pan.scale }; } }
.down  { @on &.click { $pan <- { "deltaX": $pan.deltaX, "deltaY": $pan.deltaY + 10, "scale": $pan.scale }; } }
.zoom  { @on &.click { $pan <- { "deltaX": $pan.deltaX, "deltaY": $pan.deltaY, "scale": $pan.scale + 10 }; } }
.reset { @on &.click { $pan <- { "deltaX": 0, "deltaY": 0, "scale": 100 }; } }

/* ---- Styling ------------------------------------------------------------- */

body {
  margin: 0;
  min-height: 100vh;
  display: grid;
  place-items: center;
  background: radial-gradient(120% 120% at 50% 0%, #1a1d29 0%, #0b0c10 60%);
  color: #e8eaf2;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

.card {
  width: min(420px, 92vw);
  padding: 2.5rem;
  border-radius: 20px;
  background: linear-gradient(180deg, rgba(255,255,255,0.06), rgba(255,255,255,0.02));
  border: 1px solid rgba(255,255,255,0.08);
  box-shadow: 0 30px 80px rgba(0,0,0,0.45);
}

.eyebrow {
  margin: 0;
  font-size: 0.75rem;
  letter-spacing: 0.2em;
  text-transform: uppercase;
  color: #7c83ff;
}

.card__head h1 { margin: 0.25rem 0 1.5rem; font-size: 1.7rem; letter-spacing: -0.02em; }

.readout {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
}

.readout__cell {
  flex: 1;
  text-align: center;
  padding: 1rem 0.5rem;
  border-radius: 12px;
  background: rgba(0,0,0,0.3);
  border: 1px solid rgba(255,255,255,0.06);
}

.readout__label {
  display: block;
  font-size: 0.7rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: #6b7080;
  margin-bottom: 0.4rem;
}

.readout__value {
  font-size: 1.6rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: #c9ccff;
}

.pad {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 0.5rem;
}

.pad button {
  padding: 0.8rem;
  border: 1px solid rgba(124,131,255,0.3);
  background: rgba(124,131,255,0.1);
  color: #c9ccff;
  border-radius: 10px;
  font-size: 1rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease, transform 0.1s ease;
}

.pad button:hover { background: rgba(124,131,255,0.22); }
.pad button:active { transform: scale(0.95); }

.pad .reset {
  border-color: rgba(255,107,107,0.4);
  background: rgba(255,107,107,0.1);
  color: #ffb3b3;
}

.spacer { visibility: hidden; }
