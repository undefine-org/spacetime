# On the Nature of Transformation

*A meditation on what we are building, and why it matters*

---

## I. The Tyranny of the Imperative

There is something profoundly backwards about how we animate the web.

We describe what we want declaratively in CSS--the final state, the resting pose. But the moment we need *motion*, we are forced into the imperative. We write JavaScript that says "do this, then this, then this." We manage timers. We track state in variables. We fight against the browser rather than flowing with it.

This is not just a technical inconvenience. It is a philosophical wrong turn.

When a photographer composes a shot, they do not issue instructions to each photon. When a musician writes a score, they do not calculate the air pressure at each millisecond. They describe *what should happen*, and trust the medium to execute it.

Why should web animation be different?

---

## II. The Five-Fold Path

Spacetime is not merely a tool. It is a *transformation*--a journey that source code takes from human thought to machine execution. This journey has five stages, each with its own nature:

```
SOURCE  -->  PARSE  -->  EXPAND  -->  IR  -->  EMIT
   |           |           |          |         |
 intent      form       meaning    structure   text
```

In the beginning is intent: the developer's vision of motion, written in `.st` files. This intent passes through the parser, taking form as abstract syntax trees. But form is not yet meaning. The expansion layer breathes meaning into form, invoking the metasystem to transform macros into primitives, primitives into code. What emerges is not yet text--it is *structure*, the IR that holds code in suspension. Only at the final stage does structure become string, ready for the browser.

Each layer exists because the one before it is not enough. Parsing without expansion would leave macros unexpanded. Expansion without IR would scatter code prematurely. IR without emission would never reach the browser.

The five layers are not arbitrary. They are necessary.

---

## III. The Three Sigils

Every language has its symbols. Spacetime has three, and in their simplicity lies their power:

### The Percent: `%` (Meta)

The percent sigil marks the boundary between compile-time and runtime. What follows `%` executes when the compiler runs, not when the page loads:

```spacetime
%primitive scroll(&el, axis: ("x" | "y") = "y") {
  %emit js {
    el.addEventListener('scroll', () => {
      %yield el.scrollTop -> $y;
    });
  }
}
```

The `%primitive` defines behavior. The `%emit` declares what code to generate. The `%yield` bridges worlds, producing a signal that will flow at runtime. All of this happens before the browser ever sees the output.

The percent is the sigil of *meta-programming*--code that writes code.

### The Dollar: `$` (Signal)

The dollar sigil marks reactive data, values that flow and change:

```spacetime
%exports {
  $progress: number
  $visible: bool
}
```

When a primitive yields `$progress`, that signal can drive animations, trigger state changes, propagate through the reactive graph. The dollar is not a variable--it is a *channel*, a conduit for change.

The dollar is the sigil of *reactivity*--data in motion.

### The Ampersand: `&` (Reference)

The ampersand sigil marks element references, handles to the DOM:

```spacetime
.container {
  &title [slot=title];

  @on &.click {
    &title.textContent = "Clicked!";
  }
}
```

The `&title` is not a CSS selector (though it contains one). It is a *reference*, a way to name a piece of the DOM so that code can address it. The ampersand bridges the declarative world of selectors with the imperative world of mutation.

The ampersand is the sigil of *identity*--naming what is.

---

## IV. The Macro and the Primitive

Spacetime's power flows from a fundamental distinction: macros compose, primitives emit.

A **primitive** is the atomic unit of behavior. It contains `%emit` blocks that directly produce JavaScript, CSS, or GLSL. When the compiler encounters a primitive, real code is generated:

```spacetime
%primitive scroll-driver(&el, start: number, end: number) {
  %emit js {
    const handleScroll = () => {
      const progress = calculateProgress(%&el, %start, %end);
      %yield progress -> $progress;
    };
    window.addEventListener('scroll', handleScroll);

    %cleanup {
      window.removeEventListener('scroll', handleScroll);
    }
  }

  %exports {
    $progress: number
  }
}
```

