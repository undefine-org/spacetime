@template &hero($title, $subtitle, &slot) {
  <section class="hero"><h1>`$title`</h1><p>`$subtitle`</p></section>
}
@template &card($name, $body?) {
  <article class="card"><h3>`$name`</h3></article>
}
@reveal fade(duration: 600, delay: 100) { }
@on &.scroll(name: parallax, scope: cover, start: 0, end: 1, scrub: true) { }
