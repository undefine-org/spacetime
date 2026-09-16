/// Platform Primitive
///
/// Detects the current platform (iOS, Android, browser) and provides
/// signals for conditional styling and behavior.
///
/// @param &el - Element to bind platform classes to
///
/// @yields $os - Current OS ('ios', 'android', 'browser', 'ios-browser', 'android-browser')
/// @yields $isNative - Whether running in native Tauri app
/// @yields $isMobile - Whether on a mobile device (native or browser)
/// @yields $isIOS - Whether on iOS (native or browser)
/// @yields $isAndroid - Whether on Android (native or browser)

%primitive platform(&el) {
  %emit js {
    // Import the SpacetimeMobile API
    const mobile = globalThis.__SpacetimeMobile || {
      platform: () => ({ os: 'browser', isNative: false })
    };

    // Get platform info
    const info = mobile.platform();
    const os = info.os || 'browser';
    const isNative = info.isNative || false;

    // Determine platform flags
    const isIOS = os === 'ios' || os === 'ios-browser';
    const isAndroid = os === 'android' || os === 'android-browser';
    const isMobile = isNative || isIOS || isAndroid || info.isMobileBrowser;

    // Yield platform signals
    %yield os -> $os;
    %yield isNative -> $isNative;
    %yield isMobile -> $isMobile;
    %yield isIOS -> $isIOS;
    %yield isAndroid -> $isAndroid;

    // Add platform classes for CSS targeting
    el.classList.add(`st-platform-${os.replace('-browser', '')}`);
    if (isNative) el.classList.add('st-native');
    if (isMobile) el.classList.add('st-mobile');
    if (isIOS) el.classList.add('st-ios');
    if (isAndroid) el.classList.add('st-android');

    // Also set on document root for global CSS targeting
    if (typeof document !== 'undefined') {
      const root = document.documentElement;
      root.dataset.stPlatform = os.replace('-browser', '');
      root.dataset.stNative = isNative ? 'true' : 'false';
      root.dataset.stMobile = isMobile ? 'true' : 'false';

      // Set CSS custom properties for platform-specific styling
      root.style.setProperty('--st-platform', os);
      root.style.setProperty('--st-is-native', isNative ? '1' : '0');
      root.style.setProperty('--st-is-mobile', isMobile ? '1' : '0');
      root.style.setProperty('--st-is-ios', isIOS ? '1' : '0');
      root.style.setProperty('--st-is-android', isAndroid ? '1' : '0');
    }
  }

  %exports {
    $os: string
    $isNative: bool
    $isMobile: bool
    $isIOS: bool
    $isAndroid: bool
  }
}


/// iOS-only conditional macro
///
/// @example
/// .nav-bar {
///   @ios {
///     backdrop-filter: blur(20px);
///   }
/// }

%macro ios-only {
  %form {
    @ios {
      $styles:properties
    }
  }

  %binds {
    platform(&self) -> { $isIOS }
  }

  %when $isIOS {
    %applies {
      $styles
    }
  }
}

/// Android-only conditional macro
%macro android-only {
  %form {
    @android {
      $styles:properties
    }
  }

  %binds {
    platform(&self) -> { $isAndroid }
  }

  %when $isAndroid {
    %applies {
      $styles
    }
  }
}

/// Native-only conditional macro
%macro native-only {
  %form {
    @native {
      $styles:properties
    }
  }

  %binds {
    platform(&self) -> { $isNative }
  }

  %when $isNative {
    %applies {
      $styles
    }
  }
}

/// Mobile-only conditional macro
%macro mobile-only {
  %form {
    @mobile {
      $styles:properties
    }
  }

  %binds {
    platform(&self) -> { $isMobile }
  }

  %when $isMobile {
    %applies {
      $styles
    }
  }
}
