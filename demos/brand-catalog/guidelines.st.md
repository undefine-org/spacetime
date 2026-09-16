# Aurora Guidelines

How the brand's forms are meant to be used. This page is a **literate
Spacetime document** (`.st.md`): the prose is prose, and the ```st fences
ARE the program — they run where they sit, against the same `_prelude.st`
declarations the catalog lists.

## One attribute, one home

A brand attribute lives in exactly one place: `_prelude.st`. Pages never
restyle a card, retune a curve, or re-time an entrance — they splice the
form. If a surface needs a variant, the variant is a **parameter**, not a
copy.

```st src
.card-a { --card-surface; }        // the default
.card-b { --card-surface(36px); }  // the variant, one argument
```

<div class="guide-row">
  <div class="card-a"><p>Default surface.</p></div>
  <div class="card-b"><p>36px breathing room.</p></div>
</div>

## Motion is a brand decision

Everything eases with `--aurora-glide`. Authors never write a raw
`cubic-bezier` — the curve is declared once, registered into the runtime by
the compiler, and resolved by name everywhere:

```st src
.enter { @on &.visible: --rise; }
```

<div class="enter guide-chip">I arrived on the brand curve.</div>

## Scores are choreography, not timing

Durations belong to drivers; a score declares **who moves when** relative to
its parent window. `--duet` splits a window in two — the same form works
under a scroll driver here and a timed driver in the hero reel, unchanged.

## The spec kinds

`value` and `markup` forms are declarations of intent today — `--golden` and
`--badge` are written down canonically so their consumers (landing in a
follow-up wave) have exactly one definition to honor.

```st hidden
body {
  font-family: system-ui, sans-serif;
  color: #1c1926;
  background: #faf9fc;
  max-width: 760px;
  margin: 0 auto;
  padding: 48px 32px 120px;
  line-height: 1.65;
}
h1 { font-size: 36px; letter-spacing: -0.02em; margin: 0 0 8px; }
h2 { margin-top: 48px; }
code { background: #f1eef8; padding: 2px 6px; border-radius: 6px; font-size: 0.9em; }
pre {
  background: #f8f6fc;
  border: 1px solid #efeaf8;
  border-radius: 10px;
  padding: 14px 16px;
  overflow-x: auto;
}
pre code { background: none; padding: 0; }
.guide-row { display: flex; gap: 16px; margin: 18px 0; }
.guide-row p { margin: 0; color: #5a5470; font-size: 14px; }
.guide-chip {
  display: inline-block;
  background: #fff; border: 1px solid #e7e3ee; border-radius: 12px;
  padding: 14px 22px; font-weight: 600; color: #39334e; margin-top: 8px;
}
```
