/**
 * Spacetime Mobile - Browser Adapter
 *
 * This adapter provides browser-based implementations for mobile APIs.
 * Use this for development/preview in a desktop browser.
 *
 * Features:
 *   - Graceful degradation for native-only features
 *   - Web API fallbacks where available (Vibration, Notifications, Share, Geolocation)
 *   - Development mode warnings for unsupported features
 *
 * Usage:
 *   // In your development environment
 *   import 'stdlib/mobile/adapters/browser.js';
 */

// Check if we're in development mode
const isDev = typeof process !== 'undefined'
  ? process.env.NODE_ENV === 'development'
  : typeof window !== 'undefined' && window.location?.hostname === 'localhost';

/**
 * Log a warning for features that don't work in browser
 */
const warnNativeOnly = (feature) => {
  if (isDev) {
    console.warn(
      `[SpacetimeMobile] ${feature} requires native runtime. ` +
      `Using browser fallback or no-op.`
    );
  }
};

/**
 * Browser implementation of SpacetimeMobile API
 */
const browserAdapter = {
  /**
   * Platform detection - always returns 'browser'
   */
  platform: () => {
    const ua = typeof navigator !== 'undefined' ? navigator.userAgent : '';
    let os = 'browser';

    // Detect mobile browsers for better simulation
    if (/iPhone|iPad|iPod/.test(ua)) {
      os = 'ios-browser';
    } else if (/Android/.test(ua)) {
      os = 'android-browser';
    }

    return {
      os,
      version: null,
      isNative: false,
      isMobileBrowser: /iPhone|iPad|iPod|Android/.test(ua),
    };
  },

  /**
   * Haptic feedback - uses Web Vibration API on supported devices
   */
  haptic: (style) => {
    // Only Android Chrome supports vibration
    if (typeof navigator !== 'undefined' && navigator.vibrate) {
      const patterns = {
        light: [10],
        medium: [20],
        heavy: [40],
        success: [10, 30, 10],
        warning: [20, 10, 20],
        error: [50, 30, 50],
        selection: [5],
      };
      navigator.vibrate(patterns[style] || patterns.medium);
    } else if (isDev) {
      console.log(`[SpacetimeMobile] Haptic: ${style} (no vibration support)`);
    }
  },

  /**
   * Biometric - not available in browser, shows alert in dev mode
   */
  biometric: async (message) => {
    warnNativeOnly('Biometric authentication');

    if (isDev) {
      // In dev mode, show a confirm dialog as mock
      const confirmed = window.confirm(
        `[DEV MODE] Biometric Authentication\n\n"${message}"\n\nClick OK to simulate success, Cancel for failure.`
      );
      return confirmed
        ? { success: true }
        : { success: false, error: 'User cancelled (simulated)' };
    }

    return Promise.reject(new Error('Biometric authentication requires native runtime'));
  },

  /**
   * Notifications - uses Web Notifications API
   */
  notify: async (options) => {
    if (typeof Notification === 'undefined') {
      warnNativeOnly('Push notifications');
      return Promise.reject(new Error('Notifications not supported'));
    }

    if (Notification.permission === 'granted') {
      new Notification(options.title, {
        body: options.body,
        icon: options.icon,
      });
      return;
    }

    if (Notification.permission !== 'denied') {
      const permission = await Notification.requestPermission();
      if (permission === 'granted') {
        new Notification(options.title, {
          body: options.body,
          icon: options.icon,
        });
        return;
      }
    }

    return Promise.reject(new Error('Notification permission denied'));
  },

  /**
   * Request notification permission
   */
  notifyPermission: async () => {
    if (typeof Notification === 'undefined') {
      return 'denied';
    }

    if (Notification.permission === 'default') {
      return Notification.requestPermission();
    }

    return Notification.permission;
  },

  /**
   * Share - uses Web Share API on supported browsers
   */
  share: async (options) => {
    if (typeof navigator !== 'undefined' && navigator.share) {
      try {
        await navigator.share({
          title: options.title,
          text: options.text,
          url: options.url,
        });
        return;
      } catch (err) {
        if (err.name === 'AbortError') {
          // User cancelled - not an error
          return;
        }
        throw err;
      }
    }

    warnNativeOnly('Native share sheet');

    // Fallback: copy to clipboard
    if (isDev && typeof navigator !== 'undefined' && navigator.clipboard) {
      const text = options.url || options.text || options.title;
      await navigator.clipboard.writeText(text);
      alert(`[DEV MODE] Share content copied to clipboard:\n${text}`);
      return;
    }

    return Promise.reject(new Error('Share not supported in this browser'));
  },

  /**
   * Geolocation - uses Web Geolocation API
   */
  location: (options = {}) => {
    return new Promise((resolve, reject) => {
      if (typeof navigator === 'undefined' || !navigator.geolocation) {
        warnNativeOnly('Geolocation');
        reject(new Error('Geolocation not supported'));
        return;
      }

      navigator.geolocation.getCurrentPosition(
        (pos) => {
          resolve({
            latitude: pos.coords.latitude,
            longitude: pos.coords.longitude,
            accuracy: pos.coords.accuracy,
            altitude: pos.coords.altitude,
            altitudeAccuracy: pos.coords.altitudeAccuracy,
            heading: pos.coords.heading,
            speed: pos.coords.speed,
          });
        },
        (err) => {
          reject(new Error(err.message));
        },
        {
          enableHighAccuracy: options.enableHighAccuracy ?? true,
          timeout: options.timeout ?? 10000,
          maximumAge: options.maximumAge ?? 0,
        }
      );
    });
  },

  /**
   * Camera - barcode scanning not available in basic browser
   */
  camera: {
    scan: async (options = {}) => {
      warnNativeOnly('Barcode scanning');

      if (isDev) {
        // Dev mode: prompt for manual input
        const value = window.prompt(
          '[DEV MODE] Barcode Scanner\n\nEnter a barcode value to simulate scan:'
        );
        if (value) {
          return { format: 'MANUAL', value };
        }
        throw new Error('Scan cancelled (simulated)');
      }

      return Promise.reject(new Error('Barcode scanning requires native runtime'));
    },
  },

  /**
   * Storage - uses localStorage (not secure, but works for development)
   */
  storage: {
    get: async (key) => {
      if (typeof localStorage !== 'undefined') {
        return localStorage.getItem(`spacetime_secure_${key}`);
      }
      return null;
    },

    set: async (key, value) => {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(`spacetime_secure_${key}`, value);
      }
    },

    remove: async (key) => {
      if (typeof localStorage !== 'undefined') {
        localStorage.removeItem(`spacetime_secure_${key}`);
      }
    },
  },

  /**
   * Safe area insets - reads from CSS env() variables
   */
  safeArea: () => {
    if (typeof document === 'undefined') {
      return { top: 0, right: 0, bottom: 0, left: 0 };
    }

    // Try to read from CSS custom properties first (may be set by viewport-fit: cover)
    const root = document.documentElement;
    const style = getComputedStyle(root);

    // These CSS variables should be set if using viewport-fit: cover
    const getInset = (name) => {
      // Try custom property first
      const customProp = style.getPropertyValue(`--safe-area-inset-${name}`);
      if (customProp) {
        return parseInt(customProp, 10) || 0;
      }
      // env() can't be read directly in JS, so we need a workaround
      return 0;
    };

    return {
      top: getInset('top'),
      right: getInset('right'),
      bottom: getInset('bottom'),
      left: getInset('left'),
    };
  },

  /**
   * Keyboard handling - uses visualViewport API
   */
  keyboard: {
    onShow: (callback) => {
      if (typeof visualViewport === 'undefined') {
        return () => {};
      }

      let wasVisible = false;
      const handler = () => {
        const keyboardHeight = window.innerHeight - visualViewport.height;
        const isVisible = keyboardHeight > 100; // Threshold to detect keyboard

        if (isVisible && !wasVisible) {
          wasVisible = true;
          callback(keyboardHeight);
        } else if (!isVisible) {
          wasVisible = false;
        }
      };

      visualViewport.addEventListener('resize', handler);
      return () => visualViewport.removeEventListener('resize', handler);
    },

    onHide: (callback) => {
      if (typeof visualViewport === 'undefined') {
        return () => {};
      }

      let wasVisible = false;
      const handler = () => {
        const keyboardHeight = window.innerHeight - visualViewport.height;
        const isVisible = keyboardHeight > 100;

        if (!isVisible && wasVisible) {
          wasVisible = false;
          callback();
        } else if (isVisible) {
          wasVisible = true;
        }
      };

      visualViewport.addEventListener('resize', handler);
      return () => visualViewport.removeEventListener('resize', handler);
    },

    dismiss: () => {
      if (typeof document !== 'undefined' && document.activeElement) {
        document.activeElement.blur();
      }
    },
  },
};

// Initialize the browser adapter
const initBrowserAdapter = () => {
  // Only set if no adapter is already set (Tauri adapter takes precedence)
  if (!globalThis.__SpacetimeMobile) {
    globalThis.__SpacetimeMobile = browserAdapter;

    if (isDev) {
      console.log('[SpacetimeMobile] Browser adapter initialized (development mode)');
    }
  }
};

// Auto-initialize
initBrowserAdapter();

// Export for manual use
export { browserAdapter };
export default browserAdapter;
