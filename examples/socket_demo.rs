//! Headless Socket Pattern Matching Demo
//!
//! Demonstrates the typed union socket primitive with pattern matching using V8.
//!
//! ## Architecture Note (Spacetime Upgrade Path)
//!
//! This demo is currently written in JavaScript evaluated by V8 via rustyscript.
//! The architecture is designed for future migration to Spacetime:
//!
//! 1. **Current**: JavaScript state machine in `SOCKET_DEMO_JS`
//! 2. **Future**: Replace with compiled output from `socket-demo.st`:
//!    ```st
//!    component socket-demo {
//!      @use socket(&self, url: "ws://localhost:8080") as $ws
//!
//!      @state(when: $ws is Disconnected) {
//!        .status { background: gray; }
//!      }
//!
//!      @state(when: $ws is Connected { $send, $received }) {
//!        .status { background: green; }
//!      }
//!    }
//!    ```
//!
//! 3. **Migration Steps**:
//!    a. Create `socket-demo.st` with the component above
//!    b. Compile it: `spacetime compile socket-demo.st`
//!    c. Replace `SOCKET_DEMO_JS` with `include_str!("socket-demo.compiled.js")`
//!    d. The mock WebSocket infrastructure remains unchanged
//!
//! ## Running
//!
//! ```bash
//! cargo run --example socket_demo --features headless
//! ```

use rustyscript::{Runtime, RuntimeOptions};
use serde_json::Value;

/// WebSocket mock that simulates connection lifecycle
///
/// ## Spacetime Upgrade Note
/// This mock will continue to work after migrating to Spacetime.
/// The socket primitive's `%emit js` block uses standard WebSocket API,
/// which this mock provides.
const WEBSOCKET_MOCK_JS: &str = r#"
// Mock WebSocket with controllable state transitions
class MockWebSocket {
    static CONNECTING = 0;
    static OPEN = 1;
    static CLOSING = 2;
    static CLOSED = 3;

    constructor(url) {
        this.url = url;
        this.readyState = MockWebSocket.CONNECTING;
        this.onopen = null;
        this.onmessage = null;
        this.onclose = null;
        this.onerror = null;

        // Store instance for test control
        globalThis.__mockWs = this;

        // Auto-connect after microtask (simulates async connection)
        setTimeout(() => this._simulateOpen(), 10);
    }

    _simulateOpen() {
        if (this.readyState === MockWebSocket.CONNECTING) {
            this.readyState = MockWebSocket.OPEN;
            if (this.onopen) this.onopen({ type: 'open' });
        }
    }

    _simulateMessage(data) {
        if (this.readyState === MockWebSocket.OPEN && this.onmessage) {
            this.onmessage({ data: typeof data === 'string' ? data : JSON.stringify(data) });
        }
    }

    _simulateError(message) {
        if (this.onerror) {
            this.onerror({ message: message || 'Connection failed' });
        }
    }

    _simulateClose() {
        this.readyState = MockWebSocket.CLOSED;
        if (this.onclose) this.onclose({ type: 'close' });
    }

    send(data) {
        if (this.readyState !== MockWebSocket.OPEN) {
            throw new Error('WebSocket is not open');
        }
        console.log('[WS] Sent:', data);
        // Echo back for demo purposes
        setTimeout(() => this._simulateMessage({ echo: data, timestamp: Date.now() }), 50);
    }

    close() {
        if (this.readyState === MockWebSocket.OPEN) {
            this.readyState = MockWebSocket.CLOSING;
            setTimeout(() => this._simulateClose(), 10);
        }
    }
}

globalThis.WebSocket = MockWebSocket;

// Helper to control WebSocket state from tests
globalThis.__wsControl = {
    connect: () => globalThis.__mockWs?._simulateOpen(),
    message: (data) => globalThis.__mockWs?._simulateMessage(data),
    error: (msg) => globalThis.__mockWs?._simulateError(msg),
    close: () => globalThis.__mockWs?._simulateClose(),
};
"#;

