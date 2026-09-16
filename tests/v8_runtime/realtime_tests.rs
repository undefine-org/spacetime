//! Runtime tests for SpacetimeRealtime (realtime.js)
//!
//! These tests verify the JavaScript runtime layer for real-time features:
//! - Connection pooling
//! - Auto-reconnect
//! - Optimistic updates with rollback
//! - Presence with throttling
//! - Backend-agnostic message handling

use super::context::V8TestContext;

/// Load realtime.js into a test context
fn ctx_with_realtime() -> V8TestContext {
    let mut ctx = V8TestContext::new();

    // Mock WebSocket for headless testing
    ctx.eval(
        r#"
        // Mock WebSocket for headless testing
        class MockWebSocket {
            static instances = [];
            static CONNECTING = 0;
            static OPEN = 1;
            static CLOSING = 2;
            static CLOSED = 3;

            constructor(url) {
                this.url = url;
                this.readyState = MockWebSocket.OPEN;  // Open immediately for V8
                this._sentMessages = [];
                this._onopen = null;
                this._opened = false;
                MockWebSocket.instances.push(this);
            }

            // Use setter to trigger onopen when it's assigned
            set onopen(fn) {
                this._onopen = fn;
                if (fn && !this._opened && this.readyState === MockWebSocket.OPEN) {
                    this._opened = true;
                    fn(new Event('open'));
                }
            }

            get onopen() {
                return this._onopen;
            }

            send(data) {
                if (this.readyState !== MockWebSocket.OPEN) {
                    throw new Error('WebSocket is not open');
                }
                this._sentMessages.push(JSON.parse(data));
            }

            close() {
                this.readyState = MockWebSocket.CLOSED;
                if (this.onclose) this.onclose(new Event('close'));
            }

            // Test helper: simulate receiving a message
            _receive(msg) {
                if (this.onmessage) {
                    this.onmessage({ data: JSON.stringify(msg) });
                }
            }
        }
        globalThis.WebSocket = MockWebSocket;

        // Also mock location for URL normalization
        if (typeof location === 'undefined') {
            globalThis.location = { protocol: 'http:', host: 'localhost:3000' };
        }

        // Mock setTimeout/clearTimeout for V8 (executes immediately in tests)
        if (typeof setTimeout === 'undefined') {
            let timerId = 0;
            const timers = new Map();
            globalThis.setTimeout = (fn, delay) => {
                const id = ++timerId;
                // Execute immediately for synchronous testing
                fn();
                return id;
            };
            globalThis.clearTimeout = (id) => {
                timers.delete(id);
            };
        }
        "#,
    )
    .expect("Mock WebSocket setup failed");

    // Load st.js first (required for signal system)
    let st_js = include_str!("../../public/runtime/st.js");
    ctx.eval(st_js).expect("Failed to load st.js");

    // Load realtime.js
    let realtime_js = include_str!("../../public/runtime/realtime.js");
    ctx.eval(realtime_js).expect("Failed to load realtime.js");

    ctx
}

// =============================================================================
// Connection Pooling Tests
// =============================================================================

#[test]
fn test_connection_pooling_same_url() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        const conn1 = SpacetimeRealtime.connect('/ws');
        const conn2 = SpacetimeRealtime.connect('/ws');
        conn1 === conn2
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Same URL should return same connection instance"
    );
}

#[test]
fn test_connection_pooling_different_urls() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        const conn1 = SpacetimeRealtime.connect('/ws1');
        const conn2 = SpacetimeRealtime.connect('/ws2');
        conn1 !== conn2
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Different URLs should return different connection instances"
    );
}

#[test]
fn test_connection_url_normalization() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        const conn = SpacetimeRealtime.connect('/api/ws');
        conn.url.startsWith('ws://')
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "URL should be normalized to ws://"
    );
}

// =============================================================================
// Subscription Tests
// =============================================================================

#[test]
fn test_subscribe_returns_unsubscribe() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        const conn = SpacetimeRealtime.connect('/ws');
        const unsub = conn.subscribe('todos', {
            onSnapshot: () => {},
        });
        typeof unsub === 'function'
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "subscribe should return an unsubscribe function"
    );
}

#[test]
fn test_subscribe_sends_subscribe_message() {
    let mut ctx = ctx_with_realtime();

    // Need to wait for WebSocket to open
    ctx.eval(
        r#"
        const conn = SpacetimeRealtime.connect('/ws');
        conn.subscribe('todos', { onSnapshot: () => {} });
        "#,
    )
    .unwrap();

    // Check sent messages after a tick
    let result = ctx.eval(
        r#"
        (() => {
            const ws = MockWebSocket.instances[0];
            // Find Subscribe message
            const msg = ws._sentMessages.find(m => m.type === 'Subscribe');
            return msg ? msg.channel === 'todos' : false;
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Should send Subscribe message with channel"
    );
}

#[test]
fn test_snapshot_handler_called() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            let received = null;
            const conn = SpacetimeRealtime.connect('/ws');
            conn.subscribe('todos', {
                onSnapshot: (data) => { received = data; }
            });

            // Simulate server sending Snapshot
            const ws = MockWebSocket.instances[0];
            ws._receive({ type: 'Snapshot', channel: 'todos', data: [{id: '1', text: 'Test'}] });

            return received && received.length === 1 && received[0].id === '1';
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "onSnapshot should receive data"
    );
}

