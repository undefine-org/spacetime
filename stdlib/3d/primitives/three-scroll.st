/// Three Scroll Primitive — drive the stage from page scroll.
///
/// Maps normalized page-scroll progress (0..1 over the scrollable height, or over
/// a named container) onto one stage target, eased for smoothness:
///
///   drive: "morph"  → canvas._stParticleMorph = progress * to   (particle field)
///   drive: "dollyZ" → camera.position.z = from + (to-from)*progress (fly toward/along)
///   drive: "spinY"  → most-recent object's rotation.y = progress * to
///   drive: "camY"   → camera.position.y = from + (to-from)*progress
///
/// Scroll work piggybacks the stage frame loop (eased toward the latest scroll
/// sample) so it never fights rAF or stutters. One scroll listener, passive.
///
/// @param &el      - the canvas hosting the @stage
/// @param drive    - "morph"|"dollyZ"|"spinY"|"camY"
/// @param from     - value at progress 0 (for dollyZ/camY)
/// @param to       - value at progress 1
/// @param ease     - per-frame smoothing 0..1 (default 0.08)
///
/// @yields $progress - eased scroll progress 0..1

%primitive three-scroll(
  &el,
  drive: string = "morph",
  from: number = 0,
  to: number = 1,
  ease: number = 0.08
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-scroll: no @stage on this canvas.'); return; }
    const { camera } = ctx;

    const drive = %drive, from = %from, to = %to, E = %ease;

    const sample = () => {
      const max = document.documentElement.scrollHeight - window.innerHeight;
      const y = window.scrollY || window.pageYOffset || 0;
      return max > 0 ? Math.max(0, Math.min(1, y / max)) : 0;
    };
    let target = sample();
    const onScroll = () => { target = sample(); };
    window.addEventListener('scroll', onScroll, { passive: true });

    let cur = target;
    const unregister = ctx.registerFrame(() => {
      cur += (target - cur) * E;
      const v = from + (to - from) * cur;
      switch (drive) {
        case 'dollyZ': camera.position.z = v; camera.lookAt(0, 0, 0); break;
        case 'camY':   camera.position.y = v; camera.lookAt(0, 0, 0); break;
        case 'spinY':  if (canvas._stObject) canvas._stObject.rotation.y = cur * to; break;
        case 'morph':
        default:       canvas._stParticleMorph = cur * to; break;
      }
      %yield cur -> $progress;
    });

    %cleanup {
      window.removeEventListener('scroll', onScroll);
      if (unregister) unregister();
    }
  }

  %exports {
    $progress: number
  }
}
