/// Media Query Primitive
///
/// Wraps the matchMedia API for responsive behavior.
/// Reactively updates when media query match state changes.

%primitive media(
  /// CSS media query string
  query: string,
  styles: properties
) {
  %emit css {
    @media (%query) {
      %self {
        %styles
      }
    }
  }

  %emit js {
    const mql = window.matchMedia('%query');

    const update = (e) => {
      %yield e ? e.matches : mql.matches -> $matches;
      %yield mql.media -> $media;
    };

    // Modern API (addEventListener)
    if (mql.addEventListener) {
      mql.addEventListener('change', update);
    } else {
      // Legacy API (addListener) for older browsers
      mql.addListener(update);
    }

    // Initial state
    update();

    %cleanup {
      if (mql.removeEventListener) {
        mql.removeEventListener('change', update);
      } else {
        mql.removeListener(update);
      }
    }
  }

  %exports {
    /// Whether the query currently matches
    $matches: bool   // true if media query matches
    /// The media query string
    $media: string   // the media query string
  }
}


/// Prefers Color Scheme Primitive
///
/// Convenience wrapper for color scheme detection.

%primitive colorScheme(
  scheme: string
) {
  %emit css {
    @media (prefers-color-scheme: %scheme) {
      %self {
        %styles
      }
    }
  }

  %emit js {
    const darkMql = window.matchMedia('(prefers-color-scheme: dark)');
    const lightMql = window.matchMedia('(prefers-color-scheme: light)');

    const update = () => {
      const dark = darkMql.matches;
      const light = lightMql.matches;

      %yield dark -> $dark;
      %yield light -> $light;
      %yield dark ? 'dark' : 'light' -> $scheme;
    };

    darkMql.addEventListener?.('change', update) || darkMql.addListener?.(update);
    lightMql.addEventListener?.('change', update) || lightMql.addListener?.(update);

    update();

    %cleanup {
      darkMql.removeEventListener?.('change', update) || darkMql.removeListener?.(update);
      lightMql.removeEventListener?.('change', update) || lightMql.removeListener?.(update);
    }
  }

  %exports {
    /// Whether user prefers dark mode
    $dark: bool      // prefers dark mode
    /// Whether user prefers light mode
    $light: bool     // prefers light mode
    /// "dark" or "light"
    $scheme: string  // "dark" or "light"
  }
}


/// Prefers Reduced Motion Primitive
///
/// Convenience wrapper for reduced motion detection.

%primitive reducedMotion() {
  %emit js {
    const mql = window.matchMedia('(prefers-reduced-motion: reduce)');

    const update = () => {
      const reduced = mql.matches;
      %yield reduced -> $reduced;
      %yield !reduced -> $allowed;
    };

    mql.addEventListener?.('change', update) || mql.addListener?.(update);
    update();

    %cleanup {
      mql.removeEventListener?.('change', update) || mql.removeListener?.(update);
    }
  }

  %exports {
    /// Whether user prefers reduced motion
    $reduced: bool   // prefers reduced motion
    /// Whether animations are allowed (inverse of reduced)
    $allowed: bool   // animations allowed
  }
}
