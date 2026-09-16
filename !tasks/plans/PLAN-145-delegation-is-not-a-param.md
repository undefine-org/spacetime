# `target:`/`matching:` are the wrong answer — you were right

Short version: **you are right, and further than you put it.** Not only is
`target:` the wrong *spelling*, delegation is the wrong *mechanism* — Spacetime
already solves the problem delegation exists to solve, by a different route, and
`&name` already covers the case a param was being proposed for.

I verified each step rather than agreeing on plausibility. Here is the chain.

---

## 1. The facet you describe already exists and already works

`stdlib/syntax/element-ref.st` declares:

```spacetime
&item .item;          // bind the name `item` to the element matching `.item`
```

It registers the resolved element as `window.__stRefs['item']`, exports
`$width`/`$height`/`$top`/`$left`, and keeps them live under a `ResizeObserver`.

And `@on` already accepts it in exactly the position you propose:

```spacetime
&item .item;
.list { @on &item.click { $n <- 1; } }
```

That compiles, and the emit proves it is honored — not merely accepted:

```js
const __stTargetEl = ST.ref("item");   // @on &item.click
const __stTargetEl = el;               // @on &.click
```

An A/B build diff of those two sources differs at exactly that line. So the
`&name.event` rail is real, live, and the design intent is already in the tree.
It was never mentioned in BUG-334 — which is how a *parameter* came to be
proposed for a job the *sigil* already has.

---

## 2. So what is `&` actually for?

Your question — *what would be the point of it being a facet?* — is the right
one. Reading the two spellings together:

| spelling | meaning |
|---|---|
| `&.click` | *this* element (the selector scope I am written in) |
| `&item.click` | the element named `item` |

`&` means **"an element"** in both. The leaf after the dot is the driver. That is
one rule, learnable in one sitting, and it extends: a new driver works on both
spellings for free, because the sigil resolved the element before the driver was
consulted.

Introducing `@on &.click(target: ".item")` would break that. The element would
then be named *two* ways — by the sigil for one case, by a parameter for
another — and an author would have to know which situation calls for which.
That is precisely the failure your AGENTS rule names: *knowing one thing should
tell you the next.* Here, knowing `&item.click` would tell you nothing about
`target:`, and vice versa.

Worse for the metasystem: `target:` is a param on ONE primitive. Every future
driver would have to redeclare it to get the same capability, or silently not
have it. The sigil composes; the param does not.

---

## 3. But does `&name` cover DELEGATION specifically?

This is where I expected to find a real gap, and did not.

Delegation exists in DOM programming for two reasons:

1. **Many elements** — one listener on a parent beats N listeners on N children.
2. **Elements that don't exist yet** — rows added later still work, because the
   parent was listening all along.

`ST.ref` returns a *single* element (`__stRefs[name] || null`), so `&name` alone
does not cover either. If Spacetime had no other answer, delegation would be a
genuine missing capability.

**It has another answer.** From `public/runtime/st.js:195`:

```js
/**
 * Register a selector-based initializer for dynamic elements.
 * Used by generated code to support @each/@template created elements.
 */
ST.registerSelectorInit = function(selector, init) { ... }
```

And a `MutationObserver` on `document.body` with `{ childList: true, subtree:
true }` feeds `ST._scheduleInit(node)` for every added element, batched through
`_pendingInit` and flushed per selector.

I verified the whole chain on a real build. Given:

```spacetime
<ul class="list">
  @each $item in $items { <li class="row">`$item.label`</li> }
</ul>

.row { @on &.click { $n <- 1; } }
```

the emit ends with:

```js
ST.registerSelectorInit('.row', init);
ST._scheduleInit(document.body);
```

and `init` attaches the listener with an idempotency guard and a cleanup hook:

```js
const __stOnSet = (__stTargetEl.__stOnBound || (__stTargetEl.__stOnBound = new Set()));
if (!__stOnSet.has(__stOnSig)) {
  __stOnSet.add(__stOnSig);
  __stTargetEl.addEventListener(eventType, handler);
  ST.onCleanup(__stTargetEl, () => { ... });
}
```

