/// BUG: Revelation headline clipped behind nav at desktop viewports
///
/// The "You can have it today." headline in Section 4 (Revelation) gets clipped
/// behind the 65px sticky nav bar at desktop viewports (>= 1024px wide).
///
/// Root cause: `.ora-revelation__sticky` uses `justify-content: center` to
/// vertically center `.ora-revelation__content`, but the content block includes
/// the timeline (~490px) which takes layout space even when invisible (opacity: 0).
/// This leaves minimal top margin. The Phase 2 scroll animation
/// (`translate-y: 0 -> -40px`) then pushes the headline behind the nav.
///
/// Fix: At >= 1024px, switch to `justify-content: flex-start` with
/// `padding-top: max(15vh, 110px)` so the headline starts well below the nav.
/// After the -40px Phase 2 shift, headline top >= 70px, clearing the 65px nav.
///
/// Since headless tests can't evaluate media queries, the test applies the fix
/// styles inline to validate the positioning math directly.

// =============================================================================
// Test: headline remains visible after Phase 2 shift at desktop viewports
// =============================================================================

@test "revelation headline stays above nav after -40px shift (desktop fix)" needs layout {
  @fixture {
    <div class="ora-revelation__sticky"
         style="position:relative; height:768px; width:1024px;
                display:flex; flex-direction:column; align-items:center;
                justify-content:flex-start; padding-top:max(15vh, 110px);
                overflow:hidden;">
      <div class="ora-revelation__content"
           style="position:relative; text-align:center; width:100%; max-width:900px;">
        <h2 class="ora-revelation__headline"
            style="font-size:52px; line-height:1.0; margin-bottom:4rem;
                   transform:translateY(-40px);">
          You can have it today.
        </h2>
        <div class="ora-timeline" style="opacity:0; height:490px;">
          <div class="ora-timeline__track"></div>
        </div>
      </div>
    </div>
  }

  @eval (eval("                                                              \
    var headline = document.querySelector('.ora-revelation__headline');        \
    var rect = headline.getBoundingClientRect();                               \
    window.__headlineTop = rect.top;                                          \
  "))

  // Nav is 65px tall — headline must be fully below it after the -40px shift
  @assert (window.__headlineTop >= 65)
    "Headline top must be >= 65px (nav height) after Phase 2 shift"
}

@test "revelation headline has adequate clearance at 1280x800 viewport" needs layout {
  @fixture {
    <div class="ora-revelation__sticky"
         style="position:relative; height:800px; width:1280px;
                display:flex; flex-direction:column; align-items:center;
                justify-content:flex-start; padding-top:max(15vh, 110px);
                overflow:hidden;">
      <div class="ora-revelation__content"
           style="position:relative; text-align:center; width:100%; max-width:900px;">
        <h2 class="ora-revelation__headline"
            style="font-size:52px; line-height:1.0; margin-bottom:4rem;
                   transform:translateY(-40px);">
          You can have it today.
        </h2>
        <div class="ora-timeline" style="opacity:0; height:490px;">
          <div class="ora-timeline__track"></div>
        </div>
      </div>
    </div>
  }

  @eval (eval("                                                              \
    var headline = document.querySelector('.ora-revelation__headline');        \
    var rect = headline.getBoundingClientRect();                               \
    window.__headlineTop1280 = rect.top;                                      \
  "))

  @assert (window.__headlineTop1280 >= 65)
    "Headline top must be >= 65px at 1280x800 after Phase 2 shift"
}

@test "soft-sticky grows above viewport when content is taller" needs layout {
  @fixture {
    <section class="soft-sticky-shell" style="height:2400px; width:390px; position:relative;">
      <div class="soft-sticky"
           style="position:sticky; top:0; min-height:100vh; height:max-content; overflow:visible; display:flex; flex-direction:column;">
        <div class="soft-sticky-content" style="height:1200px;">Long content block</div>
      </div>
    </section>
  }

  @eval (eval("                                                              \
    var sticky = document.querySelector('.soft-sticky');                          \
    var rect = sticky.getBoundingClientRect();                                    \
    window.__softStickyTallHeight = rect.height;                                  \
    window.__softStickyViewport = window.innerHeight;                             \
  "))

  @assert (window.__softStickyTallHeight > window.__softStickyViewport)
    "Sticky height should exceed viewport when content is taller"
}

@test "soft-sticky stays at viewport height when content fits" needs layout {
  @fixture {
    <section class="soft-sticky-shell" style="height:2400px; width:390px; position:relative;">
      <div class="soft-sticky"
           style="position:sticky; top:0; min-height:100vh; height:max-content; overflow:visible; display:flex; flex-direction:column;">
        <div class="soft-sticky-content" style="height:220px;">Short content block</div>
      </div>
    </section>
  }

  @eval (eval("                                                              \
    var sticky = document.querySelector('.soft-sticky');                          \
    var rect = sticky.getBoundingClientRect();                                    \
    window.__softStickyShortHeight = rect.height;                                 \
    window.__softStickyShortViewport = window.innerHeight;                        \
  "))

  @assert (window.__softStickyShortHeight >= window.__softStickyShortViewport)
    "Sticky height should be at least viewport height when content fits"
}
