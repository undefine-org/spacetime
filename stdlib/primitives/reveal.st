/// Kinetic Typography Reveal Primitive
///
/// Splits text into characters, words, or lines and animates them
/// with staggered reveals. Supports IntersectionObserver triggers,
/// custom keyframes, and configurable stagger patterns.
///
/// @param &el - The text element to split and animate
/// @param split - Split mode: "chars", "words", or "lines"
/// @param trigger - Animation trigger: "visible", "load", or "scroll"
/// @param stagger - Delay between each unit (ms)
/// @param staggerFrom - Stagger pattern: "first", "last", "center", "edges", "random"
/// @param duration - Animation duration per unit (ms)
/// @param threshold - IntersectionObserver threshold for visible trigger
/// @param easing - CSS easing function name
/// @param from - Reveal direction: "bottom", "top", "left", "right", "none"
/// @param distance - Travel distance from starting position
/// @param once - Whether to animate only once
/// @param keyframes - Optional compiled custom keyframes body (from `$body:keyframes`).
///   When present, drives each unit's WAAPI animation instead of the built-in
///   from/distance opacity+transform pair (gh-18).
///
/// @yields $revealProgress - Overall reveal progress (0-1)
/// @yields $isRevealed - Whether the reveal is complete

%primitive reveal-engine(
  &el,
  split: ("chars" | "words" | "lines" | "outline") = "chars",
  trigger: ("visible" | "load" | "scroll") = "visible",
  stagger: number = 30,
  staggerFrom: ("first" | "last" | "center" | "edges" | "random") = "first",
  duration: number = 600,
  threshold: number = 0.2,
  easing: string = "ease-out",
  from: ("bottom" | "top" | "left" | "right" | "none") = "bottom",
  distance: string = "100%",
  once: bool = true,
  keyframes: any = null
) {
  %emit js {
    const prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    let revealed = false;
    let observer = null;
    let resizeObserver = null;
    let units = [];
    let originalHTML = el.innerHTML;
    // IMMUTABLE SNAPSHOT of AUTHOR content, captured ONCE before any splitting.
    // splitText() must read from this, never from el.childNodes, so a second split
    // rebuilds from the SAME author source instead of re-snapshotting the generated
    // reveal tree and nesting it (gh-24). Idempotence makes the result independent
    // of observer ordering.
    const originalNodes = Array.from(el.childNodes);

    // Preserve accessibility: add sr-only span with full text
    const srSpan = document.createElement('span');
    srSpan.className = 'sr-only';
    srSpan.style.cssText = 'position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0';
    srSpan.textContent = el.textContent;
    const splitText = () => {
      const mode = %split;
      units = [];

      // Clear element and add sr-only text
      el.innerHTML = '';
      el.appendChild(srSpan);

      const wrapChar = (ch, container) => {
        const mask = document.createElement('span');
        mask.className = 'st-reveal-mask';
        mask.style.cssText = 'overflow:hidden;display:inline-block';
        mask.setAttribute('aria-hidden', 'true');

        const span = document.createElement('span');
        span.className = 'st-reveal-char';
        span.style.cssText = 'display:inline-block;will-change:transform,opacity';
        span.textContent = ch;

        mask.appendChild(span);
        container.appendChild(mask);
        units.push(span);
      };

      const wrapWord = (word, container) => {
        const mask = document.createElement('span');
        mask.className = 'st-reveal-mask';
        mask.style.cssText = 'overflow:hidden;display:inline-block';
        mask.setAttribute('aria-hidden', 'true');

        const span = document.createElement('span');
        span.className = 'st-reveal-word';
        span.style.cssText = 'display:inline-block;will-change:transform,opacity';
        span.textContent = word;

        mask.appendChild(span);
        container.appendChild(mask);
        units.push(span);
      };

      const splitChars = (text, container) => {
        for (const ch of text) {
          if (ch === ' ') {
            container.appendChild(document.createTextNode(' '));
          } else {
            wrapChar(ch, container);
          }
        }
      };

      const splitWords = (text, container) => {
        const parts = text.split(/(\s+)/);
        for (const part of parts) {
          if (!part) continue;
          if (/^\s+$/.test(part)) {
            container.appendChild(document.createTextNode(part));
          } else {
            wrapWord(part, container);
          }
        }
      };

      // Walk the original child nodes and rebuild animation units in place,
      // preserving element boundaries (<br>, <em>, <a>, etc.) and splitting only
      // text nodes into word/char spans.
      const walkNodes = (nodes, container, walkMode) => {
        for (const node of nodes) {
          if (node.nodeType === Node.TEXT_NODE) {
            const text = node.textContent;
            if (!text) continue;
            if (walkMode === 'chars') {
              splitChars(text, container);
            } else if (walkMode === 'words') {
              splitWords(text, container);
            } else {
              container.appendChild(document.createTextNode(text));
            }
          } else if (node.nodeType === Node.ELEMENT_NODE) {
            if (node.tagName === 'BR') {
              container.appendChild(node.cloneNode(true));
              continue;
            }
            const clone = node.cloneNode(false);
            walkNodes(node.childNodes, clone, walkMode);
            container.appendChild(clone);
          }
        }
      };

      if (mode === 'chars' || mode === 'words') {
        for (const node of originalNodes) {
          walkNodes([node], el, mode);
        }
      } else if (mode === 'lines') {
        // Visual line grouping. Preferred path: the vendored pretext engine
        // (present when the site `@import`s stdlib/text) computes line breaks
        // arithmetically — NO DOM reflow. Fallback: group by offsetTop, which
        // forces a synchronous layout per word. Progressive enhancement keeps
        // this core primitive free of a hard dependency on the opt-in module.
        const text = originalNodes.map(n => n.textContent).join('');
        let lineTexts = null;

        if (typeof pretext !== 'undefined' && el.clientWidth > 0) {
          const cs = getComputedStyle(el);
          const font = `${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize}/${cs.lineHeight} ${cs.fontFamily}`;
          const prepared = pretext.prepareWithSegments(text, font);
          const result = pretext.layoutWithLines(prepared, el.clientWidth, 1);
          lineTexts = result.lines.map(l => l.text);
        }

        if (!lineTexts) {
          // Fallback: offsetTop grouping (forces reflow). Preserve the original
          // inline structure in a temporary off-screen container so we can
          // measure where the layout wraps without losing markup.
          const temp = document.createElement('div');
          const cs = getComputedStyle(el);
          temp.style.cssText = 'position:absolute;visibility:hidden;';
          temp.style.width = el.clientWidth + 'px';
          temp.style.font = cs.font;
          temp.style.lineHeight = cs.lineHeight;
          temp.style.whiteSpace = cs.whiteSpace;
          temp.style.wordBreak = cs.wordBreak;
          temp.style.overflowWrap = cs.overflowWrap;
          temp.style.letterSpacing = cs.letterSpacing;
          temp.style.textTransform = cs.textTransform;

          for (const node of originalNodes) {
            walkNodes([node], temp, 'words');
          }
          document.body.appendChild(temp);

          const inlineNodes = [];
          const collectInline = (node) => {
            if (node.nodeType === Node.TEXT_NODE) {
              inlineNodes.push(node);
            } else if (node.nodeType === Node.ELEMENT_NODE) {
              if (node.classList.contains('st-reveal-word')) {
                inlineNodes.push(node);
              } else {
                for (const child of node.childNodes) {
                  collectInline(child);
                }
              }
            }
          };
          collectInline(temp);

          const grouped = [];
          let currentLine = [];
          let lastTop = -1;
          for (const node of inlineNodes) {
            const top = node.offsetTop;
            if (lastTop !== -1 && Math.abs(top - lastTop) > 2) {
              grouped.push(currentLine);
              currentLine = [];
            }
            currentLine.push(node);
            lastTop = top;
          }
          if (currentLine.length) grouped.push(currentLine);
          lineTexts = grouped.map(line => line.map(n => n.textContent).join(''));

          document.body.removeChild(temp);
        }

        // Rebuild with line masks, preserving the original element structure.
        const lineSpans = [];
        if (!lineTexts || lineTexts.length === 0) {
          const mask = document.createElement('span');
          mask.className = 'st-reveal-mask';
          mask.style.cssText = 'overflow:hidden;display:block';
          mask.setAttribute('aria-hidden', 'true');

          const lineSpan = document.createElement('span');
          lineSpan.className = 'st-reveal-line';
          lineSpan.style.cssText = 'display:block;will-change:transform,opacity';

          mask.appendChild(lineSpan);
          el.appendChild(mask);
          lineSpans.push(lineSpan);
          units.push(lineSpan);

          for (const node of originalNodes) {
            walkNodes([node], lineSpan, 'lines');
          }
          return;
        }

        for (const lineText of lineTexts) {
          const mask = document.createElement('span');
          mask.className = 'st-reveal-mask';
          mask.style.cssText = 'overflow:hidden;display:block';
          mask.setAttribute('aria-hidden', 'true');

          const lineSpan = document.createElement('span');
          lineSpan.className = 'st-reveal-line';
          lineSpan.style.cssText = 'display:block;will-change:transform,opacity';

          mask.appendChild(lineSpan);
          el.appendChild(mask);
          lineSpans.push(lineSpan);
          units.push(lineSpan);
        }

        let lineIdx = 0;
        let linePos = 0;

        const getLineClone = (ancestors, targetLineIdx) => {
          let parent = lineSpans[targetLineIdx];
          for (const ancestor of ancestors) {
            let clone = parent._cloneMap && parent._cloneMap.get(ancestor);
            if (!clone) {
              clone = ancestor.cloneNode(false);
              if (!parent._cloneMap) parent._cloneMap = new Map();
              parent._cloneMap.set(ancestor, clone);
              parent.appendChild(clone);
            }
            parent = clone;
          }
          return parent;
        };

        const processLines = (nodes, ancestors) => {
          for (const node of nodes) {
            if (node.nodeType === Node.TEXT_NODE) {
              let text = node.textContent;
              while (text.length > 0) {
                if (lineIdx >= lineTexts.length) {
                  getLineClone(ancestors, lineTexts.length - 1).appendChild(document.createTextNode(text));
                  text = '';
                  break;
                }
                const lineText = lineTexts[lineIdx];
                const remaining = lineText.length - linePos;
                if (remaining === 0) {
                  lineIdx++;
                  linePos = 0;
                  continue;
                }
                const take = Math.min(text.length, remaining);
                getLineClone(ancestors, lineIdx).appendChild(document.createTextNode(text.slice(0, take)));
                text = text.slice(take);
                linePos += take;
              }
            } else if (node.nodeType === Node.ELEMENT_NODE) {
              if (node.tagName === 'BR') {
                getLineClone(ancestors, lineIdx).appendChild(node.cloneNode(true));
              } else {
                processLines(node.childNodes, ancestors.concat(node));
              }
            }
          }
        };

        processLines(originalNodes, []);
      } else if (mode === 'outline') {
        // PLAN-150 W10: text as GEOMETRY. The text stays text (the sr-only span
        // above keeps it accessible); we add an SVG twin whose <text> renders
        // the SAME font as strokable geometry. `draw` animates the stroke
        // dash-offset (a stroke travelling along the letterforms); `fill` fades
        // the real fill in over the outline. No font parsing: an SVG <text>
        // element IS the glyph geometry, natively strokable.
        const text = originalNodes.map(n => n.textContent).join('');
        const cs = getComputedStyle(el);
        const svgNS = 'http://www.w3.org/2000/svg';
        const svg = document.createElementNS(svgNS, 'svg');
        svg.setAttribute('aria-hidden', 'true');
        svg.style.cssText = 'overflow:visible;display:block;width:100%;';
        const t = document.createElementNS(svgNS, 'text');
        t.setAttribute('x', '0');
        t.setAttribute('y', '0.8em');
        t.setAttribute('dominant-baseline', 'alphabetic');
        t.textContent = text;
        t.style.cssText =
          'font:' + cs.font + ';letter-spacing:' + cs.letterSpacing + ';' +
          'fill:' + cs.color + ';fill-opacity:0;' +
          'stroke:' + cs.color + ';stroke-width:2px;paint-order:stroke;';
        svg.appendChild(t);
        // Size the svg to the text box so it flows like the original.
        el.appendChild(svg);
        // Estimate a dash length from the glyph box: perimeter-ish, generous so
        // the whole run hides at full offset. Stored on the unit for the
        // keyframe mapping (draw 0..1 -> dashoffset dashLen..0).
        let dashLen = 1000;
        try {
          const bb = t.getBBox();
          dashLen = Math.max(100, (bb.width + bb.height) * 2.2);
          svg.setAttribute('viewBox', '0 0 ' + Math.max(1, bb.width) + ' ' + Math.max(1, bb.height + bb.y));
          svg.setAttribute('width', Math.max(1, bb.width));
          svg.setAttribute('height', Math.max(1, bb.height + bb.y));
        } catch (_e) {}
        t._stDashLen = dashLen;
        t.style.strokeDasharray = dashLen + 'px';
        t.style.strokeDashoffset = dashLen + 'px';  // start fully hidden (draw=0)
        // The animated unit is the <text>: `draw` and `fill` are custom props on
        // it, driven by the ordinary keyframe engine (the @on &.clip body sets
        // `draw: 0 -> 1; fill: 0 -> 1 …`, mapped to --st-reveal-draw/-fill).
        t._stRevealOutline = true;
        units.push(t);
      }
    };
    // Headless-test hook: expose the split entry so a logic-rung gate can drive a
    // SECOND split directly (no ResizeObserver exists at the logic rung) and prove
    // idempotence — the unit count and textContent must not multiply (gh-24).
    el._revealSplit = splitText;
    // Calculate stagger order indices
    const getStaggerOrder = (count) => {
      const indices = Array.from({ length: count }, (_, i) => i);
      const pattern = %staggerFrom;

      switch (pattern) {
        case 'last':
          return indices.reverse();
        case 'center': {
          const mid = Math.floor(count / 2);
          return indices.sort((a, b) => Math.abs(a - mid) - Math.abs(b - mid));
        }
        case 'edges': {
          const mid = Math.floor(count / 2);
          return indices.sort((a, b) => Math.abs(b - mid) - Math.abs(a - mid));
        }
        case 'random':
          for (let i = count - 1; i > 0; i--) {
            const j = Math.floor(Math.random() * (i + 1));
            [indices[i], indices[j]] = [indices[j], indices[i]];
          }
          return indices;
        default: // 'first'
          return indices;
      }
    };

    // Get initial transform based on from direction
    const getFromTransform = () => {
      const dist = %distance;
      switch (%from) {
        case 'bottom': return 'translateY(' + dist + ')';
        case 'top':    return 'translateY(-' + dist + ')';
        case 'left':   return 'translateX(-' + dist + ')';
        case 'right':  return 'translateX(' + dist + ')';
        default:       return 'none';
      }
    };

    // Set initial hidden state
    const setInitialState = () => {
      const transform = getFromTransform();
      for (const unit of units) {
        // An OUTLINE unit (SVG <text> twin) is NEVER opacity-hidden: it manages
        // its own visibility through stroke dash-offset (draw) and fill-opacity.
        // Zeroing its opacity here would hide the whole reveal permanently
        // (the outline tick never restores opacity) — the letterforms would
        // draw on stroke-first while the element stayed transparent.
        if (unit._stRevealOutline) continue;
        unit.style.opacity = '0';
        if (transform !== 'none') {
          unit.style.transform = transform;
        }
      }
    };

    // Custom keyframes body (gh-18): `$body:keyframes` compiled into the same
    // { properties, scopes } structure apply-animations consumes. The reveal body
    // is typically `& { prop: a -> b; ... }` — a `&` scope whose properties drive
    // each unit's WAAPI animation. Falls back to the built-in from/distance pair.
    const customKeyframes = (function () {
      const data = %keyframes;
      if (!data) return null;
      let props = data.properties || [];
      const scopes = data.scopes || [];
      if (scopes.length > 0) {
        const self = scopes.find(s => s.selector === '&' || s.selector === '&self');
        props = (self ? self.properties : scopes[0].properties) || [];
      }
      if (!props.length) return null;
      const offsets = [];
      props.forEach(p => (p.keyframes || []).forEach(k => {
        if (offsets.indexOf(k.at) === -1) offsets.push(k.at);
      }));
      offsets.sort((a, b) => a - b);
      return offsets.map(at => {
        const frame = { offset: at };
        props.forEach(p => {
          const kf = (p.keyframes || []).find(k => k.at === at);
          if (kf) frame[p.property] = kf.value;
        });
        return frame;
      });
    })();
    // Headless-test hook: expose the resolved custom keyframes so a logic-rung
    // gate can assert the author's keyframe property reached the ENGINE (gh-18).
    // The WAAPI shim exposes no effect.getKeyframes(), so this is the headless
    // proof that `$body:keyframes` was forwarded through %binds; the --cdp gate
    // asserts the actual WAAPI keyframes.
    el._revealKeyframes = customKeyframes;

    // Animate reveal
    const reveal = () => {
      if (revealed && %once) return;
      revealed = true;

      const order = getStaggerOrder(units.length);
      // `%stagger` is a `stagger_value` — an OBJECT `{ delay, from, grid }`
      // (spacetime-values.st, the apply-animations contract). Reading the
      // delay out of it is not optional: multiplying the object by a number
      // gives NaN, WAAPI throws on the non-finite delay, and every reveal on
      // the page dies in init (BUG-354). The bare-number path stays for any
      // older emission.
      const staggerData = %stagger;
      const staggerMs = (typeof staggerData === 'object' && staggerData !== null)
        ? (staggerData.delay || 0)
        : staggerData;
      const durationMs = %duration;
      let completed = 0;

      if (prefersReduced) {
        // Instant reveal
        for (const unit of units) {
          unit.style.opacity = '1';
          unit.style.transform = '';
        }
        %yield 1 -> $revealProgress;
        %yield true -> $isRevealed;
        return;
      }

      for (let i = 0; i < order.length; i++) {
        const unitIndex = order[i];
        const unit = units[unitIndex];
        const delay = i * staggerMs;

        const fromTransform = getFromTransform();
        let keyframes = customKeyframes || (fromTransform !== 'none'
          ? [
              { opacity: 0, transform: fromTransform },
              { opacity: 1, transform: 'translateX(0) translateY(0)' }
            ]
          : [
              { opacity: 0 },
              { opacity: 1 }
            ]);

        // PLAN-150 W10: for an OUTLINE unit (an SVG <text> twin), the author's
        // `draw`/`fill` keyframe properties map to the custom properties the
        // twin reads (`--st-reveal-draw` = stroke dash progress, `--st-reveal-
        // fill` = fill-opacity). WAAPI animates custom properties as strings.
        if (unit._stRevealOutline) {
          // An OUTLINE unit draws its stroke on (dashoffset dashLen -> 0) and
          // fades its fill in (fill-opacity 0 -> 1) over the reveal window. When
          // the author supplied a custom body its `draw`/`fill` stops set the
          // endpoints and positions; otherwise the default is a full draw. The
          // draw is a real SVG presentation prop, so WAAPI interpolates it and
          // getComputedStyle reflects it — no custom-property registration.
          const dashLen = unit._stDashLen || 1;
          const drawStops = [];
          const fillStops = [];
          (customKeyframes || []).forEach(function (frame) {
            if (frame.draw !== undefined) drawStops.push({ at: frame.offset, v: parseFloat(frame.draw) });
            if (frame.fill !== undefined) fillStops.push({ at: frame.offset, v: parseFloat(frame.fill) });
          });
          if (!drawStops.length) { drawStops.push({ at: 0, v: 0 }, { at: 1, v: 1 }); }
          if (!fillStops.length) { fillStops.push({ at: 0, v: 0 }, { at: 1, v: 1 }); }
          // Merge onto a common offset set so one WAAPI keyframe list carries
          // both props.
          const offs = {};
          drawStops.concat(fillStops).forEach(function (s) { offs[s.at] = true; });
          const sortedOffs = Object.keys(offs).map(Number).sort(function (a, b) { return a - b; });
          const at = function (stops, o) {
            // piecewise-linear sample of the stops at offset o
            if (o <= stops[0].at) return stops[0].v;
            if (o >= stops[stops.length - 1].at) return stops[stops.length - 1].v;
            for (var i = 0; i < stops.length - 1; i++) {
              if (o >= stops[i].at && o <= stops[i + 1].at) {
                var t = (stops[i + 1].at - stops[i].at) > 0 ? (o - stops[i].at) / (stops[i + 1].at - stops[i].at) : 0;
                return stops[i].v + (stops[i + 1].v - stops[i].v) * t;
              }
            }
            return stops[stops.length - 1].v;
          };
          // Drive the outline via a rAF loop rather than WAAPI: WAAPI on an SVG
          // element's `stroke-dashoffset` does not reliably reflect through
          // getComputedStyle, so we set the inline style each frame (which does).
          // Deterministic, and the same interpolation the WAAPI path would use.
          const outlineDur = durationMs;
          // Start is captured LAZILY from the first rAF timestamp, not from
          // performance.now(): under the render backend's VIRTUAL clock the tick
          // `now` is ST._clock.now (virtual), while performance.now() is frozen
          // wall time — mixing them makes `p` permanently negative and the stroke
          // never draws (the whole outline stays hidden under render). Reading
          // start from the same clock the ticks arrive on makes the draw run in
          // both real and virtual time.
          let outlineStart = null;
          const raf = (typeof requestAnimationFrame !== 'undefined')
            ? requestAnimationFrame : function (f) { return setTimeout(function () { f(Date.now()); }, 16); };
          const tick = function (now) {
            if (outlineStart === null) outlineStart = now + delay;
            const p = Math.max(0, Math.min(1, (now - outlineStart) / outlineDur));
            unit.style.strokeDashoffset = ((1 - at(drawStops, p)) * dashLen) + 'px';
            unit.style.fillOpacity = String(at(fillStops, p));
            %yield p -> $revealProgress;
            if (p < 1) { raf(tick); }
            else { completed++; if (completed === units.length) { %yield true -> $isRevealed; } }
          };
          raf(tick);
          continue;
        }

        const anim = unit.animate(keyframes, {
          duration: durationMs,
          delay: delay,
          easing: %easing,
          fill: 'forwards'
        });

        anim.onfinish = () => {
          completed++;
          %yield completed / units.length -> $revealProgress;
          if (completed === units.length) {
            %yield true -> $isRevealed;
          }
        };
      }
    };

    // Split the text
    splitText();
    setInitialState();

    // Initial signals
    %yield 0 -> $revealProgress;
    %yield false -> $isRevealed;

    // Set up trigger
    const triggerMode = %trigger;

    if (triggerMode === 'load') {
      reveal();
    } else if (triggerMode === 'visible') {
      observer = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (entry.isIntersecting) {
              reveal();
              if (%once && observer) {
                observer.disconnect();
                observer = null;
              }
            }
          }
        },
        { threshold: %threshold }
      );
      observer.observe(el);
    }

    // ResizeObserver for lines mode to re-split on resize
    if (%split === 'lines') {
      resizeObserver = new ResizeObserver(() => {
        if (!revealed) {
          splitText();
          setInitialState();
        }
      });
      resizeObserver.observe(el);
    }

    %cleanup {
      if (observer) observer.disconnect();
      if (resizeObserver) resizeObserver.disconnect();
      el.innerHTML = originalHTML;
    }
  }

  %exports {
    $revealProgress: number  // overall reveal progress (0-1)
    $isRevealed: bool        // whether the reveal animation is complete
  }
}
