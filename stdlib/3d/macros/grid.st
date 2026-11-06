/// Grid Macro — a light-up dance floor (PLAN-152).
///
/// @example a ripple floor that pulses on the kick
/// canvas.disco {
///   @stage(shadows: "soft") {
///     @grid &floor (cols: 10, rows: 10, pattern: "ripple")
///   }
/// }
/// .reel { @on &.clip { &floor { pulse: 0.3 -> 1 --steps(32); } } }
///
/// @param cols/rows - tiles across / deep
/// @param tile/gap  - tile size and spacing (world units)
/// @param palette   - space-separated colours
/// @param pattern   - checker | ripple | sweep | random | chase
/// @param glow      - a lit tile's emissive strength
/// @param speed     - the pattern's speed (when `phase` isn't driven)

%macro grid {
  %scope selector

  %capture_type grid_name { $name:element }

  %form {
    @grid $named:grid_name? (
      cols: $cols:number = 8,
      rows: $rows:number = 8,
      tile: $tile:number = 0.9,
      gap: $gap:number = 0.06,
      palette: $palette:string = "#ff3ea5 #3ef0ff #ffe23e #7c4dff",
      pattern: $pattern:string = "ripple",
      glow: $glow:number = 2.5,
      speed: $speed:number = 1,
      posY: $posY:number = 0
    ) {
      $children*
    }
  }

  %binds {
    three-grid(&self, name: $named, cols: $cols, rows: $rows, tile: $tile, gap: $gap,
      palette: $palette, pattern: $pattern, glow: $glow, speed: $speed, posY: $posY) -> {
      $added
    }
  }

  %template {
    $children
  }
}
