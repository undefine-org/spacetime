/// FLIP transition for server-driven signal writes.
///
/// `signal-subscribe` emits `signal:<name>:before-write` immediately before its
/// established SpacetimeLocal write. This primitive captures the rendered rect,
/// lets the reactive write settle, and uses the existing rAF clock to interpolate
/// the visual delta back to the new layout position. A write during motion takes
/// its first measurement from the current rendered position, then cancels the
/// old frame loop: it retargets instead of snapping back to an old origin.
%primitive signal-flip(
  &el,
  signal: binding,
  duration: time = 300ms,
  easing: string = "ease-out"
) {
  %emit js {
    (function() {
      var node = %&el;
      var signal = String(%signal || '').replace(/^\$/, '');
      var duration = Number(%duration) || 300;
      var easingName = %easing || 'ease-out';
      var baseTransform = node.style.transform || '';
      var frame = null;
      var start = 0;
      var fromX = 0;
      var fromY = 0;

      function ease(t) {
        if (easingName === 'linear') return t;
        // Cubic ease-out keeps the authored surface compact while retaining a
        // deterministic curve under ST._clock's virtual rAF driver.
        return 1 - Math.pow(1 - t, 3);
      }

      function translate(x, y) {
        var move = 'translate(' + x + 'px, ' + y + 'px)';
        node.style.transform = baseTransform ? move + ' ' + baseTransform : move;
      }

      // ST._raf is the shared animation clock: @clock advances it in tests and
      // production uses its single coordinated requestAnimationFrame loop.
      function stop() {
        if (frame != null) frame();
        frame = null;
      }

      function once(callback) {
        var remove = ST._raf.add(function(timestamp) {
          remove();
          callback(timestamp);
        });
        return remove;
      }

      function animate(timestamp) {
        if (!start) start = timestamp;
        var progress = Math.min(1, (timestamp - start) / duration);
        var remaining = 1 - ease(progress);
        translate(fromX * remaining, fromY * remaining);
        if (progress >= 1) { stop(); node.style.transform = baseTransform; }
      }

      function onWrite() {
        // getBoundingClientRect includes our current transform, yielding the
        // visual position even if this write interrupts a prior transition.
        var previous = node.getBoundingClientRect();
        stop();
        node.style.transform = baseTransform;

        // Reactive bindings settle before the next frame. Measuring there gives
        // the post-write layout rect without relying on wall time or WAAPI.
        frame = once(function() {
          var next = node.getBoundingClientRect();
          fromX = previous.left - next.left;
          fromY = previous.top - next.top;
          if (!fromX && !fromY) { frame = null; return; }
          translate(fromX, fromY);
          start = 0;
          frame = ST._raf.add(animate);
        });
      }
      document.addEventListener('signal:' + signal + ':before-write', onWrite);
      %cleanup {
        document.removeEventListener('signal:' + signal + ':before-write', onWrite);
        stop();
      }
    })();
  }
}
