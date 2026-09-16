/// ECharts option builder — grammar of graphics → ECharts option map
///
/// The translator under the `@chart <geom>` macro-module. Given a geom and the
/// aesthetic mappings (which data column drives x / y / colour / size / weight),
/// it assembles a full ECharts `option` from a `data` list of row objects. The
/// macro layer supplies the geom string and column names; this is where a
/// grammar-of-graphics spec becomes the concrete option ECharts renders.
///
/// The one non-obvious job is the COLOUR aesthetic: mapping a column to colour
/// means SPLITTING the rows into one series per distinct colour value (ECharts'
/// model is "a series is a group"), so a coloured line/bar becomes several
/// series sharing an axis. Without a colour mapping there is a single series.
///
/// @param geom   - "line" | "bar" | "area" | "scatter" | "heatmap"
/// @param data   - a list of row objects (the resolved data)
/// @param x      - the column name for the x aesthetic
/// @param y      - the column name for the y aesthetic
/// @param color  - optional column name → one series per distinct value
/// @param size   - optional column name → mark size (scatter)
/// @param weight - optional column name → intensity (heatmap)
/// @yields $option - the assembled ECharts option map (reactive to `data`)

%primitive echarts-option(
  geom: string = "line",
  data: any = [],
  x: string = "",
  y: string = "",
  color: string = "",
  size: string = "",
  weight: string = ""
) {
  %emit js {
    const geom = %geom, xCol = %x, yCol = %y, colorCol = %color,
          sizeCol = %size, weightCol = %weight;

    // ECharts wants the geom under a `type`; "area" is a line with an areaStyle.
    const seriesType = (geom === "area") ? "line" : geom;
    const isArea = (geom === "area");

    const distinct = (rows, col) => {
      const seen = [];
      for (const r of rows) {
        const v = r[col];
        if (!seen.includes(v)) seen.push(v);
      }
      return seen;
    };

    const build = (rows) => {
      rows = Array.isArray(rows) ? rows : [];

      // Heatmap: [xIndex, yIndex, weight] cells over two categorical axes.
      if (geom === "heatmap") {
        const xs = distinct(rows, xCol);
        const ys = distinct(rows, yCol);
        const cells = rows.map((r) => [
          xs.indexOf(r[xCol]),
          ys.indexOf(r[yCol]),
          r[weightCol],
        ]);
        return {
          tooltip: { position: "top" },
          xAxis: { type: "category", data: xs },
          yAxis: { type: "category", data: ys },
          visualMap: { min: 0, max: Math.max(1, ...cells.map((c) => c[2] || 0)), calculable: true },
          series: [{ type: "heatmap", data: cells }],
        };
      }

      const categories = distinct(rows, xCol);

      // Colour aesthetic → one series per distinct colour value; else a single
      // series over all rows.
      let series;
      let legend = undefined;
      if (colorCol) {
        const groups = distinct(rows, colorCol);
        legend = { data: groups };
        series = groups.map((g) => {
          const gr = rows.filter((r) => r[colorCol] === g);
          const byX = new Map(gr.map((r) => [r[xCol], r]));
          return {
            name: g,
            type: seriesType,
            areaStyle: isArea ? {} : undefined,
            data: categories.map((c) => {
              const r = byX.get(c);
              if (!r) return null;
              return sizeCol ? [c, r[yCol], r[sizeCol]] : r[yCol];
            }),
          };
        });
      } else {
        const byX = new Map(rows.map((r) => [r[xCol], r]));
        series = [{
          type: seriesType,
          areaStyle: isArea ? {} : undefined,
          symbolSize: sizeCol
            ? ((val) => Array.isArray(val) ? Math.max(4, Math.sqrt(val[2] || 1)) : 6)
            : undefined,
          data: categories.map((c) => {
            const r = byX.get(c);
            if (!r) return null;
            return sizeCol ? [c, r[yCol], r[sizeCol]] : r[yCol];
          }),
        }];
      }

      const scatter = (seriesType === "scatter");
      return {
        tooltip: { trigger: scatter ? "item" : "axis" },
        legend: legend,
        xAxis: scatter ? { type: "category", data: categories } : { type: "category", data: categories },
        yAxis: { type: "value" },
        series: series,
      };
    };

    %yield build(%data) -> $option;
  }
}
