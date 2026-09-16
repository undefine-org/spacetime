/// `@scatter(from: &src, count: N) { :shape { … } }` — positions from a shape
/// (PLAN-150 W5, film.st.md §6).
///
/// A particle system needs to know WHERE its particles go. CSS trigonometry
/// over `sibling-index()` gives a generic burst; it cannot know where the
/// glyphs of a word are. `@scatter` samples a SOURCE (`from:`) — its rendered
/// glyph coverage — and stamps per-child custom properties naming those
/// positions, so `<script>` can shatter into its own letters.
///
/// It has SLOTS, like `@each`'s `:entering`/`:exiting`, because a particle
/// system has more than one arrangement:
///
///   .burst {
///     @scatter(from: &tag, count: 400) {
///       :shape { --x; --y; }     // sampled from the rendered glyphs of &tag
///       :rest  { --rx; --ry; }   // where a particle starts (default: :shape)
///     }
///     @on &.clip { --tau: 0 -> 1; }
///   }
///   .burst i {
///     translate: calc(var(--x) + cos(sibling-index() * 137.5deg) * 400px * var(--tau))
///                calc(var(--y) + 620px * var(--tau) * var(--tau));
///   }
///
/// `@scatter` stamps custom properties ONLY; motion stays in `@on`, and the
/// per-particle geometry stays CSS. The slot bodies name which properties to
/// stamp — `:shape { --x; --y; }` binds `--x`/`--y` to the sampled point; the
/// two-property form is the common case (x, y). Positions are sampled at
/// hydration from a rasterised copy of the source via a canvas coverage pass.
///
/// @param from    - element selector to sample (`&tag`, `.headline`)
/// @param count   - number of particles / children to stamp (default 300)
/// @param x/y     - property names for the sampled point (default --x/--y)

%macro scatter {

  %scope selector

  // A slot MAPS the custom properties to stamp to a sample axis:
  //   :shape { --x: sample-x; --y: sample-y; }
  // `sample-x`/`sample-y` are the sampled point's coordinates (px, relative to
  // the scatter host); `--x`/`--y` are whatever property names the author's CSS
  // reads. The slots are PSEUDO-SELECTOR body params (`(:shape { … })`), exactly
  // like `@each`'s `:entering`/`:exiting`, captured as a `properties` map.
  //
  // Why a mapping and not a bare `--x;` list: a bare `--x;` collides with the
  // form-splice grammar (`--role;`) — the block would parse as a scope and each
  // name become an unknown form. One sigil, one meaning: `--x:` introduces a
  // VALUE (here the sample axis), which is exactly what a slot declares.
  %form {
    @scatter(
      from: $from:element,
      count: $count:number = 300
    ) {
      (:shape { $shape:properties })
      (:rest  { $rest:properties })?
    }
  }

  %binds {
    scatter-sample(
      &self,
      from: $from,
      count: $count,
      shape: $shape,
      rest: $rest
    ) -> { $ready }
  }
}
