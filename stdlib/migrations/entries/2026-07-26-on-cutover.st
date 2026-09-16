/// SIP-001 cutover (2026-07-26): the eight animation directives —
/// `@scroll`, `@load`, `@hover`, `@click`, `@mouse`, `@pointer`, `@time`,
/// `@loop` — retire in favor of ONE head: `@on <driver>` (PLAN-124 W3.4).
///
/// `@value-change` ALSO retires here, to `@on &.text-change(…)` — the
/// `%rewrite value-change` rule below is its mechanical target (the header
/// previously claimed it stayed current, which its own rewrite contradicted).
/// The WAAPI variants (@wscroll/@wload/@wloop) are a separate, supported
/// compositor path and do stay current.
///
/// Mapping (all fields arrive identically — anything dropped below was
/// ALREADY dead in the old binds, verified against emitted output):
///   name/duration/delay/scope/start/end/scrub/threshold/iterations/axis/
///   mode/phase/invert  → driver params on the new head
///   easing/stagger/staggerFrom → settings lines inside the body
///   trigger (on @scroll) → dropped: it never reached any %binds (dead param)
///   stray duration/threshold on @scroll → dropped: they matched via the
///     arg-list tolerance and were silently ignored before this wave
///
/// While a project's `@version` predates this wave: mechanical shapes are
/// auto-rewritten in memory (W0715) and persisted by `spacetime migrate`;
/// the retired definitions' `%binds` keep old calls working meanwhile. Once
/// `@version` reaches 2026-07-26, any lingering use is E0911.
%migration on-cutover {
  %date 2026-07-26
  %docs "SIP-001: the eight animation directives became one head — `@on <driver>`."

  // ── Retired definitions, verbatim (stdlib/macros/timeline.st, loop.st) ──

  %macro scroll-timeline {
    // trigger was ALREADY dead in the old %binds (never forwarded to
    // scroll-driver) — a documented drop (SIP-001).
    %drops ( trigger )
  
    %form {
      @scroll $name:ident (
        scope: $scope:ident = cover,
        start: $start:number = 0,
        end: $end:number = 1,
        scrub: $scrub:bool = true,
        trigger: $trigger:selector? = "",
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:keyframes
      }
    }
  
    %binds {
      scroll-driver(&self, name: $name, scope: $scope, start: $start, end: $end, scrub: $scrub) -> { $progress }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
    }
  }
  
    %macro load-timeline {
  
    %form {
      @load $name:ident (
        duration: $duration:time = 1000ms,
        delay: $delay:time = 0ms,
        threshold: $threshold:number = 0.1,
        easing: $easing:ident = ease-out-quad,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:keyframes
      }
    }
  
    %binds {
      load-driver(&self, name: $name, duration: $duration, delay: $delay, threshold: $threshold) -> { $progress }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
    }
  }
  
    %macro hover-timeline {
  
    %form {
      @hover $name:ident (
        duration: $duration:time = 300ms,
        easing: $easing:ident = ease-out-quad,
        stagger: $stagger:number = 0,
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
  
    %macro click-timeline {
  
    %form {
      @click $name:ident (
        duration: $duration:time = 300ms,
        easing: $easing:ident = ease-out-quad,
        stagger: $stagger:number = 0,
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
  
    %macro mouse-timeline {
  
    %form {
      @mouse $name:ident (
        axis: $axis:string = "both",
        invert: $invert:bool = false,
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:keyframes
      }
    }
  
    %binds {
      mouse-driver(&self, name: $name, axis: $axis, invert: $invert) -> { $progress }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom, interactive: true)
    }
  }
  
    %macro pointer-timeline {
  
    %form {
      @pointer $name:ident (
        axis: $axis:string = "both",
        invert: $invert:bool = false,
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:keyframes
      }
    }
  
    %binds {
      mouse-driver(&self, name: $name, axis: $axis, invert: $invert) -> { $progress }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom, interactive: true)
    }
  }
  
    %macro time-timeline {
  
    %form {
      @time $name:ident (
        duration: $duration:time = 1000ms,
        delay: $delay:time = 0ms,
        iterations: $iterations:number = 1,
        alternate: $alternate:bool = false,
        autoplay: $autoplay:bool = true,
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:keyframes
      }
    }
  
    %binds {
      time-driver(
        &self,
        name: $name,
        duration: $duration,
        delay: $delay,
        iterations: $iterations,
        alternate: $alternate,
        autoplay: $autoplay
      ) -> { $progress, $play, $pause, $reset }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
    }
  }
  
    %macro loop-positional {
    %form {
      @loop $name:ident (
        $duration:time,
        mode: $mode:string = "pingpong",
        phase: $phase:number = 0,
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:keyframes
      }
    }
  
    %binds {
      loop-driver(name: $name, duration: $duration, mode: $mode, phase: $phase, autoplay: true) -> { $progress, $start, $stop }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
    }
  }
  
    %macro loop-named {
    %form {
      @loop $name:ident (
        duration: $duration:time = 1000ms,
        mode: $mode:string = "pingpong",
        phase: $phase:number = 0,
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:keyframes
      }
    }
  
    %binds {
      loop-driver(name: $name, duration: $duration, mode: $mode, phase: $phase, autoplay: true) -> { $progress, $start, $stop }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
    }
  }

  %macro value-change-timeline {

    %form {
      @value-change(
        duration: $duration:time = 300ms,
        stagger: $stagger:time = 30ms,
        from: $staggerFrom:string = "start"
      ) {
        :entering {
          $enterAnim:keyframes?
        }

        :exiting {
          $exitAnim:keyframes?
        }
      }
    }

    // Value-change uses its own driver that handles character animations internally
    %binds {
      text-change-driver(
        &self,
        duration: $duration,
        charStagger: $stagger,
        staggerFrom: $staggerFrom,
        enterKeyframes: $enterAnim,
        exitKeyframes: $exitAnim
      ) -> { $changing, $destroy }
    }
  }

  // ── Mechanical rewrites ──

  %rewrite value-change {
    // Named-only retired form: the old grammar had no positional bindings.
    %drops ( _ )
    %match {
      @value-change(
        duration: $duration:time = 300ms,
        stagger: $stagger:time = 30ms,
        from: $staggerFrom:string = "start"
      ) {
        :entering { $enterAnim:keyframes }
        :exiting { $exitAnim:keyframes }
      }
    }
    %into {
      @on &.text-change(duration: `$duration`, stagger: `$stagger`, from: `$staggerFrom`) {
        :entering {
          `$enterAnim`
        }
        :exiting {
          `$exitAnim`
        }
      }
    }
  }

  %rewrite scroll {
    %drops ( _ duration threshold speed trigger )
    %match {
      @scroll $name:ident (
        scope: $scope:ident = cover,
        start: $start:number = 0,
        end: $end:number = 1,
        scrub: $scrub:bool = true,
        trigger: $trigger:selector? = "",
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.scroll(name: `$name`, scope: `$scope`, start: `$start`, end: `$end`, scrub: `$scrub`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite load {
    // Named-only form: bare positionals never bound (dead in the old matcher).
    %drops ( _ )
    %match {
      @load $name:ident (
        duration: $duration:time = 1000ms,
        delay: $delay:time = 0ms,
        threshold: $threshold:number = 0.1,
        easing: $easing:ident = ease-out-quad,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.load(name: `$name`, duration: `$duration`, delay: `$delay`, threshold: `$threshold`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite hover {
    // Named-only retired form: `@hover x(300ms)` ran at the 300ms default
    // (the positional never bound — same measured-dead class as @time).
    %drops ( _ )
    %match {
      @hover $name:ident (
        duration: $duration:time = 300ms,
        easing: $easing:ident = ease-out-quad,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.hover(name: `$name`, duration: `$duration`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite click {
    // Named-only retired form: `@click x(300ms)` ran at the 300ms default
    // (the positional never bound — same measured-dead class as @time).
    %drops ( _ )
    %match {
      @click $name:ident (
        duration: $duration:time = 300ms,
        easing: $easing:ident = ease-out-quad,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.click(name: `$name`, duration: `$duration`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite mouse {
    // relative-to and bare positionals were accepted and never bound.
    %drops ( _ relative-to )
    %match {
      @mouse $name:ident (
        axis: $axis:string = "both",
        invert: $invert:bool = false,
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.mouse(name: `$name`, axis: `$axis`, invert: `$invert`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite pointer {
    %drops ( _ relative-to )
    %match {
      @pointer $name:ident (
        axis: $axis:string = "both",
        invert: $invert:bool = false,
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.pointer(name: `$name`, axis: `$axis`, invert: `$invert`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite time {
    // Named-only form: `@time pop(300ms)` ran at 1000ms (measured pre-cutover).
    %drops ( _ )
    %match {
      @time $name:ident (
        duration: $duration:time = 1000ms,
        delay: $delay:time = 0ms,
        iterations: $iterations:number = 1,
        alternate: $alternate:bool = false,
        autoplay: $autoplay:bool = true,
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.time(name: `$name`, duration: `$duration`, delay: `$delay`, iterations: `$iterations`, alternate: `$alternate`, autoplay: `$autoplay`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite loop-positional {
    // direction/delay were accepted and never bound (dead in the old %binds).
    %drops ( _ direction delay )
    %match {
      @loop $name:ident (
        $duration:time,
        mode: $mode:string = "pingpong",
        phase: $phase:number = 0,
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.loop(name: `$name`, duration: `$duration`, mode: `$mode`, phase: `$phase`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite loop-named {
    %drops ( _ direction delay )
    %match {
      @loop $name:ident (
        duration: $duration:time = 1000ms,
        mode: $mode:string = "pingpong",
        phase: $phase:number = 0,
        easing: $easing:ident = linear,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.loop(name: `$name`, duration: `$duration`, mode: `$mode`, phase: `$phase`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }


  // ── Retired: the @on <event> keyframes dispatcher (the `%if $event ==`
  // ladder — PLAN-124 acceptance: zero ladders remain in stdlib) ──

  %macro on-event-dispatcher {
  %order 700

  %form {
    @on $event:ident $name:ident(
      $duration:time,
      easing: $easing:ident = ease-out,
      threshold: $threshold:number = 0.1,
      delay: $delay:time = 0ms,
      immediate: $immediate:bool = false,
      stagger: $stagger:number = 0,
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:on_motion_body
    }
  }

  %if $event == "hover" {
    %binds {
      event-driver(&self, name: $name, trigger: "mouseenter", untrigger: "mouseleave", duration: $duration, reverse: true) -> { $progress }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom, interactive: true)
    }
  } %elif $event == "click" {
    %binds {
      event-driver(&self, name: $name, trigger: "click", toggle: true, duration: $duration, reverse: true) -> { $progress }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom, interactive: true)
    }
  } %elif $event == "focus" {
    %binds {
      event-driver(&self, name: $name, trigger: "focus", duration: $duration, reverse: true) -> { $progress }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom, interactive: true)
    }
  } %elif $event == "visible" {
    %binds {
      load-driver(&self, name: $name, duration: $duration, delay: $delay, threshold: $threshold, immediate: $immediate) -> { $progress }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
    }
  } %elif $event == "intersect" {
    %binds {
      scroll-driver(&self, name: $name, start: 0, end: 1, scrub: true) -> { $progress }
      apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
    }
  }
}

  // ── Dispatcher rewrites. Dead params are dropped exactly where the ladder
  // already dropped them (hover/click/focus used duration ONLY; visible used
  // load-driver, so @on visible maps to &.load — behavior-exact, D18). ──

  %rewrite on-hover {
    // delay/immediate/threshold were dead in the old %binds (hover used
    // duration ONLY) — documented drops (SIP-001).
    %drops ( delay immediate threshold )
    %match {
      @on &.hover $name:ident (
        $duration:time,
        easing: $easing:ident = ease-out,
        threshold: $threshold:number = 0.1,
        delay: $delay:time = 0ms,
        immediate: $immediate:bool = false,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.hover(name: `$name`, duration: `$duration`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite on-click {
    %drops ( delay immediate threshold )
    %match {
      @on &.click $name:ident (
        $duration:time,
        easing: $easing:ident = ease-out,
        threshold: $threshold:number = 0.1,
        delay: $delay:time = 0ms,
        immediate: $immediate:bool = false,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.click(name: `$name`, duration: `$duration`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite on-focus {
    %drops ( delay immediate threshold )
    %match {
      @on &.focus $name:ident (
        $duration:time,
        easing: $easing:ident = ease-out,
        threshold: $threshold:number = 0.1,
        delay: $delay:time = 0ms,
        immediate: $immediate:bool = false,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.focus(name: `$name`, duration: `$duration`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite on-visible {
    %match {
      @on &.visible $name:ident (
        $duration:time,
        easing: $easing:ident = ease-out,
        threshold: $threshold:number = 0.1,
        delay: $delay:time = 0ms,
        immediate: $immediate:bool = false,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.load(name: `$name`, duration: `$duration`, delay: `$delay`, threshold: `$threshold`, immediate: `$immediate`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %rewrite on-intersect {
    %drops ( delay duration immediate threshold )
    %match {
      @on intersect $name:ident (
        $duration:time,
        easing: $easing:ident = ease-out,
        threshold: $threshold:number = 0.1,
        delay: $delay:time = 0ms,
        immediate: $immediate:bool = false,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.scroll(name: `$name`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  // The dispatcher's %if ladder never had a `load` arm — `@on load x(…)`
  // sites were silently DEAD even before retirement. The intended semantics
  // is plainly the load driver; map there (same shape as on-visible).
  %rewrite on-load {
    %match {
      @on &.load $name:ident (
        $duration:time,
        easing: $easing:easing = ease-out,
        threshold: $threshold:number = 0.1,
        delay: $delay:time = 0ms,
        immediate: $immediate:bool = false,
        stagger: $stagger:number = 0,
        staggerFrom: $staggerFrom:string = "first"
      ) {
        $body:on_motion_body
      }
    }
    %into {
      @on &.load(name: `$name`, duration: `$duration`, delay: `$delay`, threshold: `$threshold`, immediate: `$immediate`) {
        easing: `$easing`;
        stagger: `$stagger`;
        staggerFrom: `$staggerFrom`;
        `$body`
      }
    }
  }

  %hint for @on "@on <event> is now `@on &.<driver>` — the driver IS the event (`@on hover lift(300ms) { … }` → `@on &.hover(duration: 300ms) { … }`); `@on &.visible` maps to `&.load` (its behavior was always the load driver — for viewport-entry reveals use `&.visible`). Mutation bodies (`@on click t { $x <- … }`) keep the current mutation dispatcher — no rewrite needed."

  // ── Manual guidance for the shapes no rule covers ──
  %hint for @scroll "@scroll is now `@on &.scroll(...) { … }` — same params as driver params, easing/stagger/staggerFrom as body lines. Calls with a `&element` first positional migrate by hand: pass the element as the subject — `@on &<element>.scroll(...) { … }`. See docs/specs/SIP-001-LAUNCH-ARTICLE.st.md."
  %hint for @load "@load is now `@on &.load(...) { … }` — duration/delay/threshold are driver params, easing/stagger/staggerFrom body lines."
  %hint for @value-change "@value-change is now `@on &.text-change(...) { :entering { … } :exiting { … } }`. Sources with only ONE slot (the old form's slots were optional) migrate by hand: emit only the slot you have — the runtime's default keyframes cover the other (measured: same-wave rewrite rules must have disjoint %match shapes, and %match has no optional-keyframe modifier, so the capsule cannot cover single-slot sources)."
  %hint for @hover "@hover is now `@on &.hover(...) { … }`."
  %hint for @click "@click is now `@on &.click(...) { … }`."
  %hint for @mouse "@mouse is now `@on &.mouse(...) { … }`."
  %hint for @pointer "@pointer is now `@on &.pointer(...) { … }`."
  %hint for @time "@time is now `@on &.time(...) { … }`."
  %hint for @loop "@loop is now `@on &.loop(...) { … }` — duration/mode/phase are driver params."
}
