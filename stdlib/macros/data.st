/// Data Binding Macros
///
/// High-level macros for data fetching, iteration, and reactive binding.
/// Composes primitives from stdlib/primitives/data/.


/// Legacy `@data(src:, as:, refresh:, default:) { … }` selector-scoped paren-form
/// REMOVED in PLAN-023 W5/FEAT-047. Use the unified file-scope surface:
///   @data fetch $name T : "/url" { refresh: 5m; initial: … }
/// (stdlib/macros/data-kind.st). The data-source primitive is unchanged.


/// Legacy `@each(items:, template:, key:)` paren-form REMOVED in PLAN-023 W5/FEAT-047.
/// Use the canonical `@each($source as $item) { … }` (stdlib/macros/each.st), whose
/// body accepts inline HTML (each-inline) or template invocations (&row).


/// Reactive bindings — KILLED (@bind / @show / @input / @if-block) — FEAT-072.
///
/// The `@bind`, `@show`, `@input`, and `@if-block` macros were REMOVED in a hard
/// cutover (no back-compat; Spacetime is pre-release). The sole reactive-property
/// surface is now two operators, auto-detected by the parser (no macro):
///
///   .open: $flag;        -> classList.toggle('open', !!v)      (reactive class)
///   --accent: $color;    -> style.setProperty('--accent', v)   (reactive custom prop)
///   color: red;          -> static stylesheet (no $, passthrough)
///   text <- $v;          -> el.textContent = v                 (DOM content)
///   src  <- $.url;       -> el.setAttribute('src', v)          (DOM attribute)
///   [slot="x"] <- $v;    -> descendant slot textContent
///   value <- $x; + @on input { $x <- value }                   (two-way input)
///
/// All reactive values REQUIRE the `$` sigil. The bind-text/bind-class/bind-style/
/// bind-attr/bind-visible primitives (stdlib/primitives/data/) are RETAINED and still
/// drive the `:` / `<-` lowering. Only the reactive-attr `@if-block` paren-macro
/// was deleted here.
///
/// This note used to add that control-flow `@if $cond { … }` (block form,
/// mount/unmount) was "UNCHANGED" by that deletion. Technically true and
/// thoroughly misleading: the block form does not work either, and did not
/// before the deletion. Measured — it is refused by the parser:
///
///   .h { @if $on { <p class="yes">SHOWN</p> } }
///   -> "a directive body failed to parse and was dropped (BUG-216)"
///      expected scope block, directive, or declaration
///
/// `@if` has NO implementation in any spelling (BUG-362): `%macro conditional`
/// declares a `%form` and a `%when` but no `%emit` and no primitive, so the
/// paren form reports `E0956: unknown primitive: conditional` and the block
/// form is refused as above. See stdlib/macros/each.st.
///
/// TOMBSTONES REMOVED (PLAN-076/079): the `%form`-only `%macro bind/show/input-bind`
/// stubs that kept the retired directives RECOGNIZED (so `check_bind_deprecation`
/// could reject them with E0910) are now the CAPSULE
/// stdlib/migrations/entries/2026-06-09-reactive-surface.st — ONE %migration
/// embedding the real retired %macro definitions verbatim (grammar + %binds).
/// Rewrite-rule shapes auto-migrate in memory (W0715); @show/@input compile
/// through the window via their embedded %binds (W0715 + %hint); multi-shape
/// @bind calls degrade to E0910 with the entry's %hint.



/// Legacy `@computed(name:, from:, as:)` paren-form REMOVED in PLAN-023 W5/FEAT-047.
/// Use the unified surface: `@data derive $x : <expr>;` (scalar) or
/// `@data fold $x from $src : <step>;` (reduce) or `@data query` (array pipeline).


/// Persist Macro
///
/// Persists data to localStorage.

%macro persist-data {

  %form {
    @persist(
      /// localStorage key
      key: $key:string,
      /// Data to persist (reactive)
      data: $data:expr,
      /// Debounce delay in ms
      debounce: $debounce:time = 300ms
    )
  }

  %binds {
    persist(...) -> { $save, $clear }
  }

  %example "@persist-data" {
    @persist(key: "user-preferences", data: $preferences)
  }
}


/// Cycle Macro
///
/// Cycles through items in a data source on a timed interval,
/// updating the element's textContent each tick.
/// Pairs with @value-change for character-level animations.

%macro cycle {

  %form {
    @cycle(
      /// Name of the data source (registered via @data / ST.setData)
      source: $source:ident,
      /// Milliseconds between cycles
      interval: $interval:time = 3000ms,
      /// Optional object field to extract (for object arrays)
      field: $field:string? = ""
    )
  }

  %binds {
    text-cycle(&self, ...)
  }

  %example "@cycle" {
    @data inline $heroTerms : [{ text: "faster" }, { text: "calmer" }];
    .hero-flap {
      @cycle(source: heroTerms, interval: 3000ms, field: "text")
      @on &.text-change(duration: 400ms, stagger: 50ms) {
        :entering { opacity: 0 -> 1; translate-y: 8px -> 0; }
        :exiting { opacity: 1 -> 0; }
      }
    }
    <span class="hero-flap">faster</span>
  }
}


/// ERROR SIBLING (BUG-264): a `field:` value that is NOT a quoted string
/// (`field: $field`, `field: 5`) is a near-miss for the optional `$field:string?`
/// capture on `@cycle`. The productive `cycle` form REJECTS it (strict-capture:
/// `:string` won't bind a sigil or a number), so the clause would otherwise be
/// silently dropped and the field vanish. This sibling — which binds the
/// offending value — reports it. Declared AFTER `cycle`: the productive form
/// wins cleanly on a quoted string, leaving this sibling to fire only when the
/// field value is the wrong kind.
%macro cycle-invalid-field {
  %diagnostic



  %form {
    @cycle(
      source: $source:ident,
      interval: $interval:time = 3000ms,
      field: $field:expr
    )
  }

  %binds {
    optional-clause-error(
      message: "`field:` must be a quoted string",
      hint: "write `field: \"text\"` — the cycle field names which property of the source item to display"
    )
  }
}
