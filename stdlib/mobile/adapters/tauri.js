/**
 * Spacetime Mobile - Tauri Adapter
 *
 * This adapter binds Spacetime Mobile API to Tauri plugins.
 * Import this file in your Tauri app to enable native mobile features.
 *
 * Required Tauri plugins:
 *   - @tauri-apps/plugin-biometric
 *   - @tauri-apps/plugin-notification
 *   - @tauri-apps/plugin-geolocation
 *   - @tauri-apps/plugin-barcode-scanner
 *   - @tauri-apps/plugin-haptics
 *
 * Usage:
 *   // In your Tauri app's main.js
 *   import 'stdlib/mobile/adapters/tauri.js';
 */

// Lazy imports to avoid errors when plugins aren't installed
let biometricPlugin = null;
let notificationPlugin = null;
let geolocationPlugin = null;
let barcodePlugin = null;
let hapticsPlugin = null;

const loadPlugins = async () => {
  try {
    biometricPlugin = await import('@tauri-apps/plugin-biometric');
  } catch (e) {
    console.warn('[SpacetimeMobile] Biometric plugin not available:', e.message);
  }

  try {
    notificationPlugin = await import('@tauri-apps/plugin-notification');
  } catch (e) {
    console.warn('[SpacetimeMobile] Notification plugin not available:', e.message);
  }

  try {
    geolocationPlugin = await import('@tauri-apps/plugin-geolocation');
  } catch (e) {
    console.warn('[SpacetimeMobile] Geolocation plugin not available:', e.message);
  }

  try {
    barcodePlugin = await import('@tauri-apps/plugin-barcode-scanner');
  } catch (e) {
    console.warn('[SpacetimeMobile] Barcode scanner plugin not available:', e.message);
  }

  try {
    hapticsPlugin = await import('@tauri-apps/plugin-haptics');
  } catch (e) {
    console.warn('[SpacetimeMobile] Haptics plugin not available:', e.message);
  }
};

// Haptic patterns for different feedback types
const hapticPatterns = {
  light: { duration: 10, intensity: 0.3 },
  medium: { duration: 20, intensity: 0.5 },
  heavy: { duration: 40, intensity: 0.8 },
  success: [
    { duration: 10, intensity: 0.3 },
    { pause: 30 },
    { duration: 10, intensity: 0.3 },
  ],
  warning: [
    { duration: 20, intensity: 0.5 },
    { pause: 10 },
    { duration: 20, intensity: 0.5 },
  ],
  error: [
    { duration: 50, intensity: 0.8 },
    { pause: 30 },
    { duration: 50, intensity: 0.8 },
  ],
  selection: { duration: 5, intensity: 0.2 },
};

/**
 * Tauri implementation of SpacetimeMobile API
 */
