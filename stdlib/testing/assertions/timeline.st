/// Timeline & Animation Assertions
///
/// First-party assertions for inspecting animation internals.
/// These provide direct access to timeline progress, completion,
/// and driver state that external frameworks cannot see.
///
/// @example
/// @then .hero should have_timeline_progress "intro" >= 0.5
/// @wait_for_timeline "intro" timeout 2s
/// @then .hero should have_timeline_complete "intro"

// =============================================================================
// Timeline Progress Assertions
// =============================================================================

/// Assert timeline progress meets a threshold
///
/// First-party capability: Direct access to animation progress (0-1).
/// External frameworks can only guess via computed styles.
///
/// @example
/// @then .element should have_timeline_progress "fadeIn" >= 0.5

%macro then-timeline-progress-gte {

  %form {
    @then $target:selector should have_timeline_progress $name:string >= $threshold:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __timelineName = %$name;
    const __threshold = %$threshold;

    const __progress = ST.resolve(__el, __timelineName);
    if (__progress === undefined) {
      throw new Error(`Timeline "${__timelineName}" not found`);
    }

    if (__progress < __threshold) {
      throw new Error(`Expected timeline "${__timelineName}" progress >= ${__threshold}, got ${__progress.toFixed(3)}`);
    }
    }
  }
}

%macro then-timeline-progress-lte {

  %form {
    @then $target:selector should have_timeline_progress $name:string <= $threshold:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __timelineName = %$name;
    const __threshold = %$threshold;

    const __progress = ST.resolve(__el, __timelineName);
    if (__progress === undefined) {
      throw new Error(`Timeline "${__timelineName}" not found`);
    }

    if (__progress > __threshold) {
      throw new Error(`Expected timeline "${__timelineName}" progress <= ${__threshold}, got ${__progress.toFixed(3)}`);
    }
    }
  }
}

%macro then-timeline-progress-eq {

  %form {
    @then $target:selector should have_timeline_progress $name:string == $value:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __timelineName = %$name;
    const __expected = %$value;
    const __tolerance = 0.01; // 1% tolerance for float comparison

    const __progress = ST.resolve(__el, __timelineName);
    if (__progress === undefined) {
      throw new Error(`Timeline "${__timelineName}" not found`);
    }

    if (Math.abs(__progress - __expected) > __tolerance) {
      throw new Error(`Expected timeline "${__timelineName}" progress = ${__expected}, got ${__progress.toFixed(3)}`);
    }
    }
  }
}

// =============================================================================
// Timeline Completion Assertions
// =============================================================================

/// Assert timeline has completed
///
/// @example
/// @then .hero should have_timeline_complete "intro"

%macro then-timeline-complete {

  %form {
    @then $target:selector should have_timeline_complete $name:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __timelineName = %$name;

    const __progress = ST.resolve(__el, __timelineName);
    if (__progress === undefined) {
      throw new Error(`Timeline "${__timelineName}" not found`);
    }

    if (!(__progress >= 1)) {
      throw new Error(`Expected timeline "${__timelineName}" to be complete`);
    }
    }
  }
}

/// Assert timeline has not completed
///
/// @example
/// @then .modal should have_timeline_running "expand"

%macro then-timeline-running {

  %form {
    @then $target:selector should have_timeline_running $name:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __timelineName = %$name;

    const __progress = ST.resolve(__el, __timelineName);
    if (__progress === undefined) {
      throw new Error(`Timeline "${__timelineName}" not found`);
    }

    if (!(__progress > 0 && __progress < 1)) {
      throw new Error(`Expected timeline "${__timelineName}" to be running, but it has completed`);
    }
    }
  }
}

// =============================================================================
// Wait for Timeline
// =============================================================================

/// Wait for a timeline to complete
///
/// First-party capability: Subscribe to timeline completion events.
/// External frameworks must poll or use arbitrary timeouts.
///
/// @example
/// @wait_for_timeline "intro" timeout 2s
/// @then .subtitle should be_visible

