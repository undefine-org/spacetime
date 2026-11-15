/// A message keeps its anatomy, surface and arrival independent.
@form markup --message(&content, $fill color = "#eaf0f5") {
  <article class="sample-message" style="--message-fill: `$fill`"><span class="sample-message__avatar">R</span><div><b>Research Bot</b><p>`&content`</p></div></article>
}
/// A light's strength is a typed quantity, independent of its placement.
@form style --light-surface($intensity number = 1.8) {
  --sample-intensity: $intensity;
}
/// Translation, focus and visibility are one reusable motion treatment.
@form motion --arrival($distance length = 80px, $softness length = 12px) {
  translate-y: $distance -> 0px;
  blur: $softness -> 0px;
  opacity: 0 -> 1;
  easing: --ease-out-cubic;
}
