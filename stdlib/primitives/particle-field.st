/// Particle Field Primitive
///
/// Creates an SVG overlay with particles that drift organically and repel
/// from the cursor. Inter-particle soft repulsion prevents clumping.
/// Designed for background ambient effects on dark sections.
///
/// @param &el - Container element (SVG overlay created inside)
/// @param count - Number of particles (default 40)
/// @param repulsion - Cursor flee intensity 0-1 (default 0.6)
/// @param spread - Inter-particle spacing force 0-1 (default 0.5)
/// @param drift - Ambient movement energy 0-1 (default 0.3)
/// @param color - Base particle color (default "rgba(255,255,255,0.08)")
///
/// @yields $particleCount - Actual particle count
/// @yields $destroy - Cleanup function

%primitive particle-field(
  &el,
  count: number = 40,
  repulsion: number = 0.6,
  spread: number = 0.5,
  drift: number = 0.3,
  color: string = "rgba(255,255,255,0.08)"
) {
  %emit js {
    // Touch detection + reduced motion
    const isTouch = window.matchMedia('(pointer: coarse)').matches;
    const prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    // Create SVG overlay
    const svgNS = 'http://www.w3.org/2000/svg';
    const svg = document.createElementNS(svgNS, 'svg');
    svg.setAttribute('width', '100%');
    svg.setAttribute('height', '100%');
    svg.style.cssText = 'position:absolute;inset:0;pointer-events:none;overflow:hidden;';
    svg.setAttribute('aria-hidden', 'true');
    if (!getComputedStyle(el).position || getComputedStyle(el).position === 'static') el.style.position = 'relative';
    el.appendChild(svg);

    const rect = el.getBoundingClientRect();
    const w = rect.width || 800;
    const h = rect.height || 600;
    svg.setAttribute('viewBox', '0 0 ' + w + ' ' + h);

    // Particle state arrays (SoA for cache efficiency)
    const n = %count;
    const px = new Float32Array(n);
    const py = new Float32Array(n);
    const vx = new Float32Array(n);
    const vy = new Float32Array(n);
    const sizes = new Float32Array(n);
    const circles = [];

    // Initialize particles
    const baseColor = '%color';
    for (let i = 0; i < n; i++) {
      px[i] = Math.random() * w;
      py[i] = Math.random() * h;
      const driftScale = %drift;
      vx[i] = (Math.random() - 0.5) * driftScale * 1.5;
      vy[i] = (Math.random() - 0.5) * driftScale * 1.5;
      sizes[i] = 2 + Math.random() * 3;

      const circle = document.createElementNS(svgNS, 'circle');
      circle.setAttribute('r', sizes[i]);
      circle.setAttribute('fill', baseColor);
      circle.style.opacity = 0.5 + Math.random() * 0.5;
      circle.setAttribute('cx', px[i]);
      circle.setAttribute('cy', py[i]);
      svg.appendChild(circle);
      circles.push(circle);
    }

    %yield n -> $particleCount;

    if (prefersReduced) {
      // Static constellation — no animation
      const destroy = () => { svg.remove(); };
      %yield destroy -> $destroy;
      ST.onCleanup(el, destroy);
    } else {
      // Mouse tracking
      let mouseX = -9999, mouseY = -9999;
      let mouseOver = false;
      const controller = new AbortController();

      if (!isTouch) {
        el.addEventListener('pointermove', (e) => {
          const r = el.getBoundingClientRect();
          mouseX = e.clientX - r.left;
          mouseY = e.clientY - r.top;
          mouseOver = true;
        }, { signal: controller.signal, passive: true });

        el.addEventListener('pointerleave', () => {
          mouseOver = false;
        }, { signal: controller.signal, passive: true });
      }

      // Physics constants derived from normalised params
      const repulsionRadius = 150;
      const repulsionStrength = %repulsion * 1200;
      const spreadRadius = 40;
      const spreadStrength = %spread * 0.6;
      const damping = 0.998;
      const nudgeInterval = 120;
      const driftEnergy = %drift * 0.15;
      let frame = 0;

      let animId = null;
      const animate = () => {
        frame++;

        // Resize check (every 60 frames)
        let cw = w, ch = h;
        if (frame % 60 === 0) {
          const r = el.getBoundingClientRect();
          if (r.width > 0 && r.height > 0) {
            cw = r.width;
            ch = r.height;
            svg.setAttribute('viewBox', '0 0 ' + cw + ' ' + ch);
          }
        }

        for (let i = 0; i < n; i++) {
          // 1. Ambient drift — damping + random nudge
          vx[i] *= damping;
          vy[i] *= damping;

          if (frame % nudgeInterval === 0) {
            vx[i] += (Math.random() - 0.5) * driftEnergy;
            vy[i] += (Math.random() - 0.5) * driftEnergy;
          }

          // 2. Cursor repulsion (desktop only)
          if (mouseOver && !isTouch) {
            const dx = px[i] - mouseX;
            const dy = py[i] - mouseY;
            const d2 = dx * dx + dy * dy;
            const rr = repulsionRadius * repulsionRadius;
            if (d2 < rr && d2 > 1) {
              const d = Math.sqrt(d2);
              const force = repulsionStrength / (d2 + 100);
              vx[i] += (dx / d) * force * 0.016;
              vy[i] += (dy / d) * force * 0.016;
            }
          }

          // 3. Inter-particle repulsion (O(n²) — fine for n≤60)
          for (let j = i + 1; j < n; j++) {
            const dx = px[i] - px[j];
            const dy = py[i] - py[j];
            const d2 = dx * dx + dy * dy;
            if (d2 < spreadRadius * spreadRadius && d2 > 1) {
              const d = Math.sqrt(d2);
              const force = (spreadRadius - d) * spreadStrength * 0.01;
              const fx = (dx / d) * force;
              const fy = (dy / d) * force;
              vx[i] += fx;
              vy[i] += fy;
              vx[j] -= fx;
              vy[j] -= fy;
            }
          }

          // Apply velocity
          px[i] += vx[i];
          py[i] += vy[i];

          // 4. Boundary wrap (toroidal)
          if (px[i] < -20) px[i] += cw + 40;
          else if (px[i] > cw + 20) px[i] -= cw + 40;
          if (py[i] < -20) py[i] += ch + 40;
          else if (py[i] > ch + 20) py[i] -= ch + 40;
        }

        // Batch SVG update
        for (let i = 0; i < n; i++) {
          circles[i].setAttribute('cx', px[i]);
          circles[i].setAttribute('cy', py[i]);
        }

        animId = requestAnimationFrame(animate);
      };

      animId = requestAnimationFrame(animate);

      const destroy = () => {
        controller.abort();
        if (animId) cancelAnimationFrame(animId);
        svg.remove();
      };

      %yield destroy -> $destroy;

      %cleanup {
        controller.abort();
        if (animId) cancelAnimationFrame(animId);
        svg.remove();
      }
    }
  }

  %exports {
    $particleCount: number
    $destroy: fn() ~> effect
  }
}