So both delegation motives are already handled — **by the selector scope
itself**:

- *Many elements* → the initializer runs per matching element, so `.row { @on
  &.click … }` wires every row.
- *Elements added later* → the MutationObserver schedules init for new nodes;
  a row appended in five minutes gets its handler.

∴ `@on &.click(target: ".item")` on the parent and `.item { @on &.click { … } }`
produce the same user-visible behavior. The second is already the language's
way, needs no new vocabulary, and reads as what it is: *this is what a `.item`
does when clicked.*

---

## 4. What this means for BUG-334

BUG-334 says *"event delegation lost its spelling in the `@on` sigil cutover."*
That framing is wrong in a way worth correcting on the ticket: **delegation
never had an author-facing spelling to lose.** The `delegateSelector` machinery
is a lower-level implementation detail that the old dispatcher happened to
expose; the cutover did not delete a feature, it stopped surfacing an internal
knob.

So the options are not *"which word do we pick"* but:

### Option 1 — no author-facing delegation param at all (recommended)

Selector scope + `&name` already span the use cases. `delegateSelector` remains
what it is: an internal parameter of `on-mutation-handler`, hardcoded `null`
because nothing author-facing sets it.

- ✓ No new vocabulary. One way to say "which element": the `&` sigil.
- ✓ Nothing to learn, nothing to document, nothing to keep consistent across 28
  drivers.
- ✓ BUG-352's validation then does the right thing *for free*: `from:` on a
  click driver becomes a clear error naming the accepted params, instead of
  silence.
- ✗ Loses one genuine optimization: **one** listener instead of N. For a
  thousand-row table that is a real difference. But that is a *performance*
  concern, and the right fix is for the compiler to choose delegation as an
  emit strategy when a selector scope is inside a repeat — invisible to the
  author, no syntax at all.

### Option 2 — surface it later, as a strategy hint, not an element name

If the N-listener cost ever measurably bites, the honest spelling is one that
says *how*, not *what*:

```spacetime
.row { @on &.click(delegated: true) { … } }
```

The element is still named by the scope; the parameter tunes the mechanism. That
keeps the "one way to name an element" invariant intact. But this is speculative
and should wait for a page that actually needs it — the entire class of bugs in
PLAN-145 came from parameters that existed before a use case did.

> **Recommendation: Option 1.** Close BUG-334 as *not a defect* — with the
> correction recorded — and let BUG-352's refusal give `from:` a real error
> message. That deletes a proposed surface instead of adding one, which is the
> elegance bar working as intended.

---

## 5. What I got wrong, and why

I proposed `target:` because I traced from the *implementation upward*: the
primitive declares `target: selector?`, the runtime has a delegation path, the
`param_map` mechanism exists to route it — therefore surface it. Every step true,
conclusion wrong.

I never asked the design question you asked: *given `&` is a facet for naming
elements, why would an element need naming a second way?* Had I looked at `&`
first, `@on &item.click` was one grep away, and it is right there in a comment
in the driver primitive I was already reading.

The lesson generalizes past this bug: **an unwired internal parameter is not
evidence of a missing feature.** It is equally consistent with a feature that
was deliberately *not* surfaced, or one superseded by a better mechanism. Both
turned out to be true here. Before proposing syntax for an orphaned knob, the
question is *"what does the language already offer for this need"* — not *"what
should we call this knob."*

That is the same shape as the `once:` error earlier in this arc (concluding
"unimplemented" from a grep that could not see `stdlib/`), and both would have
been caught by the same habit: **look for the existing affordance before adding
a parallel one.** Which is, verbatim, the standing instruction I am given —
*"∃ affordance? → extend, don't add parallel."*

---

## Revised decision table

| # | decision | status |
|---|---|---|
| 1 | delegation param name | **dissolved** — no param; `&name` + selector scope already cover it |
| 2 | `once:` on `visible` | **dissolved** — already implemented, route it |
| 3 | zero-page build | open — warn / refuse-all / refuse-routes |
| 4 | stdlib location | open — toolchain+project / project-only / explicit |

Two remain, and neither is about syntax an author types: one is what `✓`
promises, the other is where a file is found.
