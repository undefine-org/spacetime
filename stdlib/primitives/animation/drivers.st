/// Animation Driver Primitives
///
/// Drivers control how animation progress is updated.
/// Each driver type responds to different triggers:
/// - scroll: Scroll position drives progress
/// - time: Duration-based playback
/// - loop: Repeating time-based animation
/// - mouse: Mouse position drives progress
/// - event: Event triggers animation
/// - load: Plays on page/element load


/// Scroll Driver Primitive
///
/// Drives animation progress based on scroll position.
/// Progress 0->1 as element scrolls through viewport.
/// Supports timeline registration for @after chaining.
///
/// @param &el - Element to track (trigger element)
/// @param name - Optional timeline name for chaining (used by @after)
/// @param scope - Reference frame: "cover" (default), "entry", "exit", "contain"
/// @param start - Start percentage within scope (0-1)
/// @param end - End percentage within scope (0-1)
/// @param scrub - Continuous update vs single trigger
///
/// @yields $progress - Current scroll progress (0-1)
/// @yields $destroy - Cleanup function

%primitive scroll-driver(
  &el,
  name: string = "",
  scope: string = "cover",
  start: number = 0,
  end: number = 1,
  scrub: bool = true
) %uses scroll-registry {

  %emit js {
    (() => {
    let progress = 0;
    let rafId = null;
    const controller = new AbortController();
    const timelineName = %name;


    const update = () => {
      const rect = %&el.getBoundingClientRect();
      const vh = window.innerHeight;
      const h = rect.height;
      const scope = '%scope';

      // Compute range boundaries (rect.top values at progress 0 and 1)
      let rangeStart, rangeEnd;
      if (scope === 'entry') {
        rangeStart = vh;
        rangeEnd = Math.max(0, vh - h);
      } else if (scope === 'exit') {
        rangeStart = Math.min(0, vh - h);
        rangeEnd = -h;
      } else if (scope === 'contain') {
        if (h >= vh) { rangeStart = vh; rangeEnd = -h; }
        else { rangeStart = vh - h; rangeEnd = 0; }
      } else {
        // cover (default)
        rangeStart = vh;
        rangeEnd = -h;
      }

      // Map start/end percentages to positions within scope
      const fullRange = rangeStart - rangeEnd;
      const subStart = rangeStart - (%start * fullRange);
      const subEnd = rangeStart - (%end * fullRange);

      let newProgress;
      if (rect.top > subStart) newProgress = 0;
      else if (rect.top < subEnd) newProgress = 1;
      else newProgress = (subStart - rect.top) / (subStart - subEnd);

      const oldProgress = progress;
      progress = Math.max(0, Math.min(1, newProgress));
      %yield progress -> $%name;
      if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, progress);

    };

    // Set initial progress to 0 so watchers receive initial state
    %yield 0 -> $%name;
    if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, 0);

    if (%scrub) {
      const scrollEntry = () => {
        if (controller.signal.aborted) return false;
        update();
        return true;
      };
      window.__stScrollHandlers.push(scrollEntry);
      // Defer initial update to next animation frame so watchers register first
      // and getBoundingClientRect doesn't force layout reflow in the microtask queue.
      // With 18+ scroll timelines, queueMicrotask causes 18+ forced reflows before
      // the browser can paint — causing the page to hang.
      (typeof requestAnimationFrame !== 'undefined' ? requestAnimationFrame : setTimeout)(() => {
        if (!controller.signal.aborted) update();
      });
    } else {
      const observer = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (entry.isIntersecting) {
              progress = 1;
              %yield 1 -> $progress;
              if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, 1);
              observer.disconnect();
            }
          }
        },
        { threshold: %start }
      );
      // Defer observer setup to allow watchers to be registered first
      queueMicrotask(() => observer.observe(%&el));
    }

    const destroy = () => {
      controller.abort();
      if (rafId) cancelAnimationFrame(rafId);
    };

    %yield destroy -> $destroy;

    // Register cleanup inside IIFE where destroy is in scope
    ST.onCleanup(%&el, destroy);
    })();
  }

  %exports {
    $progress: number
    $destroy: fn() ~> effect
  }
}


/// Time Driver Primitive
///
/// Plays animation over a duration with optional delay.
/// Supports iteration count and alternate direction.
/// Supports timeline registration for @after chaining.
///
/// @param &el - Element to animate
/// @param name - Optional timeline name for chaining (used by @after)
/// @param duration - Animation duration in ms
/// @param delay - Delay before starting in ms
/// @param iterations - Number of times to play (0 = infinite)
/// @param alternate - Reverse direction on alternate iterations
/// @param autoplay - Start automatically
///
/// @yields $progress - Current progress (0-1)
/// @yields $play - Start playback
/// @yields $pause - Pause playback
/// @yields $reset - Reset to start
/// @yields $destroy - Cleanup function

%primitive time-driver(
  &el,
  name: string = "time",
  duration: number = 1000,
  delay: number = 0,
  iterations: number = 1,
  alternate: bool = false,
  autoplay: bool = true
) {
  %emit js {
    (() => {
    let progress = 0;
    let playing = false;
    let startTime = null;
    let currentIteration = 0;
    let rafUnsub = null;
    let timeoutId = null;

    const dur = %duration;
    const delayMs = typeof %delay === 'string' ? parseFloat(%delay) || 0 : (%delay || 0);
    const maxIterations = %iterations;
    const alt = %alternate;
    const timelineName = %name;


    const animate = (timestamp) => {
      if (!playing) return;

      if (!startTime) startTime = timestamp + delayMs;
      if (timestamp < startTime) {
        return; // Still in delay, coordinator will call us next frame
      }

      const elapsed = timestamp - startTime;
      let rawProgress = Math.min(1, elapsed / dur);

      // Handle alternate direction
      if (alt && currentIteration % 2 === 1) {
        rawProgress = 1 - rawProgress;
      }

      progress = rawProgress;
      %yield progress -> $%name;
      if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, progress);

      if (elapsed >= dur) {
        currentIteration++;

        if (maxIterations === 0 || currentIteration < maxIterations) {
          startTime = null; // Reset for next iteration
        } else {
          playing = false;
          if (rafUnsub) { rafUnsub(); rafUnsub = null; }
          // Notify completion when iterations finish
        }
      }
    };

    const play = () => {
      if (playing) return;
      playing = true;
      startTime = null;
      // Use RAF coordinator for synchronized updates
      rafUnsub = ST._raf.add(animate);
    };

    const pause = () => {
      playing = false;
      if (rafUnsub) {
        rafUnsub();
        rafUnsub = null;
      }
    };

    const reset = () => {
      pause();
      progress = 0;
      currentIteration = 0;
      startTime = null;
      %yield 0 -> $progress;
      if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, 0);
    };

    const destroy = () => {
      pause();
      if (timeoutId) clearTimeout(timeoutId);
    };

    // Set initial progress to 0 so watchers receive initial state
    %yield 0 -> $%name;
    if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, 0);

    %yield play -> $play;
    %yield pause -> $pause;
    %yield reset -> $reset;
    %yield destroy -> $destroy;

    if (%autoplay) {
      // Defer start to allow watchers to be registered first
      queueMicrotask(() => play());
    }

    // Register cleanup inside IIFE where destroy is in scope
    ST.onCleanup(%&el, destroy);
    })();
  }

  %exports {
    $progress: number
    $play: fn() ~> effect
    $pause: fn() ~> effect
    $reset: fn() ~> effect
    $destroy: fn() ~> effect
  }
}


/// Loop Driver Primitive
///
/// Continuously loops animation (ping-pong or restart).
///
/// @param duration - One cycle duration in ms
/// @param mode - "pingpong" or "restart"
/// @param phase - Initial phase offset (0-1), e.g., 0.25 starts at 25%
/// @param autoplay - Start automatically
///
/// @yields $progress - Current progress (0-1)
/// @yields $start - Start loop
/// @yields $stop - Stop loop
/// @yields $destroy - Cleanup function

%primitive loop-driver(
  name: string = "loop",
  duration: number = 1000,
  mode: string = "pingpong",
  phase: number = 0,
  autoplay: bool = true
) {
  %emit js {
    (() => {
    let progress = 0;
    let running = false;
    let rafUnsub = null;
    let direction = 1;
    let startTime = null;

    const dur = %duration;
    const loopMode = '%mode';
    const phaseOffset = %phase;
    const timelineName = %name;


    const animate = (timestamp) => {
      if (!running) return;

      if (!startTime) startTime = timestamp;

      const elapsed = timestamp - startTime;
      // Apply phase offset to shift the starting position
      let rawProgress = ((elapsed % dur) / dur + phaseOffset) % 1;

      if (loopMode === 'pingpong') {
        const cycle = Math.floor((elapsed / dur + phaseOffset * 2) % 2);
        if (cycle === 1) {
          rawProgress = 1 - ((elapsed % dur) / dur + phaseOffset) % 1;
        }
      }

      progress = rawProgress;
      %yield progress -> $%name;
      if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, progress);
      // Coordinator continues calling, no need to re-request
    };

    const start = () => {
      if (running) return;
      running = true;
      startTime = null;
      // Use RAF coordinator for synchronized updates
      rafUnsub = ST._raf.add(animate);
    };

    const stop = () => {
      running = false;
      if (rafUnsub) {
        rafUnsub();
        rafUnsub = null;
      }
    };

    const destroy = () => {
      stop();
    };

    // Set initial progress to 0 so watchers receive initial state
    %yield 0 -> $%name;
    if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, 0);

    %yield start -> $start;
    %yield stop -> $stop;
    %yield destroy -> $destroy;

    if (%autoplay) {
      // Defer start to allow watchers to be registered first
      queueMicrotask(() => start());
    }

    // Register global cleanup (no element context for loop-driver)
    ST.addCleanupHandler(destroy);
    })();
  }

  %exports {
    $progress: number
    $start: fn() ~> effect
    $stop: fn() ~> effect
    $destroy: fn() ~> effect
  }
}


