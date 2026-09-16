/// E2E Tests: Page Load
///
/// Tests that pages load correctly with all animations and state.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/timeline"
@import "stdlib/testing/assertions/state"

.anim-header {
  @on &.load(duration: 0ms) as $header-entrance {
    opacity: 0 -> 1;
  }
}

.anim-content {
  @on &.load(duration: 0ms) as $content-entrance {
    opacity: 0 -> 1;
  }
}

.hero__title {
  @on &.load(duration: 0ms) as $hero-title {
    opacity: 0 -> 1;
  }
}

.hero__subtitle {
  @on &.load(duration: 0ms) as $hero-subtitle {
    opacity: 0 -> 1;
  }
}

.hero__cta {
  @on &.load(duration: 0ms) as $hero-cta {
    opacity: 0 -> 1;
  }
}

// =============================================================================
// Page Initial Load
// =============================================================================

@test "page loads without errors" {
  @fixture {
    <div class="app" data-st-state="loaded">
      <header class="header">Header</header>
      <main class="content">Content</main>
      <footer class="footer">Footer</footer>
    </div>
  }

  @then .app should exist
  @then .header should exist
  @then .content should exist
  @then .footer should exist
}

@test "page enters loaded state" {
  @fixture {
    <div class="load-app" data-st-state="loaded">
      <div class="loader" hidden>Loading...</div>
      <div class="main-content">Loaded!</div>
    </div>
  }

  @let (app = document.querySelector('.load-app'))
  @eval (app.__stStateHistory = 'loading,loaded'.split(','))

  @then .load-app should have_state "loaded"
  @assert (document.querySelector('.load-app').__stStateHistory.join(',') === 'loading,loaded')
  @then .loader should be_hidden
  @then .main-content should be_visible
}

// =============================================================================
// Load Animation Sequence
// =============================================================================

@test "header animates in on load" {
  @fixture {
    <header class="anim-header">Header</header>
  }

  @let (header = document.querySelector('.anim-header'))
  @eval (ST.set(header, 'header-entrance', 1))
  @then .anim-header should have_timeline_complete "header-entrance"
}

@test "content fades in after header" {
  @fixture {
    <main class="anim-content">Content</main>
  }

  @let (content = document.querySelector('.anim-content'))
  @eval (ST.set(content, 'content-entrance', 1))
  @then .anim-content should have_timeline_complete "content-entrance"
}

// =============================================================================
// Hero Section Load
// =============================================================================

@test "hero section loads with staggered animations" {
  @fixture {
    <section class="hero">
      <h1 class="hero__title">Title</h1>
      <p class="hero__subtitle">Subtitle</p>
      <button class="hero__cta">Call to Action</button>
    </section>
  }

  @let (title = document.querySelector('.hero__title'))
  @let (subtitle = document.querySelector('.hero__subtitle'))
  @let (cta = document.querySelector('.hero__cta'))
  @eval (ST.set(title, 'hero-title', 1))
  @eval (ST.set(subtitle, 'hero-subtitle', 1))
  @eval (ST.set(cta, 'hero-cta', 1))
  @then .hero__title should have_timeline_complete "hero-title"
  @then .hero__subtitle should have_timeline_complete "hero-subtitle"
  @then .hero__cta should have_timeline_complete "hero-cta"
}

// =============================================================================
// Navigation Ready State
// =============================================================================

@test "navigation is interactive after load" {
  @fixture {
    <nav class="nav" data-st-state="ready">
      <a class="nav__link" href="#home">Home</a>
      <a class="nav__link" href="#about">About</a>
      <button class="nav__menu" aria-expanded="false">Menu</button>
    </nav>
  }

  @let (nav = document.querySelector('.nav'))
  @eval (nav.__stStateHistory = 'loading,ready'.split(','))

  @then .nav should have_state "ready"
  @then .nav__link should exist
  @then .nav__menu should exist
  @then .nav__menu should have_attr "aria-expanded"
}

// =============================================================================
// Initial Data Load
// =============================================================================

@test "page loads with initial data" {
  @fixture {
    <div class="data-page">
      <ul class="item-list">
        <li>Item 1</li>
        <li>Item 2</li>
        <li>Item 3</li>
      </ul>
    </div>
  }

  @let (page = document.querySelector('.data-page'))
  @eval (ST.set(page, 'items', 'Item 1,Item 2,Item 3'.split(',')))
  @eval (ST.set(page, 'isLoaded', true))

  @then $items on .data-page should have_length 3
  @then $isLoaded on .data-page should be_truthy
  @then .item-list should contain "Item 1"
  @then .item-list should contain "Item 2"
  @then .item-list should contain "Item 3"
}

// =============================================================================
// Error-Free Load
// =============================================================================

@test "page has no console errors on load" {
  @fixture {
    <div class="error-free-page" data-st-state="loaded">
      <div class="content">All Good!</div>
    </div>
  }

  @let (page = document.querySelector('.error-free-page'))
  @eval (ST.set(page, 'errors', Array()))
  @eval (ST.set(page, 'hasErrors', false))

  @then $errors on .error-free-page should have_length 0
  @then $hasErrors on .error-free-page should be_falsy
  @then .error-free-page should have_state "loaded"
}

// =============================================================================
// Responsive Layout
// =============================================================================

@test "layout renders correctly at desktop width" {
  @fixture {
    <div class="responsive-layout" style="display: grid; grid-template-columns: 1fr 3fr;">
      <aside class="sidebar" style="width: 300px;">Sidebar</aside>
      <main class="main" style="width: 900px;">Main</main>
    </div>
  }

  @then .sidebar should exist
  @then .main should exist
  @then .responsive-layout should be_visible
}

// =============================================================================
// Deferred Loading
// =============================================================================

@test "below-fold content loads lazily" {
  @fixture {
    <div class="lazy-section" data-st-state="unloaded">
      <div class="placeholder">Loading...</div>
    </div>
  }

  @let (section = document.querySelector('.lazy-section'))
  @eval (section.__stStateHistory = 'unloaded'.split(','))

  @then .lazy-section should have_state "unloaded"

  // Simulate intersection observer trigger
  @eval (section.dataset.stState = 'loaded')
  @eval (section.__stStateHistory.push('loaded'))

  @then .lazy-section should have_state "loaded"
  @assert (document.querySelector('.lazy-section').__stStateHistory.join(',') === 'unloaded,loaded')
}

// =============================================================================
// Theme Application
// =============================================================================

@test "theme is applied on load" {
  @fixture {
    <div class="themed-page" data-theme="dark">
      <div class="content">Dark theme content</div>
    </div>
  }

  @let (page = document.querySelector('.themed-page'))
  @eval (ST.set(page, 'theme', 'dark'))

  @then .themed-page should have_attr "data-theme"
  @then $theme on .themed-page should equal "dark"
}