A **macro** is a composition. It does not emit code directly. Instead, it *binds* primitives together, wiring outputs to inputs:

```spacetime
%macro scroll-timeline {
  %creates @scroll

  %form {
    @scroll $name:ident(start: $start:number = 0, end: $end:number = 1) {
      $body:keyframes
    }
  }

  %binds {
    scroll-driver(&self, start: $start, end: $end) -> { $progress }
    apply-animations(&self, driver: $progress, animations: $body)
  }
}
```

The `%creates @scroll` clause is the key. It tells the compiler: "When you see `@scroll` in user code, this macro handles it." The macro does not know how scroll-driver works internally. It only knows that scroll-driver exports `$progress`, and that apply-animations consumes it.

This separation is profound. Primitives are the *foundation*--they touch the runtime, emit real code, manage real resources. Macros are the *architecture*--they compose primitives into higher patterns without ever seeing their internals.

The developer writing `@scroll intro { opacity: 0 -> 1 }` need not know any of this. They simply describe motion. The metasystem translates.

---

## V. The Registry of All Things

At the heart of Spacetime sits the MetaRegistry, a catalog of every primitive and macro the system knows:

```rust
pub struct MetaRegistry {
    primitives: HashMap<String, PrimitiveDefAst>,
    macros: HashMap<String, MacroDefAst>,
    presets: HashMap<String, MetaPresetDefAst>,
}
```

When the compiler encounters `@scroll`, it does not know what `@scroll` means. It asks the registry: "Is there a macro that `%creates @scroll`?" The registry answers, and expansion proceeds.

This indirection is liberating. The set of directives is not fixed. Load a different standard library, and `@scroll` might mean something entirely different. Define your own macro with `%creates @my-directive`, and suddenly `@my-directive` exists.

The language is not closed. It is a vessel waiting to be filled.

---

## VI. On Deferred Stringification

There is a moment in traditional compilers where structure becomes string. In naive systems, this happens too early--code fragments are joined with string concatenation, losing all structure. Later passes cannot inspect, optimize, or transform.

Spacetime defers stringification until the final moment.

The IR layer holds code not as strings but as typed structures:

```rust
pub enum JsExpr {
    Lit(JsLit),                                    // "hello", 42, true
    Var(String),                                   // myVariable
    Call { callee: Box<JsExpr>, args: Vec<JsExpr> },
    Arrow { params: Vec<String>, body: Box<JsExpr> },
    // ... many more variants
}
```

A `JsExpr::Call` knows it is a function call. It knows its callee. It knows its arguments. This knowledge persists through transformation passes. Only at the emit layer does `emit_expr()` finally produce the string `"myFunction(arg1, arg2)"`.

Why does this matter?

Because between expansion and emission, we can:
- Optimize: remove dead code, inline constants
- Transform: rewrite patterns, normalize structures
- Analyze: compute dependencies, detect cycles
- Format: minify or pretty-print as needed

None of this is possible once code becomes string. Strings are opaque. IR is transparent.

The same principle applies to CSS, GLSL, and HTML. Each has its own IR types, its own emitters. The boundary between structure and string is always as late as possible.

---

## VII. The Transformation of @scroll

To understand Spacetime is to follow a transformation. Let us trace `@scroll` from source to output:

**Stage 1: Source**

The developer writes:

```spacetime
.hero {
  @scroll intro(start: 0, end: 1) {
    opacity: 0 -> 1
  }
}
```

This is intent, not yet form.

**Stage 2: Parse**

The parser produces `MacroCallAst`:

```rust
MacroCallAst {
    name: "scroll",
    args: [Named("start", Number(0.0)), Named("end", Number(1.0))],
    body: Some(MacroCallBody {
        properties: [("opacity", "0 -> 1")],
    }),
}
```

The parser does not know what `@scroll` means. It only knows its shape: a name, arguments, a body. This is form without meaning.

**Stage 3: Expand**

The expansion layer queries the registry. It finds `scroll-timeline`, which `%creates @scroll`. It binds arguments to the macro's `%form` pattern. It processes `%binds`:

