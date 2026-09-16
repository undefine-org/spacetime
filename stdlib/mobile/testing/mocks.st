/// Native API Mocking Utilities
///
/// Provides declarative mock patterns for native APIs in mobile testing.
/// The actual mocking happens in JavaScript; these macros provide
/// declarative test configuration.
///
/// @example
/// @test "handles camera permission denied" {
///   @mock-native("camera") {
///     permission: "denied"
///   }
///   @when .camera-button click
///   @then .permission-error should be_visible
/// }
///
/// @example
/// @test "geolocation returns mock position" {
///   @mock-native("geolocation") {
///     latitude: 37.7749
///     longitude: -122.4194
///   }
///   @when .locate-button click
///   @then .location-display should have_text "San Francisco"
/// }


/// Mock Native Primitive
///
/// Sets up mock configuration for a native API.
///
/// @param $api - Name of the native API to mock
/// @param $config - Configuration object for the mock
/// @yields $mockActive - Whether the mock is currently active
/// @yields $mockApi - Name of the mocked API
/// @yields $mockCalls - Number of times the mock was called

%primitive mock-native($api, $config) {
  %emit js {
    const __mockApi = %$api;
    const __mockConfig = %$config || {};

    // Initialize mock registry if needed
    globalThis.__spacetime_mocks = globalThis.__spacetime_mocks || {};

    // Store mock configuration
    globalThis.__spacetime_mocks[__mockApi] = {
      config: __mockConfig,
      calls: [],
      active: true
    };

    // Set CSS variables for mock state visibility
    if (typeof document !== 'undefined') {
      const root = document.documentElement;
      root.style.setProperty('--st-mock-' + __mockApi, '1');
      root.dataset['stMock' + __mockApi.charAt(0).toUpperCase() + __mockApi.slice(1)] = 'true';
    }

    // Mock specific APIs based on name
    const __originalApis = {};

    switch (__mockApi) {
      case 'camera':
        // Mock camera/media devices API
        if (navigator.mediaDevices) {
          __originalApis.getUserMedia = navigator.mediaDevices.getUserMedia;
          navigator.mediaDevices.getUserMedia = async (constraints) => {
            globalThis.__spacetime_mocks[__mockApi].calls.push({ constraints, timestamp: Date.now() });
            if (__mockConfig.permission === 'denied') {
              throw new DOMException('Permission denied', 'NotAllowedError');
            }
            if (__mockConfig.error) {
              throw new Error(__mockConfig.error);
            }
            // Return a mock stream
            return __mockConfig.stream || new MediaStream();
          };
        }
        break;

      case 'geolocation':
        // Mock geolocation API
        if (navigator.geolocation) {
          __originalApis.getCurrentPosition = navigator.geolocation.getCurrentPosition;
          __originalApis.watchPosition = navigator.geolocation.watchPosition;

          navigator.geolocation.getCurrentPosition = (success, error, options) => {
            globalThis.__spacetime_mocks[__mockApi].calls.push({ type: 'getCurrentPosition', timestamp: Date.now() });
            if (__mockConfig.permission === 'denied') {
              error && error({ code: 1, message: 'User denied geolocation' });
              return;
            }
            if (__mockConfig.error) {
              error && error({ code: 2, message: __mockConfig.error });
              return;
            }
            success({
              coords: {
                latitude: __mockConfig.latitude || 0,
                longitude: __mockConfig.longitude || 0,
                accuracy: __mockConfig.accuracy || 100,
                altitude: __mockConfig.altitude || null,
                altitudeAccuracy: __mockConfig.altitudeAccuracy || null,
                heading: __mockConfig.heading || null,
                speed: __mockConfig.speed || null
              },
              timestamp: Date.now()
            });
          };

          navigator.geolocation.watchPosition = (success, error, options) => {
            globalThis.__spacetime_mocks[__mockApi].calls.push({ type: 'watchPosition', timestamp: Date.now() });
            navigator.geolocation.getCurrentPosition(success, error, options);
            return 1; // Mock watch ID
          };
        }
        break;

      case 'notifications':
        // Mock notification API
        __originalApis.Notification = window.Notification;
        window.Notification = class MockNotification {
          constructor(title, options) {
            globalThis.__spacetime_mocks[__mockApi].calls.push({ title, options, timestamp: Date.now() });
            this.title = title;
            this.options = options;
          }
          static get permission() {
            return __mockConfig.permission || 'granted';
          }
          static async requestPermission() {
            return __mockConfig.permission || 'granted';
          }
        };
        break;

      case 'storage':
        // Mock localStorage/sessionStorage
        const mockStorage = {};
        __originalApis.localStorage = window.localStorage;
        const mockLocalStorage = {
          getItem: (key) => {
            globalThis.__spacetime_mocks[__mockApi].calls.push({ method: 'getItem', key, timestamp: Date.now() });
            return __mockConfig[key] !== undefined ? __mockConfig[key] : mockStorage[key] || null;
          },
          setItem: (key, value) => {
            globalThis.__spacetime_mocks[__mockApi].calls.push({ method: 'setItem', key, value, timestamp: Date.now() });
            mockStorage[key] = value;
          },
          removeItem: (key) => {
            globalThis.__spacetime_mocks[__mockApi].calls.push({ method: 'removeItem', key, timestamp: Date.now() });
            delete mockStorage[key];
          },
          clear: () => {
            globalThis.__spacetime_mocks[__mockApi].calls.push({ method: 'clear', timestamp: Date.now() });
            Object.keys(mockStorage).forEach(k => delete mockStorage[k]);
          },
          get length() { return Object.keys(mockStorage).length; },
          key: (i) => Object.keys(mockStorage)[i] || null
        };
        Object.defineProperty(window, 'localStorage', { value: mockLocalStorage, writable: true });
        break;

      case 'fetch':
        // Mock fetch API
        __originalApis.fetch = window.fetch;
        window.fetch = async (url, options) => {
          globalThis.__spacetime_mocks[__mockApi].calls.push({ url, options, timestamp: Date.now() });
          if (__mockConfig.error) {
            throw new Error(__mockConfig.error);
          }
          return {
            ok: __mockConfig.ok !== false,
            status: __mockConfig.status || 200,
            statusText: __mockConfig.statusText || 'OK',
            json: async () => __mockConfig.json || __mockConfig.data || {},
            text: async () => __mockConfig.text || JSON.stringify(__mockConfig.data || {}),
            blob: async () => new Blob([__mockConfig.text || '']),
            headers: new Headers(__mockConfig.headers || {})
          };
        };
        break;

      case 'vibration':
        // Mock vibration API
        __originalApis.vibrate = navigator.vibrate;
        navigator.vibrate = (pattern) => {
          globalThis.__spacetime_mocks[__mockApi].calls.push({ pattern, timestamp: Date.now() });
          return __mockConfig.success !== false;
        };
        break;

      case 'clipboard':
        // Mock clipboard API
        __originalApis.clipboard = navigator.clipboard;
        navigator.clipboard = {
          writeText: async (text) => {
            globalThis.__spacetime_mocks[__mockApi].calls.push({ method: 'writeText', text, timestamp: Date.now() });
            if (__mockConfig.permission === 'denied') {
              throw new DOMException('Permission denied', 'NotAllowedError');
            }
            globalThis.__spacetime_mocks[__mockApi].clipboardContent = text;
          },
          readText: async () => {
            globalThis.__spacetime_mocks[__mockApi].calls.push({ method: 'readText', timestamp: Date.now() });
            if (__mockConfig.permission === 'denied') {
              throw new DOMException('Permission denied', 'NotAllowedError');
            }
            return __mockConfig.text || globalThis.__spacetime_mocks[__mockApi].clipboardContent || '';
          }
        };
        break;
    }

    // Store original APIs for cleanup
    globalThis.__spacetime_mocks[__mockApi].originals = __originalApis;

    // Yield signals
    %yield true -> $mockActive;
    %yield __mockApi -> $mockApi;
    %yield globalThis.__spacetime_mocks[__mockApi].calls.length -> $mockCalls;

    %cleanup {
      // Restore original APIs
      const mock = globalThis.__spacetime_mocks[__mockApi];
      if (mock && mock.originals) {
        switch (__mockApi) {
          case 'camera':
            if (mock.originals.getUserMedia && navigator.mediaDevices) {
              navigator.mediaDevices.getUserMedia = mock.originals.getUserMedia;
            }
            break;
          case 'geolocation':
            if (navigator.geolocation) {
              if (mock.originals.getCurrentPosition) {
                navigator.geolocation.getCurrentPosition = mock.originals.getCurrentPosition;
              }
              if (mock.originals.watchPosition) {
                navigator.geolocation.watchPosition = mock.originals.watchPosition;
              }
            }
            break;
          case 'notifications':
            if (mock.originals.Notification) {
              window.Notification = mock.originals.Notification;
            }
            break;
          case 'storage':
            if (mock.originals.localStorage) {
              Object.defineProperty(window, 'localStorage', { value: mock.originals.localStorage, writable: true });
            }
            break;
          case 'fetch':
            if (mock.originals.fetch) {
              window.fetch = mock.originals.fetch;
            }
            break;
          case 'vibration':
            if (mock.originals.vibrate) {
              navigator.vibrate = mock.originals.vibrate;
            }
            break;
          case 'clipboard':
            if (mock.originals.clipboard) {
              navigator.clipboard = mock.originals.clipboard;
            }
            break;
        }
      }

      // Clean up CSS variables
      if (typeof document !== 'undefined') {
        const root = document.documentElement;
        root.style.removeProperty('--st-mock-' + __mockApi);
        delete root.dataset['stMock' + __mockApi.charAt(0).toUpperCase() + __mockApi.slice(1)];
      }

      // Remove from registry
      delete globalThis.__spacetime_mocks[__mockApi];
    }
  }

  %exports {
    $mockActive: bool
    $mockApi: string
    $mockCalls: number
  }
}


