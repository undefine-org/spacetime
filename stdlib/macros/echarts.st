/// ECharts grammar-of-graphics — `@chart <geom> { … }`
///
/// A chart is a GEOM (how marks are drawn) over AESTHETIC MAPPINGS (which data
/// column drives x, y, colour, …). That is Wilkinson's grammar of graphics, and
/// it is exactly the shape a declarative page wants:
///
///   @chart line { data: $spend; x: day; y: cents; }
///   @chart bar  { data: $byModel; x: model; y: cents; color: provider; }
///   @chart area { data: $usage; x: t; y: tokens; }
///   @chart point{ data: $lat; x: at; y: ms; size: tokens; }
///
/// THE GEOM IS THE MACRO: each geom word (line/bar/area/point/heatmap) is its
/// own `%macro` whose `%form` carries the literal word, so the parser selects by
/// specificity — the same "the kind is the macro" discipline `@data <kind>` uses.
/// Every geom desugars to the ONE `echarts-option(...)` primitive call that
/// assembles an ECharts option map from the aesthetics, then hands it to the
/// `echarts-chart` primitive which mounts and renders it.
///
/// The mapping fields are column NAMES into the bound `data` (a list of row
/// objects, e.g. the resolved rows a server sent). `color`/`size`/`facet` are
/// optional aesthetics; absent ones simply do not enter the option.
///
/// This is the ergonomic layer over the raw `echarts-chart` primitive
/// (stdlib/primitives/echarts): a page author writes the grammar, never an
/// ECharts option map by hand — unless they want to, in which case the primitive
/// is still there.

@import "stdlib/primitives/echarts"

/// @chart line — a line geom. x/y are required column names; color optional
/// (splits into one series per colour value).
%macro chart-line {
  %scope selector
  %order 250

  %form {
    @chart line {
      data: $data:expr ;
      x: $x:ident ;
      y: $y:ident ;
      color: $color:ident? ;
    }
  }

  %binds {
    echarts-option(
      geom: "line",
      data: $data,
      x: $x,
      y: $y,
      color: $color
    ) -> { $option }

    echarts-chart(&self, option: $option) -> {
      $chart, $ready, setOption
    }
  }

  %registers chart($x, geom: "line") {
    y: $y
    color: $color
  }
}

/// @chart bar — a bar geom. color groups bars into series (a stacked/grouped
/// categorical comparison).
%macro chart-bar {
  %scope selector
  %order 250

  %form {
    @chart bar {
      data: $data:expr ;
      x: $x:ident ;
      y: $y:ident ;
      color: $color:ident? ;
    }
  }

  %binds {
    echarts-option(
      geom: "bar",
      data: $data,
      x: $x,
      y: $y,
      color: $color
    ) -> { $option }

    echarts-chart(&self, option: $option) -> {
      $chart, $ready, setOption
    }
  }

  %registers chart($x, geom: "bar") {
    y: $y
    color: $color
  }
}

/// @chart area — a filled line geom (a line with `areaStyle`), for cumulative or
/// volume-over-time reads.
%macro chart-area {
  %scope selector
  %order 250

  %form {
    @chart area {
      data: $data:expr ;
      x: $x:ident ;
      y: $y:ident ;
      color: $color:ident? ;
    }
  }

  %binds {
    echarts-option(
      geom: "area",
      data: $data,
      x: $x,
      y: $y,
      color: $color
    ) -> { $option }

    echarts-chart(&self, option: $option) -> {
      $chart, $ready, setOption
    }
  }

  %registers chart($x, geom: "area") {
    y: $y
    color: $color
  }
}

/// @chart point — a scatter geom. `size` maps a column to mark size (a third
/// quantitative channel), optional.
%macro chart-point {
  %scope selector
  %order 250

  %form {
    @chart point {
      data: $data:expr ;
      x: $x:ident ;
      y: $y:ident ;
      color: $color:ident? ;
      size: $size:ident? ;
    }
  }

  %binds {
    echarts-option(
      geom: "scatter",
      data: $data,
      x: $x,
      y: $y,
      color: $color,
      size: $size
    ) -> { $option }

    echarts-chart(&self, option: $option) -> {
      $chart, $ready, setOption
    }
  }

  %registers chart($x, geom: "point") {
    y: $y
    color: $color
    size: $size
  }
}

/// @chart heatmap — a heatmap geom over two categorical axes (x, y) with an
/// intensity column (`weight`). The canonical hit-rate/latency-by-bucket view.
%macro chart-heatmap {
  %scope selector
  %order 250

  %form {
    @chart heatmap {
      data: $data:expr ;
      x: $x:ident ;
      y: $y:ident ;
      weight: $weight:ident ;
    }
  }

  %binds {
    echarts-option(
      geom: "heatmap",
      data: $data,
      x: $x,
      y: $y,
      weight: $weight
    ) -> { $option }

    echarts-chart(&self, option: $option) -> {
      $chart, $ready, setOption
    }
  }

  %registers chart($x, geom: "heatmap") {
    y: $y
    weight: $weight
  }
}