/// Mouse Driver Primitive
///
/// Drives animation based on mouse position relative to element.
///
/// @param &el - Element to track mouse within
/// @param axis - "x", "y", or "both"
/// @param invert - Invert progress direction
///
/// @yields $progress - Current progress (0-1) or { x, y } for both
/// @yields $destroy - Cleanup function

%primitive mouse-driver(
  &el,
  name: string = "mouse",
  axis: string = "both",
  invert: bool = false
) {
  %emit js {
    if (!%&el) {
      %yield (() => {}) -> $destroy;
      return;
    }

    let progress = '%axis' === 'both' ? { x: 0.5, y: 0.5 } : 0.5;
    const controller = new AbortController();

    const handleMove = (e) => {
      const rect = %&el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) return;
      let x = (e.clientX - rect.left) / rect.width;
      let y = (e.clientY - rect.top) / rect.height;

      x = Math.max(0, Math.min(1, x));
      y = Math.max(0, Math.min(1, y));

      if (%invert) {
        x = 1 - x;
        y = 1 - y;
      }

      if (%axis === 'x') {
        progress = x;
      } else if (%axis === 'y') {
        progress = y;
      } else {
        progress = { x, y };
      }

      %yield progress -> $%name;
    };

    %&el.addEventListener('mousemove', handleMove, { signal: controller.signal });

    // Set initial progress to center so watchers receive neutral state
    %yield progress -> $%name;

    const destroy = () => {
      controller.abort();
    };

    %yield destroy -> $destroy;

    // Register cleanup with element
    ST.onCleanup(%&el, destroy);
  }

  %exports {
    $progress: any
    $destroy: fn() ~> effect
  }
}


/// Event Driver Primitive
///
/// Triggers animation playback on element events.
/// Supports forward/reverse on enter/leave.
///
/// @param &el - Element to listen on
/// @param trigger - Event name ("hover", "click", or custom)
/// @param duration - Animation duration in ms
/// @param reverse - Reverse on leave/toggle
///
/// @yields $progress - Current progress (0-1)
/// @yields $destroy - Cleanup function

%primitive event-driver(
  &el,
  name: string = "event",
  trigger: string = "hover",
  duration: number = 300,
  reverse: bool = true,
  untrigger: string = "",
  toggle: bool = false,
  key: string = ""
) {
  %emit js {
    (() => {
    let progress = 0;
    let direction = 0;
    let rafUnsub = null;
    let startTime = null;
    let startProgress = 0;
    const controller = new AbortController();

    const dur = %duration;
    const timelineName = %name;

    const animate = (timestamp) => {
      if (direction === 0) return;

      if (!startTime) {
        startTime = timestamp;
        startProgress = progress;
      }

      const elapsed = timestamp - startTime;
      const delta = elapsed / dur;

      if (direction === 1) {
        progress = Math.min(1, startProgress + delta);
      } else {
        progress = Math.max(0, startProgress - delta);
      }

      %yield progress -> $%name;
      if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, progress);

      // Check if animation is complete
      if ((direction === 1 && progress >= 1) || (direction === -1 && progress <= 0)) {
        if (rafUnsub) { rafUnsub(); rafUnsub = null; }
        direction = 0;
      }
      // Otherwise coordinator continues calling
    };

    const forward = () => {
      if (rafUnsub) { rafUnsub(); rafUnsub = null; }
      direction = 1;
      startTime = null;
      startProgress = progress;
      // Use RAF coordinator for synchronized updates
      rafUnsub = ST._raf.add(animate);
    };

    const backward = () => {
      if (rafUnsub) { rafUnsub(); rafUnsub = null; }
      direction = -1;
      startTime = null;
      startProgress = progress;
      // Use RAF coordinator for synchronized updates
      rafUnsub = ST._raf.add(animate);
    };

    // Set initial progress to 0 so watchers receive initial state
    %yield 0 -> $%name;
    if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, 0);

    const trig = %trigger;

    // The trigger is a real DOM event name, supplied by the driver ROW
    // (`primitive_args: "trigger:click"`). `untrigger` and `toggle` carry the
    // two REVERSAL SHAPES, also as row data — they are behavior, not dispatch:
    //
    //   untrigger  a second event that runs the transport BACKWARD
    //              (hover: mouseleave). A pair of opposed events.
    //   toggle     the SAME event reverses on re-fire, by current position
    //              (click). One event, two directions.
    //
    // Both used to be `if (trig === 'hover') … else if (trig === 'click')`
    // branches here, which is why the trigger name had to be known to this
    // file — and why five rows sharing this primitive were indistinguishable
    // (BUG-253). Now a new event driver is a registry row and this code never
    // changes.
    const untrig = %untrigger;

    // A KEYBOARD driver may name the one key it answers to — row data, like
    // `untrigger` and `toggle` above, for exactly the reason stated there: a
    // new event driver is a registry row and this code never changes.
    //
    // BUG-375: without it every keyboard binding fired on EVERY key. The
    // spelling authors reached for (`@on &.keydown:Enter`) has no slot in
    // `driver_expr`'s grammar, so the qualifier was parsed and discarded in
    // silence — a chat composer bound that way sent a message per keystroke,
    // and the compiler said yes. `@on &.keydown("Enter")` is the spelling.
    //
    // Wrapping rather than guarding at each listener: `forward`/`backward` are
    // attached in three places below (toggle, plain, untrigger) and a filter
    // pasted three times is a filter that will be added to two of them next
    // time.
    const __stKey = %key;
    const keyed = (fn) => __stKey
      ? (e) => { if (e && e.key === __stKey) fn(e); }
      : fn;

    if (%toggle) {
      %&el.addEventListener(trig, keyed(() => {
        if (progress < 0.5) forward();
        else if (%reverse) backward();
      }), { signal: controller.signal });
    } else {
      %&el.addEventListener(trig, keyed(forward), { signal: controller.signal });
      if (untrig && %reverse) {
        %&el.addEventListener(untrig, keyed(backward), { signal: controller.signal });
      }
    }

    const destroy = () => {
      controller.abort();
      if (rafUnsub) { rafUnsub(); rafUnsub = null; }
    };

    %yield destroy -> $destroy;

    // Register cleanup inside IIFE where destroy is in scope
    ST.onCleanup(%&el, destroy);
    })();
  }

  %exports {
    $progress: number
    $destroy: fn() ~> effect
  }
}


/// Steps Driver Primitive — the DISCRETE clock (SIP-001 `.steps`).
///
/// A slide deck: the score advances through N steps, one per advance event, and
/// progress is the current step index over the deck — the spec's "transport
/// (ordinal)", a seekable playhead whose positions are steps rather than
/// instants:
///
///   progress = step_index / (count - 1)      (count = number of steps)
///
/// `count` is the number of steps in the score, passed by the score lowering
/// from the body's own lines — the driver never learns which ELEMENTS the deck
/// holds, only how many slots there are (which is what keeps it projection-
/// agnostic: the same primitive would drive a deck of any shape). The advance
/// event is the spec's `advance:` event-expr, resolved by the score lowering
/// through the SAME registry the `@on` rail reads (member → `trigger:`), so a
/// step fires on the same DOM event a `@on &.<event>` would listen for — no
/// second event mechanism.
///
/// @param &el       - the element the deck is scoped to (advance events bind here)
/// @param name      - the published progress signal name
/// @param count     - number of steps in the deck
/// @param advance   - DOM event name(s) that advance one step (default "click")
///
/// @yields $<name>  - the deck's progress (0..1)
%primitive steps-driver(
  &el,
  name: string = "steps",
  count: number = 1,
  advance: string = "click"
) {
  %emit js {
    (() => {
      // NB do NOT bind `%&el` to a variable literally named `el`: `%&el`
      // renders as `el`, which is ALSO the init wrapper's parameter, so
      // `var el = el;` shadows it to undefined (the RHS binds the hoisted
      // inner). Bind to a distinct local and use it throughout.
      var node = %&el;
      var controller = new AbortController();
      var stepIndex = 0;
      var stepCount = %count < 1 ? 1 : %count;
      var stepName = %name;

      var publish = function() {
        // A one-step deck is always at 0; otherwise the index maps linearly
        // onto 0..1. Denominated by (count - 1) so the LAST step is exactly 1.
        var progress = stepCount <= 1 ? 0 : (stepIndex / (stepCount - 1));
        progress = progress < 0 ? 0 : (progress > 1 ? 1 : progress);
        %yield progress -> $%name;
        if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(stepName, progress);
      };

      // Publish once so a watcher registered later never sees undefined.
      %yield 0 -> $%name;
      if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(stepName, 0);

      // `advance` is a comma-separated DOM event list, resolved by the score
      // lowering through the @on rail's own registry (member → `trigger:`), so
      // a `.steps` advance is the same event a `@on &.<event>` would bind.
      var advanceEvents = (%advance || "click").split(',').map(function(s){ return s.trim(); }).filter(function(s){ return s.length; });
      advanceEvents.forEach(function(ev) {
        node.addEventListener(ev, function() {
          if (stepIndex < stepCount - 1) { stepIndex++; publish(); }
        }, { signal: controller.signal });
      });

      var destroy = function() {
        controller.abort();
      };
      %yield destroy -> $destroy;
      ST.onCleanup(node, destroy);
    })();
  }

  %exports {
    $progress: number
    $destroy: fn() ~> effect
  }
}


