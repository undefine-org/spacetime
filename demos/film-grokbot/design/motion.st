/// A clip owns its visibility. Tiny edge ramps make cuts seekable without
/// leaving waiting or finished scenes visible. All timing lives in the score.
@form motion --film-window {
  opacity: 0 -> 1 at 0.01% -> 1 at 99.99% -> 0;
}
/// Translation, easing and focus travel together. The blur envelope is an
/// authored lens treatment, deterministic on seek, not a velocity estimator.
/// Override distance and softness at the use site, independently of layout.
@form motion --film-arrive($distance = 80px, $softness = 12px) {
  translate-y: $distance -> 0px at 28% -> 0px;
  blur: $softness -> 0px at 28% -> 0px;
  opacity: 0 -> 1 at 20% -> 1;
  easing: --ease-out-cubic;
}
@form motion --film-focus($softness = 14px) {
  blur: $softness -> 0px at 22% -> 0px;
  opacity: 0 -> 1 at 18% -> 1;
  scale: 1.04 -> 1 at 22% -> 1;
  easing: --ease-out-cubic;
}
@form motion --film-pan($from = 160px, $to = -160px) {
  translate-x: $from -> $to;
  easing: --ease-in-out-cubic;
}
@form motion --film-pop {
  scale: 0.01 -> 1.08 at 18% -> 1 at 30% -> 1;
  opacity: 0 -> 1 at 10% -> 1;
  blur: 8px -> 0px at 18% -> 0px;
  easing: --ease-out-cubic;
}
@form motion --film-pointer {
  translate-x: -270px -> 0px at 48% -> 26px at 78% -> 0px;
  translate-y: 170px -> 0px at 48% -> -20px at 78% -> 0px;
  blur: 5px -> 0px at 48% -> 0px;
  easing: --ease-in-out-cubic;
}
