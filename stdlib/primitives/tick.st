/// Animation Frame Primitive
///
/// Wraps requestAnimationFrame for smooth animation loops.
/// Optionally throttles to a target FPS.
///
/// @param fps - Target frames per second (0 = native refresh rate)
/// @param autoStart - Start immediately, default true
///
/// @yields $t - Current timestamp (ms since page load)
/// @yields $dt - Delta time since last frame (ms)
/// @yields $frame - Frame count since start
/// @yields $elapsed - Total elapsed time (ms)
/// @yields $running - Whether the loop is currently running

%primitive tick(
  fps: number = 0,
  autoStart: bool = true
) {
  %emit js {
    let running = %autoStart;
    let frameId = null;
    let startTime = performance.now();
    let lastTime = startTime;
    let frameCount = 0;
    let accumulated = 0;

    const targetInterval = %fps > 0 ? 1000 / %fps : 0;

    const loop = (now) => {
      if (!running) return;

      const rawDt = now - lastTime;

      if (targetInterval > 0) {
        // Throttled mode: accumulate time and only tick at target FPS
        accumulated += rawDt;

        if (accumulated >= targetInterval) {
          frameCount++;

          %yield now -> $t;
          %yield targetInterval -> $dt;
          %yield frameCount -> $frame;
          %yield now - startTime -> $elapsed;
          %yield true -> $running;

          // FPS calculation
          const actualFps = 1000 / rawDt;
          %yield actualFps -> $fps;

          accumulated -= targetInterval;
        }
      } else {
        // Native mode: tick every frame
        frameCount++;

        %yield now -> $t;
        %yield rawDt -> $dt;
        %yield frameCount -> $frame;
        %yield now - startTime -> $elapsed;
        %yield true -> $running;

        const actualFps = rawDt > 0 ? 1000 / rawDt : 60;
        %yield actualFps -> $fps;
      }

      lastTime = now;
      frameId = requestAnimationFrame(loop);
    };

    // Control functions exposed as yields
    const start = () => {
      if (running) return;
      running = true;
      startTime = performance.now();
      lastTime = startTime;
      frameCount = 0;
      accumulated = 0;
      frameId = requestAnimationFrame(loop);
      %yield true -> $running;
    };

    const stop = () => {
      running = false;
      if (frameId) {
        cancelAnimationFrame(frameId);
        frameId = null;
      }
      %yield false -> $running;
    };

    const reset = () => {
      startTime = performance.now();
      lastTime = startTime;
      frameCount = 0;
      accumulated = 0;
      %yield 0 -> $frame;
      %yield 0 -> $elapsed;
    };

    // Expose control methods
    %yield start -> $start;
    %yield stop -> $stop;
    %yield reset -> $reset;

    if (%autoStart) {
      frameId = requestAnimationFrame(loop);
    }

    %cleanup {
      running = false;
      if (frameId) {
        cancelAnimationFrame(frameId);
      }
    }
  }

  %exports {
    $t: number         // current timestamp (ms)
    $dt: number        // delta time (ms)
    $frame: number     // frame count
    $elapsed: number   // total elapsed time (ms)
    $running: bool     // is loop running
    $fps: number       // actual frames per second
    $start: fn() ~> effect         // start the loop
    $stop: fn() ~> effect          // stop the loop
    $reset: fn() ~> effect         // reset counters
  }
}


/// Interval Primitive
///
/// Simple interval timer (like setInterval but reactive).
///
/// @param interval - Interval duration in ms
/// @param autoStart - Start immediately, default true
///
/// @yields $count - Number of times interval has fired
/// @yields $running - Whether interval is running

%primitive interval(
  interval: number,
  autoStart: bool = true
) {
  %emit js {
    let count = 0;
    let running = %autoStart;
    let timerId = null;

    const tick = () => {
      count++;
      %yield count -> $count;
    };

    const start = () => {
      if (running) return;
      running = true;
      timerId = setInterval(tick, %interval);
      %yield true -> $running;
    };

    const stop = () => {
      running = false;
      if (timerId) {
        clearInterval(timerId);
        timerId = null;
      }
      %yield false -> $running;
    };

    const reset = () => {
      count = 0;
      %yield 0 -> $count;
    };

    %yield start -> $start;
    %yield stop -> $stop;
    %yield reset -> $reset;

    if (%autoStart) {
      timerId = setInterval(tick, %interval);
    }

    %cleanup {
      if (timerId) {
        clearInterval(timerId);
      }
    }
  }

  %exports {
    $count: number     // tick count
    $running: bool     // is running
    $start: fn() ~> effect         // start interval
    $stop: fn() ~> effect          // stop interval
    $reset: fn() ~> effect         // reset counter
  }
}