/// Media Driver Primitive — the MEDIA clock (SIP-001 `.playback`).
///
/// Binds to an <audio>/<video> element and publishes its playhead as a 0..1
/// progress signal, exactly as time-driver does for a declared duration:
///
///   progress = currentTime / duration
///
/// The driver OWNS no duration of its own — the media element's is the domain
/// (`domain: inherited`, R1). It publishes that duration alongside the progress
/// so the score's window-map can scale placements (`at 2s`) against it at
/// runtime: duration lives under `<name>__dur`, and `score-window` resolves its
/// `total` from there when handed a signal name instead of a number.
///
/// `&el` is the SCOPE node (where the signals are published — the score host).
/// The media element itself may be that same node (a `&.playback` score ON the
/// media element) or a named subject (`&voice.playback`), passed as `media` —
/// resolved via ST.ref, the same rail `@on &voice.playback` would use.
///
/// @param &el      - the scope node the signals resolve through
/// @param media    - optional named media element ref (`&voice`); when empty, &el IS the media element
/// @param name     - the published progress signal name
///
/// @yields $<name>  - the media playhead's progress (0..1)
%primitive media-driver(
  &el,
  media: string = "",
  name: string = "media",
  rate: number = 1
) {
  %emit js {
    (() => {
      // NB do NOT bind `%&el` to a variable literally named `el`: `%&el`
      // renders as `el`, which is ALSO the init wrapper's parameter, so
      // `var el = el;` shadows it to undefined (the RHS binds the hoisted
      // inner). Bind to a distinct local and use it throughout.
      var node = %&el;
      var controller = new AbortController();
      var mediaName = %name;

      // The named media ref (`&voice`) is registered by the element-ref
      // primitive during page init; if it has not run yet, retry until it
      // does so a `.playback` subject never silently binds nothing.
      var start = function(target) {
        if (!target || typeof target.duration === 'undefined') {
          // Not a media element (or not loaded): publish a defined 0 so nothing
          // that reads the signal produces NaN — loud at the boundary, never a
          // silent broken value. `loadedmetadata` below re-publishes once the
          // duration becomes known.
          %yield 0 -> $%name;
          ST.set(node, %name + "__dur", 0);
          return;
        }

        var publish = function() {
          var dur = target.duration || 0;
          var t = target.currentTime || 0;
          var progress = dur > 0 ? (t / dur) : 0;
          progress = progress < 0 ? 0 : (progress > 1 ? 1 : progress);
          %yield progress -> $%name;
          // The duration under a DETERMINISTIC name the score lowering hands
          // to `score-window` as its `total` — in MILLISECONDS (the score's
          // time domain), so a placement `at 2s` reads as 2000 against a
          // total that is the media's duration × 1000. HTMLMediaElement's
          // duration is in seconds; the score's domain is ms, and the driver
          // is the one place the two units meet.
          ST.set(node, %name + "__dur", dur * 1000);
          if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(mediaName, progress);
        };

        // Publish once (and the duration) so a watcher never sees undefined.
        publish();

        // `timeupdate` fires as the playhead advances; `loadedmetadata` fires
        // when the duration first becomes known — both drive the SAME pure map,
        // so a score under this driver is seekable by construction exactly as
        // under `.time`.
        target.addEventListener('timeupdate', publish, { signal: controller.signal });
        target.addEventListener('loadedmetadata', publish, { signal: controller.signal });
        target.addEventListener('durationchange', publish, { signal: controller.signal });
      };

      var attemptTimer = null;
      var attempt = function() {
        if (!%media) { start(node); return; }
        var t = ST.ref(%media);
        if (t) { start(t); return; }
        // The named ref is not registered yet (the element-ref primitive may
        // not have run): retry on a short poll. Keep the timer id so destroy()
        // can cancel the chain — a missing `&voice` must not poll forever, and
        // a late start() must not resurrect signals after teardown.
        attemptTimer = setTimeout(attempt, 16);
      };
      attempt();

      var destroy = function() {
        controller.abort();
        if (attemptTimer) { clearTimeout(attemptTimer); attemptTimer = null; }
      };
      %yield destroy -> $destroy;
      ST.onCleanup(node, destroy);
    })();
  }

  %exports {
    $progress: number
    $destroy: fn() ~> effect
  }
}


/// Signal Change Driver Primitive
///
/// Watches a scope signal and replays a one-shot 0→1 transport whenever its
/// value genuinely changes. The initial ST.watch callback primes the observer
/// only; it never starts an animation.
/// Prev-Signal Primitive
///
/// Publishes a signal's PREVIOUS value (the `.prev` facet, PLAN-126 D31-ii).
/// Derive is stateless — prev needs memory: this subscribes and, on each
/// change, publishes the value it is leaving behind. At mount the previous
/// value IS the initial value (the arms primed-guard convention), so guards
/// and arms never see an undefined prev.
///
/// @param &el - Scope node the signal resolves through
/// @param signal - Signal name to track
/// @yields $prev - The previous value, republished on every change

%primitive prev-signal(
  &el,
  signal: string
) {
  %emit js {
    (() => {
      const el = %&el;
      const sig = %signal;
      const controller = new AbortController();

      if (!el || typeof ST === 'undefined' || !ST.watch || !ST.resolve) return;

      let prev = ST.resolve(el, sig);
      %yield prev -> $prev;

      const unwatch = ST.watch(el, sig, (next) => {
        %yield prev -> $prev;
        prev = next;
      });

      const destroy = () => {
        controller.abort();
        if (unwatch) unwatch();
      };
      %yield destroy -> $destroy;
      ST.onCleanup(el, destroy);
    })();
  }

  %exports {
    prev: "The previous value of the watched signal"
    destroy: "Cleanup function"
  }
}

%primitive change-driver(
  &el,
  signal: string,
  duration: number = 300,
  immediate: bool = false,
  debounce: number = 0,
  actions: expr = ""
) {
  %emit js {
    (() => {
      // NB: bound as `targetEl`, NOT `el` — for a self-bound change-driver
      // (`@on $sig.change`, no element subject) `%&el` resolves to the outer
      // init's `el`, so `const el = %&el` would become `const el = el`, a TDZ
      // self-reference that throws at init and kills the whole subscription
      // (BUG-271). A distinct name lets the element expr bind from the outer
      // scope instead of shadowing it.
      const targetEl = %&el;
      const sig = %signal;
      const dur = %duration;
      const runImmediate = %immediate;
      const debounceMs = typeof %debounce === 'string' ? (parseFloat(%debounce) || 0) : (%debounce || 0);
      const actions = %actions;
      const hasActions = typeof actions === 'string' && actions.trim() !== '';
      let progress = 0;
      let rafUnsub = null;
      let startTime = null;
      let startProgress = 0;
      let timer = null;
      const controller = new AbortController();

      if (!targetEl || typeof ST === 'undefined' || !ST.watchChanges) return;

      const animate = (timestamp) => {
        if (startTime === null) startTime = timestamp;
        const elapsed = timestamp - startTime;
        progress = Math.min(1, startProgress + elapsed / dur);
        %yield progress -> $progress;
        if (progress >= 1 && rafUnsub) {
          rafUnsub();
          rafUnsub = null;
        }
      };

      const forward = () => {
        const replaying = rafUnsub !== null;
        if (rafUnsub) { rafUnsub(); rafUnsub = null; }
        if (!replaying) progress = 0;
        startTime = null;
        startProgress = progress;
        rafUnsub = ST._raf.add(animate);
      };

      // MUTATION consequence (`@on $sig.change { $dst <- $src + 1; }`, BUG-265):
      // a mutation body is a SIDE-EFFECT on the signal rail — run it via the
      // shared ST.runMutations (the same processor @effect / on-mutation-handler
      // use), coalesced by debounce, seeded once at mount when immediate. A body
      // is either a mutation consequence or a motion body (the compiler enforces
      // this), so `hasActions` cleanly switches the two paths.
      const runActions = () => {
        try { ST.runMutations(targetEl, actions); }
        catch (e) { console.error('[ST] change-driver body error:', e); }
      };
      const fireMutations = () => {
        if (debounceMs > 0) {
          if (timer != null) clearTimeout(timer);
          timer = setTimeout(() => { timer = null; runActions(); }, debounceMs);
        } else {
          runActions();
        }
      };

      const onChanged = () => {
        if (hasActions) { fireMutations(); return; }
        forward();
      };

      // ST.watchChanges, not ST.watch + a primed flag: the signal resolves by
      // nearest owner (element / ancestor / file scope), so the subscription
      // must too, and priming must be state-based — for a file-scope signal
      // there is no opening current-value call, so a call-count guard ate the
      // FIRST change and `.change` never replayed. See ST.watchChanges.
      const unwatch = ST.watchChanges(targetEl, sig, onChanged);

      // immediate: run the mutation body once on mount (after wiring),
      // bypassing debounce — the @effect seeding semantic.
      if (hasActions && runImmediate) runActions();

      const destroy = () => {
        controller.abort();
        if (rafUnsub) { rafUnsub(); rafUnsub = null; }
        if (timer != null) clearTimeout(timer);
        if (unwatch) unwatch();
      };

      %yield destroy -> $destroy;
      ST.onCleanup(targetEl, destroy);
    })();
  }

  %exports {
    $progress: number
    $destroy: fn() ~> effect
  }
}