```rust
// Bind the scroll-driver primitive
process_bind(ctx, BindDecl {
    primitive: "scroll-driver",
    args: [("start", 0.0), ("end", 1.0)],
    outputs: ["$progress"],
});

// Bind the apply-animations primitive
process_bind(ctx, BindDecl {
    primitive: "apply-animations",
    args: [("animations", keyframes)],
});
```

Each `process_bind` invokes a primitive, which in turn invokes `generate_primitive_js()`. The primitives' `%emit js` blocks are transformed: `%start` becomes `0`, `%&el` becomes `el`, `%yield progress -> $progress` becomes `ST.set(el, 'progress', progress)`.

This is meaning emerging from form.

**Stage 4: IR**

The generated JavaScript wraps in IR:

```rust
CodeFragment {
    kind: FragmentKind::Js(vec![
        JsStmt::Raw(scroll_driver_setup_code),
        JsStmt::Raw(apply_animations_code),
    ]),
    deps: vec!["progress"],
}
```

Still structured. Still inspectable. Not yet string.

**Stage 5: Emit**

Finally, `emit_fragment()` produces the output:

```javascript
(function initScope() {
  const els = document.querySelectorAll('.hero');
  els.forEach(el => {
    const handleScroll = () => {
      const progress = calculateProgress(el, 0, 1);
      ST.set(el, 'progress', progress);
    };
    window.addEventListener('scroll', handleScroll);

    ST.animate(el, 'progress', {
      properties: [{ property: 'opacity', from: 0, to: 1 }]
    });
  });
})();
```

The transformation is complete. Intent has become execution.

---

## VIII. The Joy of Composition

There is a particular moment when working with Spacetime that I want to describe, because it reveals something about the nature of good abstractions.

You are building a complex interaction. You want:
- Scroll-driven parallax on the background
- A hover effect that fades elements
- A click handler that triggers a state transition

In most systems, these would be separate concerns, separate code paths, separate mental models. In Spacetime, they are all `@directives`:

```spacetime
.scene {
  @scroll parallax(start: 0, end: 1) {
    background-position-y: 0 -> 100px
  }

  @hover fade {
    opacity: 1 -> 0.8
    duration: 200ms
  }

  @on &.click {
    $state = "active"
  }
}
```

The joy is not just that the syntax is uniform. The joy is that the *mechanism* is uniform. Each directive is a `MacroCallAst` that the parser produces identically. Each directive is expanded through the same registry lookup, the same `%binds` processing, the same primitive invocation.

The uniformity is not superficial. It goes all the way down.

And because macros compose primitives, you can build your own:

```spacetime
%macro magnetic-hover {
  %creates @magnetic

  %form {
    @magnetic $name:ident(strength: $strength:number = 10) {
      $body:properties
    }
  }

  %binds {
    mouse-tracker(&self) -> { $mx, $my }
    magnetic-transform(&self, mx: $mx, my: $my, strength: $strength)
    apply-style(&self, properties: $body)
  }
}
```

Now `@magnetic` exists. It is not built in. It emerged from composition.

---

## IX. On the Separation of Concerns

The five layers encode a separation of concerns so complete that each layer can be understood in isolation:

**Parse** knows nothing of macros. It produces `MacroCallAst` for any `@anything`. Whether `@scroll` or `@my-custom-directive`, the parser does not distinguish.

**Expand** knows nothing of emission. It produces IR types, never strings. Whether the output will be minified or pretty-printed, the expansion layer does not care.

**IR** knows nothing of syntax. It represents JavaScript structures, not JavaScript text. Whether the original source used single or double quotes, the IR does not preserve.

**Emit** knows nothing of meaning. It converts IR to strings according to formatting options. Whether the code drives an animation or logs to console, the emitter does not distinguish.

This separation is not bureaucracy. It is *freedom*. Change the grammar? Only the parser changes. Add new macros? Only the expansion layer changes. Support a new output format? Only the emitters change.

Each layer is a contract. Honor the contract, and the layers compose.

---