/// Mock Native API Macro
///
/// Declarative macro for setting up native API mocks.
///
/// @example
/// .camera-test {
///   @mock-native("camera") {
///     permission: "granted"
///   }
/// }

%macro mock-native {

  %form {
    @mock-native($api:string) {
      $config:properties
    }
  }

  %binds {
    mock-native($api, $config) -> { $mockActive, $mockApi, $mockCalls }
  }
}


/// Mock Permission Macro
///
/// Shorthand for mocking permission-based APIs.
///
/// @example
/// @mock-permission("camera", "denied")
/// @mock-permission("geolocation", "granted")

%macro mock-permission {

  %form {
    @mock-permission($api:string, $status:("granted" | "denied" | "prompt"))
  }

  %emit js {
    const __api = %$api;
    const __status = %$status;

    // Initialize mock registry
    globalThis.__spacetime_mocks = globalThis.__spacetime_mocks || {};
    globalThis.__spacetime_mocks[__api] = globalThis.__spacetime_mocks[__api] || {
      config: {},
      calls: [],
      active: true
    };

    // Set permission in config
    globalThis.__spacetime_mocks[__api].config.permission = __status;

    // Update CSS
    if (typeof document !== 'undefined') {
      const root = document.documentElement;
      root.style.setProperty('--st-mock-' + __api + '-permission', __status);
    }
  }
}


