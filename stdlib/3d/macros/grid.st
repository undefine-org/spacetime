/// Grid Macro — a light-up dance floor (PLAN-152).

%macro grid {
  %scope selector

  %capture_type grid_name { $name:element }

  %form {
    @grid $named:grid_name? (
      /// tiles across / deep
      cols: $cols:number = 8,
      /// tiles across / deep
      rows: $rows:number = 8,
      /// tile size and spacing (world units)
      tile: $tile:number = 0.9,
      /// tile size and spacing (world units)
      gap: $gap:number = 0.06,
      /// space-separated colours
      palette: $palette:string = "#ff3ea5 #3ef0ff #ffe23e #7c4dff",
      /// checker | ripple | sweep | random | chase
      pattern: $pattern:string = "ripple",
      /// a lit tile's emissive strength
      glow: $glow:number = 2.5,
      /// the pattern's speed (when `phase` isn't driven)
      speed: $speed:number = 1,
      /// the floor's height
      posY: $posY:number = 0
    ) {
      $children*
    }
  }

  %binds {
    three-grid(&self, name: $named, ...) -> {
      $added
    }
  }

  %template {
    $children
  }

  %example "a ripple floor that pulses on the kick" {
    canvas.disco {
      @stage(shadows: "soft") {
        @grid &floor (cols: 10, rows: 10, pattern: "ripple")
      }
    }
    .reel { @on &.clip { &floor { pulse: 0.3 -> 1 --steps(32); } } }
  }
}
