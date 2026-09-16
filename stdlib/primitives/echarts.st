/// ECharts Chart Primitive
///
/// Mounts an Apache ECharts instance onto an element and keeps its `option`
/// reactive: whenever the bound option signal changes, the chart re-renders via
/// `setOption` (ECharts' own diffing merges the update, so a data refresh is not
/// a full teardown). This is the low-level behaviour the grammar-of-graphics
/// `@chart` macro-module (stdlib/macros/echarts) desugars onto — a page author
/// writes the grammar, this instantiates the library.
///
/// The chart's OPTION is a full ECharts option map. The GoG macro builds that
/// map from a geom + aesthetic mappings; a caller may also pass a hand-built
/// option for a chart the grammar does not yet cover.
///
/// @param &el     - The container element the chart mounts into
/// @param option  - The initial ECharts option map
/// @param theme   - Optional ECharts theme name (registered theme, or "")
/// @yields $chart  - The ECharts instance handle (for imperative escape hatches)
/// @yields $ready  - True once the instance has mounted
/// @yields setOption - A function to push a new option map (data refresh)
///
/// Reactivity model: the primitive renders the initial `option` and yields a
/// `setOption` function. A page refreshes the chart by calling that function
/// with a new option map (e.g. from an `@handle` arm when analytics data
/// arrives) — ECharts' own diffing merges the update rather than rebuilding.
/// This follows the established yielded-function pattern (cf. websocket's
/// `send`) rather than a bespoke observe directive.

%primitive echarts-chart(&el, option: any, theme: string = "") {
  %emit js {
    const el = %&el;

    // ECharts is a peer dependency the host page loads (a <script> or a bundle
    // import exposing `echarts`). Fail loud-but-safe if it is absent, rather
    // than throwing deep inside a render and blanking the page.
    if (typeof echarts === "undefined") {
      console.error("echarts-chart: global `echarts` is not loaded on the page");
      %yield false -> $ready;
      return;
    }

    const theme = %theme;
    const chart = theme ? echarts.init(el, theme) : echarts.init(el);
    %yield chart -> $chart;
    %yield true -> $ready;

    // `notMerge:false` lets ECharts diff a data refresh against the live option
    // instead of tearing the chart down and rebuilding it.
    const render = (opt) => {
      if (opt && typeof opt === "object") {
        chart.setOption(opt, { notMerge: false });
      }
    };
    render(%option);

    // The data-refresh handle: a page calls this with a new option map.
    %yield render -> setOption;

    // Keep the canvas correct through container resizes — a dashboard panel
    // that reflows must not leave a stretched chart.
    const ro = new ResizeObserver(() => chart.resize());
    ro.observe(el);

    %cleanup {
      ro.disconnect();
      chart.dispose();
    }
  }
}
