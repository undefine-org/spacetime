/// Media Query Macro
///
/// Applies styles and behaviors based on media queries.
///
/// @example
/// .sidebar {
///   @media("min-width: 768px") {
///     width: 300px
///     position: fixed
///   }
///
///   @media("max-width: 767px") {
///     width: 100%
///     position: relative
///   }
/// }

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
}


/// Dark Mode Macro
///
/// Convenience for dark/light mode switching.
///
/// @example
/// .card {
///   @dark {
///     background: #1a1a1a
///     color: #ffffff
///   }
///
///   @light {
///     background: #ffffff
///     color: #1a1a1a
///   }
/// }

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
///
/// @example
/// .animated-element {
///   @reduced-motion {
///     // Disable or simplify animations
///     animation: none
///     transition: none
///   }
/// }

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
}


/// Container Query Macro (future)
///
/// Responds to container size changes.
///
/// @example
/// .card {
///   @container(min-width: 400px) {
///     display: flex
///     flex-direction: row
///   }
///
///   @container(max-width: 399px) {
///     display: block
///   }
/// }

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
}


/// Breakpoint Macro
///
/// Named breakpoint shortcuts.
///
/// @example
/// .layout {
///   @breakpoint(sm) {
///     grid-template-columns: 1fr
///   }
///
///   @breakpoint(md) {
///     grid-template-columns: 1fr 1fr
///   }
///
///   @breakpoint(lg) {
///     grid-template-columns: 1fr 1fr 1fr
///   }
/// }

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
}


/// Orientation Macro
///
/// Responds to device orientation.
///
/// @example
/// .gallery {
///   @portrait {
///     grid-template-columns: 1fr
///   }
///
///   @landscape {
///     grid-template-columns: 1fr 1fr 1fr
///   }
/// }

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
///
/// @example
/// .navigation {
///   @print {
///     display: none
///   }
/// }

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
}

