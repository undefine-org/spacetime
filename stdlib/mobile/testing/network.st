/// Network Condition Simulation
///
/// Provides network condition presets for testing mobile apps
/// under various connectivity scenarios.
///
/// Presets:
/// - offline: No connection
/// - slow-3g: 400kbps, 400ms latency
/// - fast-3g: 1.6Mbps, 150ms latency
/// - fast-4g: 10Mbps, 50ms latency
/// - wifi: 30Mbps, 10ms latency
///
/// @example
/// .api-consumer {
///   @network("slow-3g") {
///     // Styles applied during slow-3g simulation
///     opacity: 0.8
///   }
/// }
///
/// @example
/// // Test offline behavior
/// @test "shows offline banner when disconnected" {
///   @network-set "offline"
///   @then .offline-banner should be_visible
/// }


/// Network Condition Primitive
///
/// Sets up network condition simulation and exposes signals.
///
/// @param $condition - Network condition preset
/// @yields $isOffline - True when simulating offline
/// @yields $isSlow - True when on slow connection (3g)
/// @yields $isFast - True when on fast connection (4g/wifi)
/// @yields $bandwidth - Current bandwidth in kbps
/// @yields $latency - Current latency in ms
/// @yields $condition - Current condition name

%primitive network-condition($condition) {
  %emit js {
    // Network condition presets
    const __networkPresets = {
      'offline': { bandwidth: 0, latency: Infinity, online: false },
      'slow-3g': { bandwidth: 400, latency: 400, online: true },
      'fast-3g': { bandwidth: 1600, latency: 150, online: true },
      'fast-4g': { bandwidth: 10000, latency: 50, online: true },
      'wifi': { bandwidth: 30000, latency: 10, online: true }
    };

    // Get current condition
    const __conditionName = %$condition || 'wifi';
    const __preset = __networkPresets[__conditionName] || __networkPresets['wifi'];

    // Calculate derived states
    const __isOffline = !__preset.online;
    const __isSlow = __preset.bandwidth > 0 && __preset.bandwidth <= 1600;
    const __isFast = __preset.bandwidth > 1600;

    // Yield signals for test coordination
    %yield __isOffline -> $isOffline;
    %yield __isSlow -> $isSlow;
    %yield __isFast -> $isFast;
    %yield __preset.bandwidth -> $bandwidth;
    %yield __preset.latency -> $latency;
    %yield __conditionName -> $condition;

    // Set CSS custom properties for styling
    if (typeof document !== 'undefined') {
      const root = document.documentElement;
      root.style.setProperty('--st-network-condition', __conditionName);
      root.style.setProperty('--st-network-bandwidth', String(__preset.bandwidth));
      root.style.setProperty('--st-network-latency', String(__preset.latency));
      root.style.setProperty('--st-network-offline', __isOffline ? '1' : '0');
      root.style.setProperty('--st-network-slow', __isSlow ? '1' : '0');
      root.style.setProperty('--st-network-fast', __isFast ? '1' : '0');

      // Set data attribute for CSS selectors
      root.dataset.stNetworkCondition = __conditionName;
    }

    // Store globally for test access
    globalThis.__spacetime_network_condition = {
      name: __conditionName,
      preset: __preset,
      isOffline: __isOffline,
      isSlow: __isSlow,
      isFast: __isFast
    };
  }

  %exports {
    $isOffline: bool
    $isSlow: bool
    $isFast: bool
    $bandwidth: number
    $latency: number
    $condition: string
  }
}


/// Network Condition Macro
///
/// Applies styles based on network condition.
///
/// @example
/// .loading-indicator {
///   @network("slow-3g") {
///     display: block
///   }
/// }

%macro network {

  %form {
    @network($condition:("offline" | "slow-3g" | "fast-3g" | "fast-4g" | "wifi") = "wifi") {
      $styles:properties
    }
  }

  %binds {
    network-condition($condition) -> { $condition as $currentCondition }
  }

  %derives {
    $matches: $currentCondition == $condition
  }

  %when $matches {
    %applies {
      $styles
    }
  }
}


/// Offline Macro
///
/// Shorthand for offline-only styles.
///
/// @example
/// .offline-banner {
///   @offline {
///     display: flex
///   }
/// }

%macro offline {

  %form {
    @offline {
      $styles:properties
    }
  }

  %binds {
    network-condition("offline") -> { $isOffline }
  }

  %when $isOffline {
    %applies {
      $styles
    }
  }
}


/// Slow Network Macro
///
/// Applies styles when on slow connection (3g speeds).
///
/// @example
/// .high-res-image {
///   @slow-network {
///     display: none
///   }
/// }

%macro slow-network {

  %form {
    @slow-network {
      $styles:properties
    }
  }

  %binds {
    network-condition("slow-3g") -> { $isSlow }
  }

  %when $isSlow {
    %applies {
      $styles
    }
  }
}


/// Network Set Test Macro
///
/// Sets the network condition for testing purposes.
/// Used in test bodies to simulate network changes.
///
/// @example
/// @test "handles offline mode" {
///   @network-set "offline"
///   @then .offline-indicator should be_visible
/// }

%macro network-set {

  %form {
    @network-set $condition:("offline" | "slow-3g" | "fast-3g" | "fast-4g" | "wifi")
  }

  %emit js {
    // Network condition presets
    const __networkPresets = {
      'offline': { bandwidth: 0, latency: Infinity, online: false },
      'slow-3g': { bandwidth: 400, latency: 400, online: true },
      'fast-3g': { bandwidth: 1600, latency: 150, online: true },
      'fast-4g': { bandwidth: 10000, latency: 50, online: true },
      'wifi': { bandwidth: 30000, latency: 10, online: true }
    };

    const __conditionName = %$condition;
    const __preset = __networkPresets[__conditionName];

    // Update global state
    globalThis.__spacetime_network_condition = {
      name: __conditionName,
      preset: __preset,
      isOffline: !__preset.online,
      isSlow: __preset.bandwidth > 0 && __preset.bandwidth <= 1600,
      isFast: __preset.bandwidth > 1600
    };

    // Update CSS custom properties
    if (typeof document !== 'undefined') {
      const root = document.documentElement;
      root.style.setProperty('--st-network-condition', __conditionName);
      root.style.setProperty('--st-network-bandwidth', String(__preset.bandwidth));
      root.style.setProperty('--st-network-latency', String(__preset.latency));
      root.style.setProperty('--st-network-offline', !__preset.online ? '1' : '0');
      root.dataset.stNetworkCondition = __conditionName;
    }

    // Dispatch event for reactive updates
    if (typeof document !== 'undefined') {
      document.dispatchEvent(new CustomEvent('spacetime:network-change', {
        detail: globalThis.__spacetime_network_condition
      }));
    }
  }
}
