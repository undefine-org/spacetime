// Generative testing (PLAN-027 W4): @property / @fuzz support.
// A seeded PRNG + structural value generators + a shrinker. Values are generated
// from a compact type spec the macros emit (see stdlib/testing/generative.st):
//   { kind:'number', min, max } | { kind:'int', min, max } | { kind:'bool' }
//   { kind:'string', corpus } | { kind:'array', of:<spec>, max } | { kind:'union', of:[...] }
// =============================================================================

// Mulberry32 — a tiny deterministic PRNG so failures are reproducible by seed.
window.__stRng = (seed) => {
  let a = seed >>> 0;
  return () => {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
};

window.__stGenString = (rng, corpus) => {
  const pools = {
    ascii: 'abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789',
    unicode: 'a\u00e9\u4e2d\ud83d\ude00\n\t"\'\\<>&{} \u0000\u00ff',
    json: '{}[]":,0123456789truefalsenul \\',
    html: '<>/="\' abcdivspan&;{}',
  };
  const pool = pools[corpus] || pools.ascii;
  const len = Math.floor(rng() * 24);
  let s = '';
  for (let i = 0; i < len; i++) s += pool[Math.floor(rng() * pool.length)];
  return s;
};

// Split a binding string `name: spec` (the raw text inside forall(...)/inputs(...))
// into { name, spec } where spec is the parsed generator spec object.
window.__stParseBinding = (text) => {
  let t = (text || '').trim();
  // The balanced capture may include the wrapping parens from forall(...)/inputs(...).
  if (t.startsWith('(')) t = t.slice(1);
  if (t.endsWith(')')) t = t.slice(0, -1);
  t = t.trim();
  const i = t.indexOf(':');
  if (i < 0) return { name: t || 'x', spec: window.__stSpecFromText('string') };
  const name = t.slice(0, i).trim();
  const spec = window.__stSpecFromText(t.slice(i + 1).trim());
  return { name, spec };
};

// Parse a compact type-spec STRING (as written after `:` in a @property/@fuzz
// binding) into the spec object __stGen understands. Examples:
//   'int 0..100' 'number' 'number 1..2' 'bool' 'string' 'string ~ unicode' 'int[]'
window.__stSpecFromText = (text) => {
  let t = (text || '').trim();
  let isArray = false;
  if (t.endsWith('[]')) { isArray = true; t = t.slice(0, -2).trim(); }
  let spec;
  // string ~ corpus
  const corpusMatch = t.match(/^string\s*~\s*(\w+)$/);
  const rangeMatch = t.match(/^(int|number)\s+(-?[0-9.]+)\s*\.\.\s*(-?[0-9.]+)$/);
  if (corpusMatch) {
    spec = { kind: 'string', corpus: corpusMatch[1] };
  } else if (rangeMatch) {
    spec = { kind: rangeMatch[1], min: parseFloat(rangeMatch[2]), max: parseFloat(rangeMatch[3]) };
  } else if (t === 'int' || t === 'number') {
    spec = { kind: t };
  } else if (t === 'bool') {
    spec = { kind: 'bool' };
  } else if (t === 'string') {
    spec = { kind: 'string', corpus: 'ascii' };
  } else {
    // Unknown / @type name — fall back to a string (FEAT-065 will resolve @type).
    spec = { kind: 'string', corpus: 'ascii' };
  }
  return isArray ? { kind: 'array', of: spec, max: 16 } : spec;
};

window.__stGen = (spec, rng) => {
  switch (spec.kind) {
    case 'number': {
      const min = spec.min ?? 0, max = spec.max ?? 100;
      return min + rng() * (max - min);
    }
    case 'int': {
      const min = Math.ceil(spec.min ?? 0), max = Math.floor(spec.max ?? 100);
      return min + Math.floor(rng() * (max - min + 1));
    }
    case 'bool': return rng() < 0.5;
    case 'string': return window.__stGenString(rng, spec.corpus || 'ascii');
    case 'array': {
      const n = Math.floor(rng() * ((spec.max ?? 16) + 1));
      return Array.from({ length: n }, () => window.__stGen(spec.of, rng));
    }
    case 'union': return spec.of[Math.floor(rng() * spec.of.length)];
    default: return undefined;
  }
};

// Structural shrink: yield 'simpler' candidates toward a minimal failing case.
window.__stShrinkCandidates = (spec, value) => {
  const out = [];
  switch (spec.kind) {
    case 'number': case 'int':
      if (value !== 0) { out.push(0); out.push(spec.kind === 'int' ? Math.trunc(value / 2) : value / 2); }
      break;
    case 'string':
      if (value.length > 0) { out.push(''); out.push(value.slice(0, Math.floor(value.length / 2))); }
      break;
    case 'array':
      if (value.length > 0) {
        out.push([]);
        out.push(value.slice(0, Math.floor(value.length / 2)));
        if (value.length > 1) out.push(value.slice(1));
      }
      break;
  }
  return out;
};

// Run a property: generate `cases` inputs, find a failure, shrink it, throw the
// minimal counterexample. `check(value)` returns true on PASS, false/throw on FAIL.
window.__stProperty = (spec, cases, seed, check) => {
  const rng = window.__stRng(seed);
  const fails = (v) => { try { return !check(v); } catch (_e) { return true; } };
  for (let i = 0; i < cases; i++) {
    const v = window.__stGen(spec, rng);
    if (fails(v)) {
      // Shrink: greedily replace with a simpler still-failing candidate.
      let cur = v;
      let improved = true;
      while (improved) {
        improved = false;
        for (const cand of window.__stShrinkCandidates(spec, cur)) {
          if (fails(cand)) { cur = cand; improved = true; break; }
        }
      }
      throw new Error('property failed (seed ' + seed + ') minimal counterexample: ' + JSON.stringify(cur));
    }
  }
};

// Run a fuzz target: generate `cases` inputs; FAIL if `body(value)` throws or
// exceeds the per-case time budget (a hang proxy).
window.__stFuzz = (spec, cases, seed, budgetMs, body) => {
  const rng = window.__stRng(seed);
  for (let i = 0; i < cases; i++) {
    const v = window.__stGen(spec, rng);
    const t0 = (typeof performance !== 'undefined' ? performance.now() : Date.now());
    try {
      body(v);
    } catch (e) {
      throw new Error('fuzz crash (seed ' + seed + ') on input ' + JSON.stringify(v) + ': ' + e.message);
    }
    const dt = (typeof performance !== 'undefined' ? performance.now() : Date.now()) - t0;
    if (dt > budgetMs) {
      throw new Error('fuzz timeout (' + dt.toFixed(0) + 'ms > ' + budgetMs + 'ms, seed ' + seed + ') on input ' + JSON.stringify(v));
    }
  }
};

