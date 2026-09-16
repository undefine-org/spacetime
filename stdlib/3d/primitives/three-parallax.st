/// Three Parallax Primitive — pointer-driven camera sway.
///
/// Eases the stage camera toward a small offset derived from the pointer position
/// (normalized to the viewport), producing the subtle "the scene follows your
/// mouse" depth cue seen on 3D hero sections. The offset is applied around the
/// camera's initial position each frame and the camera re-aims at the origin, so
/// it composes cleanly with auto-spin objects. If @orbit is also present, orbit
/// owns the camera and parallax is skipped (they would fight).
///
/// @param &el        - the canvas hosting the @stage
/// @param strength   - max world-space offset at the screen edge (default 0.15)
/// @param ease       - smoothing factor per frame 0..1 (default 0.06)
///
/// @yields $active   - true once tracking

%primitive three-parallax(
  &el,
  strength: number = 0.15,
  ease: number = 0.06
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-parallax: no @stage on this canvas.'); return; }
    const { camera } = ctx;

    // Capture the camera's base position; parallax sways around it.
    const base = camera.position.clone();
    const baseDist = base.length();
    const S = %strength;
    const E = %ease;

    let targetX = 0, targetY = 0;
    const onMove = (e) => {
      targetX = (e.clientX / window.innerWidth) * 2 - 1;     // -1..1
      targetY = (e.clientY / window.innerHeight) * 2 - 1;    // -1..1
    };
    window.addEventListener('pointermove', onMove, { passive: true });

    let curX = 0, curY = 0;
    const unregister = ctx.registerFrame(() => {
      // If orbit controls took over the camera, don't fight them.
      if (canvas._stOrbit) return;
      curX += (targetX - curX) * E;
      curY += (targetY - curY) * E;
      camera.position.x = base.x + curX * S * baseDist;
      camera.position.y = base.y - curY * S * baseDist;
      camera.lookAt(0, 0, 0);
    });

    %yield true -> $active;

    %cleanup {
      window.removeEventListener('pointermove', onMove);
      if (unregister) unregister();
    }
  }

  %exports {
    $active: bool
  }
}
