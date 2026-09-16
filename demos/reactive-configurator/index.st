
/* ============================================================================
   Styling (plain CSS — emitted to spacetime.css alongside the reactive
   bindings above). Dark, minimal, awwwards-leaning.
   ============================================================================ */

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
  backdrop-filter: blur(12px);
}

.eyebrow {
  margin: 0;
  font-size: 0.75rem;
  letter-spacing: 0.2em;
  text-transform: uppercase;
  color: #7c83ff;
}

.card__head h1 {
  margin: 0.25rem 0 0.5rem;
  font-size: 1.9rem;
  letter-spacing: -0.02em;
}

.lede {
  margin: 0 0 1.75rem;
  color: #9aa0b4;
  font-size: 0.92rem;
  line-height: 1.5;
}

.control {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.9rem 0;
  border-top: 1px solid rgba(255,255,255,0.07);
}

.control__label {
  font-size: 0.95rem;
  color: #c7ccdb;
}

.stepper {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  background: rgba(0,0,0,0.3);
  border-radius: 12px;
  padding: 0.25rem;
}

.stepper button {
  width: 38px;
  height: 38px;
  border: none;
  border-radius: 9px;
  background: rgba(255,255,255,0.06);
  color: #e8eaf2;
  font-size: 1.25rem;
  cursor: pointer;
  transition: background 0.15s ease, transform 0.1s ease;
}

.stepper button:hover { background: rgba(124,131,255,0.25); }
.stepper button:active { transform: scale(0.94); }

.stepper__value {
  min-width: 3ch;
  text-align: center;
  font-variant-numeric: tabular-nums;
  font-size: 1.1rem;
  font-weight: 600;
}

.pill {
  border: 1px solid rgba(124,131,255,0.4);
  background: rgba(124,131,255,0.12);
  color: #c9ccff;
  padding: 0.5rem 1.1rem;
  border-radius: 999px;
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease;
}

.pill:hover { background: rgba(124,131,255,0.22); }

.savings-note {
  margin: 1rem 0 0;
  font-size: 0.82rem;
  color: #7c83ff;
  min-height: 1.2em;
}

.summary {
  margin-top: 1.75rem;
  padding-top: 1.5rem;
  border-top: 1px solid rgba(255,255,255,0.1);
}

.summary__row {
  display: flex;
  justify-content: space-between;
  color: #9aa0b4;
  font-size: 0.9rem;
  margin-bottom: 0.75rem;
}

.summary__total {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
}

.summary__amount {
  font-size: 2rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
}

.billing-period {
  font-size: 0.95rem;
  font-weight: 500;
  color: #9aa0b4;
  margin-left: 0.15rem;
}
