/// Fade-In Engine Primitive (BUG-364)
///
/// Fades an element in — as a WHOLE — when it enters the viewport, optionally
/// travelling a short distance as it does.
///
/// WHY THIS PRIMITIVE EXISTS. `@fade-in` used to bind an intersection observer
/// and then do its work in a `%when $visible` body, which was empty with a TODO:
/// "the intersection observer triggers but no animation plays". That was not an
/// unfinished implementation of a workable design — `%when` is evaluated at
/// COMPILE time against captured macro bindings (see `stdlib/macros/each.st`,
/// BUG-362), so it can never see a runtime signal. The directive was waiting on
/// a mechanism that could not arrive.
///
/// `@reveal` is the model: a primitive with an `%emit` block that owns its
/// observer and drives a real WAAPI animation. This is that primitive for the
/// whole-element case — `reveal-engine` splits text into per-character units,
/// which is the wrong shape for "fade this box in".
///
/// @param &el       - Element to fade in
/// @param duration  - Animation duration in ms
/// @param delay     - Delay before the animation starts, in ms
/// @param distance  - How far the element travels while fading (CSS length)
/// @param direction - Which way it travels FROM ("bottom" moves up into place)
/// @param easing    - Easing function name; a declared `@form easing --name`
///                    curve resolves through `ST.ease` exactly as elsewhere
/// @param threshold - Visibility ratio that triggers the fade
/// @param once      - Fade only the first time it becomes visible
///
/// @param keyframes - Optional custom keyframes body (`@fade-in(...) { scale: 0.95 -> 1 }`)
///
/// @yields $faded - true once the fade has finished

%primitive fade-in-engine(
  &el,
  duration: number = 600,
  delay: number = 0,
  distance: string = "20px",
  direction: ("bottom" | "top" | "left" | "right" | "none") = "bottom",
  easing: string = "ease-out",
  threshold: number = 0.1,
  once: bool = true,
  keyframes: any = null
) {
  %emit js {
    (() => {
      // NB: bind to a DISTINCT name. The generated wrapper is
      // `const init = function(el) { ... }`, so naming this `el` emits
      // `const el = el;` — a temporal-dead-zone self-reference that throws
      // ReferenceError and kills the whole IIFE silently. `reveal-engine` uses
      // the bare `el` parameter for the same reason: it never re-declares it.
      const node = %&el;
      if (!node) return;

      const durationMs = Number(%duration) || 600;
      const delayMs = Number(%delay) || 0;
      const dist = String(%distance || '20px');
      const dir = %direction;
      const easingName = %easing;
      const onceOnly = %once;

      // A declared curve (`@form easing --friendly { spring(300, 22, 1) }`)
      // registers on `ST.ease` (BUG-297). Fall back to the literal string so a
      // plain CSS keyword like `ease-out` still works — the browser understands
      // those natively and refusing them would break every default.
      const resolveEasing = () => {
        const registered = (typeof ST !== 'undefined' && ST.ease) ? ST.ease[easingName] : null;
        return registered ? registered : easingName;
      };

      // Respect the user's stated preference. A fade that ignores
      // prefers-reduced-motion is an accessibility defect, not a flourish.
      const prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

      const offsetFor = (d) => {
        if (d === 'none') return null;
        if (d === 'bottom') return 'translateY(' + dist + ')';
        if (d === 'top') return 'translateY(-' + dist + ')';
        if (d === 'left') return 'translateX(-' + dist + ')';
        if (d === 'right') return 'translateX(' + dist + ')';
        return null;
      };

      const fromTransform = offsetFor(dir);

      // The element must start hidden, or it flashes at full opacity before the
      // observer fires. Set synchronously, not in the callback.
      if (!prefersReduced) {
        node.style.opacity = '0';
        if (fromTransform) node.style.transform = fromTransform;
      }

      let played = false;

      const play = () => {
        if (played && onceOnly) return;
        played = true;

        if (prefersReduced) {
          // Honour the preference by ARRIVING, not by animating: the element
          // must still end up visible.
          node.style.opacity = '1';
          node.style.transform = '';
          return;
        }

        // A custom body REPLACES the built-in fade, exactly as in
        // `reveal-engine`: the author who writes keyframes is describing the
        // whole animation, not decorating ours.
        const customKeyframes = ST.toWaapiKeyframes(%keyframes);
        // PARENTHESES ARE LOAD-BEARING: `a || b ? x : y` parses as `(a || b) ? x : y`,
        // so without them a custom body is computed and then thrown away.
        const keyframes = customKeyframes || (fromTransform
          ? [
              { opacity: 0, transform: fromTransform },
              { opacity: 1, transform: 'translateX(0) translateY(0)' }
            ]
          : [
              { opacity: 0 },
              { opacity: 1 }
            ]);

        const anim = node.animate(keyframes, {
          duration: durationMs,
          delay: delayMs,
          easing: resolveEasing(),
          fill: 'forwards'
        });

        anim.onfinish = () => {
          // Hand the final state back to CSS so the element is not left owned
          // by a finished animation (which blocks later style changes).
          node.style.opacity = '1';
          node.style.transform = '';
          %yield true -> $faded;
        };
      };

      const observer = new IntersectionObserver((entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            play();
            if (onceOnly) observer.disconnect();
          }
        }
      }, { threshold: %threshold });

      observer.observe(node);
    })();
  }
}