const tauriAdapter = {
  /**
   * Get platform information from Tauri
   */
  platform: () => {
    if (typeof window !== 'undefined' && window.__TAURI__) {
      const os = window.__TAURI__.os;
      return {
        os: os?.type?.() || 'unknown',
        version: os?.version?.() || null,
        isNative: true,
      };
    }
    return { os: 'unknown', version: null, isNative: true };
  },

  /**
   * Trigger haptic feedback via Tauri haptics plugin
   */
  haptic: async (style) => {
    if (!hapticsPlugin) {
      await loadPlugins();
    }

    if (hapticsPlugin?.vibrate) {
      const pattern = hapticPatterns[style] || hapticPatterns.medium;

      if (Array.isArray(pattern)) {
        // Complex pattern
        for (const step of pattern) {
          if (step.pause) {
            await new Promise((r) => setTimeout(r, step.pause));
          } else {
            await hapticsPlugin.vibrate({ duration: step.duration });
          }
        }
      } else {
        // Simple pattern
        await hapticsPlugin.vibrate({ duration: pattern.duration });
      }
    } else if (hapticsPlugin?.impact) {
      // Alternative API
      const impacts = { light: 'light', medium: 'medium', heavy: 'heavy' };
      await hapticsPlugin.impact({ style: impacts[style] || 'medium' });
    }
  },

  /**
   * Request biometric authentication via Tauri biometric plugin
   */
  biometric: async (message) => {
    if (!biometricPlugin) {
      await loadPlugins();
    }

    if (!biometricPlugin?.authenticate) {
      throw new Error('Biometric plugin not available');
    }

    try {
      const result = await biometricPlugin.authenticate(message);
      return { success: true, ...result };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Send notification via Tauri notification plugin
   */
  notify: async (options) => {
    if (!notificationPlugin) {
      await loadPlugins();
    }

    if (!notificationPlugin?.sendNotification) {
      throw new Error('Notification plugin not available');
    }

    await notificationPlugin.sendNotification({
      title: options.title,
      body: options.body,
      icon: options.icon,
    });
  },

  /**
   * Request notification permission
   */
  notifyPermission: async () => {
    if (!notificationPlugin) {
      await loadPlugins();
    }

    if (!notificationPlugin?.requestPermission) {
      return 'denied';
    }

    const result = await notificationPlugin.requestPermission();
    return result ? 'granted' : 'denied';
  },

  /**
   * Open native share sheet
   */
  share: async (options) => {
    if (typeof window !== 'undefined' && window.__TAURI__) {
      // Use Tauri invoke for native share
      return window.__TAURI__.invoke('plugin:share|share', {
        title: options.title,
        text: options.text,
        url: options.url,
      });
    }
    throw new Error('Share not available');
  },

  /**
   * Get current geolocation via Tauri geolocation plugin
   */
  location: async (options = {}) => {
    if (!geolocationPlugin) {
      await loadPlugins();
    }

    if (!geolocationPlugin?.getCurrentPosition) {
      throw new Error('Geolocation plugin not available');
    }

    const position = await geolocationPlugin.getCurrentPosition({
      enableHighAccuracy: options.enableHighAccuracy ?? true,
      timeout: options.timeout ?? 10000,
    });

    return {
      latitude: position.coords.latitude,
      longitude: position.coords.longitude,
      accuracy: position.coords.accuracy,
    };
  },

  /**
   * Camera / barcode scanner via Tauri barcode plugin
   */
  camera: {
    scan: async (options = {}) => {
      if (!barcodePlugin) {
        await loadPlugins();
      }

      if (!barcodePlugin?.scan) {
        throw new Error('Barcode scanner plugin not available');
      }

      const result = await barcodePlugin.scan({
        formats: options.formats || ['QR_CODE', 'EAN_13', 'EAN_8', 'CODE_128'],
      });

      return {
        format: result.format,
        value: result.content,
      };
    },
  },

  /**
   * Secure storage via Tauri
   */
  storage: {
    get: async (key) => {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        try {
          return await window.__TAURI__.invoke('plugin:store|get', {
            key: `spacetime_secure_${key}`,
          });
        } catch {
          // Fallback to localStorage
          return localStorage.getItem(`spacetime_secure_${key}`);
        }
      }
      return null;
    },

    set: async (key, value) => {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        try {
          await window.__TAURI__.invoke('plugin:store|set', {
            key: `spacetime_secure_${key}`,
            value,
          });
          return;
        } catch {
          // Fallback to localStorage
        }
      }
      localStorage.setItem(`spacetime_secure_${key}`, value);
    },

    remove: async (key) => {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        try {
          await window.__TAURI__.invoke('plugin:store|delete', {
            key: `spacetime_secure_${key}`,
          });
          return;
        } catch {
          // Fallback to localStorage
        }
      }
      localStorage.removeItem(`spacetime_secure_${key}`);
    },
  },

  /**
   * Get safe area insets from Tauri
   */
  safeArea: async () => {
    if (typeof window !== 'undefined' && window.__TAURI__) {
      try {
        const insets = await window.__TAURI__.invoke('get_safe_area_insets');
        return {
          top: insets.top || 0,
          right: insets.right || 0,
          bottom: insets.bottom || 0,
          left: insets.left || 0,
        };
      } catch {
        // Fallback to CSS env() variables
      }
    }

    // CSS env() fallback
    if (typeof getComputedStyle !== 'undefined' && typeof document !== 'undefined') {
      const root = document.documentElement;
      return {
        top: parseInt(getComputedStyle(root).getPropertyValue('env(safe-area-inset-top)') || '0', 10),
        right: parseInt(getComputedStyle(root).getPropertyValue('env(safe-area-inset-right)') || '0', 10),
        bottom: parseInt(getComputedStyle(root).getPropertyValue('env(safe-area-inset-bottom)') || '0', 10),
        left: parseInt(getComputedStyle(root).getPropertyValue('env(safe-area-inset-left)') || '0', 10),
      };
    }

    return { top: 0, right: 0, bottom: 0, left: 0 };
  },

  /**
   * Keyboard handling
   */
  keyboard: {
    onShow: (callback) => {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const unlisten = window.__TAURI__.listen('keyboard-did-show', (event) => {
          callback(event.payload.height);
        });
        return async () => {
          const fn = await unlisten;
          fn();
        };
      }

      // Fallback to visualViewport
      if (typeof visualViewport !== 'undefined') {
        const handler = () => {
          const height = window.innerHeight - visualViewport.height;
          if (height > 100) {
            callback(height);
          }
        };
        visualViewport.addEventListener('resize', handler);
        return () => visualViewport.removeEventListener('resize', handler);
      }

      return () => {};
    },

    onHide: (callback) => {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const unlisten = window.__TAURI__.listen('keyboard-did-hide', () => {
          callback();
        });
        return async () => {
          const fn = await unlisten;
          fn();
        };
      }

      // Fallback to visualViewport
      if (typeof visualViewport !== 'undefined') {
        const handler = () => {
          const height = window.innerHeight - visualViewport.height;
          if (height < 100) {
            callback();
          }
        };
        visualViewport.addEventListener('resize', handler);
        return () => visualViewport.removeEventListener('resize', handler);
      }

      return () => {};
    },

    dismiss: () => {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        window.__TAURI__.invoke('dismiss_keyboard').catch(() => {
          // Fallback
          if (document.activeElement) {
            document.activeElement.blur();
          }
        });
      } else if (typeof document !== 'undefined' && document.activeElement) {
        document.activeElement.blur();
      }
    },
  },
};

// Initialize the Tauri adapter
const initTauriAdapter = () => {
  // Check if we're in a Tauri environment
  if (typeof window !== 'undefined' && window.__TAURI__) {
    globalThis.__SpacetimeMobile = tauriAdapter;
    console.log('[SpacetimeMobile] Tauri adapter initialized');

    // Pre-load plugins
    loadPlugins().catch(console.warn);
  } else {
    console.warn('[SpacetimeMobile] Not in Tauri environment, adapter not loaded');
  }
};

// Auto-initialize
initTauriAdapter();

// Export for manual use
export { tauriAdapter, loadPlugins };
export default tauriAdapter;
