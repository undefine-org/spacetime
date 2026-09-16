/**
 * Spacetime Mobile Abstract API
 *
 * This module defines the abstract interface for mobile native APIs.
 * Adapters (tauri.js, browser.js) implement this interface.
 *
 * Usage:
 *   import { SpacetimeMobile } from 'stdlib/mobile/adapters/api.js';
 *   SpacetimeMobile.haptic('medium');
 *
 * The global __SpacetimeMobile is set by the adapter (tauri.js or browser.js)
 * before this module loads. If no adapter is present, graceful defaults are used.
 */

// Default implementations for browser preview mode
// These provide graceful degradation when native features aren't available
const defaults = {
  /**
   * Get platform information
   * @returns {{ os: string, version: string | null, isNative: boolean }}
   */
  platform: () => ({
    os: 'browser',
    version: null,
    isNative: false,
  }),

  /**
   * Trigger haptic feedback
   * @param {'light' | 'medium' | 'heavy' | 'success' | 'warning' | 'error' | 'selection'} style
   * @returns {void}
   */
  haptic: (style) => {
    // Silent no-op in browser - no haptic hardware
    if (typeof navigator !== 'undefined' && navigator.vibrate) {
      // Web Vibration API as fallback (Android Chrome only)
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
    }
  },

  /**
   * Request biometric authentication
   * @param {string} message - Prompt message to display
   * @returns {Promise<{ success: boolean, error?: string }>}
   */
  biometric: (message) => {
    return Promise.reject(new Error('Biometric authentication requires native runtime'));
  },

  /**
   * Send a notification
   * @param {{ title: string, body: string, icon?: string }} options
   * @returns {Promise<void>}
   */
  notify: async (options) => {
    // Try Web Notification API as fallback
    if (typeof Notification !== 'undefined') {
      if (Notification.permission === 'granted') {
        new Notification(options.title, { body: options.body, icon: options.icon });
        return;
      } else if (Notification.permission !== 'denied') {
        const permission = await Notification.requestPermission();
        if (permission === 'granted') {
          new Notification(options.title, { body: options.body, icon: options.icon });
          return;
        }
      }
    }
    return Promise.reject(new Error('Notifications require native runtime or browser permission'));
  },

  /**
   * Request notification permission
   * @returns {Promise<'granted' | 'denied' | 'default'>}
   */
  notifyPermission: async () => {
    if (typeof Notification !== 'undefined') {
      if (Notification.permission === 'default') {
        return Notification.requestPermission();
      }
      return Notification.permission;
    }
    return 'denied';
  },

  /**
   * Open native share sheet
   * @param {{ title?: string, text?: string, url?: string }} options
   * @returns {Promise<void>}
   */
  share: async (options) => {
    if (typeof navigator !== 'undefined' && navigator.share) {
      return navigator.share(options);
    }
    return Promise.reject(new Error('Share requires native runtime or Web Share API'));
  },

  /**
   * Get current geolocation
   * @param {{ enableHighAccuracy?: boolean, timeout?: number }} options
   * @returns {Promise<{ latitude: number, longitude: number, accuracy: number }>}
   */
  location: (options = {}) => {
    return new Promise((resolve, reject) => {
      if (typeof navigator !== 'undefined' && navigator.geolocation) {
        navigator.geolocation.getCurrentPosition(
          (pos) => resolve({
            latitude: pos.coords.latitude,
            longitude: pos.coords.longitude,
            accuracy: pos.coords.accuracy,
          }),
          (err) => reject(new Error(err.message)),
          {
            enableHighAccuracy: options.enableHighAccuracy ?? true,
            timeout: options.timeout ?? 10000,
          }
        );
      } else {
        reject(new Error('Geolocation requires native runtime or browser permission'));
      }
    });
  },

  /**
   * Camera / barcode scanner
   */
  camera: {
    /**
     * Scan a barcode/QR code
     * @param {{ formats?: string[] }} options
     * @returns {Promise<{ format: string, value: string }>}
     */
    scan: (options = {}) => {
      return Promise.reject(new Error('Camera scanning requires native runtime'));
    },
  },

  /**
   * Secure storage (uses localStorage as fallback)
   */
  storage: {
    /**
     * Get a value from secure storage
     * @param {string} key
     * @returns {Promise<string | null>}
     */
    get: async (key) => {
      if (typeof localStorage !== 'undefined') {
        return localStorage.getItem(`spacetime_secure_${key}`);
      }
      return null;
    },

    /**
     * Set a value in secure storage
     * @param {string} key
     * @param {string} value
     * @returns {Promise<void>}
     */
    set: async (key, value) => {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(`spacetime_secure_${key}`, value);
      }
    },

    /**
     * Remove a value from secure storage
     * @param {string} key
     * @returns {Promise<void>}
     */
    remove: async (key) => {
      if (typeof localStorage !== 'undefined') {
        localStorage.removeItem(`spacetime_secure_${key}`);
      }
    },
  },

  /**
   * Get safe area insets
   * @returns {{ top: number, right: number, bottom: number, left: number }}
   */
  safeArea: () => {
    // Use CSS env() variables if available (iOS Safari, modern browsers)
    if (typeof getComputedStyle !== 'undefined' && typeof document !== 'undefined') {
      const style = getComputedStyle(document.documentElement);
      return {
        top: parseInt(style.getPropertyValue('--sat') || '0', 10) ||
             parseInt(style.getPropertyValue('env(safe-area-inset-top)') || '0', 10),
        right: parseInt(style.getPropertyValue('--sar') || '0', 10) ||
               parseInt(style.getPropertyValue('env(safe-area-inset-right)') || '0', 10),
        bottom: parseInt(style.getPropertyValue('--sab') || '0', 10) ||
                parseInt(style.getPropertyValue('env(safe-area-inset-bottom)') || '0', 10),
        left: parseInt(style.getPropertyValue('--sal') || '0', 10) ||
              parseInt(style.getPropertyValue('env(safe-area-inset-left)') || '0', 10),
      };
    }
    return { top: 0, right: 0, bottom: 0, left: 0 };
  },

  /**
   * Keyboard handling
   */
  keyboard: {
    /**
     * Listen for keyboard show events
     * @param {(height: number) => void} callback
     * @returns {() => void} Unsubscribe function
     */
    onShow: (callback) => {
      // Use visualViewport API as fallback
      if (typeof visualViewport !== 'undefined') {
        const handler = () => {
          const height = window.innerHeight - visualViewport.height;
          if (height > 100) { // Threshold to detect keyboard
            callback(height);
          }
        };
        visualViewport.addEventListener('resize', handler);
        return () => visualViewport.removeEventListener('resize', handler);
      }
      return () => {};
    },

    /**
     * Listen for keyboard hide events
     * @param {() => void} callback
     * @returns {() => void} Unsubscribe function
     */
    onHide: (callback) => {
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

    /**
     * Dismiss the keyboard
     * @returns {void}
     */
    dismiss: () => {
      if (typeof document !== 'undefined' && document.activeElement) {
        document.activeElement.blur();
      }
    },
  },
};

/**
 * SpacetimeMobile API
 *
 * Uses the native adapter if available (globalThis.__SpacetimeMobile),
 * otherwise falls back to browser defaults with graceful degradation.
 */
export const SpacetimeMobile = globalThis.__SpacetimeMobile || defaults;

// Also export as default
export default SpacetimeMobile;

// Export the defaults for testing
export { defaults as SpacetimeMobileDefaults };
