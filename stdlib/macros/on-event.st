/// On Event Macro
///
/// Creates event-triggered animations for hover, click, focus, and visibility.
/// Uses driver primitives and apply-animations for direct JS/CSS emission.
///
/// @example hover
/// .card {
///   @on hover lift(300ms) {
///     translate-y: 0 -> -8px
///     box-shadow: none -> 0 4px 20px rgba(0,0,0,0.15)
///   }
/// }
///
/// @example click
/// .button {
///   @on click press(150ms) {
///     scale: 1 -> 0.95 -> 1
///   }
/// }
///
/// @example visible
/// .section {
///   @on visible reveal(800ms, threshold: 0.3) {
///     opacity: 0 -> 1
///   }
/// }

/// On Event Dispatcher
///
/// Routes @on to the appropriate event-specific macro.




/// On Hover Macro
///
/// Hover-triggered animation with forward/reverse on enter/leave.

%macro on-hover {

  %form {
    @on-hover $name:ident(
      duration: $duration:time = 300ms,
      easing: $easing:easing = ease-out,
      stagger: $stagger:stagger_value = 0,
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    event-driver(&self, name: $name, trigger: "mouseenter", untrigger: "mouseleave", duration: $duration, reverse: true) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom, interactive: true)
  }
}


/// On Click Macro
///
/// Click-triggered toggle animation.

%macro on-click {

  %form {
    @on-click $name:ident(
      duration: $duration:time = 300ms,
      easing: $easing:easing = ease-out,
      stagger: $stagger:stagger_value = 0,
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    event-driver(&self, name: $name, trigger: "click", toggle: true, duration: $duration, reverse: true) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom, interactive: true)
  }
}


/// On Focus Macro
///
/// Focus-triggered animation with forward/reverse.

%macro on-focus {

  %form {
    @on-focus $name:ident(
      duration: $duration:time = 300ms,
      easing: $easing:easing = ease-out,
      stagger: $stagger:stagger_value = 0,
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    event-driver(&self, name: $name, trigger: "focus", duration: $duration, reverse: true) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom, interactive: true)
  }
}


/// On Visible Macro
///
/// Visibility-triggered animation (intersection observer).

%macro on-visible {

  %form {
    @on-visible $name:ident(
      $duration:time,
      threshold: $threshold:number = 0.5,
      delay: $delay:time = 0ms,
      immediate: $immediate:bool = false,
      easing: $easing:easing = ease-out,
      stagger: $stagger:stagger_value = 0,
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    load-driver(&self, name: $name, duration: $duration, delay: $delay, threshold: $threshold, immediate: $immediate) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
  }
}


/// On Intersect Macro
///
/// Scroll-driven intersection animation (progress tied to intersection ratio).

%macro on-intersect {

  %form {
    @on-intersect $name:ident(
      start: $start:number = 0,
      end: $end:number = 1,
      easing: $easing:easing = linear,
      stagger: $stagger:stagger_value = 0,
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    scroll-driver(&self, name: $name, start: $start, end: $end, scrub: true) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
  }
}


// =============================================================================
// Mutation Event Handlers
// =============================================================================

/// On Mutation Macro
///
/// Handles event-triggered state mutations (no animation).
/// Used for: @on click { $state <- value; }
///
/// This is distinct from animation @on which creates timelines.
/// Mutation handlers run code when events occur.
///
/// @example click handler
/// .pack-card {
///   @on click {
///     $currentPackId <- $.dataset.packId;
///   }
/// }
///
/// @example with event delegation
/// .variant-grid {
///   @on click [data-option-id] {
///     $selectedVariant <- $.dataset.optionId;
///   }
/// }
///
/// @example form submit
/// form {
///   @on submit.prevent {
///     $formData <- new FormData($);
///   }
/// }

/// On Mutation Dispatcher
///
/// Routes @on to mutation handling when no animation name is provided.
/// This handles the mutation-only form: @on event { mutations }
///
/// @example click handler
/// .pack-card {
///   @on click {
///     $currentPackId <- $.dataset.packId;
///     @emit open_configurator;
///   }
/// }

// BUG-329 CUTOVER (2026-08-07): the bare-event heads are RETIRED.
//
// `@on click { … }` and `@on click .child { … }` used to be declared here as
// `on-mutation-dispatcher{,-simple}`. Both named two capture types —
// `event_name` and `mutation_actions` — that HAVE NEVER EXISTED in any commit
// (`git log --all -S'%capture_type event_name'` is empty). A `%form` naming an
// undeclared capture type does not fail at load; it fails at USE, as a grammar
// mismatch pointing at the author's page. That is the same class as BUG-299's
// undeclared `@use` clauses, one level deeper: not a clause the grammar forgot
// to declare, but a TYPE the grammar names and nothing defines.
//
// The sigil form `@on &.click { … }` (stdlib/macros/on.st, `driver_expr` +
// `on_motion_body`, which already accepts `$mut:mutation`) is now the ONE
// spelling. Behaviour equivalence was proven before the corpus was migrated —
// see tests/lang/on-event/selector-scope-click.test.st, which fires both
// spellings in the same position and asserts the cell actually changed.
//
// The event-specific macros below (`@on-hover`, `@on-click`, `@on-focus`,
// `@on-visible`) are untouched: they are their own directives, not a routing
// target, and nothing bare depends on them.
//
// 175 call sites across 52 files were migrated in the same commit. Spacetime
// is pre-public, so this is a hard break with no compatibility window.

// Legacy support - keep old mutation handler for backwards compatibility
%macro on-mutation-handler {

  %form {
    @on-mutation(
      event: $event:ident,
      target: $target:selector? = "",
      mutations: $mutations:array
    )
  }

  %emit js {
    (function() {
      const el = %&self;
      const target = %$target;
      const mutations = %$mutations;

      const handler = function(e) {
        const $ = target ? e.target.closest(target) : el;
        if (!$) return;

        // Execute each mutation
        for (const mut of mutations) {
          const signalName = mut.target;
          const expr = mut.expr;
          // Evaluate expression and update signal
          try {
            const value = eval(expr);
            if (window.ST && ST.signals && ST.signals[signalName]) {
              ST.signals[signalName].set(value);
            }
          } catch (err) {
            console.error('Mutation error:', err);
          }
        }
      };

      el.addEventListener('%$event', handler);

      // Cleanup on element removal
      if (window.ST && ST.cleanup) {
        ST.cleanup(el, () => el.removeEventListener('%$event', handler));
      }
    })();
  }
}


/// ERROR SIBLING (BUG-264): `target:` names the delegated event element and must
/// be a SELECTOR (`.btn`, `button`, `[data-id]`) or be omitted. A near-miss — a
/// bare number (`target: 5`) or a non-element identifier (`target: submitBtn`) —
/// is refused by the strict `$target:selector?` capture and becomes a clause the
/// productive `on-mutation-handler` form DROPS SILENTLY (a lenient match). This
/// sibling matches the refused value via `$targetInvalid:expr` and reports it.
/// Declared AFTER `on-mutation-handler`: on a VALID selector the productive form
/// matches cleanly and wins; only a refused near-miss leaves the productive
/// match lenient, letting this sibling fire.
%macro on-mutation-target-invalid {
  %diagnostic



  %form {
    @on-mutation(
      event: $event:ident,
      target: $targetInvalid:expr,
      mutations: $mutations:array
    )
  }

  %binds {
    optional-clause-error(
      message: "`target:` must be a selector, not `$targetInvalid`",
      hint: "write `target: .selector` (e.g. `target: .btn`) or `target: button` — target names the element the event is delegated to",
      value: $targetInvalid
    )
  }
}