/// Text Change Driver Primitive (line-aware rewrite)
///
/// Animates text content changes with two DISTINCT, purpose-matched
/// transitions, chosen automatically per change:
///
/// 1. REFLOW (a persisting word's LINE POSITION changed, e.g. a sentence
///    growing from 1 line to 2): the word GLIDES (WAAPI `transform:
///    translate`) from its old (line, x) to its new (line, x) — lean,
///    bounded-duration, no per-character stagger. This is what makes text
///    "know it's going to wrap" instead of visually overlapping/jumping.
/// 2. CONTENT CHANGE (a word's TEXT itself changed, e.g. a price digit): the
///    classic per-character split-flap — old graphemes exit, new graphemes
///    enter, staggered — exactly as before.
///
/// Both are driven by REAL line layout computed BEFORE any DOM mutation, via
/// the shared `stdlib/text` vendored `pretext` engine (`@import "stdlib/text"`
/// — see stdlib/text/MODULE.st): `pretext.layoutWithLines(prepared, maxWidth,
/// lineHeight)` returns the exact per-line `{text, width}` breakdown the
/// browser itself would produce, computed arithmetically (no DOM reflow, no
/// guessing from stale `offsetWidth`). Word (line, x, y) coordinates are
/// derived from that same real layout, so the "will this go to two lines"
/// question is ANSWERED, not inferred after the fact from box-level side
/// effects — the architectural gap the previous box-hack version papered
/// over with `elIsBlockLevel` heuristics, height-jump/width-snap fixes, and
/// baseline-toggle guards (BUG-172, BUG-175, BUG-176 — all symptoms of the
/// same missing real-line-layout foundation).
///
/// Uses the CURRENT, shared vendored pretext bundle (not a private, stale,
/// inlined copy) — one vendoring rail for text layout across the whole
/// stdlib, per the AGENTS elegance bar (no parallel implementations of the
/// same stdlib dependency).
///
/// @param &el - Element to observe for text changes
/// @param duration - Animation duration per character (content changes) / per
///                    reflow move, in ms
/// @param charStagger - Stagger delay between characters (content changes only)
/// @param staggerFrom - Direction: "start", "end", "center", or "random"
/// @param enterKeyframes - Keyframes for entering characters (content change)
/// @param exitKeyframes - Keyframes for exiting characters (content change)
///
/// @yields $changing - True while animation is in progress
/// @yields $destroy - Cleanup function

