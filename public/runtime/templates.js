/**
 * Spacetime Template Registry
 *
 * Provides a global registry for template factory functions.
 * Templates are defined with @template and compiled to factory functions
 * that create DOM elements from data.
 *
 * Usage:
 *   // Registering (done by compiler)
 *   Spacetime.templates.set('card', ($item) => {
 *     const el = document.createElement('div');
 *     el.className = 'card';
 *     el.innerHTML = `<h3>${$item.title}</h3>`;
 *     return el;
 *   });
 *
 *   // Invoking (done by @each or manual)
 *   const factory = Spacetime.templates.get('card');
 *   const element = factory({ title: 'Hello' });
 *   container.appendChild(element);
 *
 * ## Optional Parameters (`?` modifier)
 *
 * Templates support optional parameters marked with `?` suffix.
 * Optional params allow flexible template invocation where some arguments
 * can be omitted.
 *
 * ### Value parameters ($param?)
 * When a `$param?` is not provided:
 * - The parameter value is `undefined`
 * - In HTML interpolation, undefined renders as empty string
 * - Use OR (||) or nullish coalescing (??) for default values
 *
 * Example:
 *   @template &greeting($name, $title?) { ... }
 *   &greeting("Alice")        // $title is undefined
 *   &greeting("Alice", "Dr.") // $title is "Dr."
 *
 * ### Element parameters (&param?)
 * When an `&param?` is not provided:
 * - The parameter value is `null`
 * - Use @if(&param) for conditional rendering
 *
 * Example:
 *   @template &modal($title, &footer?) { ... }
 *   &modal("Title")                    // &footer is null
 *   &modal("Title", <button>OK</button>) // &footer is the button
 */

