/// Socket Primitive (Typed Union State)
///
/// WebSocket connection with typed union state for pattern matching.
/// Yields a single $state signal that is a typed union, enabling:
///
/// ```st
/// @state(when: $ws is Connected { $send, $received }) {
///   // Only active when connected, with $send and $received in scope
/// }
/// ```
///
/// State variants:
/// - Disconnected: Not connected
/// - Connecting: Connection in progress
/// - Connected { $send, $received }: Active connection with send function and last message
/// - Error { $error }: Connection error with message
///
/// @param &el - Element to bind lifecycle to
/// @param url - WebSocket URL (default: '/ws')
/// @yields $state - Typed union connection state

%primitive socket(&el, url: string = "/ws") {
  %emit js {
    // Initial state
    let state = { type: 'Disconnected' };
    %yield state -> $state;

    // Transition to connecting
    state = { type: 'Connecting' };
    %yield state -> $state;

    try {
      const ws = new WebSocket(%url);
      let received = null;

      // Create send function
      const send = (msg) => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(typeof msg === 'object' ? JSON.stringify(msg) : msg);
        }
      };

      ws.onopen = () => {
        state = { type: 'Connected', send, received };
        %yield state -> $state;
        %&el.dispatchEvent(new CustomEvent('socket.open', { bubbles: false }));
      };

      ws.onmessage = (e) => {
        try {
          received = JSON.parse(e.data);
        } catch {
          received = e.data;
        }
        // Update state with new received value
        if (state.type === 'Connected') {
          state = { type: 'Connected', send, received };
          %yield state -> $state;
        }
        %&el.dispatchEvent(new CustomEvent('socket.message', {
          detail: received,
          bubbles: false
        }));
      };

      ws.onclose = () => {
        state = { type: 'Disconnected' };
        %yield state -> $state;
        %&el.dispatchEvent(new CustomEvent('socket.close', { bubbles: false }));
      };

      ws.onerror = (e) => {
        state = { type: 'Error', error: e.message || 'Connection failed' };
        %yield state -> $state;
        %&el.dispatchEvent(new CustomEvent('socket.error', {
          detail: state.error,
          bubbles: false
        }));
      };

      %cleanup {
        ws.close();
      }
    } catch (e) {
      state = { type: 'Error', error: e.message };
      %yield state -> $state;
    }
  }

  %exports {
    $state: Disconnected | Connecting | Connected { $send, $received } | Error { $error }
  }
}