%macro wait-for-timeline {

  %form {
    @wait_for_timeline $name:string timeout $timeout:time = 5000
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __timelineName = %$name;
    const __timeout = ST.parseMs(%$timeout);

    const __publishers = ST._published.get(__timelineName);
    let __el;
    if (__publishers) {
      for (const __candidate of __publishers) {
        if (document.contains(__candidate)) {
          __el = __candidate;
          break;
        }
        __publishers.delete(__candidate);
      }
    }

    const __progress = __el && ST.resolve(__el, __timelineName);
    if (__progress === undefined) {
      throw new Error(`Timeline "${__timelineName}" not found`);
    }
    if (__progress >= 1) {
      return;
    }

    await new Promise((resolve, reject) => {
      let __settled = false;
      let __unsubscribe = () => {};
      const __timer = setTimeout(() => {
        if (__settled) return;
        __settled = true;
        __unsubscribe();
        reject(new Error(`Timeout after ${__timeout}ms waiting for timeline "${__timelineName}" to complete`));
      }, __timeout);

      __unsubscribe = ST.watch(__el, __timelineName, (__nextProgress) => {
        if (__settled || !(__nextProgress >= 1)) return;
        __settled = true;
        clearTimeout(__timer);
        resolve();
      });

      if (__settled) __unsubscribe();
    });
    }
  }
}

// =============================================================================
// Timeline Existence
// =============================================================================

/// Assert a timeline exists/is registered
///
/// @example
/// @then .element should have_timeline "fadeIn"

%macro then-has-timeline {

  %form {
    @then $target:selector should have_timeline $name:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __timelineName = %$name;

    if (ST.resolve(__el, __timelineName) === undefined) {
      throw new Error(`Timeline "${__timelineName}" not registered`);
    }
    }
  }
}

// =============================================================================
// Animation State Assertions
// =============================================================================

/// Assert an element has animation applied
///
/// @example
/// @then .box should be_animating

%macro then-animating {

  %form {
    @then $target:selector should be_animating
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    // Check if element has any active animations
    const __animations = __el.getAnimations?.() || [];
    const __hasTransition = getComputedStyle(__el).transition !== 'none' &&
                           getComputedStyle(__el).transition !== 'all 0s ease 0s';

    // Also check for ST-managed animations
    const __stSignals = ST.signals.get(__el);
    const __hasStAnimation = __stSignals && Object.keys(__stSignals).some(k => k.endsWith('_progress'));

    if (__animations.length === 0 && !__hasTransition && !__hasStAnimation) {
      throw new Error(`Expected ${%$target} to be animating, but no active animations found`);
    }
    }
  }
}

/// Assert an element is not animating
///
/// @example
/// @then .box should not_be_animating

%macro then-not-animating {

  %form {
    @then $target:selector should not_be_animating
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __animations = __el.getAnimations?.() || [];

    if (__animations.length > 0) {
      throw new Error(`Expected ${%$target} to not be animating, but found ${__animations.length} active animations`);
    }
    }
  }
}

// =============================================================================
// Driver Type Assertions (First-Party Only)
// =============================================================================

/// Assert timeline is driven by a specific driver type
///
/// First-party capability: Know whether animation is scroll-driven,
/// time-driven, event-driven, etc.
///
/// @example
/// @then .parallax should have_driver_type "scroll"
/// @then .intro should have_driver_type "time"
/// @then .hover-effect should have_driver_type "event"

%macro then-driver-type {

  %form {
    @then $target:selector should have_driver_type $type:string for $timeline:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __expectedType = %$type;
    const __timelineName = %$timeline;

    // Check for driver type in ST signals or timeline config
    const __driverType = ST.get(__el, __timelineName + '_driver') ||
                        ST.get(__el, 'driver_type');

    if (!__driverType) {
      throw new Error(`Could not determine driver type for timeline "${__timelineName}"`);
    }

    if (__driverType !== __expectedType) {
      throw new Error(`Expected driver type "${__expectedType}" for timeline "${__timelineName}", got "${__driverType}"`);
    }
    }
  }
}

// =============================================================================
// Timing Report (First-Party Debug Tool)
// =============================================================================

/// Enable timing capture for performance analysis
///
/// @example
/// @enable_timing
/// @when .box scroll
/// @wait 1s
/// @capture_timing_report

%macro enable-timing {

  %form {
    @enable_timing
  }

  %emit js {
    {
    ST._timing.enable();
    __ctx.timingEnabled = true;
    }
  }
}

%macro capture-timing-report {

  %form {
    @capture_timing_report
  }

  %emit js {
    {
    if (!__ctx.timingEnabled) {
      console.warn('Timing not enabled - call @enable_timing first');
    }
    __ctx.timingReport = ST._timing.report();
    ST._timing.disable();
    console.log('[Timing Report]', __ctx.timingReport);
    }
  }
}


