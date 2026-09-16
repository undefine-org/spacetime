/// Signal-arms primitive — the runtime for `@on $signal { pat => --form; … }`
/// (PLAN-124 W3.3).
///
/// Watches the HEAD signal; on each change, the FIRST arm whose pattern
/// matches plays its resolved motion form via WAAPI. Patterns:
///   value      — matches when the signal equals it ("true"/"false" coerce to
///                booleans, numeric strings to numbers — arms are written
///                against the signal's VALUE, not its text)
///   [from, to] — a two-ended transition: matches when prev == from and the
///                new value == to. `->` is the opt-in; a bare value arm means
///                "arrived here from anywhere" (N states never force N² arms)
///   "*"        — the wildcard `_`: matches any change
///
/// The arms array is resolved at COMPILE time by the drive-arms expansion
/// (src/pipeline/drivers.rs): each arm's form is looked up in the form
/// registry, its keyframes param-substituted, so this primitive is pure
/// dispatch — it never learns what a "form" is.
///
/// @param signal - the watched binding (`$open` → head key `open`)
/// @param arms   - [{ match, animations, duration, easing, guard? }]
%primitive signal-arms(
  &el,
  signal: binding,
  arms: any
) {
  %emit js {
    (function() {
      // NB: bound to a DISTINCT name. `var el = %&el;` self-shadowed whenever
      // the splice site also names its element `el` (the selector-init closure
      // does) — the var was `undefined` and the `!node` guard below silently
      // killed EVERY arms instance. One name, one meaning.
      var node = %&el;
      var sigHead = String(%signal || '').replace(/^\$/, '').split('.')[0];
      var arms = %arms || [];
      if (!node || typeof ST === 'undefined' || !ST.watch) return;

      function coerce(s) {
        if (s === 'true') return true;
        if (s === 'false') return false;
        if (typeof s === 'string' && s !== '' && !isNaN(+s)) return +s;
        return s;
      }
      arms.forEach(function(a, index) {
        if (Array.isArray(a.match)) a.match = [coerce(a.match[0]), coerce(a.match[1])];
        else if (a.match !== '*') a.match = coerce(a.match);
        if (!a.guard) return;
        try {
          a.__signalArmsGuard = new Function('__old', '__new', 'return (' + a.guard + ');');
        } catch (error) {
          a.__signalArmsGuardInvalid = true;
          console.warn('signal-arms: guard compilation failed for arm ' + index, error);
        }
      });

      // `prev` is maintained by ST.watchChanges (which delivers next AND prev);
      // a two-ended arm `a -> b` needs the value being left behind.
      var prev;
      function matches(arm, v) {
        var m = arm.match;
        if (m === '*') return true;
        if (Array.isArray(m)) return prev === m[0] && v === m[1];
        return v === m;
      }
      function passesGuard(arm, index, v) {
        if (!arm.guard) return true;
        if (arm.__signalArmsGuardInvalid) return false;
        try {
          return !!arm.__signalArmsGuard(coerce(prev), coerce(v));
        } catch (error) {
          console.warn('signal-arms: guard failed for arm ' + index, error);
          return false;
        }
      }
      function play(arm) {
        var props = (arm.animations && arm.animations.properties) || [];
        props.forEach(function(p) {
          var kf = (p.keyframes || []).map(function(k) {
            var frame = { offset: k.at };
            frame[p.property] = k.value;
            return frame;
          });
          if (kf.length === 0) return;
          node.animate(kf, {
            duration: arm.duration || 300,
            easing: arm.easing || 'ease-out',
            fill: 'forwards'
          });
        });
      }
      // ST.watchChanges: subscribes in the scope the author's reads resolve
      // through, primes from the current value, and delivers only genuine
      // changes — exactly once each. See its doc comment for the two bugs the
      // hand-rolled version of this had.
      var unwatch = ST.watchChanges(node, sigHead, function(current, was) {
        prev = was;
        for (var i = 0; i < arms.length; i++) {
          if (matches(arms[i], current) && passesGuard(arms[i], i, current)) { play(arms[i]); break; }
        }
        prev = current;
      });
      if (ST.onCleanup) ST.onCleanup(node, function() { if (unwatch) unwatch(); });
    })();
  }
}