%primitive text-change-driver(
  &el,
  duration: number = 300,
  charStagger: number = 30,
  staggerFrom: string = "start",
  enterKeyframes: any?,
  exitKeyframes: any?
) {
  %emit js {
    var isChanging = false;
    var observer = null;
    var controller = new AbortController();
    var destroyed = false;

    // Convert Spacetime animation IR to WAAPI keyframes
    // IR format: { properties: [{ property, keyframes: [{ at, value }] }], easing }
    // WAAPI format: [{ transform, opacity, offset, ... }]
    var irToWaapi = function(ir) {
      if (!ir || !ir.properties) return { frames: ir, easing: null };
      var len = ir.properties[0] && ir.properties[0].keyframes ? ir.properties[0].keyframes.length : 2;
      var frames = [];
      for (var i = 0; i < len; i++) {
        var frame = {};
        var transforms = [];
        for (var pi = 0; pi < ir.properties.length; pi++) {
          var p = ir.properties[pi];
          var kf = p.keyframes[i];
          if (!kf) continue;
          var val = kf.value;
          var off = kf.at;
          if (off !== undefined) frame.offset = off;
          if (/^(translate|rotate|scale)-[xyz]$/.test(p.property) || p.property === 'scale') {
            var fn = p.property.replace(/-([a-z])/g, function(_, c) { return c.toUpperCase(); });
            transforms.push(fn + '(' + val + ')');
          } else {
            frame[p.property] = isNaN(val) ? val : Number(val);
          }
        }
        if (transforms.length) frame.transform = transforms.join(' ');
        frames.push(frame);
      }
      return { frames: frames, easing: ir.easing || null };
    };

    var dur = %duration;
    var stagger = %charStagger;
    var staggerDir = '%staggerFrom';
    var rawEnter = %enterKeyframes;
    var rawExit = %exitKeyframes;
    var enterParsed = rawEnter ? irToWaapi(rawEnter) : null;
    var exitParsed = rawExit ? irToWaapi(rawExit) : null;
    var enterKf = (enterParsed ? enterParsed.frames : null) || [
      { opacity: 0, transform: 'translateY(-0.5em)' },
      { opacity: 1, transform: 'translateY(0)' }
    ];
    var exitKf = (exitParsed ? exitParsed.frames : null) || [
      { opacity: 1, transform: 'translateY(0)' },
      { opacity: 0, transform: 'translateY(0.5em)' }
    ];
    var enterEasing = (enterParsed && enterParsed.easing) || 'ease-out';
    var exitEasing = (exitParsed && exitParsed.easing) || 'ease-out';

    // Stagger delay per character (content-change path only — reflow moves
    // are never staggered, see module doc above).
    var getStaggerDelay = function(index, total) {
      switch (staggerDir) {
        case 'end':
          return (total - 1 - index) * stagger;
        case 'center':
          var center = (total - 1) / 2;
          return Math.abs(index - center) * stagger;
        case 'random':
          return Math.random() * total * stagger;
        case 'start':
        default:
          return index * stagger;
      }
    };

    // Grapheme segmenter (shared across cycles)
    var segmenter = new Intl.Segmenter(undefined, { granularity: 'grapheme' });
    var toGraphemes = function(text) {
      var result = [];
      var iter = segmenter.segment(text);
      for (var seg of iter) result.push(seg.segment);
      return result;
    };
    var countGraphemes = function(text) {
      if (!text) return 0;
      var n = 0;
      for (var _g of segmenter.segment(text)) n++;
      return n;
    };

    // Register @keyframes once per instance (not per cycle) — content-change
    // character transforms only; reflow moves are pure WAAPI transform anims
    // (no CSS @keyframes needed for a two-point translate).
    var animId = 'vc_' + Math.random().toString(36).slice(2, 8);
    var styleEl = document.createElement('style');
    var ob = String.fromCharCode(123);
    var cb = String.fromCharCode(125);
    var kfToCss = function(kf) {
      var parts = [];
      for (var i = 0; i < kf.length; i++) {
        var frame = kf[i];
        var pct = kf.length === 2
          ? (i === 0 ? '0%%' : '100%%')
          : (frame.offset !== undefined ? (frame.offset * 100) + '%%' : (i / (kf.length - 1) * 100) + '%%');
        var props = [];
        if (frame.transform !== undefined) props.push('transform: ' + frame.transform);
        parts.push(pct + ' ' + ob + ' ' + props.join('; ') + ' ' + cb);
      }
      return parts.join('\n');
    };
    styleEl.textContent = [
      '@keyframes ' + animId + '-enter ' + ob + '\n' + kfToCss(enterKf) + '\n' + cb,
      '@keyframes ' + animId + '-exit ' + ob + '\n' + kfToCss(exitKf) + '\n' + cb
    ].join('\n');
    document.head.appendChild(styleEl);

    // -------------------------------------------------------------------
    // Real line layout (pretext) — the foundation the old box-hack version
    // never had. `pretext` is the shared, current, always-up-to-date vendored
    // engine from `stdlib/text` (see stdlib/text/vendor.st) — referencing the
    // global here (not a private inlined copy) means this primitive always
    // gets whatever pretext capability that module ships, no drift between
    // two pretext copies.
    // -------------------------------------------------------------------

    // Natural (unwrapped) width of a text run, letter-spacing-correct via
    // pretext's OWN native `letterSpacing` prepare option (not an
    // after-the-fact arithmetic correction bolted onto canvas measurement —
    // pretext's canvas passes account for it internally).
    var measureNatural = function(text, font, letterSpacingPx) {
      if (!text) return 0;
      var opts = letterSpacingPx ? { letterSpacing: letterSpacingPx } : undefined;
      var prepared = pretext.prepareWithSegments(text, font, opts);
      return pretext.measureNaturalWidth(prepared);
    };

    // Real per-line layout at a given container width — this is the actual
    // "will this wrap to two lines" answer, computed BEFORE any DOM mutation.
    var layoutLines = function(text, font, maxWidth, lineHeight, letterSpacingPx) {
      if (!text) return { lineCount: 0, height: 0, lines: [] };
      var opts = letterSpacingPx ? { letterSpacing: letterSpacingPx } : undefined;
      var prepared = pretext.prepareWithSegments(text, font, opts);
      return pretext.layoutWithLines(prepared, maxWidth, lineHeight);
    };

    // Per-word (x, y) coordinates within a laid-out multi-line block. For
    // each line, each word's x-start is the natural width of every word (+
    // joining space) before it on that SAME line — re-measuring a growing
    // prefix via the same pretext pipeline the line layout itself used, so
    // it matches pixel-for-pixel (same font, same letter-spacing handling).
    // pretext calls are ~0.0002ms each (its own documented cost); a prefix
    // walk over a realistic sentence (tens of words) is microseconds, not a
    // performance concern.
    var positionWords = function(lines, font, lineHeight, letterSpacingPx) {
      var positioned = [];
      for (var li = 0; li < lines.length; li++) {
        var words = lines[li].text.split(' ').filter(function(w) { return w.length > 0; });
        var prefix = '';
        var y = li * lineHeight;
        for (var wi = 0; wi < words.length; wi++) {
          if (wi > 0) prefix += ' ';
          var x = measureNatural(prefix, font, letterSpacingPx);
          prefix += words[wi];
          positioned.push({ word: words[wi], x: x, y: y, line: li });
        }
      }
      return positioned;
    };

    // Word-sequence LCS diff (classic DP, O(n*m) — n/m are word counts,
    // realistically tens, never a performance concern). Tags each word as
    // `keep` (persists, may have relocated — REFLOW), `remove` (only in old
    // — split-flap exit), or `add` (only in new — split-flap enter).
    var diffWordSequence = function(oldWords, newWords) {
      var n = oldWords.length, m = newWords.length;
      var dp = new Array(n + 1);
      for (var i = 0; i <= n; i++) dp[i] = new Array(m + 1).fill(0);
      for (var i2 = n - 1; i2 >= 0; i2--) {
        for (var j2 = m - 1; j2 >= 0; j2--) {
          if (oldWords[i2].word === newWords[j2].word) {
            dp[i2][j2] = dp[i2 + 1][j2 + 1] + 1;
          } else {
            dp[i2][j2] = Math.max(dp[i2 + 1][j2], dp[i2][j2 + 1]);
          }
        }
      }
      var ops = [];
      var i = 0, j = 0;
      while (i < n && j < m) {
        if (oldWords[i].word === newWords[j].word) {
          ops.push({ type: 'keep', oldW: oldWords[i], newW: newWords[j] });
          i++; j++;
        } else if (dp[i + 1][j] >= dp[i][j + 1]) {
          ops.push({ type: 'remove', oldW: oldWords[i] });
          i++;
        } else {
          ops.push({ type: 'add', newW: newWords[j] });
          j++;
        }
      }
      while (i < n) { ops.push({ type: 'remove', oldW: oldWords[i] }); i++; }
      while (j < m) { ops.push({ type: 'add', newW: newWords[j] }); j++; }
      return ops;
    };

    // Grapheme-correct prefix/suffix detection — still used for the
    // single-line, same-line-count fast path (unchanged from the proven
    // original: cheap, avoids building the full line-layout machinery for
    // the overwhelmingly common case of a counter/price/single-word flap
    // that never wraps).
    var findPrefixSuffix = function(oldText, newText) {
      var oldG = toGraphemes(oldText);
      var newG = toGraphemes(newText);
      var prefixLen = 0;
      var minLen = Math.min(oldG.length, newG.length);
      while (prefixLen < minLen && oldG[prefixLen] === newG[prefixLen]) prefixLen++;
      var suffixLen = 0;
      while (suffixLen < (minLen - prefixLen) &&
             oldG[oldG.length - 1 - suffixLen] === newG[newG.length - 1 - suffixLen]) suffixLen++;
      return {
        prefix: oldG.slice(0, prefixLen).join(''),
        oldMiddle: oldG.slice(prefixLen, oldG.length - suffixLen).join(''),
        newMiddle: newG.slice(prefixLen, newG.length - suffixLen).join(''),
        suffix: suffixLen > 0 ? oldG.slice(oldG.length - suffixLen).join('') : ''
      };
    };

    // Build one grapheme-split layer (enter or exit) for a single word,
    // registering its per-char CSS animation + returning the char spans for
    // layer-level WAAPI opacity. Shared by both the single-line fast path
    // and the multi-line reflow path's add/remove words.
    var buildCharLayer = function(word, kind, letterSpacingPx) {
      var layer = document.createElement('span');
      layer.style.display = 'inline-block';
      layer.style.whiteSpace = 'nowrap';
      // Each grapheme becomes its own `inline-block` span so it can animate
      // independently — but per-element `inline-block` boxes do NOT inherit
      // the parent's letter-spacing the way a single text run does (verified
      // empirically: a 6-char word at letter-spacing:-1.911px measures
      // 232.19px as one text node vs 245.20px split into inline-block chars
      // with no compensation — a ~13px/word error, growing with word count,
      // never corrected until cleanup swaps back to real textContent). Setting
      // letter-spacing on this LAYER wrapper (not each char) reproduces the
      // single-text-node width exactly (233.75px measured vs 232.19px
      // reference, matching the SAME sub-pixel-class delta the uncorrected
      // case already had at ls=0 — 233.75-232.19=1.56px ≈ 245.20-243.66=1.54px
      // baseline inline-block layout overhead, NOT a letter-spacing residual),
      // because the browser applies letter-spacing at the wrapper's own
      // inline formatting context across its child boxes.
      if (letterSpacingPx) { layer.style.letterSpacing = letterSpacingPx + 'px'; }
      var graphemes = toGraphemes(word);
      var chars = [];
      for (var gi = 0; gi < graphemes.length; gi++) {
        var ch = graphemes[gi];
        var span = document.createElement('span');
        span.textContent = ch === ' ' ? '\u00A0' : ch;
        span.style.display = 'inline-block';
        layer.appendChild(span);
        chars.push(span);
      }
      return { layer: layer, chars: chars };
    };

    // `baseDelay` (default 0) offsets every character's stagger delay by a
    // fixed amount — used by the multi-line reflow path to stagger ACROSS
    // changed words (one word-slot of delay per word) while each word's own
    // characters still stagger normally WITHIN that word, instead of the
    // whole diff's characters sharing one flat, unbounded stagger index.
    var applyCharStagger = function(chars, kind, animName, easing, baseDelay) {
      var offset = baseDelay || 0;
      var maxDelay = offset;
      chars.forEach(function(span, index) {
        var delay = offset + getStaggerDelay(index, chars.length);
        if (delay > maxDelay) maxDelay = delay;
        span.style.animation = animName + ' ' + dur + 'ms ' + easing + ' ' + delay + 'ms forwards';
      });
      return maxDelay;
    };

    var animateChange = function(oldText, newText) {
      isChanging = true;
      observer.disconnect();
      %yield true -> $changing;

      // Accessibility: instant swap for reduced-motion preference
      if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
        %&el.textContent = newText;
        currentText = newText;
        isChanging = false;
        %yield false -> $changing;
        observer.observe(%&el, { characterData: true, childList: true, subtree: true, characterDataOldValue: true });
        return;
      }

      // Re-read computed style each cycle (font/letter-spacing/display may
      // change via responsive breakpoints).
      var cs = getComputedStyle(%&el);
      var font = cs.font || [cs.fontStyle, cs.fontWeight, cs.fontSize, cs.fontFamily].filter(Boolean).join(' ');
      var lsRaw = parseFloat(cs.letterSpacing);
      var letterSpacingPx = isNaN(lsRaw) ? 0 : lsRaw;
      var lhRaw = parseFloat(cs.lineHeight);
      var lineHeight = isNaN(lhRaw) ? (parseFloat(cs.fontSize) || 16) * 1.2 : lhRaw;
      var elFontSize = parseFloat(cs.fontSize) || 16;

      // Real wrap constraint: a block-level element's box already stretches
      // to its container regardless of content, so its CURRENT clientWidth
      // (measured BEFORE any DOM mutation) is a real, meaningful wrap
      // boundary. An auto-width inline-level element (the "flap" case —
      // inline-block, no explicit width) never force-wraps ITSELF by CSS
      // auto-width semantics alone — but BUG-172's classification stopped
      // there and assumed Infinity, missing that the element can still SIT
      // inside an ancestor that constrains where its own line can run (e.g.
      // a `display:block` sibling above/below forcing it onto its own line
      // within a fixed-width parent). Verified empirically: the ora-ventures
      // hero flap (inline-block, sole occupant of its own line in an h1
      // whose siblings are block) genuinely wraps to 2-3 lines at narrow
      // viewport widths — `containerWidth: Infinity` made the driver always
      // DESIGN a single-line transition, so the real (wrapped) layout only
      // appeared as a visible snap the instant cleanup restored real
      // textContent. Fix: measure the REAL constraint for every element via
      // a display/white-space toggle (forces this element alone onto a
      // block box at its actual position, giving its true available width
      // from the nearest constraining ancestor — the DOM's own layout
      // engine, not a display-type heuristic), then restore its authored
      // inline styles immediately. Only Infinity when even that toggle
      // yields 0 (detached/hidden element).
      var elDisplay = cs.display;
      var elIsBlockLevel = elDisplay === 'block' || elDisplay === 'flex' ||
        elDisplay === 'grid' || elDisplay === 'list-item' ||
        elDisplay === 'table' || elDisplay === 'flow-root';
      var measureRealConstraint = function() {
        var prevDisplay = %&el.style.display;
        var prevWhiteSpace = %&el.style.whiteSpace;
        %&el.style.display = 'block';
        %&el.style.whiteSpace = 'normal';
        var w = %&el.clientWidth;
        %&el.style.display = prevDisplay;
        %&el.style.whiteSpace = prevWhiteSpace;
        return w;
      };
      // NOTE: this callback runs FROM a MutationObserver, which by definition
      // only fires AFTER the DOM mutation (the textContent swap to newText)
      // already completed. A live DOM read here (offsetHeight/offsetWidth)
      // would ALREADY reflect the NEW text's box, not the old one — reading
      // it as "current" (meaning: the OLD, pre-change state) would measure
      // the wrong box entirely. The ORIGINAL driver had this same
      // read-after-mutation timing but never surfaced it: its flap was
      // ALWAYS single-line, and a single line's height is constant regardless
      // of text CONTENT (only line COUNT changes height), so reading
      // offsetHeight "too late" never actually returned a different value.
      // This rewrite's reflow path is the first to genuinely need the OLD
      // (pre-mutation) box geometry — which is why this race was latent
      // until now. `containerWidth` is still safe to read live (via the
      // toggle above): a display/white-space swap measures the SAME
      // constraining ancestor regardless of pre/post mutation, because that
      // ancestor's own width doesn't change just because the flap's TEXT
      // did. The only real "never wraps" signal is the element's OWN resting
      // (author-set, pre-animation) `white-space` — `nowrap` means the
      // author explicitly opted this element out of wrapping (e.g. a
      // single-line counter that must never wrap regardless of container
      // width), which the toggle-based measurement must not override.
      var elRestingWhiteSpace = cs.whiteSpace;
      var containerWidth = elRestingWhiteSpace === 'nowrap' ? Infinity : measureRealConstraint();
      if (!(containerWidth > 0)) { containerWidth = Infinity; }

      // Compute BOTH layouts (old and new) against the SAME real constraint,
      // BEFORE touching the DOM — this is the actual "will this wrap"
      // answer, not an inference from a box-level side effect afterward.
      var oldLayout = layoutLines(oldText, font, containerWidth, lineHeight, letterSpacingPx);
      var newLayout = layoutLines(newText, font, containerWidth, lineHeight, letterSpacingPx);

      // currentHeight/currentWidth MUST come from the OLD text's computed
      // layout (a pure string->geometry function, immune to DOM timing), NOT
      // a live offsetHeight/offsetWidth DOM read — by the time this
      // MutationObserver callback runs, the DOM mutation (textContent ->
      // newText) has ALREADY happened, so a live read would silently measure
      // the NEW text's box while the code believes it's reading the OLD one
      // (see the read-after-mutation note above `containerWidth`). Falling
      // back to a live read only when the computed layout is degenerate
      // (0 height, e.g. truly empty oldText).
      var liveHeightFallback = %&el.offsetHeight;
      var currentHeight = oldLayout.height > 0 ? oldLayout.height : liveHeightFallback;
      var targetHeight = newLayout.height > 0 ? newLayout.height : currentHeight;

      // Width animation only makes sense for an auto-width (shrink-to-fit)
      // box — a block-level element's width is dictated by its container,
      // not its content, so it never needs a width animation regardless of
      // line count. For an auto-width element, `targetWidth` is the widest
      // LINE in the real (now line-count-aware) layout — single-line text
      // still gets the exact `measureNatural` value (a 1-line layout's one
      // line IS the natural width at that constraint), and a genuinely
      // multi-line auto-width box now correctly widens/narrows to its
      // widest wrapped line instead of always assuming one unwrapped run.
      var liveWidthFallback = %&el.offsetWidth;
      var widestLine = function(layout, text) {
        if (!layout.lines || layout.lines.length === 0) {
          return measureNatural(text, font, letterSpacingPx);
        }
        var max = 0;
        for (var li2 = 0; li2 < layout.lines.length; li2++) {
          if (layout.lines[li2].width > max) max = layout.lines[li2].width;
        }
        return max;
      };
      var currentWidth = elIsBlockLevel
        ? liveWidthFallback
        : (function() {
            var w = widestLine(oldLayout, oldText);
            return w > 0 ? w : liveWidthFallback;
          })();

      var targetWidth = elIsBlockLevel
        ? currentWidth
        : widestLine(newLayout, newText);
      if (targetWidth <= 0 && newText.length > 0) targetWidth = currentWidth;

      var totalAnimDur;
      var overflowPad = Math.ceil(elFontSize * 0.6);

      // Phase 2 — Overflow setup (shared by both paths). `inset(-pad -pad
      // -pad -pad)` protects all four edges, not just top/bottom — the reflow
      // path can render new lines below the old box's bottom edge transiently
      // before the box height animation eases open to match, AND (for the
      // single-line fast path) each changed word's char-split layer renders
      // WIDER than its own box's real (kerned) width: splitting a word into
      // one `inline-block` span per grapheme structurally prevents cross-glyph
      // kerning — no CSS property recovers it once glyphs are in separate
      // boxes — so the char-split rendering is always at least as wide as the
      // real kerned text pretext's `measureNatural`/`widestLine` compute for
      // `currentWidth`/`targetWidth`. A zero horizontal inset clipped that
      // excess against the box edge for the animation's entire duration
      // (verified live: a real word like "waits on IT." renders 8.48px wider
      // char-split than real text; "AVAV"/"WAWA"-class kerning pairs lose
      // 8-16px). The SAME horizontal pad used for the vertical case is
      // generous enough to cover this (0.6x font-size comfortably exceeds any
      // realistic per-pair kerning loss) — this only widens the PAINT-time
      // clip rect, not the box's own animated width (which stays on the real
      // kerned currentWidth->targetWidth trajectory and so still lands
      // exactly on the real text's width once cleanup restores textContent).
      %&el.style.overflow = 'visible';
      %&el.style.clipPath = 'inset(-' + overflowPad + 'px -' + overflowPad + 'px -' + overflowPad + 'px -' + overflowPad + 'px)';
      %&el.style.height = currentHeight + 'px';
      if (!elIsBlockLevel) {
        %&el.style.width = currentWidth + 'px';
        %&el.style.whiteSpace = 'nowrap';
      }

      var wrapper = document.createElement('span');
      wrapper.style.display = elIsBlockLevel ? 'block' : 'inline';
      wrapper.style.position = 'relative';

      var allExitChars = [];
      var allEnterChars = [];

      if (oldLayout.lineCount <= 1 && newLayout.lineCount <= 1) {
        // ---------------------------------------------------------------
        // Single-line fast path (unchanged from the proven original): a
        // counter, price, or single-word flap that never wraps. Cheap
        // prefix/suffix trim + per-word char-diff, no line-position bookkeeping.
        // ---------------------------------------------------------------
        var psx = findPrefixSuffix(oldText, newText);
        var prefix = psx.prefix;
        var oldMiddle = psx.oldMiddle;
        var newMiddle = psx.newMiddle;
        var suffix = psx.suffix;

        var oldWords = oldMiddle ? oldMiddle.split(' ') : [''];
        var newWords = newMiddle ? newMiddle.split(' ') : [''];
        var maxWords = Math.max(oldWords.length, newWords.length);

        if (prefix) { wrapper.appendChild(document.createTextNode(prefix)); }

        var wordContainers = [];
        for (var w = 0; w < maxWords; w++) {
          if (w > 0) { wrapper.appendChild(document.createTextNode(' ')); }
          var oldWord = oldWords[w] || '';
          var newWord = newWords[w] || '';

          var wc = document.createElement('span');
          wc.style.display = 'inline-block';
          wc.style.position = 'relative';
          wc.style.whiteSpace = 'nowrap';
          wc.style.verticalAlign = 'bottom';
          wc.style.isolation = 'isolate';
          wc.style.willChange = 'transform, opacity';

          var enter = buildCharLayer(newWord, 'enter', letterSpacingPx);
          enter.layer.style.opacity = '0';
          wc.appendChild(enter.layer);
          allEnterChars.push.apply(allEnterChars, enter.chars);

          var exit = buildCharLayer(oldWord, 'exit', letterSpacingPx);
          exit.layer.style.position = 'absolute';
          exit.layer.style.left = '0';
          exit.layer.style.top = '0';
          // No explicit `width` here (unlike an earlier version of this code):
          // `exit.layer` is `display:inline-block` + `position:absolute`, so it
          // ALREADY shrink-wraps to its own rendered content width. Pinning it to
          // `measureNatural(oldWord, ...)` (pretext's KERNED measurement) was
          // itself the bug — splitting a word into per-character inline-block
          // spans structurally prevents cross-glyph kerning (verified empirically:
          // "AVAV"/"WAWA"-class kerning pairs lose 8-16px; a plain repeated-char
          // word like "aaaaaaaaaa" loses ~0px, isolating the effect to kerning,
          // not letter-spacing, which the wrapper's own `letter-spacing` already
          // handles correctly). Pinning the box to the kerned width while its
          // rendered content is wider (un-kerned) clipped the tail of the exit
          // layer's own text against the flap's zero-inset horizontal clip-path
          // for the animation's whole duration — not just a boundary snap.
          // Letting the layer auto-size to its real (un-kerned) rendered width
          // is both simpler and correct.
          exit.layer.style.opacity = '1';
          wc.appendChild(exit.layer);
          allExitChars.push.apply(allExitChars, exit.chars);

          wrapper.appendChild(wc);
          wordContainers.push({ enterLayer: enter.layer, exitLayer: exit.layer });
        }
        if (suffix) { wrapper.appendChild(document.createTextNode(suffix)); }

        var maxDelay1 = applyCharStagger(allExitChars, 'exit', animId + '-exit', exitEasing);
        var maxDelay2 = applyCharStagger(allEnterChars, 'enter', animId + '-enter', enterEasing);
        totalAnimDur = dur + Math.max(maxDelay1, maxDelay2);

        var enterOpacity = enterKf.length > 0 && enterKf[0].opacity !== undefined ? enterKf[0].opacity : 0;
        var exitOpacityEnd = exitKf.length > 0 && exitKf[exitKf.length - 1].opacity !== undefined ? exitKf[exitKf.length - 1].opacity : 0;
        wordContainers.forEach(function(wci) {
          wci.enterLayer.animate(
            [{ opacity: String(enterOpacity) }, { opacity: '1' }],
            { duration: totalAnimDur, easing: enterEasing, fill: 'forwards' }
          );
          wci.exitLayer.animate(
            [{ opacity: '1' }, { opacity: String(exitOpacityEnd) }],
            { duration: totalAnimDur, easing: exitEasing, fill: 'forwards' }
          );
        });
      } else {
        // ---------------------------------------------------------------
        // Multi-line REFLOW path: at least one side spans >1 line. Diff at
        // the WORD level (not character level) across the real line/x/y
        // grid computed above — persisting words GLIDE to their new
        // position; only genuinely added/removed words get the
        // per-character split-flap treatment.
        // ---------------------------------------------------------------
        var oldPositioned = positionWords(oldLayout.lines, font, lineHeight, letterSpacingPx);
        var newPositioned = positionWords(newLayout.lines, font, lineHeight, letterSpacingPx);
        var ops = diffWordSequence(oldPositioned, newPositioned);

        // Trim maximal leading/trailing runs of UNMOVED kept words (same
        // line, same x — a true no-op) to plain text, mirroring the
        // single-line path's prefix/suffix optimization: a paragraph whose
        // first two lines never change shouldn't pay any span/animation cost
        // for them.
        var isStaticKeep = function(op) {
          return op.type === 'keep' && op.oldW.x === op.newW.x && op.oldW.y === op.newW.y;
        };
        var leadEnd = 0;
        while (leadEnd < ops.length && isStaticKeep(ops[leadEnd])) leadEnd++;
        var trailStart = ops.length;
        while (trailStart > leadEnd && isStaticKeep(ops[trailStart - 1])) trailStart--;

        var leadWords = ops.slice(0, leadEnd).map(function(op) { return op.newW.word; });
        var midOps = ops.slice(leadEnd, trailStart);
        var trailWords = ops.slice(trailStart).map(function(op) { return op.newW.word; });

        if (leadWords.length) { wrapper.appendChild(document.createTextNode(leadWords.join(' ') + (midOps.length || trailWords.length ? ' ' : ''))); }

        // Changed-word index (add/remove ops only — NOT a running character
        // count) drives cross-word stagger below. Bounding stagger by WORD
        // count rather than character count is what keeps a large multi-word
        // reflow "lean, bounded duration": a sentence with 14 words changing
        // stagger over 14 word-slots, not over the ~70 characters those words
        // contain — the earlier, unbounded version scaled `totalAnimDur` with
        // total changed CHARACTERS (all words flattened into one shared
        // stagger array), so a long sentence rewrite took 2.5s+ to settle
        // instead of the intended lean handful of hundred ms.
        var changedWordIndex = 0;
        var maxWordDelay = 0;

        var moveAnims = [];
        midOps.forEach(function(op, idx) {
          if (idx > 0) { wrapper.appendChild(document.createTextNode(' ')); }

          if (op.type === 'keep') {
            var span = document.createElement('span');
            span.textContent = op.newW.word;
            span.style.display = 'inline-block';
            span.style.position = 'relative';
            span.style.whiteSpace = 'nowrap';
            span.style.willChange = 'transform';
            wrapper.appendChild(span);
            var dx = op.newW.x - op.oldW.x;
            var dy = op.newW.y - op.oldW.y;
            if (dx !== 0 || dy !== 0) {
              var anim = span.animate(
                [
                  { transform: 'translate(' + (-dx) + 'px, ' + (-dy) + 'px)' },
                  { transform: 'translate(0px, 0px)' }
                ],
                { duration: dur, easing: 'ease-in-out', fill: 'both' }
              );
              moveAnims.push(anim.finished.catch(function() {}));
            }
          } else if (op.type === 'remove') {
            var exitWc = document.createElement('span');
            exitWc.style.display = 'inline-block';
            exitWc.style.position = 'relative';
            exitWc.style.whiteSpace = 'nowrap';
            exitWc.style.isolation = 'isolate';
            var exitLayer = buildCharLayer(op.oldW.word, 'exit', letterSpacingPx);
            // Layer-level opacity via WAAPI, matching the single-line path
            // (kfToCss only emits `transform` to the CSS @keyframes — opacity
            // is always a separate WAAPI layer animation, see its own comment
            // above). Without this, a removed word pops out abruptly instead
            // of fading, even though the author's :exiting block sets opacity.
            exitLayer.layer.style.opacity = '1';
            exitWc.appendChild(exitLayer.layer);
            wrapper.appendChild(exitWc);
            var wordDelay = changedWordIndex * stagger;
            var localMax = applyCharStagger(exitLayer.chars, 'exit', animId + '-exit', exitEasing, wordDelay);
            if (localMax > maxWordDelay) maxWordDelay = localMax;
            changedWordIndex++;
            var exitOpacityEndR = exitKf.length > 0 && exitKf[exitKf.length - 1].opacity !== undefined ? exitKf[exitKf.length - 1].opacity : 0;
            exitLayer.layer.animate(
              [{ opacity: '1' }, { opacity: String(exitOpacityEndR) }],
              { duration: dur + localMax, easing: exitEasing, fill: 'forwards' }
            );
          } else {
            var enterWc = document.createElement('span');
            enterWc.style.display = 'inline-block';
            enterWc.style.position = 'relative';
            enterWc.style.whiteSpace = 'nowrap';
            enterWc.style.isolation = 'isolate';
            var enterLayer = buildCharLayer(op.newW.word, 'enter', letterSpacingPx);
            // See the matching exit-side comment above: layer-level opacity is
            // always a separate WAAPI animation (kfToCss only emits `transform`).
            var enterOpacityR = enterKf.length > 0 && enterKf[0].opacity !== undefined ? enterKf[0].opacity : 0;
            enterLayer.layer.style.opacity = String(enterOpacityR);
            enterWc.appendChild(enterLayer.layer);
            wrapper.appendChild(enterWc);
            var wordDelay2 = changedWordIndex * stagger;
            var localMax2 = applyCharStagger(enterLayer.chars, 'enter', animId + '-enter', enterEasing, wordDelay2);
            if (localMax2 > maxWordDelay) maxWordDelay = localMax2;
            changedWordIndex++;
            enterLayer.layer.animate(
              [{ opacity: String(enterOpacityR) }, { opacity: '1' }],
              { duration: dur + localMax2, easing: enterEasing, fill: 'forwards' }
            );
          }
        });

        if (trailWords.length) { wrapper.appendChild(document.createTextNode((midOps.length ? ' ' : '') + trailWords.join(' '))); }

        totalAnimDur = dur + maxWordDelay;
      }

      // Phase 4 — DOM swap
      %&el.textContent = '';
      %&el.appendChild(wrapper);

      // Widen the horizontal clip pad to the REAL measured overflow (single-
      // line fast path). `wc` (word-container) is `display:inline-block`, so
      // its own box width comes only from its in-flow child — the enter
      // layer. The exit layer is `position:absolute` (so enter/exit can
      // cross-fade in place without pushing layout), so it does NOT
      // contribute to `wc`'s width. When a word shrinks or is removed
      // outright (fewer new words than old), the enter layer is narrow/empty
      // while the exit layer still paints the full old word — wider still
      // once you add the kerning-loss the char-split rendering carries (see
      // the buildCharLayer comment above). An earlier version of this fix
      // tried reserving that width via `minWidth` on each `wc` — wrong: the
      // OUTER box (`%&el`) legitimately eases its own width from
      // `currentWidth` toward `targetWidth` over the transition (the WAAPI
      // tween below), and forcing the wrapper's rendered content to stay at
      // its full original width fights that shrink — the gap (and visible
      // clipped overflow) GREW monotonically as the box narrowed (verified:
      // 3.6px → 168px within one transition). The actual fix: let the box
      // keep tweening on its real (kerned) width trajectory, and instead
      // widen the clip so nothing paintable is cut off. `wrapper` is
      // attached now, so its real rendered width (all word-containers, exit
      // layers included since they're absolutely positioned WITHIN their own
      // wc and don't collapse it) is measurable directly — no parallel
      // pretext re-measurement, just reading what got painted. The needed
      // pad is the excess of that real width over the NARROWER of
      // currentWidth/targetWidth (whichever the box is closer to at the
      // worst moment of the tween).
      // `position:absolute` elements are removed from normal flow entirely
      // and do NOT contribute to any ancestor's `getBoundingClientRect()` —
      // not `wc`'s, not `wrapper`'s. The exit layers (each word-container's
      // absolutely-positioned child) are exactly that, so the true painted
      // extent has to come from the LEAF spans themselves (each grapheme),
      // taking the widest left/right excursion across all of them relative
      // to the wrapper's own left edge — the same measurement technique
      // that found this bug live via CDP in the first place.
      var leafSpans = wrapper.querySelectorAll('span');
      var wrapperLeft = wrapper.getBoundingClientRect().left;
      var maxRightOffset = 0, maxLeftOffset = 0;
      for (var lsI = 0; lsI < leafSpans.length; lsI++) {
        if (leafSpans[lsI].children.length > 0) continue; // only leaf (grapheme) spans
        var lsRect = leafSpans[lsI].getBoundingClientRect();
        if (lsRect.width === 0) continue;
        var rightOffset = lsRect.right - wrapperLeft;
        var leftOffset = wrapperLeft - lsRect.left;
        if (rightOffset > maxRightOffset) maxRightOffset = rightOffset;
        if (leftOffset > maxLeftOffset) maxLeftOffset = leftOffset;
      }
      var narrowestBoxWidth = elIsBlockLevel ? currentWidth : Math.min(currentWidth, targetWidth);
      var extraRightOverflow = maxRightOffset - narrowestBoxWidth;
      var extraLeftOverflow = maxLeftOffset;
      if (extraRightOverflow > 0 || extraLeftOverflow > 0) {
        var padRight = Math.ceil(overflowPad + Math.max(0, extraRightOverflow));
        var padLeft = Math.ceil(overflowPad + Math.max(0, extraLeftOverflow));
        %&el.style.clipPath = 'inset(-' + overflowPad + 'px -' + padRight + 'px -' + overflowPad + 'px -' + padLeft + 'px)';
      }

      // Height + width animation via WAAPI — ONE animation covering both
      // properties so they stay in lockstep. targetHeight/targetWidth are
      // known UPFRONT from the real line layout above, not re-derived
      // reactively from box-level measurement after the fact.
      var boxAnimDone = Promise.resolve();
      var boxFrom = {};
      var boxTo = {};
      var needsBoxAnim = false;
      if (targetHeight !== currentHeight) {
        boxFrom.height = currentHeight + 'px';
        boxTo.height = targetHeight + 'px';
        needsBoxAnim = true;
      }
      if (!elIsBlockLevel && targetWidth !== currentWidth) {
        boxFrom.width = currentWidth + 'px';
        boxTo.width = targetWidth + 'px';
        needsBoxAnim = true;
      }
      if (needsBoxAnim) {
        boxAnimDone = %&el.animate(
          [boxFrom, boxTo],
          { duration: totalAnimDur, easing: 'ease-in-out', fill: 'forwards' }
        ).finished.catch(function() {});
      }

      setTimeout(function() {
        if (destroyed) return;
        boxAnimDone.then(function() {
          if (destroyed) return;
          // Phase 5 — Cleanup
          %&el.style.height = targetHeight + 'px';
          if (!elIsBlockLevel) {
            %&el.style.width = targetWidth + 'px';
          }
          %&el.textContent = newText;
          %&el.getAnimations().forEach(function(a) { a.cancel(); });
          %&el.style.height = '';
          %&el.style.width = '';
          %&el.style.whiteSpace = '';
          %&el.style.overflow = '';
          %&el.style.clipPath = '';

          if (ST.debug && typeof console !== 'undefined') {
            var actualH = %&el.offsetHeight;
            var actualW = %&el.offsetWidth;
            if (Math.abs(actualH - targetHeight) > 2) {
              console.warn('[value-change] height delta: ' + (actualH - targetHeight) + 'px');
            }
            if (!elIsBlockLevel && Math.abs(actualW - targetWidth) > 2) {
              console.warn('[value-change] width delta: ' + (actualW - targetWidth) + 'px');
            }
          }

          isChanging = false;
          %yield false -> $changing;

          observer.observe(%&el, {
            characterData: true,
            childList: true,
            subtree: true,
            characterDataOldValue: true
          });
        });
      }, totalAnimDur);
    };

    // Store current text for comparison
    var currentText = %&el.textContent || '';

    // Pin baseline mode once at mount time (not just during a transition):
    // per CSS2.1 10.8.1, an atomic inline-level box (inline-block/-flex/
    // -grid/-table) contributes its LAST LINE BASELINE to the surrounding
    // line box when its computed `overflow` is `visible`, but its BOTTOM
    // MARGIN EDGE when `overflow` computes to anything else (hidden, clip,
    // scroll, auto). This driver toggles `overflow` between `hidden`
    // (authored rest state) and `visible` (mid-transition) on every single
    // cycle. Each toggle flips WHICH baseline rule applies, which silently
    // grows/shrinks the height of the ENCLOSING line box. An explicit, FIXED
    // `vertical-align` makes both baseline-computation modes agree.
    // Skipped for block-level elements (irrelevant — they don't participate
    // in inline baseline alignment) and left alone if the author already set
    // an explicit vertical-align (respecting intent).
    var initialCs = getComputedStyle(%&el);
    var initialDisplay = initialCs.display;
    var isAtomicInlineBox = initialDisplay === 'inline-block' ||
      initialDisplay === 'inline-flex' || initialDisplay === 'inline-grid' ||
      initialDisplay === 'inline-table';
    if (isAtomicInlineBox && initialCs.verticalAlign === 'baseline') {
      %&el.style.verticalAlign = 'bottom';
    }

    observer = new MutationObserver(function(mutations) {
      for (var mi = 0; mi < mutations.length; mi++) {
        var mutation = mutations[mi];
        if (mutation.type === 'characterData' || mutation.type === 'childList') {
          var newText = %&el.textContent || '';
          if (newText !== currentText && !isChanging) {
            var oldText = currentText;
            currentText = newText;
            animateChange(oldText, newText);
            break;
          }
        }
      }
    });

    observer.observe(%&el, {
      characterData: true,
      childList: true,
      subtree: true,
      characterDataOldValue: true
    });

    var destroy = function() {
      destroyed = true;
      if (observer) observer.disconnect();
      if (styleEl.parentNode) styleEl.remove();
      controller.abort();
    };

    %yield destroy -> $destroy;

    // Register cleanup with element
    ST.onCleanup(%&el, destroy);
  }

  %exports {
    $changing: bool
    $destroy: fn() ~> effect
  }
}
/// After Driver Primitive
///
/// Waits for a predecessor timeline to complete, then plays animation.
/// Part of the Timeline Registration + Event System (Approach 2).
///
/// @param &el - Element to animate
/// @param predecessor - Name of the timeline to wait for
/// @param name - Name for this timeline (for chaining)
/// @param duration - Animation duration in ms
/// @param delay - Delay after predecessor completes before starting (ms)
///
/// @yields $progress - Current progress (0-1)
/// @yields $destroy - Cleanup function
///
/// @example
/// // In .st file:
/// .hero {
///   @load intro(600ms) { opacity: 0 -> 1 }
/// }
/// .subtitle {
///   @after intro subtitle-fade(delay: 200ms, duration: 300ms) {
///     opacity: 0 -> 1
///   }
/// }