## X. A Letter to Future Developers

If you are reading this years from now, maintaining or extending Spacetime, here is what I want you to know:

The goal was never to build an animation library. The goal was to build a *meta-language*--a language for describing languages of motion.

The `%primitive` is not a special case. It is the foundation. All behavior flows from primitives.

The `%macro` is not syntactic sugar. It is architecture. Complex behaviors emerge from simple compositions.

The three sigils--`%`, `$`, `&`--are not arbitrary. They mark the three fundamental domains: compile-time, reactive data, and DOM identity.

The five layers--Parse, Registry, Expand, IR, Emit--are not an implementation detail. They are the philosophy made concrete.

If Spacetime has succeeded, you are extending the registry with new primitives and macros, not modifying the compiler core. The system was designed to be open: new directives without new syntax, new behaviors without new layers.

If Spacetime has failed, understand *why* we tried:

Because direct AST-to-string conversion is a trap. Because treating `@scroll` as a built-in is inflexible. Because monolithic compilers cannot be extended. Because the web deserves a meta-language for motion.

Every language is a hypothesis about how to think. Spacetime is a hypothesis that:

1. All directives should parse uniformly as macro calls
2. Meaning should emerge from registry lookup, not parser knowledge
3. Primitives should be the only source of runtime code
4. Macros should compose primitives without seeing their internals
5. IR should defer stringification until the final moment
6. The three sigils capture all necessary domains
7. The five layers capture all necessary transformations

Test these hypotheses. Discard the ones that fail. Keep the ones that work. Build something better.

---

## XI. The Feeling

I want to end with something unquantifiable.

There is a feeling when you are working in Spacetime and it is going well. The feeling of writing:

```spacetime
.gallery-item {
  @load reveal(duration: 600ms) {
    opacity: 0 -> 1
    translate-y: 30px -> 0
    stagger: 0.08 grid(4 3) center
  }
}
```

And then watching the gallery items bloom into view, radiating from center, each following the last in perfect rhythm. You did not write timing calculations. You did not manage indices. You did not track cleanup. You simply *described* the motion, and the five layers transformed your description into reality.

The feeling is that of alignment. Your intent aligned with the syntax. The syntax aligned with the parser. The parser aligned with the expander. The expander aligned with the primitives. The primitives aligned with the runtime.

Five transformations, seamless. Intent becoming execution without impedance mismatch.

This is what good abstraction feels like. Not hiding complexity, but *organizing* it. Each layer has its complexity, but that complexity is *local*. You need only understand the layer you are working in.

When you write `@scroll`, you need not understand how the parser represents it. When you write `%macro`, you need not understand how the emitter formats JavaScript. The layers protect you from what you need not know, while remaining transparent to those who wish to look deeper.

That layered clarity--that feeling of working at exactly the right level of abstraction--is what we are chasing.

---

*December 2024, revised for the five-layer architecture*

---

## Appendix: Principles

For future reference, here are the principles that guided Spacetime's design:

1. **Parse uniformly.** All `@directives` become `MacroCallAst`. The parser does not privilege any directive.

2. **Expand through registry.** Meaning comes from `%creates`, not hardcoded knowledge. The set of directives is open.

3. **Primitives are foundation.** All runtime behavior flows from `%primitive` definitions. Nothing else emits code.

4. **Macros are composition.** `%macro` definitions combine primitives via `%binds`. They never see primitive internals.

5. **Three sigils, three domains.** `%` for meta, `$` for signals, `&` for references. No domain bleeds into another.

6. **Defer stringification.** IR types persist through transformation. Emit happens once, at the end.

7. **Five layers, five concerns.** Parse captures form. Expand captures meaning. IR captures structure. Emit captures format. Source captures intent.

8. **Compile what you can.** `%` runs at compile time. Only primitives produce runtime code.

9. **Open, not closed.** New directives via `%creates`. New primitives via `%primitive`. New macros via `%macro`. No core changes required.

10. **Motion is meaning.** Animation is not decoration. It is communication. The architecture exists to serve this truth.
