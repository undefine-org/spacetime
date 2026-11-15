/// A shared surface with one typed default and one content slot.
@form markup --card(&content, $accent color = "#70864c") {
  <article class="card" style="--card-accent: `$accent`">`&content`</article>
}