// after-driver DELETED (PLAN-126 W5): `@after` is retired — sequencing is
// `@on $x.done { true => --form; }` (arms on the done facet of a published
// progress signal). The ST.timelines string registry it chained through is
// deleted in the same wave.

%primitive load-driver(
  &el,
  name: string = "",
  duration: number = 1000,
  delay: number = 0,
  threshold: number = 0.1,
  immediate: bool = false
) {
  %emit js {
    (() => {
    let progress = 0;
    let started = false;
    let rafUnsub = null;
    let observer = null;

    const dur = %duration;
    const delayMs = typeof %delay === 'string' ? parseFloat(%delay) || 0 : (%delay || 0);
    const timelineName = %name;


    const play = () => {
      if (started) return;
      started = true;

      let startTime = null;

      const animate = (timestamp) => {
        if (!startTime) startTime = timestamp + delayMs;
        if (timestamp < startTime) {
          return; // Still in delay, coordinator will call next frame
        }

        const elapsed = timestamp - startTime;
        progress = Math.min(1, elapsed / dur);
        %yield progress -> $%name;
        if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, progress);

        if (progress >= 1) {
          if (rafUnsub) { rafUnsub(); rafUnsub = null; }
          // Notify completion for @after chaining
        }
        // Otherwise coordinator continues calling
      };

      // Use RAF coordinator for synchronized updates
      rafUnsub = ST._raf.add(animate);
    };

    // Set initial progress to 0 so watchers receive initial state
    %yield 0 -> $%name;
    if (typeof __stDebugHook !== 'undefined') __stDebugHook.timeline(timelineName, 0);

    if (%immediate) {
      // Skip observer — play after microtask so watchers register first
      queueMicrotask(() => play());
    } else {
      observer = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            // `$visible` is the SENSOR half of this driver: an author can read
            // "am I on screen" (`ST.resolve(el, 'visible')`) independently of
            // the entrance it gates. The intersection primitive published this
            // and `.visible` used to bind that primitive, so dropping it here
            // would silently retire a working read (tests/unit/macros/
            // on-load.test.st pins it).
            %yield entry.isIntersecting -> $visible;
            if (entry.isIntersecting) {
              play();
              observer.disconnect();
            }
          }
        },
        { threshold: %threshold }
      );
      // Defer observer setup to allow apply-animations watchers to be registered first.
      // Without this, IntersectionObserver fires synchronously if element is already visible,
      // causing the animation to complete before watchers exist.
      queueMicrotask(() => observer.observe(%&el));
    }

    const destroy = () => {
      if (observer) observer.disconnect();
      if (rafUnsub) { rafUnsub(); rafUnsub = null; }
    };

    %yield destroy -> $destroy;

    // Register cleanup inside IIFE where destroy is in scope
    ST.onCleanup(%&el, destroy);
    })();
  }

  %exports {
    $progress: number
    $visible: bool        // true once `threshold` of the element is on screen
    $destroy: fn() ~> effect
  }
}


