/// Media Query Macro
///
/// Applies styles and behaviors based on media queries.

%macro media-query {

  %form {
    @media($query:string) {
      $styles:properties
      $_children*
    }
  }

  %binds {
    media($query, styles: $styles) -> { $matches }
  }

  %example "@media-query" {
    .sidebar {
      @media("min-width: 768px") {
        width: 300px
        position: fixed
      }

      @media("max-width: 767px") {
        width: 100%
        position: relative
      }
    }
  }
}


/// Dark Mode Macro
///
/// Convenience for dark/light mode switching.

%macro dark-mode {

  %form {
    @dark {
      $styles:properties
      $_children*
    }
  }

  %binds {
    media("(prefers-color-scheme: dark)", styles: $styles) -> { $matches }
  }

  %example "@dark-mode" {
    .card {
      background: #ffffff;
      color: #1a1a1a;
      @dark {
        background: #1a1a1a;
        color: #ffffff;
      }
    }
    <div class="card">Hello</div>
  }
}


%macro light-mode {

  %form {
    @light {
      $styles:properties
      $_children*
    }
  }

  %binds {
    media("(prefers-color-scheme: light)", styles: $styles) -> { $matches }
  }
}


/// Reduced Motion Macro
///
/// Respects user's reduced motion preference.

%macro reduced-motion {

  %form {
    @reduced-motion {
      $styles:properties
      $_children*
    }
  }

  %binds {
    media("(prefers-reduced-motion: reduce)", styles: $styles) -> { $matches as $reduced }
  }

  %example "@reduced-motion" {
    .animated-element {
      @reduced-motion {
        // Disable or simplify animations
        animation: none
        transition: none
      }
    }
  }
}


/// Container Query Macro (future)
///
/// Responds to container size changes.

%macro container-query {

  %form {
    @container($query:string) {
      $styles:properties
      $_children*
    }
  }

  %binds {
    media($query, styles: $styles) -> { $matches }
  }

  %example "@container-query" {
    .card {
      @container(min-width: 400px) {
        display: flex
        flex-direction: row
      }

      @container(max-width: 399px) {
        display: block
      }
    }
  }
}


/// Breakpoint Macro
///
/// Named breakpoint shortcuts.

%macro breakpoint {

  %form {
    @breakpoint($name:ident) {
      $styles:properties
      $_children*
    }
  }

  %if $name == sm {
    %binds {
      media("(min-width: 640px)", styles: $styles) -> { $matches }
    }
  } %elif $name == md {
    %binds {
      media("(min-width: 768px)", styles: $styles) -> { $matches }
    }
  } %elif $name == lg {
    %binds {
      media("(min-width: 1024px)", styles: $styles) -> { $matches }
    }
  } %elif $name == xl {
    %binds {
      media("(min-width: 1280px)", styles: $styles) -> { $matches }
    }
  } %elif $name == 2xl {
    %binds {
      media("(min-width: 1536px)", styles: $styles) -> { $matches }
    }
  }

  %example "@breakpoint" {
    .layout {
      @breakpoint(sm) {
        grid-template-columns: 1fr
      }

      @breakpoint(md) {
        grid-template-columns: 1fr 1fr
      }

      @breakpoint(lg) {
        grid-template-columns: 1fr 1fr 1fr
      }
    }
  }
}


/// Orientation Macro
///
/// Responds to device orientation.

%macro portrait {

  %form {
    @portrait {
      $styles:properties
      $_children*
    }
  }

  %binds {
    media("(orientation: portrait)", styles: $styles) -> { $matches }
  }

  %example "@portrait" {
    .gallery {
      @portrait {
        grid-template-columns: 1fr
      }

      @landscape {
        grid-template-columns: 1fr 1fr 1fr
      }
    }
  }
}


%macro landscape {

  %form {
    @landscape {
      $styles:properties
      $_children*
    }
  }

  %binds {
    media("(orientation: landscape)", styles: $styles) -> { $matches }
  }
}


/// Print Macro
///
/// Print-specific styles.

%macro print {

  %form {
    @print {
      $styles:properties
      $_children*
    }
  }

  %binds {
    media("print", styles: $styles) -> { $matches }
  }

  %example "@print" {
    .navigation {
      @print {
        display: none
      }
    }
  }
}