// =============================================================================
// Optimistic Update Tests
// =============================================================================

#[test]
fn test_optimistic_insert() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            let insertedItem = null;
            const conn = SpacetimeRealtime.connect('/ws');
            conn.subscribe('todos', {
                onSnapshot: () => {},
                onInsert: (item) => { insertedItem = item; }
            });

            // Insert optimistically
            conn.insert('todos', { text: 'New todo' });

            // Check item has temp ID and _pending flag
            return insertedItem && insertedItem._pending === true && insertedItem.id.startsWith('temp_');
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Optimistic insert should have temp ID and _pending flag"
    );
}

#[test]
fn test_optimistic_insert_sends_message() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            const conn = SpacetimeRealtime.connect('/ws');
            conn.subscribe('todos', { onSnapshot: () => {} });
            conn.insert('todos', { text: 'New todo' });

            const ws = MockWebSocket.instances[0];
            const msg = ws._sentMessages.find(m => m.type === 'Insert');
            return msg && msg.channel === 'todos' && msg.item.text === 'New todo' && !!msg.op_id;
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Insert should send message with op_id"
    );
}

#[test]
fn test_reject_triggers_rollback() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            let snapshotCalls = 0;
            let items = [];
            const conn = SpacetimeRealtime.connect('/ws');
            conn.subscribe('todos', {
                onSnapshot: (data) => { snapshotCalls++; items = data; },
                onInsert: () => {}
            });

            // Start with initial snapshot
            const ws = MockWebSocket.instances[0];
            ws._receive({ type: 'Snapshot', channel: 'todos', data: [] });

            // Insert optimistically
            conn.insert('todos', { text: 'New todo' });
            const insertMsg = ws._sentMessages.find(m => m.type === 'Insert');
            const opId = insertMsg.op_id;

            // Server rejects
            ws._receive({ type: 'Reject', op_id: opId, reason: 'Validation failed' });

            // Items should be rolled back to empty
            return items.length === 0;
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Reject should rollback optimistic update"
    );
}

#[test]
fn test_ack_clears_pending() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            const conn = SpacetimeRealtime.connect('/ws');
            const sub = conn.subscriptions.get('todos') ||
                (conn.subscribe('todos', { onSnapshot: () => {}, onInsert: () => {} }),
                 conn.subscriptions.get('todos'));

            // Simulate initial state
            const ws = MockWebSocket.instances[0];
            ws._receive({ type: 'Snapshot', channel: 'todos', data: [] });

            // Insert and get op_id
            conn.insert('todos', { text: 'Test' });
            const insertMsg = ws._sentMessages.find(m => m.type === 'Insert');

            // Before Ack, pending should have the op
            const hadPending = conn.subscriptions.get('todos').pending.size > 0;

            // Server acks
            ws._receive({ type: 'Ack', op_id: insertMsg.op_id });

            // After Ack, pending should be cleared
            const noPending = conn.subscriptions.get('todos').pending.size === 0;

            return hadPending && noPending;
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Ack should clear pending operation"
    );
}

// =============================================================================
// Presence Tests
// =============================================================================

#[test]
fn test_join_presence_returns_controls() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        const conn = SpacetimeRealtime.connect('/ws');
        const presence = conn.joinPresence('room1', {
            onJoin: () => {},
            onLeave: () => {}
        });

        typeof presence.update === 'function' &&
        typeof presence.leave === 'function' &&
        typeof presence.getUsers === 'function'
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "joinPresence should return update, leave, getUsers functions"
    );
}

#[test]
fn test_presence_sends_join_message() {
    let mut ctx = ctx_with_realtime();

    ctx.eval(
        r#"
        const conn = SpacetimeRealtime.connect('/ws');
        conn.joinPresence('room1', { onState: () => {} }, { initialState: { name: 'Alice' } });
        "#,
    )
    .unwrap();

    let result = ctx.eval(
        r#"
        (() => {
            const ws = MockWebSocket.instances[0];
            const msg = ws._sentMessages.find(m => m.type === 'PresenceJoin');
            return msg && msg.room === 'room1' && msg.state.name === 'Alice';
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Should send PresenceJoin message with initial state"
    );
}

#[test]
fn test_presence_state_handler() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            let users = null;
            let myId = null;
            const conn = SpacetimeRealtime.connect('/ws');
            conn.joinPresence('room1', {
                onState: (u, id) => { users = u; myId = id; }
            });

            const ws = MockWebSocket.instances[0];
            ws._receive({
                type: 'PresenceState',
                room: 'room1',
                users: [{ id: 'u1', name: 'Alice' }, { id: 'u2', name: 'Bob' }],
                myId: 'u1'
            });

            return users.length === 2 && myId === 'u1';
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "onState should receive users and myId"
    );
}

