# W9 — shots: forms that carry markup *and* motion

*Working doc for PLAN-150 W9. Builds and runs today. The "shots" of
`../film.st.md` §10.*

```st hidden
@import "stdlib/md"
```

## The problem it solves

The same card reveal was pasted six times. A "component with a timeline" — the
thing AND how it enters — had no single spelling.

## The word

A **shot** is a form whose body has a `:markup` slot and a `:motion` slot, using
the `:slot` shape `@each` uses:

```st src
@form shot --card-reveal($title, $price, $rise = 40px) {
  :markup {
    <div class="card">
      <div class="row"><span class="nm">`$title`</span><span class="price">`$price`</span></div>
    </div>
  }
  :motion {
    opacity: 0 -> 1; translate-y: $rise -> 0;
  }
}

.panels { &card-reveal("Ceramic Mug", "$24.99"); }
```

Invoking `&card-reveal(…)` instantiates the markup (a template, with the params
interpolated) AND runs the motion as it mounts. The `:markup` slot **is** a
template; `:motion` is the same keyframe body an `@on &.clip` takes, so anything
in §2–§7 works inside it.

## How it lowers

A shot registers a template factory (`stdlib/macros/form.st` binds
`register-template`) whose body is the `:markup` slot and whose animations are
the `:motion` slot. The form declares under `--card-reveal` and is invoked as
`&card-reveal`; the register/invoke names are normalised so the two sigils meet
on one key. On mount, the factory plays the motion once as a reveal
(`Spacetime.applyAnimations` — a one-shot progress 0 → 1 over the motion's
duration). No new interpolation path.

Landing this also fixed a dead reference: `@template`'s `animations:` slot called
`Spacetime.applyAnimations`, which was never defined — so template-attached
motion had silently never run. It runs now.

## What this deletes

Six pasted card blocks in the promo, and the split between "the thing" and "how
it enters".

## Not yet (later in W9's arc)

- Placing a shot on a `@score` window (`@score &.clip { &card-reveal(…) for
  100%; }`) so its motion binds to the clip window rather than a mount reveal.
- Per-region nested motion (`.price { color: … -> …; range: 60% to 100%; }`).
- `:motion` easing / `@post` / camera forms composed inside a shot.
