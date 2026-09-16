# `@data stream` — server-sent events primitive

**Status**: ratified design (July 2026) · implementation sequenced separately
**Layer**: language surface · stdlib primitive · vendored runtime

## Why

Spacetime's data primitives are all request/response: `@data source`
fetches once, `reactive-source` re-fetches when its URL signal changes.
Nothing covers **server-push** — the server talking to the page when IT
has something to say (deploy progress, live build status, collaborative
cursors, notifications). Polling works but is annoying: latency ×
wasted requests × awkward state machines in every consumer.

One primitive unlocks the whole class. This is the harmony rule applied:
not a per-feature hack, one mechanism every live feature reuses.

## Surface

```spacetime
@data stream $push from "/__spacetime/host/push-events"
```

backs `%primitive event-source(name: ident, url: string)` — same shape
as `reactive-source`, so the grammar registry gains one entry, not a
subsystem.

### Yields

| Signal | Type | Meaning |
|---|---|---|
| `$push` | any | the latest event's JSON payload |
| `$push_type` | string | the SSE `event:` name (`"message"` default) |
| `$push_state` | string | `"connecting"` \| `"open"` \| `"closed"` |
| `$push_error` | string? | last transport error, if any |

Consumers react through the **same dual channel** as every other source
(`ST.setData` + global signal store + `local:<name>:updated`), so
`@each`, `:`, and `<-` work identically — a stream is just a source that
updates when the server says so.

## Semantics

- Transport: browser-native `EventSource` — auto-reconnect with
  backoff, `Last-Event-ID` resume, and HTTP/1.1 compatibility come free.
  No WebSocket: one-way server→client is the 95% case, and SSE rides
  plain HTTP through every proxy.
- Payloads are JSON (`data: {...}\n\n`); `JSON.parse` failure publishes
  the raw string (never throws into the reactive graph).
- Named events (`event: progress`) surface via `$<name>_type`; the
  payload always lands in `$<name>` regardless of type. (Shipped v1
  note: native `EventSource` dispatches named events only to listeners
  registered for that exact name — there is no wildcard — so the
  primitive subscribes to unnamed frames and `_type` additionally
  falls back to a string `type` field inside the JSON payload.
  Endpoints that want typed frames, like the dev server's push
  progress, send unnamed frames with a payload `type`. A `%primitive`
  events-list param for true SSE named events is a documented
  follow-up.)
- The connection opens when the primitive mounts and closes on
  navigation/unmount. `withCredentials` off (same-origin default);
  cross-origin streams go through the site's own server proxy, same as
  fetch-based sources.

## Runtime (the one JS home)

The wiring lives in the vendored runtime (`spacetime.js`) and is
**demand-injected only on pages that use the primitive** — the same rail
as `snarkdown` for `@doc` and `stdlib/text`. No custom JS in any `.st`
site; this is a compiler-provided primitive like any other.

## Server contract

Any endpoint speaking `text/event-stream` with `data:` frames. The dev
server uses it first for deploy progress events
(`building → uploading{file, done, total} → committing → done | error`),
which the host widget renders as live per-asset progress.

## Testing

- **Headless (logic world)**: a mock stream endpoint replays scripted
  frames; `@then` claims assert `$push`, `$push_state` transitions, and
  error publication. Deterministic, no network.
- **Live (automation world)**: `@drive` against a real endpoint with a
  scripted event feed — the same probe vocabulary, real EventSource.