/// Mock Response Macro
///
/// Sets up a mock fetch response for testing API calls.
///
/// @example
/// @mock-response("/api/user") {
///   status: 200
///   json: { name: "Test User", id: 1 }
/// }

%macro mock-response {

  %form {
    @mock-response($url:string) {
      $config:properties
    }
  }

  %emit js {
    const __url = %$url;
    const __config = %$config || {};

    // Initialize URL-specific mock registry
    globalThis.__spacetime_url_mocks = globalThis.__spacetime_url_mocks || {};
    globalThis.__spacetime_url_mocks[__url] = __config;

    // Override fetch to intercept matching URLs
    const __originalFetch = window.fetch;
    if (!globalThis.__spacetime_fetch_intercepted) {
      globalThis.__spacetime_fetch_intercepted = true;
      globalThis.__spacetime_original_fetch = __originalFetch;

      window.fetch = async (url, options) => {
        const urlStr = typeof url === 'string' ? url : url.toString();
        const mock = globalThis.__spacetime_url_mocks[urlStr];

        if (mock) {
          return {
            ok: mock.ok !== false && (mock.status || 200) < 400,
            status: mock.status || 200,
            statusText: mock.statusText || 'OK',
            json: async () => mock.json || mock.data || {},
            text: async () => mock.text || JSON.stringify(mock.json || mock.data || {}),
            blob: async () => new Blob([mock.text || '']),
            headers: new Headers(mock.headers || {})
          };
        }

        return globalThis.__spacetime_original_fetch(url, options);
      };
    }
  }
}


/// Assert Mock Called Macro
///
/// Asserts that a mock was called during the test.
///
/// @example
/// @test "calls geolocation API" {
///   @mock-native("geolocation") { latitude: 0, longitude: 0 }
///   @when .locate-button click
///   @assert-mock-called "geolocation"
/// }

%macro assert-mock-called {

  %form {
    @assert-mock-called $api:string times: $count:number? = 0
  }

  %emit js {
    const __api = %$api;
    const __expectedCount = %$count;
    const __mock = globalThis.__spacetime_mocks && globalThis.__spacetime_mocks[__api];

    if (!__mock) {
      throw new Error('Mock not found: ' + __api);
    }

    if (__expectedCount !== undefined && __expectedCount !== null) {
      if (__mock.calls.length !== __expectedCount) {
        throw new Error('Expected mock "' + __api + '" to be called ' + __expectedCount + ' times, but was called ' + __mock.calls.length + ' times');
      }
    } else if (__mock.calls.length === 0) {
      throw new Error('Expected mock "' + __api + '" to be called at least once');
    }
  }
}


/// Clear Mocks Macro
///
/// Clears all mock call history. Useful between test cases.
///
/// @example
/// @clear-mocks

%macro clear-mocks {

  %form {
    @clear-mocks
  }

  %emit js {
    if (globalThis.__spacetime_mocks) {
      for (const api in globalThis.__spacetime_mocks) {
        globalThis.__spacetime_mocks[api].calls = [];
      }
    }
  }
}
