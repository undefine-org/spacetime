/// Animation / timing testing (PLAN-027 W5).
///
/// Deterministic animation testing via the VIRTUAL CLOCK (ST._clock, installed
/// in the RAF coordinator). In test mode rAF is harness-driven, so easing curves,
/// stagger timing and driver progress are reproducible with no wall-clock flake.
///
/// @clock install            — switch to virtual time (auto on first advance).
/// @clock advance <ms>       — step virtual time by <ms>, ticking each frame.
/// @clock sample <n>         — step exactly <n> frames.
/// @clock run-to-end         — run until all drivers complete.
///
/// @record-timeline <name>   — snapshot ST._timing.driverUpdates[<name>] into a
///                             test-visible series (window.__stTimelines[<name>]).
///
/// These require the `timing` fidelity rung (real/seekable clock).
///
/// @example
///   @test "reveal stagger is monotonic" needs timing {
///     @mount { <p class="h">Hello world</p>
///       .h { @reveal(split: chars, stagger: 30) } }
///     @clock run-to-end
///     @record-timeline reveal
///     @then __timeline { name == "reveal"; monotonic; }
///   }

%macro clock-install {
  %form { @clock install }
  %emit js {
    if (ST._clock) ST._clock.install();
  }
}

%macro clock-advance {
  %form { @clock advance $ms:expr }
  %emit js {
    if (ST._clock) ST._clock.advance(%$ms);
  }
}

%macro clock-sample {
  %form { @clock sample $n:expr }
  %emit js {
    if (ST._clock) ST._clock.sampleFrames(%$n);
  }
}

%macro clock-run-to-end {
  %form { @clock run-to-end }
  %emit js {
    if (ST._clock) ST._clock.runToEnd();
  }
}

/// Snapshot a named driver's update series for timeline assertions. Captures
/// ST._timing.driverUpdates[name] (a {t, progress} sequence) into
/// window.__stTimelines[name].
%macro record-timeline {
  %form { @record-timeline $name:ident }
  %emit js {
    if (typeof window !== 'undefined') {
      window.__stTimelines = window.__stTimelines || {};
      const __series = (ST._timing && ST._timing.driverUpdates)
        ? (ST._timing.driverUpdates.get ? ST._timing.driverUpdates.get(%$name) : ST._timing.driverUpdates[%$name])
        : null;
      window.__stTimelines[%$name] = __series ? __series.slice() : [];
    }
  }
}

/// Assert properties of a recorded timeline series (an array of {t, p} samples).
/// Subject is a window.__stTimelines[<name>] series captured by @record-timeline,
/// or any global array of progress samples.
///
/// Claims (space/`;`-separated idents):
///   monotonic       — progress never decreases
///   ends-at-one     — final progress === 1 (±0.001)
///   in-unit         — every progress in [0, 1]
///   nonempty        — at least one sample
///
/// @example
///   @record-timeline reveal
///   @timeline reveal { monotonic; in-unit; }
%macro timeline {
  %form {
    @timeline $name:ident {
      $claims:claim_block
    }
  }

  %emit js {
    {
    const __series = (typeof window !== 'undefined' && window.__stTimelines)
      ? (window.__stTimelines[%$name] || []) : [];
    const __ps = __series.map(s => (typeof s === 'object' ? (s.p ?? s.progress ?? s.t) : s));
    const __claims = %$claims;
    const __tassert = (cond, msg) => { if (!cond) throw new Error('@timeline %$name: ' + msg); };
    for (const __c of __claims) {
      const __k = (__c.prop && typeof __c.prop === 'object') ? (__c.prop.id || __c.prop.sig) : __c.prop;
      switch (__k) {
        case 'nonempty':
          __tassert(__ps.length > 0, 'expected samples, got 0'); break;
        case 'monotonic': {
          let ok = true; for (let i = 1; i < __ps.length; i++) { if (__ps[i] < __ps[i-1] - 1e-9) { ok = false; break; } }
          __tassert(ok, 'progress decreased (not monotonic): ' + JSON.stringify(__ps)); break;
        }
        case 'in-unit': {
          const bad = __ps.find(p => p < -1e-9 || p > 1 + 1e-9);
          __tassert(typeof bad === 'undefined', 'progress escaped [0,1]: ' + bad); break;
        }
        case 'ends-at-one':
          __tassert(__ps.length > 0 && Math.abs(__ps[__ps.length-1] - 1) < 1e-3, 'final progress !== 1: ' + __ps[__ps.length-1]); break;
        default:
          throw new Error('@timeline: unknown claim ' + __k);
      }
    }
    }
  }
}
