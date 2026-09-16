/**
 * RAF Coordinator - Spacetime Runtime Module
 *
 * Synchronizes all animation callbacks into a single RAF loop.
 * Prevents jitter from multiple independent RAF loops (Firefox timing variance).
 */

(function() {
  'use strict';

  const ST = window.ST;
  if (!ST) {
    console.error('[ST] RAF Coordinator requires ST to be defined first');
    return;
  }

  // IDEMPOTENT: a second runtime bundle on the page (host/migrations widgets)
  // must NOT replace a live coordinator — in-flight animation callbacks live in
  // ST._raf.callbacks; replacing it freezes them mid-flight.
  ST._raf = ST._raf || {
    callbacks: new Set(),
    running: false,
    frameId: null,

    /**
     * Add a callback to the RAF loop
     * @param {Function} cb - Callback receiving timestamp
     * @returns {Function} Remove callback function
     */
    add(cb) {
      this.callbacks.add(cb);
      if (!this.running) this.start();
      return () => {
        this.callbacks.delete(cb);
        if (this.callbacks.size === 0) this.stop();
      };
    },

    /**
     * Start the RAF loop
     */
    start() {
      if (this.running) return;
      this.running = true;
      // Virtual-clock mode (tests): do NOT schedule real rAF. ST._clock drives
      // the callbacks deterministically via tickAll(timestamp). The loop stays
      // "running" so add()/remove() bookkeeping is unchanged.
      if (ST._clock && ST._clock.virtual) return;
      const tick = (timestamp) => {
        if (!this.running) return;
        this.tickAll(timestamp);
        if (this.callbacks.size > 0) {
          this.frameId = requestAnimationFrame(tick);
        } else {
          this.running = false;
        }
      };
      this.frameId = requestAnimationFrame(tick);
    },

    /**
     * Drive every registered callback once with `timestamp`. Used by both the
     * real rAF loop and the virtual clock (ST._clock).
     */
    tickAll(timestamp) {
      if (ST._timing && ST._timing.recordRAF) {
        ST._timing.recordRAF(timestamp);
      }
      for (const cb of this.callbacks) {
        try {
          cb(timestamp);
        } catch (e) {
          console.error('[ST] RAF callback error:', e);
        }
      }
    },

    /**
     * Stop the RAF loop
     */
    stop() {
      this.running = false;
      if (this.frameId) {
        cancelAnimationFrame(this.frameId);
        this.frameId = null;
      }
    }
  };

  // ==========================================================================
  // Virtual clock (PLAN-027 W5) — deterministic animation testing.
  // ==========================================================================
  //
  // When installed, ST._raf stops scheduling real rAF; instead the harness steps
  // virtual time via advance()/runToEnd()/sampleFrames(). Every step drives all
  // registered rAF callbacks with a controlled monotonically-increasing
  // timestamp, so easing curves, stagger timing and driver progress are
  // reproducible — no wall-clock flake. Real layout (CDP) still applies between
  // steps; only TIME is virtualised.
  ST._clock = ST._clock || {
    virtual: false,
    now: 0,
    frameMs: 16,

    // Switch ST._raf into virtual mode. Idempotent.
    install(frameMs) {
      this.virtual = true;
      this.now = 0;
      if (typeof frameMs === 'number' && frameMs > 0) this.frameMs = frameMs;
      // If a real loop was running, stop it; callbacks remain registered.
      ST._raf.running = false;
      if (ST._raf.frameId) {
        try { cancelAnimationFrame(ST._raf.frameId); } catch (_e) {}
        ST._raf.frameId = null;
      }
    },

    // Leave virtual mode (restore real rAF on next add()).
    uninstall() {
      this.virtual = false;
      this.now = 0;
    },

    // Advance virtual time by `ms`, ticking all callbacks once per frame.
    // Returns the number of frames stepped.
    advance(ms) {
      if (!this.virtual) this.install();
      const target = this.now + Math.max(0, ms);
      let frames = 0;
      while (this.now < target) {
        this.now = Math.min(target, this.now + this.frameMs);
        ST._raf.tickAll(this.now);
        frames++;
        if (frames > 100000) break; // safety
      }
      return frames;
    },

    // Step exactly `n` frames (frameMs each).
    sampleFrames(n) {
      if (!this.virtual) this.install();
      for (let i = 0; i < (n || 1); i++) {
        this.now += this.frameMs;
        ST._raf.tickAll(this.now);
      }
      return this.now;
    },

    // Run until no driver reports progress < 1 (or a max-time guard). Relies on
    // ST._timing.driverUpdates to detect completion; falls back to a frame cap.
    runToEnd(maxMs) {
      if (!this.virtual) this.install();
      const cap = (typeof maxMs === 'number' ? maxMs : 10000);
      const end = this.now + cap;
      let stable = 0;
      while (this.now < end) {
        this.now += this.frameMs;
        ST._raf.tickAll(this.now);
        // Stop once no callbacks remain (all drivers self-removed on complete).
        if (ST._raf.callbacks.size === 0) { stable++; if (stable >= 2) break; }
        else stable = 0;
      }
      return this.now;
    }
  };

})();
