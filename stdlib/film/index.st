/// A shipped film vocabulary (PLAN-150 W12, film.st.md §12).
///
/// Everything in a title sequence is a handful of moves. They ship here as
/// named stdlib forms so a film's first draft is one line per shot — these are
/// NOT new syntax, they are the promo's repeated patterns, named. The list
/// grows by adding forms; each is built only from the landed film surface
/// (§2 positioned stops, §3 @post, §4 camera, §6/§7 noise).
///
/// @example a title that focus-pulls in over a cinematic grade with a subtle shake
///   h1.title { @on &.clip { --focus-pull; } }
///   .stage   { @post --cinema; @on &.clip { --shake; } }

// --- Motion moves ----------------------------------------------------------

/// Rack-focus in: blur resolves, letter-spacing tightens, a small rise + fade.
/// The classic "the title arrives" move.
@form motion --focus-pull($from = 24px) {
  blur: $from -> 0;
  letter-spacing: 0.3em -> 0.01em;
  translate-y: 16px -> 0;
  opacity: 0 -> 1;
  easing: --ease-out-expo;
}

/// A light-sweep across the type (drive an animated `--pos` in a
/// `background-clip: text` gradient). The shimmer that says "premium".
@form motion --shimmer($width = 16%) {
  --pos: -120% -> 230%;
  easing: --ease-in-out-sine;
}

/// A typewriter reveal: advance a `--chars` count in stepped time (pair with
/// `clip-path: inset(… var(--chars) …)`), one glyph per step.
@form motion --typewriter($chars = 20) {
  --chars: 0 -> $chars;
  easing: --steps($chars);
}

/// A glitch: a quick horizontal jitter. Add skew/hue in the consuming block for
/// the full RGB-split look; the core is the position stutter.
@form motion --glitch($amount = 14px) {
  translate-x: 0 -> $amount -> calc(-0.7 * $amount) -> 0;
}

/// A slow drift, as data not hand-typed stops: 1-D noise on x and y with an
/// independent seed per axis (§7). Idle motion that never repeats obviously.
@form motion --drift($amp = 6px) {
  translate-x: --noise-1d($amp, 2);
  translate-y: --noise-1d($amp, 2, seed: 7);
}

// --- The lens --------------------------------------------------------------

/// The cinematic grade: bloom, film grain, an edge vignette. One `@post` look,
/// named. Apply with `@post --cinema;` (§3).
@form post --cinema {
  bloom: 0.45;
  grain: 0.035;
  vignette: 0.55;
}

// --- Camera moves ----------------------------------------------------------

/// Handheld shake: a small positional stutter that settles (§4). The camera
/// equivalent of `--glitch`, on the whole frame.
@form camera --shake($amount = 8px) {
  position: 0 0 0 -> $amount 0 0 -> calc(-0.75 * $amount) 0 0 -> 0 0 0;
}

/// A slow push-in (dolly): the frame eases toward the subject over its window.
@form camera --push-in($from = 12, $to = 6) {
  position: 0 0 $from -> 0 0 $to;
}