/// On Mutation Handler Primitive
///
/// Handles event-triggered mutations and @emit actions.
/// This is the primitive backing for @on click { mutations } syntax.
///
/// @param &el - Element to attach handler to
/// @param event - Event type to listen for (click, submit, etc)
/// @param actions - Array of action objects from on_body capture type
/// @param target - Optional CSS selector for event delegation
///
/// @example
/// %binds {
///   on-mutation-handler(&self, event: "click", actions: $body)
/// }

%primitive on-mutation-handler(
  &el,
  event: ident,
  target: selector?,
  key: string?,
  actions: expr
) {
  %emit js {
    (() => {
      const __stTargetEl = %&el;
      const eventType = %event;
      const delegateSelector = %target;
      const keyFilter = %key;
      const body = %actions;

      const handler = function(e) {
        // BUG-375: a keyboard binding may name the ONE key it answers to
        // (`@on &.keydown("Enter") { … }`). Without it the handler fires on
        // EVERY key — a chat composer bound this way sent a message per
        // keystroke, and nothing in the toolchain objected, because the
        // qualifier authors reached for (`&.keydown:Enter`) has no slot in
        // `driver_expr`'s grammar and was parsed and discarded in silence.
        //
        // Checked FIRST: a non-matching key is not this handler's event at
        // all, so neither delegation nor the mutations below should run.
        if (keyFilter && e.key !== keyFilter) return;

        // Event delegation: if target specified, find matching element
        let $ = __stTargetEl;
        if (delegateSelector) {
          $ = e.target.closest(delegateSelector);
          if (!$) return;  // Click wasn't on matching element
        }

        // Execute the mutation/effect body via the shared runtime helper (DRY:
        // the same processor @effect's effect-runner uses). `$` is the (possibly
        // delegated) target element; @emit source detail uses it too.
        ST.runMutations($, body);
      };

      if (eventType === 'visible') {
        // Use IntersectionObserver for visibility-triggered mutations.
        // rootMargin: '0px' observes the REAL, full viewport (BUG-185 --
        // a prior narrow top-10%-only rootMargin broke bottom-anchored
        // elements; see tests/unit/animations/on-visible-mutation.test.st
        // and this repo's org bug tracker for the full writeup).
        const observer = new IntersectionObserver(
          (entries) => {
            for (const entry of entries) {
              if (entry.isIntersecting) {
                handler({ target: __stTargetEl });
              }
            }
          },
          { threshold: 0, rootMargin: '0px' }
        );
        observer.observe(__stTargetEl);
        ST.onCleanup(__stTargetEl, () => observer.disconnect());
      } else {
        // Idempotent bind: a selector-init may run more than once over the same
        // element (page-CSS synchronous querySelectorAll fallback AND the dynamic
        // MutationObserver can both visit a node; a re-flush re-runs init). Without
        // a guard the handler binds N times and one user event fires the mutation N
        // times (an even N silently cancels a toggle). Tag the element per
        // event-type+body so a repeat init is a no-op.
        const __stOnSet = (__stTargetEl.__stOnBound || (__stTargetEl.__stOnBound = new Set()));
        const __stOnSig = eventType + '::' + (typeof body === 'object' ? JSON.stringify(body) : String(body));
        if (!__stOnSet.has(__stOnSig)) {
          __stOnSet.add(__stOnSig);
          __stTargetEl.addEventListener(eventType, handler);
          ST.onCleanup(__stTargetEl, () => { __stTargetEl.removeEventListener(eventType, handler); __stOnSet.delete(__stOnSig); });
        }
      }
    })();
  }
}