/// Socket state machine demo
///
/// ## Spacetime Upgrade Note
/// This entire block will be replaced by compiled Spacetime output.
/// The structure mirrors what the metasystem generates from:
///
/// ```st
/// @use socket(&self, url: "ws://localhost:8080") as $ws
/// @state(when: $ws is Disconnected) { ... }
/// @state(when: $ws is Connected { $send, $received }) { ... }
/// ```
const SOCKET_DEMO_JS: &str = r#"
// =============================================================================
// Socket Demo - Typed Union State Machine
// =============================================================================
// This JavaScript demonstrates what Spacetime's metasystem generates.
// After Spacetime migration, this becomes compiled output from socket-demo.st

(function() {
    'use strict';

    // State type constants (matches socket primitive's union variants)
    const StateType = {
        Disconnected: 'Disconnected',
        Connecting: 'Connecting',
        Connected: 'Connected',
        Error: 'Error'
    };

    // Current state (typed union)
    let currentState = { type: StateType.Disconnected };

    // State change callbacks (pattern match handlers)
    // NOTE: In Spacetime, these are generated from @state(when: $ws is Variant)
    const stateHandlers = {
        Disconnected: [],
        Connecting: [],
        Connected: [],
        Error: []
    };

    // Register a pattern match handler
    // In Spacetime: @state(when: $ws is Connected { $send, $received }) { ... }
    function onState(variant, handler) {
        if (stateHandlers[variant]) {
            stateHandlers[variant].push(handler);
        }
    }

    // Transition to new state and invoke handlers
    function setState(newState) {
        const oldType = currentState.type;
        currentState = newState;

        console.log(`[State] ${oldType} -> ${newState.type}`);

        // Update data attribute (for CSS pattern matching)
        // In Spacetime: %&el.setAttribute('data-st-state-type', 'Connected')
        if (typeof document !== 'undefined') {
            document.body?.setAttribute('data-st-ws-type', newState.type);
        }

        // Invoke handlers for this state
        const handlers = stateHandlers[newState.type] || [];
        for (const handler of handlers) {
            // Extract bindings from state (Connected has $send, $received)
            const bindings = {};
            if (newState.type === StateType.Connected) {
                bindings.$send = newState.send;
                bindings.$received = newState.received;
            } else if (newState.type === StateType.Error) {
                bindings.$error = newState.error;
            }
            handler(bindings);
        }
    }

    // Initialize socket connection
    // This mirrors the socket primitive's %emit js block
    function initSocket(url) {
        setState({ type: StateType.Connecting });

        try {
            const ws = new WebSocket(url);
            let received = null;

            const send = (msg) => {
                if (ws.readyState === WebSocket.OPEN) {
                    ws.send(typeof msg === 'object' ? JSON.stringify(msg) : msg);
                }
            };

            ws.onopen = () => {
                setState({ type: StateType.Connected, send, received });
            };

            ws.onmessage = (e) => {
                try {
                    received = JSON.parse(e.data);
                } catch {
                    received = e.data;
                }
                if (currentState.type === StateType.Connected) {
                    setState({ type: StateType.Connected, send, received });
                }
            };

            ws.onclose = () => {
                setState({ type: StateType.Disconnected });
            };

            ws.onerror = (e) => {
                setState({ type: StateType.Error, error: e.message || 'Connection failed' });
            };

            return () => ws.close();
        } catch (e) {
            setState({ type: StateType.Error, error: e.message });
            return () => {};
        }
    }

    // Export for test control
    globalThis.__socketDemo = {
        onState,
        initSocket,
        getState: () => currentState,
        StateType
    };
})();
"#;

/// Console setup for capturing output
const CONSOLE_SETUP_JS: &str = r#"
const __logs = [];
const __errors = [];

globalThis.console = {
    log: (...args) => { __logs.push(args.join(' ')); },
    error: (...args) => { __errors.push(args.join(' ')); },
    warn: (...args) => { __logs.push('[WARN] ' + args.join(' ')); },
    info: (...args) => { __logs.push(args.join(' ')); },
};

globalThis.__getLogs = () => __logs;
globalThis.__getErrors = () => __errors;

// Timer mocks for synchronous execution
const __pendingTimers = [];
let __timerId = 0;

globalThis.setTimeout = (fn, delay) => {
    const id = ++__timerId;
    __pendingTimers.push({ id, fn, delay: delay || 0 });
    return id;
};

globalThis.clearTimeout = (id) => {
    const idx = __pendingTimers.findIndex(t => t.id === id);
    if (idx >= 0) __pendingTimers.splice(idx, 1);
};

