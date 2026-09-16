/// Responsive Layout Example
///
/// Demonstrates media queries, dark mode, and reduced motion support.
/// No % symbols - this is end-user code.


$menuOpen bool: false;

// Theme-aware card
.card {
  // Light mode (default)
  background: white;
  color: #1a1a1a;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);

  // Dark mode override
  @dark {
    background: #2a2a2a;
    color: #f5f5f5;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }

  // Hover animation (respects reduced motion)
  @on &.hover(name: card-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -4px;
        box-shadow: 0 2px 8px rgba(0,0,0,0.08) -> 0 8px 24px rgba(0,0,0,0.12);
      }
  @reduced-motion {
    @on &.hover card-hover-simple(100ms) {
      box-shadow: 0 2px 8px rgba(0,0,0,0.08) -> 0 4px 12px rgba(0,0,0,0.15);
    }
  }
}

// Responsive navigation
.nav {
  // Mobile: hamburger menu
  @breakpoint(sm) {
    display: none;
  }

  // Tablet and up: horizontal nav
  @breakpoint(md) {
    display: flex;
    flex-direction: row;
    gap: 24px;
  }

  // Desktop: with more spacing
  @breakpoint(lg) {
    gap: 32px;
  }
}

// Mobile menu overlay
.mobile-menu {
  @media("max-width: 767px") {
    position: fixed;
    inset: 0;
    background: white;

    .is-open: $menuOpen;
    transform: translateX(-100%);

    @dark {
      background: #1a1a1a;
    }
  }

  @media("min-width: 768px") {
    display: none;
  }
}

.mobile-menu.is-open {
  transform: translateX(0);
}

// Grid that adapts to screen size
.product-grid {
  display: grid;
  gap: 16px;

  @breakpoint(sm) {
    grid-template-columns: 1fr;
  }

  @breakpoint(md) {
    grid-template-columns: repeat(2, 1fr);
    gap: 24px;
  }

  @breakpoint(lg) {
    grid-template-columns: repeat(3, 1fr);
  }

  @breakpoint(xl) {
    grid-template-columns: repeat(4, 1fr);
    gap: 32px;
  }
}

// Orientation-specific layout
.gallery {
  @portrait {
    grid-template-columns: 1fr;
  }

  @landscape {
    grid-template-columns: repeat(3, 1fr);
  }
}

// Print styles
.page-content {
  @print {
    background: white;
    color: black;
  }
}

.no-print {
  @print {
    display: none;
  }
}

// High contrast mode
.button {
  @media("prefers-contrast: high") {
    border: 2px solid currentColor;
    background: transparent;
    color: inherit;
  }
}

// Reduced data mode
.hero-video {
  @media("prefers-reduced-data: reduce") {
    display: none;
  }
}

.hero-image {
  @media("prefers-reduced-data: reduce") {
    display: block;
  }
}
