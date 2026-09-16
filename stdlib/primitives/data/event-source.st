/// Server-sent event source primitive (W1-2 / PLAN-005, contract:
/// docs/language/event-source.md).
///
/// NOTE on `_type`: native EventSource dispatches named events
/// (`event: foo`) ONLY to listeners registered for that exact name —
/// there is no wildcard. Unnamed frames always reach `onmessage`. So
/// `_type` is the SSE event name when one is captured ("message"
/// default), falling back to a string `type` field inside the JSON
/// payload — endpoints (like the dev server's push progress) that want
/// typed frames send unnamed frames with a payload type, which every
/// consumer can rely on.
%primitive event-source(
  name: ident,
  url: string
) %uses data-channel {
  %emit js {
    (() => {
      const sourceName = '%name';
      const sourceUrl = %url;
      const publish = (suffix, value) => {
        const signal = sourceName + suffix;
        // Main channel is registry-backed (__stPublish); suffixed channels
        // are plain signals (__stPublishSignal) — both owned by the
        // data-channel module (`%uses data-channel`).
        if (suffix === '') window.__stPublish(sourceName, value);
        else window.__stPublishSignal(signal, value);
      };
      const emit = (type, data) => {
        let value;
        try { value = JSON.parse(data); } catch (_) { value = data; }
        // Payload-carried type wins over the transport default so
        // unnamed frames can still be typed (see the note above).
        const payloadType = (value && typeof value === 'object' && typeof value.type === 'string') ? value.type : null;
        publish('', value);
        publish('_type', payloadType || type || 'message');
      };
      publish('_state', 'connecting');
      publish('_error', null);
      const es = new SpacetimeEventSource(sourceUrl);
      es.onopen = () => { publish('_state', 'open'); publish('_error', null); };
      es.onmessage = e => emit('message', e.data);
      es.onerror = () => {
        publish('_state', es.readyState === 2 ? 'closed' : 'connecting');
        publish('_error', 'event stream transport error');
      };
      window.__stEventSources = window.__stEventSources || [];
      window.__stEventSources.push(es);
    })();
  }
  %exports { $value: any $type: string $state: string $error: string? }
}
