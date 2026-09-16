/// Socket Pattern Matching Demo
///
/// Demonstrates the typed union socket primitive with @state pattern matching.
///
/// ## Headless Testing
///
/// ```bash
/// cargo run --example socket_demo --features headless
/// ```
///
/// ## Pattern Matching
///
/// The socket primitive yields a typed union $state:
///   Disconnected | Connecting | Connected { $send, $received } | Error { $error }
///
/// Each @state block activates when $ws matches that variant.

// Socket demo container
.socket-demo {
  // Use the typed union socket primitive
  @socket(url: "wss://echo.websocket.org") as $ws

  // Pattern match: Disconnected state
  @state(when: $ws is Disconnected) {
    .status {
      background: #6b7280;
    }
    .chat-area {
      opacity: 0.5;
    }
    .reconnect-btn {
      opacity: 1;
      pointer-events: auto;
    }
    .send-btn {
      opacity: 0;
      pointer-events: none;
    }
  }

  // Pattern match: Connecting state
  @state(when: $ws is Connecting) {
    .status {
      background: #f59e0b;
    }
    .spinner {
      opacity: 1;
    }
  }

  // Pattern match: Connected state with bindings
  // $send and $received become available for JS handlers
  @state(when: $ws is Connected { $send, $received }) {
    .status {
      background: #10b981;
    }
    .chat-area {
      opacity: 1;
    }
    .send-btn {
      opacity: 1;
      pointer-events: auto;
    }
    .spinner {
      opacity: 0;
    }
  }

  // Pattern match: Error state
  @state(when: $ws is Error { $error }) {
    .status {
      background: #ef4444;
    }
    .error-message {
      opacity: 1;
      translate-y: 0;
    }
  }
}

// CSS-only spinner animation
.socket-demo .spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

// CSS transitions for smooth state changes
.socket-demo .status {
  transition: background 300ms ease-out;
}

.socket-demo .chat-area {
  transition: opacity 300ms ease-out;
}

.socket-demo .send-btn {
  transition: opacity 200ms ease-out, transform 200ms ease-out;
}

.socket-demo .reconnect-btn {
  transition: opacity 200ms ease-out, transform 200ms ease-out;
}