#[test]
fn test_presence_join_handler() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            let joined = null;
            const conn = SpacetimeRealtime.connect('/ws');
            conn.joinPresence('room1', {
                onJoin: (user) => { joined = user; }
            });

            const ws = MockWebSocket.instances[0];
            ws._receive({
                type: 'PresenceJoined',
                room: 'room1',
                user: { id: 'u3', name: 'Charlie' }
            });

            return joined && joined.name === 'Charlie';
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "onJoin should receive new user"
    );
}

// =============================================================================
// Custom Message Handler Tests (Backend-Agnostic Mode)
// =============================================================================

#[test]
fn test_custom_message_handler() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            let received = null;
            const conn = SpacetimeRealtime.connect('/ws');
            conn.onMessage('CustomType', (msg) => { received = msg; });

            const ws = MockWebSocket.instances[0];
            ws._receive({ type: 'CustomType', data: { foo: 'bar' } });

            return received && received.data.foo === 'bar';
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Custom message handler should receive messages"
    );
}

#[test]
fn test_custom_message_mapping() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            let received = null;
            const conn = SpacetimeRealtime.connect('/ws');
            conn.subscribe('items', {
                onSnapshot: (data) => { received = data; }
            }, {
                messages: {
                    snapshot: 'ITEMS_LOADED',  // Custom server message type
                    inserted: 'ITEM_ADDED',
                    updated: 'ITEM_CHANGED',
                    deleted: 'ITEM_REMOVED'
                }
            });

            const ws = MockWebSocket.instances[0];
            ws._receive({ type: 'ITEMS_LOADED', channel: 'items', data: [{ id: 1 }] });

            return received && received.length === 1;
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Custom message mapping should work for snapshots"
    );
}

#[test]
fn test_custom_action_mapping() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            const conn = SpacetimeRealtime.connect('/ws');
            conn.subscribe('items', {
                onSnapshot: () => {},
                onInsert: () => {}
            }, {
                actions: {
                    insert: (item, opId) => ({ type: 'CREATE_ITEM', payload: item, requestId: opId }),
                    update: (id, patch, opId) => ({ type: 'UPDATE_ITEM', id, changes: patch }),
                    delete: (id, opId) => ({ type: 'REMOVE_ITEM', id })
                }
            });

            // Trigger insert with custom action
            conn.insert('items', { name: 'Test' });

            const ws = MockWebSocket.instances[0];
            const msg = ws._sentMessages.find(m => m.type === 'CREATE_ITEM');
            return msg && msg.payload.name === 'Test' && !!msg.requestId;
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Custom action mapping should transform outgoing messages"
    );
}

// =============================================================================
// Reconnection Tests
// =============================================================================

#[test]
#[ignore = "V8 reconnect uses setTimeout which doesn't fire synchronously; needs async test harness"]
fn test_reconnect_resubscribes() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            const conn = SpacetimeRealtime.connect('/ws');
            conn.subscribe('todos', { onSnapshot: () => {} });

            const ws1 = MockWebSocket.instances[0];
            const initialMsgs = ws1._sentMessages.length;

            // Simulate disconnect
            ws1.onclose(new Event('close'));

            // Wait for reconnect (triggers new WebSocket)
            // In real code this would be async, but our mock auto-opens
            const ws2 = MockWebSocket.instances[1];

            // Should re-send Subscribe after reconnect
            const hasResubscribe = ws2 && ws2._sentMessages.some(m =>
                m.type === 'Subscribe' && m.channel === 'todos'
            );

            return hasResubscribe;
        })()
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "Reconnect should resubscribe to channels"
    );
}

// =============================================================================
// Disconnect Tests
// =============================================================================

#[test]
fn test_disconnect_clears_state() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        (() => {
            try {
                const conn = SpacetimeRealtime.connect('/ws');
                conn.subscribe('todos', { onSnapshot: () => {} });
                conn.joinPresence('room1', { onState: () => {} });

                const hadSubs = conn.subscriptions.size > 0;
                const hadRooms = conn.presenceRooms.size > 0;

                conn.disconnect();

                return hadSubs && hadRooms &&
                       conn.subscriptions.size === 0 &&
                       conn.presenceRooms.size === 0 &&
                       conn.connected === false;
            } catch (e) {
                return 'error: ' + e.message;
            }
        })()
        "#,
    );

    match result {
        Ok(val) => {
            assert!(
                val.as_bool() == Some(true),
                "Disconnect should clear all state, got: {:?}",
                val
            );
        }
        Err(e) => panic!("Eval failed: {:?}", e),
    }
}

#[test]
fn test_disconnect_all_clears_pool() {
    let mut ctx = ctx_with_realtime();

    let result = ctx.eval(
        r#"
        SpacetimeRealtime.connect('/ws1');
        SpacetimeRealtime.connect('/ws2');

        const hadConnections = SpacetimeRealtime.connections.size === 2;

        SpacetimeRealtime.disconnectAll();

        hadConnections && SpacetimeRealtime.connections.size === 0
        "#,
    );

    assert!(result.is_ok(), "Eval should succeed");
    let val = result.unwrap();
    assert!(
        val.as_bool() == Some(true),
        "disconnectAll should clear connection pool"
    );
}