(function() {
  'use strict';

  // Initialize Spacetime namespace
  window.Spacetime = window.Spacetime || {};

  /**
   * Template registry - Map of template name to factory function.
   * Factory functions take data and return DOM elements.
   *
   * IDEMPOTENT (BUG-211's class, missed for this registry): a page can load
   * SEVERAL runtime bundles — each dev-dock widget (migrations, inspector, host)
   * embeds a full copy, and they share one `Spacetime` global. Resetting the map
   * here meant the LAST bundle to load wiped every template the earlier ones had
   * registered, so their `@each ... { &row($x) }` lists silently rendered nothing
   * (the ref factory is simply absent). Reuse the existing registry instead: a
   * later bundle adds its templates, it does not erase its neighbours'.
   */
  Spacetime.templates = Spacetime.templates || new Map();

  /**
   * OWNER-SCOPED TEMPLATE IDENTITY (FUP-134).
   *
   * A dev-served page loads SEVERAL runtime bundles into one `Spacetime` global:
   * the page's own, plus each dock widget (inspector, host, migrations) and the
   * shared `stdlib/fields` widgets they pull in. The registry is keyed by the bare
   * template name, and `Map.set` replaces — so the LAST bundle to load owned the
   * name outright.
   *
   * Observed, not theorised: a page declaring `@template &field-text($x)` lost it
   * entirely to the fields dock. `Spacetime.templateNames()` showed one
   * `"field-text"`, invoking it returned the DOCK's widget, and the page's own
   * `@each { &field-text($i) }` rendered NOTHING. No console error — the lookup
   * succeeded, it just answered with someone else's factory.
   *
   * The mechanism to fix it already existed for region mounts (FEAT-126): key by
   * `owner/name` and resolve owner-first, then fall back to the bare name. This
   * generalises that rail from regions to bundles rather than adding a second
   * one — same key shape, same local-first policy, one resolver.
   *
   * Policy:
   *   - A bundle with an owner registers `owner/name` AND, if the bare name is
   *     free, the bare name too. The bare registration is what keeps a dock
   *     widget reachable from a page that legitimately wants it (the shared
   *     `field-*` widgets are used exactly this way) — but it can no longer
   *     DISPLACE an existing owner, because it is skipped when occupied.
   *   - A bundle with NO owner (an ordinary page) registers the bare name and
   *     takes precedence: the page is the thing the user is looking at.
   *   - Lookup from inside an owned bundle tries `owner/name` first, so a dock
   *     widget always gets its OWN template even when a page has taken the bare
   *     name.
   */
  Spacetime._templateOwnerOf = function(key) {
    const i = key.indexOf('/');
    return i > 0 ? key.slice(0, i) : null;
  };

  /**
   * Register a factory under an owner, without stealing an occupied bare name.
   * Returns the keys written, for diagnostics.
   */
  Spacetime.registerOwnedTemplate = function(owner, name, factory) {
    const written = [];
    if (owner) {
      const scoped = owner + '/' + name;
      Spacetime.templates.set(scoped, factory);
      written.push(scoped);
      // Claim the bare name ONLY if free. This is the whole fix: a later dock
      // bundle can still be found by bare name, but cannot evict a page.
      if (!Spacetime.templates.has(name)) {
        Spacetime.templates.set(name, factory);
        written.push(name);
      }
    } else {
      // An unowned (page) registration wins the bare name outright, replacing a
      // dock bundle's opportunistic claim from step above.
      Spacetime.templates.set(name, factory);
      written.push(name);
    }
    return written;
  };

  /**
   * Resolve a template for a caller in `owner`: owner-scoped first, then bare.
   * Mirrors `resolveRefFactory`'s region-first policy so there is ONE rule.
   */
  Spacetime.resolveTemplate = function(name, owner) {
    if (owner) {
      const scoped = Spacetime.templates.get(owner + '/' + name);
      if (scoped) return scoped;
    }
    return Spacetime.templates.get(name)
        || (window.__pendingTemplates && window.__pendingTemplates[name]);
  };

  /**
   * Register a template factory function.
   * @param {string} name - Template name (without &)
   * @param {Function} factory - Factory function that takes data and returns element
   */
  Spacetime.registerTemplate = function(name, factory) {
    if (typeof factory !== 'function') {
      console.error(`Spacetime: Template "${name}" factory must be a function`);
      return;
    }
    Spacetime.templates.set(name, factory);
  };

  /**
   * Get a template factory by name.
   * @param {string} name - Template name
   * @returns {Function|undefined} Factory function or undefined if not found
   */
  Spacetime.getTemplate = function(name) {
    return Spacetime.templates.get(name);
  };

  /**
   * Invoke a template with data to create an element.
   * @param {string} name - Template name
   * @param {...any} args - Arguments to pass to the factory
   * @returns {Element|null} Created element or null if template not found
   */
  Spacetime.invokeTemplate = function(name, ...args) {
    const factory = Spacetime.templates.get(name);
    if (!factory) {
      console.error(`Spacetime: Template "${name}" not found`);
      return null;
    }
    try {
      return factory(...args);
    } catch (err) {
      console.error(`Spacetime: Error invoking template "${name}":`, err);
      return null;
    }
  };

  /**
   * Create an element from HTML string.
   * Helper for template factories.
   * @param {string} html - HTML string
   * @returns {Element} Created element
   */
  Spacetime.createElement = function(html) {
    const template = document.createElement('template');
    template.innerHTML = html.trim();
    return template.content.firstElementChild;
  };

  /**
   * Escape HTML for text content (XSS protection).
   * @param {string} str - String to escape
   * @returns {string} Escaped string
   */
  Spacetime.escapeHtml = function(str) {
    if (str == null) return '';
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  };

  /**
   * Escape for attribute values (quotes for attribute context).
   * @param {string} str - String to escape
   * @returns {string} Escaped string
   */
  Spacetime.escapeAttr = function(str) {
    if (str == null) return '';
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/"/g, '&quot;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  };

  /**
   * Interpolate values into an HTML string with context-aware escaping.
   * Handles $param.property syntax.
   * - Values in attributes (e.g., alt="$value") are escaped for attribute context
   * - Values in text content (e.g., <span>$value</span>) are escaped for HTML
   * @param {string} html - HTML template string
   * @param {Object} data - Data object
   * @returns {string} Interpolated HTML
   */
  Spacetime.interpolateHtml = function(html, data) {
    // `$$` is a LITERAL dollar — stashed before the passes so a template that
    // DISPLAYS source as text (`.clock { text <- $now; }`) keeps it. The same
    // escape the emit tokenizer (BUG-112) and the compile-time builder
    // (normalize_bare_holes) honor; restored after interpolation.
    html = String(html).replace(/\$\$/g, '\u0000');
    // Helper to resolve variable path
    const resolveValue = (varName, path) => {
      let value = data[varName];
      if (path && value != null) {
        for (const part of path.split('.')) {
          if (value == null) return '';
          value = value[part];
        }
      }
      return value ?? '';
    };

    // First pass: handle attribute contexts (="$value" or ='$value')
    html = html.replace(/(\w+)="([^"]*\$[\w.]+[^"]*)"/g, (match, attrName, attrValue) => {
      const interpolated = attrValue.replace(/\$(\w+)(?:\.(\w+(?:\.\w+)*))?/g, (m, varName, path) => {
        return Spacetime.escapeAttr(resolveValue(varName, path));
      });
      return `${attrName}="${interpolated}"`;
    });

    // PLAN-150 W9: a backtick-wrapped hole `` `$title` `` is the Spacetime hole
    // form; strip the wrapping backticks as the value is interpolated so a shot
    // (or any register-template body) that writes `` `$title` `` renders the
    // value, not `` `Ceramic Mug` ``. Runs before the bare-hole pass so the
    // backticks are gone by the time the general text pass sees the content.
    html = html.replace(/`\$(\w+)(?:\.(\w+(?:\.\w+)*))?`/g, (m, varName, path) => {
      // BUG-384: stash a `$` inside the resolved value so the bare-hole pass
      // below cannot re-read a value like "$24.99" as a `$24` variable and wipe
      // it. The trailing \u0000 -> `$` restore returns it to display text.
      return Spacetime.escapeHtml(resolveValue(varName, path)).replace(/\$/g, '\u0000');
    });

    // Second pass: handle remaining text content (>...$value...<)
    html = html.replace(/>([^<]*\$[\w.]+[^<]*)</g, (match, textContent) => {
      const interpolated = textContent.replace(/\$(\w+)(?:\.(\w+(?:\.\w+)*))?/g, (m, varName, path) => {
        return Spacetime.escapeHtml(resolveValue(varName, path));
      });
      return `>${interpolated}<`;
    });

    return html.replace(/\u0000/g, '$');
  };

  /**
   * Check if a template exists.
   * @param {string} name - Template name
   * @returns {boolean} True if template exists
   */
  Spacetime.hasTemplate = function(name) {
    return Spacetime.templates.has(name);
  };

  /**
   * Get all registered template names.
   * @returns {string[]} Array of template names
   */
  Spacetime.templateNames = function() {
    return Array.from(Spacetime.templates.keys());
  };

  /**
   * Clear all templates (useful for testing).
   */
  Spacetime.clearTemplates = function() {
    Spacetime.templates.clear();
  };

  /**
   * Build a template factory from a body payload (FEAT-126).
   *
   * This is the SHARED factory-construction path used by BOTH the page-load
   * `register-template` primitive (stdlib/primitives/template.st) and the
   * region-mount `registerBundle` primitive (stdlib/__mcp__/primitives/bundle.st).
   * One implementation: the body payload `{html, builder, states, exports, refs,
   * matches}` — produced by `component_body_to_js_from_scope` at compile time and
   * carried verbatim by a compiled Bundle — is turned into the factory that
   * `Spacetime.templates.set(name, factory)` registers. Extracting it here keeps
   * the two registration entry points in lockstep (no parallel factory builders).
   *
   * @param {string} rawName - Template name, with or without a leading `&`.
   * @param {object|string} rawBody - The body payload (structured object or html string).
   * @param {Array} rawParamsIn - Param specs (structured objects or legacy `$x`/`&x` strings).
   * @param {*} templateAnimations - Scoped keyframes, or null.
   * @returns {{name: string, factory: Function}|null} `{name, factory}` or null on bad input.
   */
  // PLAN-150 W9: play a template's scoped keyframes as a ONE-SHOT mount reveal.
  // A `@form shot`'s `:motion` slot (and any `@template` `animations:`) is the
  // `{properties:[{property, keyframes:[{at,value}]}]}` shape apply-animations
  // uses; here it is driven over a default duration on mount, so an
  // instantiated shot reveals itself. Reuses ST's easing/interp where present.
  Spacetime.applyAnimations = Spacetime.applyAnimations || function(root, anims) {
    if (!root || !anims) return;
    var props = anims.properties || (Array.isArray(anims) ? anims : []);
    if (!props.length) return;
    var dur = (anims.duration && parseFloat(anims.duration)) || 600;
    var transformKeys = ['translateX','translateY','translateZ','translate','rotate','rotateX','rotateY','rotateZ','scale','scaleX','scaleY','scaleZ','skew','skewX','skewY'];
    var interp = function(kfs, p) {
      if (!kfs || !kfs.length) return null;
      if (p <= kfs[0].at) return kfs[0].value;
      if (p >= kfs[kfs.length - 1].at) return kfs[kfs.length - 1].value;
      for (var i = 0; i < kfs.length - 1; i++) {
        var a = kfs[i], b = kfs[i + 1];
        if (p >= a.at && p <= b.at) {
          var t = (b.at - a.at) > 0 ? (p - a.at) / (b.at - a.at) : 0;
          var av = parseFloat(a.value), bv = parseFloat(b.value);
          if (!isNaN(av) && !isNaN(bv)) {
            var unit = String(b.value).match(/[a-z%]+$/i);
            return (av + (bv - av) * t).toFixed(3) + (unit ? unit[0] : '');
          }
          return a.value;
        }
      }
      return kfs[kfs.length - 1].value;
    };
    var apply = function(p) {
      var tfx = {};
      props.forEach(function(pr) {
        var v = interp(pr.keyframes, p);
        if (v == null) return;
        var norm = pr.property.replace(/-([a-z])/g, function(_, c) { return c.toUpperCase(); });
        if (pr.property.indexOf('--') === 0) { root.style.setProperty(pr.property, v); }
        else if (transformKeys.indexOf(norm) !== -1) { tfx[norm] = v; }
        else { root.style[norm] = v; }
      });
      var keys = Object.keys(tfx);
      if (keys.length) root.style.transform = keys.map(function(k) { return k + '(' + tfx[k] + ')'; }).join(' ');
    };
    apply(0);
    var start = null;
    var step = function(ts) {
      if (start === null) start = ts;
      var p = Math.min(1, (ts - start) / dur);
      apply(p);
      if (p < 1) (window.requestAnimationFrame || function(f){ setTimeout(function(){ f(Date.now()); }, 16); })(step);
    };
    (window.requestAnimationFrame || function(f){ setTimeout(function(){ f(Date.now()); }, 16); })(step);
  };

  Spacetime.buildTemplateFactory = function(rawName, rawBody, rawParamsIn, templateAnimations, regionId) {
        // PLAN-150 W9: a `@form shot --name` registers a template under its
        // form name (`--name`); it is invoked as `&name`. Strip either sigil so
        // the form declaration and the template invocation resolve to the same
        // key (`name`).
        const templateName = (typeof rawName === 'string')
      ? rawName.replace(/^--/, '').replace(/^&/, '') : rawName;
    // Region-scoped lookup (FEAT-126 isolation): when a bundle registers under a
    // region, its templates and nested refs are keyed `regionId/name` so two
    // regions can define the SAME bare name without bleeding into each other.
    // `resolveRefFactory` looks region-first, then falls back to the bare/global
    // name (a region template may still reference a globally-registered one).
    const resolveRefFactory = (nm) => {
      const reg = window.Spacetime && window.Spacetime.templates;
      if (regionId && reg) {
        const scoped = reg.get(regionId + '/' + nm);
        if (scoped) return scoped;
      }
      return (reg && reg.get(nm)) || (window.__pendingTemplates && window.__pendingTemplates[nm]);
    };
    // Extract HTML from component body (structured object) or use as-is (string)
        let templateHtml = '';
    if (typeof rawBody === 'object' && rawBody !== null && rawBody.html !== undefined) {
      templateHtml = rawBody.html || '';
    } else {
      templateHtml = rawBody || '';
    }

    // Fallback: If no HTML provided, try to find a template
    if (!templateHtml) {
      // First try DOM templates
      const domTemplate = document.querySelector(`template[data-component="${templateName}"]`);
      if (domTemplate) {
        templateHtml = domTemplate.innerHTML;
      }
      // Also check IK component templates (loaded from external file)
      else if (typeof IK !== 'undefined' && IK.templates?.[templateName]) {
        const ikTemplate = IK.templates[templateName];
        if (ikTemplate instanceof HTMLTemplateElement) {
          templateHtml = ikTemplate.innerHTML;
        }
      }
    }

    // Normalize params to array - handle case where capture returns string instead of array
    let rawParams = rawParamsIn || [];
    if (typeof rawParams === 'string') {
      // Single param was captured as string - wrap in array with $ prefix if missing
      rawParams = rawParams ? [rawParams.startsWith('$') ? rawParams : '$' + rawParams] : [];
    }

    // Parse param specs to extract names and defaults
    // Accepts structured objects {name, kind, optional} from codegen
    // or legacy string format "$name", "$name?", "&name"
    const paramSpecs = (rawParams || []).map(spec => {
      // Structured object format from codegen: {name, kind, optional}
      if (typeof spec === 'object' && spec !== null && spec.name) {
        return {
          name: spec.name,
          isElement: spec.kind === 'element',
          isOptional: !!spec.optional,
          default: spec.default
        };
      }

      // Legacy string format: "$name", "$name?", "&name", "&name?"
      if (typeof spec !== 'string') {
        return { name: String(spec), isElement: false, isOptional: false, default: undefined };
      }

      // Handle simple param names (legacy format without $ or &)
      if (!spec.startsWith('$') && !spec.startsWith('&')) {
        return { name: spec, isElement: false, isOptional: false, default: undefined };
      }

      // Parse $name?="default" or &name="default" format
      const isElement = spec.startsWith('&');
      const withoutPrefix = spec.slice(1);

      // Check for default value (name="value" or name?="value")
      const eqIdx = withoutPrefix.indexOf('=');
      if (eqIdx !== -1) {
        let paramName = withoutPrefix.slice(0, eqIdx);
        let defaultVal = withoutPrefix.slice(eqIdx + 1);
        const isOptional = paramName.endsWith('?');
        if (isOptional) paramName = paramName.slice(0, -1);

        // Unquote the default value if quoted
        if ((defaultVal.startsWith('"') && defaultVal.endsWith('"')) ||
            (defaultVal.startsWith("'") && defaultVal.endsWith("'"))) {
          defaultVal = defaultVal.slice(1, -1);
        }

        return { name: paramName, isElement, isOptional: true, default: defaultVal };
      }

      // No default - just param name with optional marker
      const isOptional = withoutPrefix.endsWith('?');
      const paramName = isOptional ? withoutPrefix.slice(0, -1) : withoutPrefix;
      return { name: paramName, isElement, isOptional, default: undefined };
    });

    // HTML escape for text content (XSS protection)
    const escapeHtml = (str) => {
      if (str == null) return '';
      return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
    };

    // Attribute escape (preserve quotes for attribute context)
    const escapeAttr = (str) => {
      if (str == null) return '';
      return String(str)
        .replace(/&/g, '&amp;')
        .replace(/\x22/g, '&quot;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
    };

    // Create factory function that interpolates values
    let factory = (...args) => {
      // Build data object from positional/named args
      const data = {};

      // First, apply compile-time defaults for all params that have them
      paramSpecs.forEach(spec => {
        if (spec.default !== undefined) {
          data[spec.name] = spec.default;
        }
      });

      // Then override with provided args. Two forms coexist (BUG-131):
      //  - POSITIONAL: factory(v0, v1, …) → data[paramSpecs[i].name] = v[i].
      //  - NAMED: a `&card(title: "A")` invocation passes a single BRANDED object
      //    `{__stNamedArgs: {title: <resolved>}}`; bind by NAME, order-independent.
      // Both may appear (mixed call): positional slots fill first, named override.
      const __named = {};
      const __positional = [];
      args.forEach(a => {
        if (a && typeof a === 'object' && a.__stNamedArgs) {
          Object.assign(__named, a.__stNamedArgs);
        } else {
          __positional.push(a);
        }
      });
      paramSpecs.forEach((spec, i) => {
        if (__positional[i] !== undefined) {
          data[spec.name] = __positional[i];
        }
      });
      // Named bindings win over a same-name positional slot (explicit > implicit).
      Object.keys(__named).forEach(name => {
        data[name] = __named[name];
      });

      // Interpolate HTML with context-aware escaping
      let html = templateHtml;

      // `$$` → literal `$`: same escape as interpolateHtml above; stashed so
      // the passes below never see it, restored before the html is used.
      html = html.replace(/\$\$/g, '\u0000');

      // Replace $param.property patterns with context-aware escaping
      // First pass: handle attribute contexts (="$value" or ='$value')
      html = html.replace(/(\w+)=\x22([^\x22]*\$[\w.]+[^\x22]*)\x22/g, (match, attrName, attrValue) => {
        const interpolated = attrValue.replace(/\$(\w+)(?:\.(\w+(?:\.\w+)*))?/g, (m, varName, path) => {
          let value = data[varName];
          if (path && value != null) {
            for (const part of path.split('.')) {
              if (value == null) return '';
              value = value[part];
            }
          }
          return escapeAttr(value);
        });
        return `${attrName}="${interpolated}"`;
      });

      // PLAN-150 W9: strip a backtick-wrapped hole `` `$title` `` to its value.
      // The Spacetime hole form is backtick-delimited; a shot's markup body
      // (register-template) writes `` `$title` ``, and without this the backticks
      // survive into the DOM (`` `Ceramic Mug` ``). Runs before the bare-hole
      // text pass so the wrapping backticks are gone first.
      html = html.replace(/`\$(\w+)(?:\.(\w+(?:\.\w+)*))?`/g, (m, varName, path) => {
        let value = data[varName];
        if (path && value != null) {
          for (const part of path.split('.')) {
            if (value == null) return '';
            value = value[part];
          }
        }
        // BUG-384: a resolved VALUE is literal text — never a hole. Stash any
        // `$` it contains as \u0000 so the bare-hole text pass below cannot
        // re-interpret e.g. a price "$24.99" as a `$24`/`.99` variable and wipe
        // it. The trailing \u0000 -> `$` restore returns it to display text.
        return escapeHtml(value).replace(/\$/g, '\u0000');
      });

      // Second pass: handle remaining text content (not in attributes)
      // These are $variables that weren't already handled in attribute context
      html = html.replace(/>([^<]*\$[\w.]+[^<]*)</g, (match, textContent) => {
        const interpolated = textContent.replace(/\$(\w+)(?:\.(\w+(?:\.\w+)*))?/g, (m, varName, path) => {
          let value = data[varName];
          if (path && value != null) {
            for (const part of path.split('.')) {
              if (value == null) return '';
              value = value[part];
            }
          }
          return escapeHtml(value);
        });
        return `>${interpolated}<`;
      });

      // Third pass: handle &param element substitution (trusted HTML, no escaping)
      paramSpecs.forEach(spec => {
        if (spec.isElement) {
          const marker = new RegExp('&' + spec.name + '(?![\\w-])', 'g');
          const content = data[spec.name];
          html = html.replace(marker, content != null ? String(content) : '');
        }
      });

      // Debug: warn about remaining uninterpolated variables
      if (html.includes('$')) {
        const remaining = html.match(/\$[a-zA-Z_][\w.]*/g);
        if (remaining && remaining.length > 0) {
          console.warn('[ST Template]', templateName, '- Uninterpolated variables:', remaining, 'Available params:', Object.keys(data));
        }
      }

      // Restore the `$$` escape AFTER the passes and the leftover-$ check —
      // a restored literal `$now` is display text, not a missed binding.
      html = html.replace(/\u0000/g, '$');

      // === Reactive DOM builder (FEAT-077) ===
      // When the compiler emitted a reactive `builder` (root-scoped HtmlExpr → DOM with
      // reactive holes), use it INSTEAD of regex string-interpolation: it constructs the
      // element tree and wires every `$x` hole to ST.get/watch on the root element. Params
      // are seeded as signals on the root so `$param` holes resolve (and become reactive —
      // a strict upgrade: re-invoking / ST.set on a param re-renders). Falls back to the
      // interpolated-HTML path when no builder is present (e.g. DOM-template fallback).
      let element;
      const builderSrc = (typeof rawBody === 'object' && rawBody !== null) ? rawBody.builder : null;
      if (builderSrc) {
        let buildFn = null;
        try {
          // builderSrc is a compiler-generated `(function(__el){…})` expression. eval is safe
          // here: the source is compiler-emitted (author-trust model), and ST is global.
          buildFn = (0, eval)(builderSrc);
        } catch (e) {
          console.error('[ST] template builder eval failed for', templateName, e);
        }
        if (typeof buildFn === 'function') {
          element = buildFn(null);
          // Seed params as signals on the root (the scope the builder's holes read).
          if (element && typeof ST !== 'undefined') {
            // Seed ALL params (binding AND element) as signals on the root so both `$param`
            // and `&param` holes resolve. Element params hold trusted HTML/Node content the
            // builder inserts via its element-substitution holes (FEAT-077).
            paramSpecs.forEach(spec => {
              if (data[spec.name] !== undefined) {
                ST.set(element, spec.name, data[spec.name]);
              }
            });
          }
        }
      }
      if (!element) {
        // Fallback: create element from regex-interpolated HTML (legacy path).
        // ST.parseHtml is the shared namespace-aware parser (BUG-082); without a
        // container context it behaves exactly like a plain <template> parse.
        element = ST.parseHtml(html.trim(), null);
      }

      // === Seed params as instance signals on EVERY path (PLAN-039 W2) ===
      // The reactive-builder branch above seeds params on the root; the string-interp
      // fallback path did NOT, so a nested construct inside the body (e.g. an @editable
      // whose `bind:$f` resolves via ST.get up the instance scope) could not read the
      // param. Seed here unconditionally so $param holes + nested-construct binds resolve
      // to the instance regardless of which build path produced `element`. Idempotent with
      // the builder-branch seeding (same value).
      if (element && typeof ST !== 'undefined') {
        paramSpecs.forEach(spec => {
          if (data[spec.name] !== undefined) {
            ST.set(element, spec.name, data[spec.name]);
          }
        });
      }

      // === Instance-scoped state initialization (PROJ-102/PROJ-101) ===
      // States are now typed structs from ComponentBodyDef:
      // { var_name: "open", type_name: "bool", initial: false }
      const bodyStates = (typeof rawBody === 'object' && rawBody !== null) ? (rawBody.states || []) : [];
      if (element && typeof ST !== 'undefined') {
        bodyStates.forEach(state => {
          if (state && state.var_name !== undefined) {
            ST.set(element, state.var_name, state.initial);
          }
        });
      }

      // === @exports metadata (PROJ-106) ===
      // Attach __stExports to root element so cross-instance access can be validated.
      // ExportDecl structs from ComponentBodyDef: { var_name: "count", mutable: false }
      const bodyExports = (typeof rawBody === 'object' && rawBody !== null)
        ? (rawBody.exports || []) : [];
      if (element && bodyExports.length > 0) {
        const exportMap = {};
        bodyExports.forEach(exp => {
          exportMap[exp.var_name] = { mutable: !!exp.mutable };
        });
        element.__stExports = exportMap;
      }

      // === Instance-scoped directive wiring (PROJ-102/PROJ-101) ===
      // Directives are now typed structs from ComponentBodyDef:
      // OnEvent: { type: "OnEvent", selector: "", event: "click", action: { type: "Toggle", var: "open" } }
      // ClassToggle: { type: "ClassToggle", class_name: "nav--open", var_name: "menuOpen", selector: null | ".child" }
      // ContentBinding: { type: "ContentBinding", selector: ".count", var_name: "count", property: "textContent", filter: null }
      // Resolve a selector against THIS instance: the root when it matches (querySelector
      // only searches descendants), else a descendant, else the root. Used by content
      // injection wiring below.
      const resolveScoped = (selector) => {
        if (!selector) return element;
        if (element.matches && element.matches(selector)) return element;
        return element.querySelector(selector) || element;
      };
      // NOTE: body DIRECTIVES (@on / class-toggle `.x: $sig` / content-binding
      // `text: $sig`) are NOT wired here. Each surfaces through the unified scope path:
      // a whole-file match/scope-declaration emits a page-level `registerSelectorInit`
      // whose per-node arc reads `ST.resolve(__node, …)` and subscribes via
      // `ST.watchScoped` (PLAN-039 Move 2a/2b). The dynamic-node observer fires that arc
      // on every instance node the factory creates, so each instance binds against its
      // own scope. Wiring it here too would DOUBLE-FIRE (two handlers, same instance
      // signal). One construct, one wiring, instance-scoped. (directives arm deleted.)

      // === Content injection — UNIFIED (PLAN-039 FEAT-115 S3c) ===
      // A body-root `target <- $x;` injection is no longer a separate payload + a
      // bespoke factory forEach. It is compiled into a synthesized selector-scoped
      // reactive binding (`injection_scopes_from_body` in cst_to_stfile) that rides
      // the SAME unified reactive-binding emitter as `.sel { text <- $x }` — a
      // per-node `registerSelectorInit` over the instance subtree, with `| filter`
      // honoured by the emitter. text/slot/[attr]/[class] targets all lower to that
      // one path; nothing to wire here.

      // === Template ref invocations (PROJ-104) ===
      // Refs are template invocations within the body: &name &template(args);
      // Named refs store element via ST.set for cross-instance access.
      // Collection refs (is_collection) build arrays for aggregation.
      const bodyRefs = (typeof rawBody === 'object' && rawBody !== null) ? (rawBody.refs || []) : [];
      // Recursion depth guard (FEAT-073): a template body may invoke another template
      // (or ITSELF) via a body ref, which renders synchronously here. A self/mutual
      // recursion without a reachable base case would otherwise overflow the stack.
      // Cap the synchronous render depth; on exceed, emit a dev warning and stop
      // descending (the partial subtree is kept, never a crash). The cap is generous
      // (schema nesting is shallow); override via window.__stMaxTemplateDepth.
      const __stDepthCap = (typeof window !== 'undefined' && window.__stMaxTemplateDepth) || 64;
      if (typeof window !== 'undefined') {
        window.__stTemplateDepth = (window.__stTemplateDepth || 0);
      }
      const __stAtDepthCap = (typeof window !== 'undefined') && window.__stTemplateDepth >= __stDepthCap;
      if (__stAtDepthCap && bodyRefs.length > 0) {
        console.warn('[ST] Template recursion depth cap (' + __stDepthCap + ') reached at \'' +
          templateName + '\' — stopping descent. Add a base case (@if/@match) or raise ' +
          'window.__stMaxTemplateDepth.');
      }
      if (element && bodyRefs.length > 0 && !__stAtDepthCap && typeof ST !== 'undefined') {
        bodyRefs.forEach(ref => {
          if (!ref || !ref.template_name) return;
          // DYNAMIC DISPATCH (FEAT-073): a `$`-prefixed template_name (`&$w()` /
          // `&$node.widget()`) names the template by a RUNTIME value. Resolve it against
          // this instance's `data` (dotted path supported) before the registry lookup.
          let __tname = ref.template_name;
          if (typeof __tname === 'string' && __tname.startsWith('$')) {
            const __path = __tname.slice(1).split('.');
            let __v = data[__path[0]];
            for (let __i = 1; __i < __path.length && __v != null; __i++) __v = __v[__path[__i]];
            if (__v == null || __v === '') {
              console.warn('[ST] Dynamic dispatch: name', ref.template_name, 'resolved to empty');
              return;
            }
            __tname = String(__v);
          }
          const refFactory = resolveRefFactory(__tname);
          if (!refFactory) {
            console.warn('[ST] Template ref: factory not found for', __tname);
            return;
          }

          // Resolve args via the shared dotted-path resolver (BUG-074): a ref arg
          // like $node.properties walks data.node.properties, not data['node.properties'].
          const resolvedArgs = ST.buildTemplateArgs(ref.args, ref.arg_names, data);

          try {
            // Track synchronous render depth so the guard above bounds self/mutual
            // recursion. Increment before descending into the child factory, restore
            // after (try/finally so a throw can't leak the counter).
            if (typeof window !== 'undefined') window.__stTemplateDepth++;
            let refEl;
            try {
              refEl = refFactory(...resolvedArgs);
            } finally {
              if (typeof window !== 'undefined') window.__stTemplateDepth--;
            }
            if (refEl && element) {
              element.appendChild(refEl);
              // Named refs: store reference via ST.set for cross-instance access
              if (ref.ref_name) {
                if (ref.is_collection) {
                  // Collection ref: accumulate elements into an array
                  const existing = ST.get(element, ref.ref_name) || [];
                  ST.set(element, ref.ref_name, [...existing, refEl]);
                } else {
                  ST.set(element, ref.ref_name, refEl);
                }
              }
            }
          } catch (err) {
            console.error('[ST] Template ref error (' + ref.template_name + '):', err);
          }
        });
      }

      // === @match render dispatch (FEAT-073) ===
      // Each match: resolve `subject` against instance data, render the FIRST arm whose
      // pattern equals it (string compare); a `_` (pattern null) arm always matches.
      // Reuses the same depth guard + name/arg resolution as body refs.
      const bodyMatches = (typeof rawBody === 'object' && rawBody !== null) ? (rawBody.matches || []) : [];
      if (element && bodyMatches.length > 0 && typeof ST !== 'undefined'
          && !(typeof window !== 'undefined' && window.__stTemplateDepth >= __stDepthCap)) {
        bodyMatches.forEach(m => {
          if (!m || !m.subject || !Array.isArray(m.arms)) return;
          // Resolve the subject expression against data (dotted path supported).
          let subj;
          const sx = String(m.subject);
          if (sx.startsWith('$')) {
            const path = sx.slice(1).split('.');
            subj = data[path[0]];
            for (let i = 1; i < path.length && subj != null; i++) subj = subj[path[i]];
          } else {
            subj = sx;
          }
          // Pick the first matching arm (null pattern = wildcard `_`).
          const arm = m.arms.find(a => a && (a.pattern == null || String(a.pattern) === String(subj)));
          if (!arm || !arm.template_ref) return;
          const tref = arm.template_ref;
          // Resolve the (possibly dynamic) template name.
          let tname = tref.template_name;
          if (typeof tname === 'string' && tname.startsWith('$')) {
            const p = tname.slice(1).split('.');
            let v = data[p[0]];
            for (let i = 1; i < p.length && v != null; i++) v = v[p[i]];
            if (v == null || v === '') return;
            tname = String(v);
          }
          const f = resolveRefFactory(tname);
          if (!f) { console.warn('[ST] @match: template not found for', tname); return; }
          // @match arm args via the shared dotted-path resolver (BUG-074),
          // named-arg-aware (BUG-131).
          const resolvedArgs = ST.buildTemplateArgs(tref.args, tref.arg_names, data);
          try {
            if (typeof window !== 'undefined') window.__stTemplateDepth++;
            let el;
            try { el = f(...resolvedArgs); } finally {
              if (typeof window !== 'undefined') window.__stTemplateDepth--;
            }
            if (el && element) element.appendChild(el);
          } catch (err) {
            console.error('[ST] @match render error (' + tname + '):', err);
          }
        });
      }

      // === Selector ref wiring (SC-009) — REMOVED (FEAT-115 S5) ===
      // The reify `selector_refs` payload is always empty in the real pipeline: a
      // body `&name .sel {}` is intercepted upstream by the flat `element-ref`
      // macro before reach the ComponentBody inner, so this forEach never had
      // data. The collection-ref + @each render path is World-A `@each` (see
      // tests/unit/templates/collection-ref-each.test.st). No factory wiring.

      // Apply scoped animations if defined. `Spacetime.applyAnimations` is
      // defined below (PLAN-150 W9) — it plays the keyframes as a one-shot
      // mount reveal (progress 0 -> 1 over a default duration). A `@form shot`'s
      // `:motion` slot rides this so an instantiated shot animates on mount.
      if (element && templateAnimations && window.Spacetime?.applyAnimations) {
        window.Spacetime.applyAnimations(element, templateAnimations);
      }

      // Reactive-markdown nodes (`<st-md>`): render their text content as
      // Markdown via the vendored snarkdown engine. The compiled reactive
      // builder (buildFn) folds `<st-md>` into a snarkdown-backed node itself
      // (component_html_to_exprs), but the STRING-interpolated factory path
      // (a template registered from a raw `{html}` body, e.g. the chat
      // history mounted by `st/view`) sets innerHTML verbatim, leaving the
      // `<st-md>` element literal. Render them here so markdown formats in
      // BOTH paths from ONE rule: the element's text (already `$`-interpolated
      // above) is its markdown source. No-op when snarkdown is absent (a page
      // with no markdown ships none of its bytes — the vendor lean law).
      if (element && typeof snarkdown !== 'undefined') {
        var __mds = [];
        if (element.tagName === 'ST-MD') __mds.push(element);
        if (element.querySelectorAll) {
          element.querySelectorAll('st-md').forEach(function(n) { __mds.push(n); });
        }
        __mds.forEach(function(n) {
          // The source is the element's text; a factory backtick hole that did
          // not resolve leaves the literal `` `expr` `` — strip a single
          // wrapping backtick pair so an un-bound hole degrades to its text
          // rather than rendering stray backticks.
          var src = n.textContent || '';
          if (src.length >= 2 && src.charAt(0) === '`' && src.charAt(src.length - 1) === '`') {
            src = src.slice(1, -1);
          }
          n.innerHTML = snarkdown(src);
          n.style.display = 'contents';
        });
      }

      return element;
    };

    // FUP-134: publish the owner for the DURATION of a factory call, so an
    // `invoke-template` nested inside this template's body resolves against THIS
    // template's owner rather than whatever bundle happens to be evaluating.
    // Set/restore, because factories nest and run interleaved with page code.
    const rawFactory = factory;
    factory = function(...args) {
      const prev = window.__ST_INVOKE_OWNER__;
      window.__ST_INVOKE_OWNER__ = regionId || undefined;
      try {
        return rawFactory.apply(this, args);
      } finally {
        window.__ST_INVOKE_OWNER__ = prev;
      }
    };

    // FUP-134: stamp the owner ONTO the factory. `regionId` here doubles as the
    // bundle owner (one slot, one meaning — see registerOwnedTemplate). The global
    // `__ST_BUNDLE_OWNER__` is live only while a bundle EVALUATES, so a factory
    // invoked later would resolve its nested refs against `undefined` and fall
    // back to bare lookup. Reading the owner off the factory makes identity a
    // property of the template rather than of the moment it runs.
    try {
      Object.defineProperty(factory, '__stOwner', {
        value: regionId || null,
        enumerable: false,
        configurable: true,
      });
    } catch (_) {
      /* frozen function: owner-first simply degrades to bare lookup */
    }
    return { name: templateName, factory: factory };
  };

})();
  /**
   * Register a region-mounted bundle (FEAT-126).
   *
   * Given a bundle payload from `/__mcp/instance/{id}/bundles`, register every
   * template via the shared factory builder, scope the bundle CSS under the
   * region's `[data-st-region]`, and set the region's stage signal so an
   * `@view $stageTpl` mounts the bundle's main template.
   *
   * @param {object} bundle - Bundle payload from the server.
   * @param {string} bundle.regionId - Region root selector value.
   * @param {string} [bundle.signal="stageTpl"] - Signal name to set on the region root.
   * @param {Array} bundle.templates - Template entries.
   * @param {string} bundle.css - Raw CSS to scope under the region.
   * @returns {boolean} true if at least one template was registered.
   */
  // Region ids flow from the `st_mount` `region` param into a CSS `@scope`
  // selector and a `querySelector`, so they MUST be a safe token: a crafted id
  // could break out of `[data-st-region="..."]` and inject unscoped CSS or throw
  // the selector parse. We constrain to the same shape the server enforces.
  function isSafeRegionId(id) {
    return typeof id === 'string' && /^[A-Za-z0-9_-]+$/.test(id);
  }

  // Region isolation (FEAT-126): bundle templates register under a region-scoped
  // key `regionId/name`, so two regions can define the SAME bare name without
  // overwriting each other (silent cross-region UI/state bleed). Nested refs
  // resolve region-first inside buildTemplateFactory (see resolveRefFactory).
  Spacetime.registerBundle = function(bundle) {
    if (!bundle || typeof bundle !== 'object') {
      console.error('[ST] registerBundle: expected bundle object');
      return false;
    }
    const regionId = bundle.regionId || bundle.region_id;
    const signalName = bundle.signal || bundle.signal_name || 'stageTpl';
    const templates = bundle.templates || [];
    const rawCss = bundle.css || '';

    if (!regionId) {
      console.error('[ST] registerBundle: missing regionId');
      return false;
    }
    if (!isSafeRegionId(regionId)) {
      console.error('[ST] registerBundle: unsafe regionId (expected [A-Za-z0-9_-]+):', regionId);
      return false;
    }

    // Register every template under its region-scoped key using the shared
    // factory builder. The builder is given `regionId` so its nested-ref lookups
    // resolve sibling templates within the SAME region first.
    let registeredCount = 0;
    let firstName = null;
    templates.forEach(function(t) {
      if (!t || !t.name) return;
      const built = Spacetime.buildTemplateFactory(
        t.name,
        t.body || '',
        t.params || [],
        t.animations || null,
        regionId
      );
      if (!built || !built.factory) {
        console.warn('[ST] registerBundle: factory build failed for', t.name);
        return;
      }
      const scopedKey = regionId + '/' + built.name;
      Spacetime.templates.set(scopedKey, built.factory);
      if (firstName === null) firstName = built.name;
      registeredCount++;
    });

    if (registeredCount === 0) {
      // BUG-125: an EMPTY bundle (the server returns `{ empty: true, templates:
      // [] }` once a region is cleared/unmounted) must TEAR DOWN the previously
      // mounted guest, not silently keep it. Reset the region's stage signal so
      // the `@view $stageTpl { _ => &$stageTpl(); }` consumer unmounts, remove the
      // scoped style element, and clear the auto-stage DOM. Guarded so a
      // steady-state empty poll is a no-op (only acts when something WAS mounted).
      var emptyRoot = document.querySelector('[data-st-region="' + regionId + '"]');
      if (emptyRoot) {
        var wasMounted = emptyRoot.__stBundleFingerprint !== undefined
          && emptyRoot.__stBundleFingerprint !== '\u0000empty';
        if (wasMounted) {
          emptyRoot.__stBundleFingerprint = '\u0000empty';
          emptyRoot.__stRegionInput = {};
          if (typeof ST !== 'undefined' && ST.set) {
            ST.set(emptyRoot, signalName, '');
          }
          // @view subscribes to `local:<signal>:updated`; fire it so the
          // wildcard arm re-evaluates with the now-empty signal and unmounts.
          if (typeof document !== 'undefined' && typeof CustomEvent !== 'undefined') {
            document.dispatchEvent(new CustomEvent('local:' + signalName + ':updated'));
          }
          // Auto-stage (standalone) roots mount the guest directly — clear it.
          if (emptyRoot.hasAttribute && emptyRoot.hasAttribute('data-st-mcp-autostage')) {
            emptyRoot.textContent = '';
            emptyRoot.__stAutoStaged = undefined;
          }
          // Drop the region's scoped style element so stale guest CSS does not
          // linger after teardown.
          var emptyStyle = document.getElementById('st-bundle-' + regionId);
          if (emptyStyle && emptyStyle.parentNode) emptyStyle.parentNode.removeChild(emptyStyle);
        }
        return true;
      }
      console.warn('[ST] registerBundle: no templates registered for region', regionId);
      return false;
    }

    // Scope CSS under the region and inject/replace the bundle's style element.
    // regionId is already validated to [A-Za-z0-9_-]+, so it is safe to embed in
    // both the selector and the style element id.
    if (rawCss.trim()) {
      const scopedCss = '@scope ([data-st-region="' + regionId + '"]) {\n' + rawCss + '\n}';
      const styleId = 'st-bundle-' + regionId;
      let styleEl = document.getElementById(styleId);
      if (!styleEl) {
        styleEl = document.createElement('style');
        styleEl.id = styleId;
        styleEl.setAttribute('data-st-bundle', regionId);
        const head = document.head || document.documentElement;
        if (head) head.appendChild(styleEl);
      }
      if (styleEl) styleEl.textContent = scopedCss;
    }

    // Set the region's stage signal to the region-qualified key of the main
    // template. The region root `@view $stageTpl { _ => &$stageTpl(...); }` mounts
    // it reactively; the qualified key routes the lookup to THIS region's factory.
    const mainBareName = (bundle.mainTemplate || bundle.main || firstName);
    const mainTemplate = regionId + '/' + mainBareName;
    const regionRoot = document.querySelector('[data-st-region="' + regionId + '"]');
    if (regionRoot && typeof ST !== 'undefined' && ST.set) {
      // M4: stash this region's param overrides on the root so the `@view` mount
      // can seed them onto the freshly-created guest element (the guest's `$param`
      // holes then reflect inspector edits). Folded into the fingerprint below so
      // a param change re-mounts the guest with the new values.
      var regionInput = bundle.input || {};
      regionRoot.__stRegionInput = regionInput;
      // FEAT-124 hot-swap: the bundle poll re-registers every interval. The
      // template NAME (the signal value) is stable across a compatible re-put,
      // so ST.set dedupes (no-op) and @view would keep showing STALE content even
      // though the factory just changed. Detect a real content change via a
      // fingerprint of the serialized templates (+ param input); on change,
      // re-trigger @view by dispatching its update event AFTER the factory is
      // replaced. Unchanged polls stay a no-op (no needless re-mount / state churn).
      const fingerprint = templates
        .map(function(t) {
          var b = t && t.body;
          var ser = b && (b.serialized || b.html || '');
          return (t && t.name) + ':' + (typeof ser === 'string' ? ser : JSON.stringify(ser || ''));
        })
        .join('\u0001') + '|css=' + rawCss + '|input=' + JSON.stringify(regionInput);
      const prevFingerprint = regionRoot.__stBundleFingerprint;
      const changed = prevFingerprint !== undefined && prevFingerprint !== fingerprint;
      regionRoot.__stBundleFingerprint = fingerprint;

      ST.set(regionRoot, signalName, mainTemplate);
      if (changed) {
        // Force a re-mount: @view subscribes to `local:<signal>:updated`.
        if (typeof document !== 'undefined' && typeof CustomEvent !== 'undefined') {
          document.dispatchEvent(new CustomEvent('local:' + signalName + ':updated'));
        }
      }
      // BUG-116: standalone auto-stage. A bare standalone instance page (an
      // agent-control panel, a preview) has NO `@view $stageTpl` consumer in its
      // DOM — only the workbench HOST page wires that. Without it, the factories
      // register but nothing mounts and the body stays empty. When the region
      // root opts in with `data-st-mcp-autostage`, DIRECTLY instantiate the main
      // template into the root (the @view-equivalent), re-mounting on a real
      // content change. This is the runtime's own self-mount — no per-page JS.
      if (regionRoot.hasAttribute && regionRoot.hasAttribute('data-st-mcp-autostage')) {
        var firstMount = regionRoot.__stAutoStaged === undefined;
        if (firstMount || changed) {
          var factory = Spacetime.templates.get(mainTemplate);
          if (typeof factory === 'function') {
            try {
              // BUG (param defaults): `regionInput` is a NAMED `{param: value}`
              // override map, so it MUST be branded as named args. Passing it
              // positionally bound the WHOLE object to param-0
              // (`data[firstParam] = {…}` -> rendered `[object Object]`) and left
              // every other param unset, defeating the factory's default-seeding.
              // Branding as `{__stNamedArgs}` binds each key BY NAME, so unfilled
              // params fall through to their declared `= default` (BUG-131 path).
              var guestEl = factory({ __stNamedArgs: regionInput || {} });
              if (guestEl) {
                regionRoot.textContent = '';
                regionRoot.appendChild(guestEl);
                regionRoot.__stAutoStaged = true;
              }
            } catch (e) {
              console.warn('[ST] registerBundle: auto-stage mount failed for', mainTemplate, e);
            }
          }
        }
      }
    } else if (regionRoot) {
      regionRoot.dataset['stSignal' + signalName.charAt(0).toUpperCase() + signalName.slice(1)] = mainTemplate;
    } else {
      console.warn('[ST] registerBundle: no region root found for', regionId);
    }

    return true;
  };

  /**
   * Poll the bundle endpoint for a mounted instance and register the result.
   *
   * @param {string} instanceId - Mounted instance id.
   * @param {string} regionId - Region id to register the bundle into.
   * @param {string} [signal="stageTpl"] - Signal name to drive @view.
   * @param {number} [intervalMs=1000] - Poll interval.
   */
  // Active poll loops keyed by `instanceId§regionId` so a region that is
  // re-initialized (selector-init can fire more than once per root) does not
  // accumulate duplicate fetch loops. The value is the interval id (or true for
  // a one-shot). `stopPolling` clears one; callers tie it to region cleanup.
  Spacetime._bundlePolls = Spacetime._bundlePolls || new Map();

  Spacetime.stopPolling = function(instanceId, regionId) {
    const key = instanceId + '\u00a7' + regionId;
    const id = Spacetime._bundlePolls.get(key);
    if (id !== undefined) {
      if (typeof id === 'number' && typeof clearInterval !== 'undefined') clearInterval(id);
      Spacetime._bundlePolls.delete(key);
    }
  };

  Spacetime.pollBundles = function(instanceId, regionId, signal, intervalMs) {
    if (!instanceId || !regionId) {
      console.error('[ST] pollBundles: instanceId and regionId required');
      return;
    }
    // De-dupe: a poll loop for this exact instance+region is already running.
    const key = instanceId + '\u00a7' + regionId;
    if (Spacetime._bundlePolls.has(key)) {
      return Spacetime._bundlePolls.get(key);
    }
    // FEAT-126 / BUG-111: scope the bundle poll to THIS region so a host page
    // (the workbench) fetches the guest function composed into the region rather
    // than its own function. The server reads `?region=` against `region_mounts`.
    const pollUrl = '/__mcp/instance/' + encodeURIComponent(instanceId)
      + '/bundles?region=' + encodeURIComponent(regionId);
    const interval = intervalMs || 1000;
    const sig = signal || 'stageTpl';

    // BUG-125 (reviewer W4 P2): order out-of-flight responses. Each fetch is
    // stamped with a monotonically increasing sequence; registerBundle drops a
    // response whose seq is OLDER than the newest already applied for this
    // region. Without this, a stale EMPTY response (dispatched during a cleared
    // interval) could arrive AFTER a quick recompose and blank the new frame.
    Spacetime._bundleSeq = Spacetime._bundleSeq || new Map();
    Spacetime._bundleSeqApplied = Spacetime._bundleSeqApplied || new Map();

    function tick() {
      if (typeof fetch === 'undefined') return;
      var mySeq = (Spacetime._bundleSeq.get(key) || 0) + 1;
      Spacetime._bundleSeq.set(key, mySeq);
      fetch(pollUrl, { cache: 'no-store' })
        .then(function(res) { return res.json(); })
        .then(function(bundle) {
          if (!bundle || typeof bundle !== 'object') return;
          // Drop a response superseded by a newer one already applied.
          var applied = Spacetime._bundleSeqApplied.get(key) || 0;
          if (mySeq < applied) return;
          Spacetime._bundleSeqApplied.set(key, mySeq);
          bundle.regionId = regionId;
          bundle.signal = sig;
          Spacetime.registerBundle(bundle);
        })
        .catch(function(err) {
          console.warn('[ST] pollBundles failed:', err);
        });
    }

    tick();
    if (interval > 0 && typeof setInterval !== 'undefined') {
      const id = setInterval(tick, interval);
      Spacetime._bundlePolls.set(key, id);
      return id;
    }
    // One-shot (no repeating interval): record presence so a later call de-dupes.
    Spacetime._bundlePolls.set(key, true);
    return undefined;
  };

  /**
   * BUG-116: auto-stage bootstrap for STANDALONE instance pages.
   *
   * A standalone MCP instance page (an agent-control panel, a preview) is a bare
   * region root with NO `@view $stageTpl` in its compiled JS — only the workbench
   * HOST page wires that. Such a page tags its region root `data-st-mcp-autostage`
   * and carries `data-st-instance` (the instance id) + `data-st-region` (its
   * region id). On load we start a bundle poll for it; the poll uses the
   * NO-REGION endpoint (the standalone instance's own function, not a host
   * region_mounts lookup), and `registerBundle` direct-mounts `main` into the
   * root (see the auto-stage branch above). This is the runtime's self-mount — no
   * per-page JS, mirroring how the host stage mounts a guest.
   */
  Spacetime.startAutoStage = function(root) {
    if (!root || !root.getAttribute) return;
    var instanceId = root.getAttribute('data-st-instance');
    var regionId = root.getAttribute('data-st-region');
    if (!instanceId || !regionId) return;
    if (root.__stAutoStagePolling) return;
    root.__stAutoStagePolling = true;
    var pollUrl = '/__mcp/instance/' + encodeURIComponent(instanceId) + '/bundles';
    function tick() {
      if (typeof fetch === 'undefined') return;
      fetch(pollUrl, { cache: 'no-store' })
        .then(function(res) { return res.json(); })
        .then(function(bundle) {
          if (!bundle || typeof bundle !== 'object') return;
          bundle.regionId = regionId;
          bundle.signal = 'stageTpl';
          Spacetime.registerBundle(bundle);
        })
        .catch(function(err) { console.warn('[ST] auto-stage poll failed:', err); });
    }
    tick();
    if (typeof setInterval !== 'undefined') setInterval(tick, 1000);
  };

  Spacetime.initAutoStages = function() {
    if (typeof document === 'undefined' || !document.querySelectorAll) return;
    var roots = document.querySelectorAll('[data-st-mcp-autostage]');
    for (var i = 0; i < roots.length; i++) Spacetime.startAutoStage(roots[i]);
  };

  if (typeof document !== 'undefined') {
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', function() { Spacetime.initAutoStages(); });
    } else {
      Spacetime.initAutoStages();
    }
  }

