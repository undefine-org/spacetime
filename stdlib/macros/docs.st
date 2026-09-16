/// Documentation macros (FEAT-082) — build docs that ARE the feature.
///
/// These rely on `%emit html`: a userland macro that emits markup into the page
/// body, with capture references substituted (`%$name`, `%$block.html` for a
/// captured block's live markup, `%$block.source` for its HTML-escaped source).

/// @example — dual-render a Spacetime snippet: the LIVE result next to its own
/// SOURCE, from ONE block. The live pane is the real markup running; the source
/// pane is the same block, HTML-escaped. They can never disagree.
///
/// @example
/// .demos { @example "A button" { <button class="btn">Press</button> } }
///
/// Renders a <figure> with a caption, the live <button>, and a <pre> showing
/// `<button class="btn">Press</button>` as text. Place the directive in a
/// selector block; the figure is emitted into the page body.
%macro example {
  %scope file | selector
  %order 250

  %form {
    @example $title:string { $demo:block }
  }

  %emit html {
    <figure class="st-example">
      <figcaption class="st-example__title">%$title</figcaption>
      <div class="st-example__live">%$demo.html</div>
      <pre class="st-example__src"><code>%$demo.source</code></pre>
    </figure>
  }
}