// Flush all pending timers (called from Rust tick loop)
globalThis.__flushTimers = () => {
    const timers = __pendingTimers.splice(0, __pendingTimers.length);
    for (const t of timers) {
        try { t.fn(); } catch (e) { console.error('Timer error:', e.message); }
    }
    return timers.length;
};
"#;

/// Demo test scenarios
const DEMO_SCENARIOS_JS: &str = r#"
// =============================================================================
// Demo Scenarios - Pattern Match State Handling
// =============================================================================

const demo = globalThis.__socketDemo;
const wsControl = globalThis.__wsControl;

// Register pattern match handlers (what @state generates)
demo.onState('Disconnected', () => {
    console.log('[Handler] Disconnected - showing reconnect UI');
});

demo.onState('Connecting', () => {
    console.log('[Handler] Connecting - showing spinner');
});

demo.onState('Connected', ({ $send, $received }) => {
    console.log('[Handler] Connected - chat UI active');
    console.log('  $send available:', typeof $send === 'function');
    console.log('  $received:', JSON.stringify($received));

    // Send a test message when connected
    if ($send && !globalThis.__sentTestMessage) {
        globalThis.__sentTestMessage = true;
        $send({ type: 'hello', text: 'Hello from pattern match!' });
    }
});

demo.onState('Error', ({ $error }) => {
    console.log('[Handler] Error - showing error:', $error);
});

// Run the demo
console.log('=== Socket Pattern Match Demo ===');
console.log('');

// Initialize connection
const cleanup = demo.initSocket('ws://localhost:8080');

// Store cleanup for later
globalThis.__cleanup = cleanup;
"#;

fn main() {
    println!("Socket Pattern Matching Demo (Headless via V8)");
    println!("================================================\n");

    let mut runtime = Runtime::new(RuntimeOptions::default()).expect("Failed to create V8 runtime");

    // 1. Set up console capture
    runtime
        .eval::<Value>(CONSOLE_SETUP_JS)
        .expect("Console setup failed");

    // 2. Set up WebSocket mock
    runtime
        .eval::<Value>(WEBSOCKET_MOCK_JS)
        .expect("WebSocket mock failed");

    // 3. Load socket demo (future: compiled Spacetime output)
    runtime
        .eval::<Value>(SOCKET_DEMO_JS)
        .expect("Socket demo failed");

    // 4. Run demo scenarios
    runtime
        .eval::<Value>(DEMO_SCENARIOS_JS)
        .expect("Demo scenarios failed");

    // 5. Simulate time passing for async operations
    // In real Spacetime, the runtime handles this via requestAnimationFrame
    for i in 0..10 {
        // Flush pending timers (simulates event loop)
        let tick = format!(
            r#"
            // Tick {} - process async work
            const flushed = globalThis.__flushTimers();
            flushed
            "#,
            i
        );
        let _ = runtime.eval::<Value>(&tick);

        // Small delay simulation
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // 6. Simulate receiving a message
    let _ =
        runtime.eval::<Value>(r#"globalThis.__wsControl.message({ type: 'welcome', users: 42 });"#);

    std::thread::sleep(std::time::Duration::from_millis(100));

    // 7. Print captured logs
    println!("Console Output:");
    println!("---------------");

    let logs_result = runtime.eval::<Value>("JSON.stringify(__getLogs())");
    if let Ok(logs) = logs_result {
        if let Some(logs_text) = logs.as_str() {
            if let Ok(logs_vec) = serde_json::from_str::<Vec<String>>(logs_text) {
                for log in logs_vec {
                    println!("  {}", log);
                }
            }
        }
    }

    // 8. Print final state
    println!("\nFinal State:");
    println!("------------");

    let state_result = runtime.eval::<Value>("JSON.stringify(globalThis.__socketDemo.getState())");
    if let Ok(state) = state_result {
        if let Some(state_str) = state.as_str() {
            println!("  {}", state_str);
        }
    }

    // 9. Cleanup
    let _ = runtime.eval::<Value>("globalThis.__cleanup?.()");

    println!("\n✓ Demo complete");
    println!("\n---");
    println!("To upgrade to Spacetime:");
    println!("  1. Create examples/socket-demo.st with @use socket primitive");
    println!("  2. Compile: cargo run -- compile examples/socket-demo.st");
    println!("  3. Replace SOCKET_DEMO_JS with compiled output");
}
