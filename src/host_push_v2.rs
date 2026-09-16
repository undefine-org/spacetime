//! Client-side Deploy Protocol v2 orchestration.

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast};

use crate::export::compress::{SMALL_BLOB_THRESHOLD_BYTES, compress, should_compress};
use crate::export::{self, ArtifactFile, Manifest};

#[derive(Debug, Clone, Serialize)]
struct PushRequest<'a> {
    manifest: &'a Manifest,
    artifact_hash: &'a str,
}
#[derive(Debug, Deserialize)]
struct InitResponse {
    missing: Vec<String>,
    presigned: HashMap<String, String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitResponse {
    pub preview_url: String,
    pub immutable_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Progress {
    Building,
    Uploading {
        file: String,
        done: usize,
        total: usize,
    },
    Committing,
    Done {
        preview_url: String,
        immutable_url: String,
        /// Post-commit edge probe result: did the preview URL actually
        /// serve the new hash within the grace window? `None` = probe
        /// not run (widget-driven finalize runs it separately).
        verified: Option<bool>,
    },
    Error {
        message: String,
    },
}

/// All host requests go through one client with hard timeouts: a
/// stalled network must surface as an Error event, never leave the
/// progress hub frozen at an active state forever (the stuck-chip bug).
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_default()
}

#[derive(Debug, thiserror::Error)]
pub enum PushError {
    #[error("host does not support deploy protocol v2")]
    Unsupported,
    #[error("a push is already running")]
    AlreadyRunning,
    #[error("no push is currently running")]
    NotRunning,
    #[error("{0}")]
    Server(String),
    #[error("{0}")]
    Local(String),
}

// ── Progress hub (SSE broadcast) ────────────────────────────────────
// One hub per dev-server process: producers (headless push, widget
// push, grace fallback) publish Progress; the /push-events SSE
// endpoint replays the latest frame to new subscribers then streams
// live. Payloads are serialized Progress JSON — unnamed SSE frames
// with a `type` field, per docs/language/event-source.md's shipped
// `_type` fallback semantics.

pub struct PushHub {
    tx: broadcast::Sender<String>,
    last: RwLock<Option<String>>,
}

static HUB: OnceLock<PushHub> = OnceLock::new();

pub fn hub() -> &'static PushHub {
    HUB.get_or_init(|| {
        let (tx, _) = broadcast::channel(64);
        PushHub {
            tx,
            last: RwLock::new(None),
        }
    })
}

/// Snapshot frames older than this in an active state are settled to
/// `idle` for new subscribers: the producer is provably dead (error
/// paths publish; anything else = panic/kill), and a reload must never
/// resurrect a hours-old "Building…" on the chip.
const STALE_ACTIVE_SECS: u64 = 180;

/// Replace a stale active snapshot with a settled `idle` frame. Pure —
/// unit-tested directly.
fn settle_snapshot(json: String, now: u64) -> String {
    let stale = serde_json::from_str::<serde_json::Value>(&json)
        .ok()
        .map(|frame| {
            let active = matches!(
                frame["type"].as_str(),
                Some("building") | Some("uploading") | Some("committing")
            );
            let age = now.saturating_sub(frame["at"].as_u64().unwrap_or(0));
            active && age > STALE_ACTIVE_SECS
        })
        .unwrap_or(false);
    if stale {
        serde_json::json!({"type": "idle", "at": now}).to_string()
    } else {
        json
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl PushHub {
    pub fn publish(&self, progress: &Progress) {
        let mut frame = serde_json::to_value(progress).expect("Progress serializes");
        frame["at"] = serde_json::json!(now_secs());
        let json = serde_json::to_string(&frame).expect("framed Progress serializes");
        *self.last.write().expect("hub last-event lock") = Some(json.clone());
        // No subscribers is normal (nobody watching) — not an error.
        let _ = self.tx.send(json);
    }

    /// (latest-frame snapshot, live receiver) for a new SSE subscriber.
    /// A stale active snapshot is replaced with a settled `idle` frame —
    /// the stored `last` is left untouched (live subscribers of the dead
    /// push already saw what there was to see).
    pub fn subscribe(&self) -> (Option<String>, broadcast::Receiver<String>) {
        let snapshot = self
            .last
            .read()
            .expect("hub last-event lock")
            .clone()
            .map(|json| settle_snapshot(json, now_secs()));
        (snapshot, self.tx.subscribe())
    }
}

/// axum handler: `GET /__spacetime/host/push-events` — text/event-stream.
pub async fn push_events_handler() -> axum::response::Response {
    use axum::response::IntoResponse;
    use axum::response::sse::{Event, Sse};

    let (snapshot, rx) = hub().subscribe();
    let stream = futures_util::stream::unfold((snapshot, rx), |(snapshot, mut rx)| async move {
        if let Some(frame) = snapshot {
            return Some((
                Ok::<_, Infallible>(Event::default().data(frame)),
                (None, rx),
            ));
        }
        match rx.recv().await {
            Ok(frame) => Some((Ok(Event::default().data(frame)), (None, rx))),
            // Lagged or closed: end the stream — EventSource
            // reconnects and gets the snapshot frame instead.
            Err(_) => None,
        }
    });
    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keepalive"),
        )
        .into_response()
}

// ── Rebuild-skip cache ──────────────────────────────────────────────
// The source fingerprint gates the expensive export: unchanged source
// + a cached manifest means init can run WITHOUT build_artifact. Only
// when the server asks for actual blob bytes (missing non-empty) do we
// pay for the build — which is deterministic, so the rebuilt manifest
// equals the cached one and the init response stays valid.

fn cache_dir(site_dir: &Path) -> std::path::PathBuf {
    site_dir.join(".spacetime")
}

fn load_cached_manifest(site_dir: &Path, fingerprint_hash: &str) -> Option<Manifest> {
    let dir = cache_dir(site_dir);
    let stored_fp = std::fs::read_to_string(dir.join("host-fingerprint")).ok()?;
    if stored_fp.trim() != fingerprint_hash {
        return None;
    }
    let json = std::fs::read_to_string(dir.join("host-manifest.json")).ok()?;
    serde_json::from_str(&json).ok()
}

fn persist_cache(
    site_dir: &Path,
    fingerprint: &export::Fingerprint,
    manifest: &Manifest,
) -> Result<(), PushError> {
    let dir = cache_dir(site_dir);
    std::fs::create_dir_all(&dir).map_err(|e| PushError::Local(e.to_string()))?;
    std::fs::write(dir.join("host-fingerprint"), &fingerprint.hash)
        .map_err(|e| PushError::Local(e.to_string()))?;
    // Full fingerprint as JSON: the next push diffs file maps for the
    // widget's changed-files list.
    let fp_json = serde_json::json!({ "hash": fingerprint.hash, "files": fingerprint.files });
    std::fs::write(dir.join("host-fingerprint.json"), fp_json.to_string())
        .map_err(|e| PushError::Local(e.to_string()))?;
    let json = serde_json::to_string(manifest).map_err(|e| PushError::Local(e.to_string()))?;
    std::fs::write(dir.join("host-manifest.json"), json)
        .map_err(|e| PushError::Local(e.to_string()))
}

/// The previous push's full fingerprint, for the changed-files diff.
fn load_cached_fingerprint(site_dir: &Path) -> Option<export::Fingerprint> {
    let json = std::fs::read_to_string(cache_dir(site_dir).join("host-fingerprint.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&json).ok()?;
    let hash = value.get("hash")?.as_str()?.to_string();
    let files = value
        .get("files")?
        .as_object()?
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
        .collect();
    Some(export::Fingerprint { hash, files })
}

/// The shared build+init front half of every push flavor. Returns the
/// manifest, the server's init response, and the artifact ONLY if it
/// was actually built (None when the cache supplied the manifest and
/// the server wants no bytes).
struct Prepared {
    manifest: Manifest,
    init: InitResponse,
    blobs: Option<BTreeMap<String, (String, Vec<u8>)>>,
    fingerprint: export::Fingerprint,
    /// Files added/removed/changed since the last successful push —
    /// the widget pill's "N changed" list.
    changed: Vec<String>,
}

async fn prepare(
    site_dir: &Path,
    api_base: &str,
    project_id: &str,
    token: &str,
    progress: &mut (dyn FnMut(Progress) + Send),
) -> Result<Prepared, PushError> {
    progress(Progress::Building);
    let fingerprint = export::fingerprint(site_dir).map_err(|e| PushError::Local(e.to_string()))?;
    // First push (no prior fingerprint): every source file is new.
    let changed = load_cached_fingerprint(site_dir)
        .map(|prev| export::changed_files(&prev, &fingerprint))
        .unwrap_or_else(|| fingerprint.files.keys().cloned().collect());
    let cached = load_cached_manifest(site_dir, &fingerprint.hash);
    let (manifest, mut artifact) = match cached {
        Some(manifest) => (manifest, None),
        None => {
            let built =
                export::build_artifact(site_dir).map_err(|e| PushError::Local(e.to_string()))?;
            let manifest = built.manifest.clone();
            (manifest, Some(built))
        }
    };
    let init = init_request(api_base, project_id, token, &manifest).await?;
    if !init.missing.is_empty() && artifact.is_none() {
        // Cache said the build could be skipped, but the server wants
        // bytes — pay for the deterministic rebuild now. The manifest
        // is identical (build-twice byte-equal is tested), so the init
        // response above remains valid.
        artifact =
            Some(export::build_artifact(site_dir).map_err(|e| PushError::Local(e.to_string()))?);
    }
    let blobs = artifact
        .as_ref()
        .map(|a| artifact_blobs(&a.files, &a.manifest));
    Ok(Prepared {
        manifest,
        init,
        blobs,
        changed,
        fingerprint,
    })
}

async fn init_request(
    api_base: &str,
    project_id: &str,
    token: &str,
    manifest: &Manifest,
) -> Result<InitResponse, PushError> {
    let client = http_client();
    let base = api_base.trim_end_matches('/');
    let response = client
        .post(format!("{base}/api/v1/projects/{project_id}/push/init"))
        .bearer_auth(token)
        .json(&PushRequest {
            manifest,
            artifact_hash: &manifest.artifact_hash,
        })
        .send()
        .await
        .map_err(|e| PushError::Server(format!("could not reach Spacetime Host: {e}")))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(PushError::Unsupported);
    }
    if !response.status().is_success() {
        return Err(PushError::Server(response.text().await.unwrap_or_default()));
    }
    response
        .json()
        .await
        .map_err(|e| PushError::Server(format!("invalid init response: {e}")))
}

/// The shared commit back half (choke point is server-side; this is
/// just the call). Used by the headless push and by widget finalize.
async fn commit_request(
    api_base: &str,
    project_id: &str,
    token: &str,
    source: &str,
    manifest: &Manifest,
) -> Result<CommitResponse, PushError> {
    let client = http_client();
    let base = api_base.trim_end_matches('/');
    let response = client
        .post(format!("{base}/api/v1/projects/{project_id}/push/commit"))
        .bearer_auth(token)
        .header("x-spacetime-source", source)
        .json(&PushRequest {
            manifest,
            artifact_hash: &manifest.artifact_hash,
        })
        .send()
        .await
        .map_err(|e| PushError::Server(format!("commit failed: {e}")))?;
    if !response.status().is_success() {
        return Err(PushError::Server(response.text().await.unwrap_or_default()));
    }
    response
        .json()
        .await
        .map_err(|e| PushError::Server(format!("invalid commit response: {e}")))
}

/// Builds, deduplicates, uploads, then commits an artifact (headless /
/// CLI flavor — the dev server uploads everything itself). A 404 from
/// init is deliberately distinguishable so the caller can retain the
/// v1 fallback. Progress publishes to the hub so SSE watchers see CLI
/// pushes too.
pub async fn push<F>(
    site_dir: &Path,
    api_base: &str,
    project_id: &str,
    token: &str,
    source: &str,
    mut progress: F,
) -> Result<CommitResponse, PushError>
where
    F: FnMut(Progress) + Send,
{
    let mut emit = |p: Progress| {
        hub().publish(&p);
        progress(p);
    };
    let prepared = prepare(site_dir, api_base, project_id, token, &mut emit).await?;
    // On a cache hit with an empty missing list there is no artifact
    // and nothing to upload — skip straight to commit.
    let empty_blobs = BTreeMap::new();
    let blobs = prepared.blobs.as_ref().unwrap_or(&empty_blobs);
    // Content-addressing dedupes: files with identical bytes share a
    // blob, so the server's missing list can name a blob twice —
    // upload each unique blob once.
    let missing: Vec<&str> = prepared
        .init
        .missing
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let total = missing.len();
    for (done, hash) in missing.iter().copied().enumerate() {
        let (path, bytes) = blobs
            .get(hash)
            .ok_or_else(|| PushError::Local(format!("init requested unknown blob {hash}")))?;
        emit(Progress::Uploading {
            file: path.clone(),
            done,
            total,
        });
        upload_blob(
            api_base,
            project_id,
            token,
            &prepared.init,
            hash,
            path,
            bytes,
        )
        .await?;
    }
    emit(Progress::Committing);
    let commit = commit_request(api_base, project_id, token, source, &prepared.manifest).await?;
    persist_cache(site_dir, &prepared.fingerprint, &prepared.manifest)?;
    let verified = probe_preview(&commit).await;
    emit(Progress::Done {
        preview_url: commit.preview_url.clone(),
        immutable_url: commit.immutable_url.clone(),
        verified: Some(verified),
    });
    Ok(commit)
}

/// Post-commit verification (W5-1): the preview URL must serve the new
/// deploy within the KV-propagation window. Up to ~30s of polling; a
/// failure here means the edge never caught up — the push COMMITTED,
/// so this is reported, never retried silently.
pub async fn probe_preview(commit: &CommitResponse) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();
    let attempts: usize = std::env::var("SPACETIME_PROBE_ATTEMPTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6);
    for _ in 0..attempts {
        if let Ok(resp) = client.get(&commit.preview_url).send().await {
            if resp.status().is_success() {
                return true;
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
    false
}

/// One blob upload: presigned PUT straight to R2 when the server
/// minted a URL (large blobs, identity wire format — R2 stores exactly
/// what the edge serves), else /push/blob through the orchestrator
/// with the compress policy's framing.
async fn upload_blob(
    api_base: &str,
    project_id: &str,
    token: &str,
    init: &InitResponse,
    hash: &str,
    path: &str,
    bytes: &[u8],
) -> Result<(), PushError> {
    let client = http_client();
    let base = api_base.trim_end_matches('/');
    if let Some(url) = init.presigned.get(hash) {
        let response = client
            .put(url)
            .body(bytes.to_vec())
            .send()
            .await
            .map_err(|e| PushError::Server(format!("presigned upload failed: {e}")))?;
        if !response.status().is_success() {
            return Err(PushError::Server(response.text().await.unwrap_or_default()));
        }
        return Ok(());
    }
    let mut request = client
        .post(format!(
            "{base}/api/v1/projects/{project_id}/push/blob?b3={hash}"
        ))
        .bearer_auth(token);
    if should_compress(path) {
        request = request
            .header(reqwest::header::CONTENT_ENCODING, "zstd")
            .body(compress(bytes).map_err(PushError::Local)?);
    } else {
        request = request.body(bytes.to_vec());
    }
    let response = request
        .send()
        .await
        .map_err(|e| PushError::Server(format!("blob upload failed: {e}")))?;
    if !response.status().is_success() {
        return Err(PushError::Server(response.text().await.unwrap_or_default()));
    }
    Ok(())
}

// ── Widget-driven push ──────────────────────────────────────────────
// The browser widget performs large-blob uploads direct to R2 via the
// presigned URLs (ratified 2026-07-20); the dev server is the
// conductor and the ONLY party that talks to the orchestrator. State
// lives in ONE struct behind a mutex. A grace timer covers a widget
// that disconnects mid-upload: the dev server finishes the remaining
// uploads itself so a closed laptop lid never wedges a push.

/// Seconds the dev server waits for the widget before taking over the
/// remaining presigned uploads itself.
fn widget_grace_secs() -> u64 {
    std::env::var("SPACETIME_WIDGET_GRACE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60)
}

pub struct PendingUpload {
    pub b3: String,
    pub url: String,
    pub path: String,
    pub size: u64,
    bytes: Vec<u8>,
}

struct InFlightPush {
    api_base: String,
    project_id: String,
    token: String,
    site_dir: std::path::PathBuf,
    manifest: Manifest,
    fingerprint: export::Fingerprint,
    pending: HashMap<String, PendingUpload>,
    grace: Option<tokio::task::JoinHandle<()>>,
}

static IN_FLIGHT: OnceLock<Mutex<Option<InFlightPush>>> = OnceLock::new();

fn in_flight() -> &'static Mutex<Option<InFlightPush>> {
    IN_FLIGHT.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingInfo {
    pub b3: String,
    pub url: String,
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WidgetPushStatus {
    /// The dev server uploaded the small blobs; the widget owns the
    /// presigned (large) ones. `changed` is the source diff since the
    /// last successful push — the pill's "N changed · M to upload".
    Uploading {
        pending: Vec<PendingInfo>,
        pending_bytes: u64,
        changed: Vec<String>,
    },
    Done {
        response: CommitResponse,
    },
}

/// `POST /__spacetime/host/push-v2/start` — widget flavor: build + init
/// + the dev server uploads all SMALL blobs itself, then the remaining
/// presigned (large) uploads are handed to the widget.
pub async fn widget_start(
    site_dir: &Path,
    api_base: &str,
    project_id: &str,
    token: &str,
) -> Result<WidgetPushStatus, PushError> {
    // The guard is NEVER held across prepare (build + network): a queued
    // start must fail fast with AlreadyRunning, not hang the HTTP handler
    // behind someone else's multi-second build.
    {
        let guard = in_flight().lock().await;
        if guard.is_some() {
            return Err(PushError::AlreadyRunning);
        }
    }
    let result = widget_start_inner(site_dir, api_base, project_id, token).await;
    if let Err(e) = &result {
        // Every failure leaves an Error frame — without it the hub's last
        // frame stays Building/Committing forever and the chip is stuck.
        // AlreadyRunning is not a failure of the running push: stay quiet.
        if !matches!(e, PushError::AlreadyRunning) {
            hub().publish(&Progress::Error {
                message: e.to_string(),
            });
        }
    }
    result
}

async fn widget_start_inner(
    site_dir: &Path,
    api_base: &str,
    project_id: &str,
    token: &str,
) -> Result<WidgetPushStatus, PushError> {
    let prepared = prepare(site_dir, api_base, project_id, token, &mut |p| {
        hub().publish(&p)
    })
    .await?;
    let blobs = prepared.blobs.unwrap_or_default();
    let missing: Vec<&str> = prepared
        .init
        .missing
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let total = missing.len();
    let mut pending = HashMap::new();
    for (done, hash) in missing.iter().copied().enumerate() {
        let (path, bytes) = blobs
            .get(hash)
            .ok_or_else(|| PushError::Local(format!("init requested unknown blob {hash}")))?;
        if let Some(url) = prepared.init.presigned.get(hash) {
            pending.insert(
                hash.to_string(),
                PendingUpload {
                    b3: hash.to_string(),
                    url: url.clone(),
                    path: path.clone(),
                    size: bytes.len() as u64,
                    bytes: bytes.clone(),
                },
            );
        } else {
            hub().publish(&Progress::Uploading {
                file: path.clone(),
                done,
                total,
            });
            upload_blob(
                api_base,
                project_id,
                token,
                &prepared.init,
                hash,
                path,
                bytes,
            )
            .await?;
        }
    }

    if pending.is_empty() {
        // Nothing for the widget — commit immediately. Error frames are
        // published by the widget_start wrapper above.
        hub().publish(&Progress::Committing);
        let response =
            commit_request(api_base, project_id, token, "widget", &prepared.manifest).await?;
        persist_cache(site_dir, &prepared.fingerprint, &prepared.manifest)?;
        let verified = probe_preview(&response).await;
        hub().publish(&Progress::Done {
            preview_url: response.preview_url.clone(),
            immutable_url: response.immutable_url.clone(),
            verified: Some(verified),
        });
        return Ok(WidgetPushStatus::Done { response });
    }

    let prepared_changed = prepared.changed.clone();
    let mut guard = in_flight().lock().await;
    if guard.is_some() {
        // A concurrent start won the slot while we built.
        return Err(PushError::AlreadyRunning);
    }
    let flight = InFlightPush {
        api_base: api_base.to_string(),
        project_id: project_id.to_string(),
        token: token.to_string(),
        site_dir: site_dir.to_path_buf(),
        manifest: prepared.manifest,
        fingerprint: prepared.fingerprint,
        grace: None,
        pending,
    };
    let pending_info = flight.pending.values().map(pending_info_of).collect();
    let pending_bytes = flight.pending.values().map(|upload| upload.size).sum();
    let changed = prepared_changed;
    *guard = Some(flight);
    drop(guard);
    // The grace clock starts only now that the widget has its work list
    // — spawning it earlier would let a slow build eat the widget's window.
    let handle = spawn_grace_timer();
    if let Some(flight) = in_flight().lock().await.as_mut() {
        flight.grace = Some(handle);
    }
    Ok(WidgetPushStatus::Uploading {
        pending: pending_info,
        pending_bytes,
        changed,
    })
}

fn pending_info_of(u: &PendingUpload) -> PendingInfo {
    PendingInfo {
        b3: u.b3.clone(),
        url: u.url.clone(),
        path: u.path.clone(),
        size: u.size,
    }
}

/// `GET /__spacetime/host/push-upload-urls` — the widget's work list.
pub async fn widget_pending() -> Result<Vec<PendingInfo>, PushError> {
    let guard = in_flight().lock().await;
    let flight = guard.as_ref().ok_or(PushError::NotRunning)?;
    Ok(flight.pending.values().map(pending_info_of).collect())
}

/// `GET /__spacetime/host/push-blob/{b3}` — bytes for one pending browser
/// upload. The b3 lookup is exact and the bytes remain available only while
/// the in-flight widget push owns that pending entry.
pub async fn widget_blob_bytes(b3: &str) -> Option<Vec<u8>> {
    let guard = in_flight().lock().await;
    guard
        .as_ref()?
        .pending
        .get(b3)
        .map(|upload| upload.bytes.clone())
}

/// `POST /__spacetime/host/push-uploads-complete` — the widget reports
/// finished presigned PUTs; the last one triggers the commit.
pub async fn widget_complete(done: &[String]) -> Result<WidgetPushStatus, PushError> {
    let mut guard = in_flight().lock().await;
    let flight = guard.as_mut().ok_or(PushError::NotRunning)?;
    for b3 in done {
        flight.pending.remove(b3);
    }
    if !flight.pending.is_empty() {
        return Ok(WidgetPushStatus::Uploading {
            pending: flight.pending.values().map(pending_info_of).collect(),
            pending_bytes: flight.pending.values().map(|upload| upload.size).sum(),
            changed: Vec::new(),
        });
    }
    let flight = guard.take().expect("checked above");
    drop(guard);
    finalize_and_report(flight).await
}

/// Finalize + guarantee an Error frame on failure: callers return the
/// error over HTTP, but the chip watches the SSE hub — a silent Err
/// leaves it stuck at Committing forever.
async fn finalize_and_report(flight: InFlightPush) -> Result<WidgetPushStatus, PushError> {
    match finalize_widget_push(flight).await {
        Ok(status) => Ok(status),
        Err(e) => {
            hub().publish(&Progress::Error {
                message: e.to_string(),
            });
            Err(e)
        }
    }
}

/// The grace timer: if the widget goes silent, the dev server uploads
/// the remaining presigned blobs itself and commits.
fn spawn_grace_timer() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(widget_grace_secs())).await;
        // Take ANY flight: pending-empty means the widget finished its
        // uploads but died before /push-uploads-complete — abandoning the
        // flight here poisons IN_FLIGHT (every future start gets
        // AlreadyRunning) and silently drops a ready-to-commit push.
        let flight = in_flight().lock().await.take();
        let Some(mut flight) = flight else { return };
        // The grace task IS the timer — never let finalize abort this
        // very task at its next await point.
        flight.grace = None;
        let _remaining = flight.pending.len(); // grace takeover; progress events narrate
        let init_urls: HashMap<String, String> = flight
            .pending
            .values()
            .map(|u| (u.b3.clone(), u.url.clone()))
            .collect();
        let init = InitResponse {
            missing: flight.pending.keys().cloned().collect(),
            presigned: init_urls,
        };
        let total = init.missing.len();
        for (done, hash) in init.missing.iter().enumerate() {
            let Some(upload) = flight.pending.remove(hash) else {
                continue;
            };
            hub().publish(&Progress::Uploading {
                file: upload.path.clone(),
                done,
                total,
            });
            if let Err(e) = upload_blob(
                &flight.api_base,
                &flight.project_id,
                &flight.token,
                &init,
                &upload.b3,
                &upload.path,
                &upload.bytes,
            )
            .await
            {
                hub().publish(&Progress::Error {
                    message: e.to_string(),
                });
                return;
            }
        }
        let _ = finalize_and_report(flight).await;
    })
}

async fn finalize_widget_push(flight: InFlightPush) -> Result<WidgetPushStatus, PushError> {
    if let Some(grace) = &flight.grace {
        grace.abort();
    }
    hub().publish(&Progress::Committing);
    let response = commit_request(
        &flight.api_base,
        &flight.project_id,
        &flight.token,
        "widget",
        &flight.manifest,
    )
    .await?;
    persist_cache(&flight.site_dir, &flight.fingerprint, &flight.manifest)?;
    let verified = probe_preview(&response).await;
    hub().publish(&Progress::Done {
        preview_url: response.preview_url.clone(),
        immutable_url: response.immutable_url.clone(),
        verified: Some(verified),
    });
    Ok(WidgetPushStatus::Done { response })
}

fn artifact_blobs(
    files: &[ArtifactFile],
    manifest: &Manifest,
) -> BTreeMap<String, (String, Vec<u8>)> {
    let mut out = BTreeMap::new();
    for (file, entry) in files.iter().zip(&manifest.files) {
        if entry.chunks.is_some() {
            for chunk in
                fastcdc::v2020::FastCDC::new(&file.bytes, 512 * 1024, 1024 * 1024, 8 * 1024 * 1024)
            {
                let bytes = file.bytes[chunk.offset..chunk.offset + chunk.length].to_vec();
                out.insert(
                    blake3::hash(&bytes).to_hex().to_string(),
                    (file.path.clone(), bytes),
                );
            }
        } else {
            debug_assert!((file.bytes.len() as u64) <= SMALL_BLOB_THRESHOLD_BYTES);
            out.insert(entry.b3.clone(), (file.path.clone(), file.bytes.clone()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunks_are_addressed_by_content() {
        let bytes = vec![1; 9 * 1024 * 1024];
        let file = ArtifactFile {
            path: "asset.bin".into(),
            bytes: bytes.clone(),
        };
        let entry = crate::export::FileEntry {
            path: file.path.clone(),
            size: bytes.len() as u64,
            b3: "x".repeat(64),
            chunks: Some(vec![]),
        };
        let manifest = Manifest {
            artifact_hash: String::new(),
            compiler: String::new(),
            files: vec![entry],
            site: crate::export::SiteMeta {
                deploy: serde_json::Value::Null,
                locales: vec![],
            },
        };
        assert!(!artifact_blobs(&[file], &manifest).is_empty());
    }

    // ── Stuck-state regressions (the "chip shows building forever" bug) ──

    #[test]
    fn stale_active_snapshot_settles_to_idle() {
        let old = now_secs() - STALE_ACTIVE_SECS - 10;
        for active in ["building", "uploading", "committing"] {
            let frame = serde_json::json!({"type": active, "at": old}).to_string();
            let settled: serde_json::Value =
                serde_json::from_str(&settle_snapshot(frame, now_secs())).unwrap();
            assert_eq!(settled["type"], "idle", "{active} must settle");
        }
    }

    #[test]
    fn fresh_or_settled_snapshots_pass_through() {
        let now = now_secs();
        for (ty, at) in [
            ("building", now),                     // fresh active
            ("building", now - STALE_ACTIVE_SECS), // boundary: not yet stale
            ("done", now - 86_400),                // settled states never settle further
            ("error", now - 86_400),
            ("idle", now - 86_400),
        ] {
            let frame = serde_json::json!({"type": ty, "at": at}).to_string();
            let out = settle_snapshot(frame.clone(), now);
            assert_eq!(out, frame, "{ty}@{at} must pass through");
        }
    }

    #[tokio::test]
    async fn widget_start_failure_publishes_error_frame() {
        let _serial = WIDGET_TEST_LOCK
            .get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await;
        // prepare fails fast: the site dir does not exist — the hub must
        // end on an error frame, never frozen at Building.
        let result = widget_start(
            Path::new("/nonexistent-site-dir-xyz"),
            "http://127.0.0.1:1",
            "proj",
            "tok",
        )
        .await;
        assert!(result.is_err());
        let last = hub()
            .last
            .read()
            .expect("hub lock")
            .clone()
            .expect("a frame");
        let frame: serde_json::Value = serde_json::from_str(&last).unwrap();
        assert_eq!(frame["type"], "error", "last frame: {last}");
        assert!(frame["at"].as_u64().is_some(), "frame carries a timestamp");
    }

    #[tokio::test]
    async fn grace_timer_commits_when_widget_dies_after_uploads() {
        let _serial = WIDGET_TEST_LOCK
            .get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await;
        // A flight whose pending list is ALREADY EMPTY (widget did every
        // upload, then the tab crashed before /push-uploads-complete):
        // the grace timer must take it and commit, not abandon it.
        let (base, mock) = spawn_orchestrator().await;
        let dir = tempfile::tempdir().unwrap();
        write_test_site(dir.path(), &[]);
        let built = crate::export::build_artifact(dir.path()).unwrap();
        let manifest = built.manifest;
        // The scenario: the widget completed EVERY upload — so the blobs
        // are server-side — then died before /push-uploads-complete.
        {
            let mut store = mock.blobs.lock().unwrap();
            for file in &manifest.files {
                match &file.chunks {
                    Some(chunks) => chunks.iter().for_each(|c| {
                        store.insert(c.b3.clone());
                    }),
                    None => {
                        store.insert(file.b3.clone());
                    }
                }
            }
        }
        let flight = InFlightPush {
            api_base: base.clone(),
            project_id: "proj".into(),
            token: "tok".into(),
            site_dir: dir.path().to_path_buf(),
            manifest,
            fingerprint: crate::export::fingerprint(dir.path()).unwrap(),
            grace: None,
            pending: HashMap::new(),
        };
        *in_flight().lock().await = Some(flight);
        let saved_grace = std::env::var("SPACETIME_WIDGET_GRACE_SECS").ok();
        unsafe { std::env::set_var("SPACETIME_WIDGET_GRACE_SECS", "0") };
        let handle = spawn_grace_timer();
        handle.await.unwrap();
        match saved_grace {
            Some(v) => unsafe { std::env::set_var("SPACETIME_WIDGET_GRACE_SECS", v) },
            None => unsafe { std::env::remove_var("SPACETIME_WIDGET_GRACE_SECS") },
        }
        assert_eq!(
            *mock.commit_count.lock().unwrap(),
            1,
            "grace must commit the upload-complete flight"
        );
        assert!(in_flight().lock().await.is_none(), "flight consumed");
    }

    // ── Mock-orchestrator integration tests ─────────────────────────
    // An axum server speaking the REAL v2 contract (init diff, blob
    // hash verification, presigned PUT, commit choke point) over an
    // in-memory blob store.

    use std::sync::{Arc, Mutex as StdMutex};

    #[derive(Default)]
    struct MockOrchestrator {
        blobs: StdMutex<std::collections::HashSet<String>>,
        blob_upload_count: StdMutex<usize>,
        commit_count: StdMutex<usize>,
        reject_blob_hash: StdMutex<Option<String>>,
        /// Own origin for minting absolute presigned URLs — each test's
        /// orchestrator lives on its OWN tokio runtime, so a shared
        /// static base would point at a dead port.
        base: StdMutex<String>,
    }

    async fn spawn_orchestrator() -> (String, Arc<MockOrchestrator>) {
        use axum::extract::{Path as AxPath, Query, State};
        use axum::response::IntoResponse;
        use axum::routing::{post, put};

        #[derive(Deserialize)]
        struct BlobQ {
            b3: String,
        }

        let mock = Arc::new(MockOrchestrator::default());
        let app = axum::Router::new()
            .route(
                "/api/v1/projects/{id}/push/init",
                post(
                    |State(mock): State<Arc<MockOrchestrator>>,
                     axum::Json(req): axum::Json<serde_json::Value>| async move {
                        let manifest = req.get("manifest").unwrap().clone();
                        let manifest: crate::export::Manifest =
                            serde_json::from_value(manifest).unwrap();
                        // Choke-point discipline even in the mock: the
                        // claimed artifact hash must verify.
                        assert_eq!(
                            crate::export::compute_artifact_hash(&manifest)
                                .to_hex()
                                .to_string(),
                            manifest.artifact_hash
                        );
                        let store = mock.blobs.lock().unwrap();
                        let mut missing = Vec::new();
                        let mut presigned = serde_json::Map::new();
                        for file in &manifest.files {
                            let blob_refs: Vec<(&str, u64)> = match &file.chunks {
                                Some(chunks) => chunks
                                    .iter()
                                    .map(|c| (c.b3.as_str(), c.size))
                                    .collect(),
                                None => vec![(file.b3.as_str(), file.size)],
                            };
                            for (b3, size) in blob_refs {
                                if !store.contains(b3) {
                                    missing.push(b3.to_string());
                                    // Mirror the server: presign blobs
                                    // >= 4 MiB.
                                    if size >= SMALL_BLOB_THRESHOLD_BYTES {
                                        let base = mock.base.lock().unwrap().clone();
                                        presigned.insert(
                                            b3.to_string(),
                                            serde_json::Value::String(format!(
                                                "{base}/mock-r2/{b3}"
                                            )),
                                        );
                                    }
                                }
                            }
                        }
                        axum::Json(serde_json::json!({
                            "missing": missing,
                            "presigned": presigned,
                            "quota": {"max_files": 10000, "max_bytes": 0, "used_files": 0, "used_bytes": 0}
                        }))
                    },
                ),
            )
            .route(
                "/api/v1/projects/{id}/push/blob",
                post(
                    |State(mock): State<Arc<MockOrchestrator>>,
                     Query(q): Query<BlobQ>,
                     headers: axum::http::HeaderMap,
                     body: axum::body::Bytes| async move {
                        let raw = if headers
                            .get(axum::http::header::CONTENT_ENCODING)
                            .and_then(|v| v.to_str().ok())
                            == Some("zstd")
                        {
                            crate::export::compress::decompress(&body, 64 * 1024 * 1024)
                                .map_err(|e| {
                                    (axum::http::StatusCode::BAD_REQUEST, e.to_string())
                                        .into_response()
                                })?
                        } else {
                            body.to_vec()
                        };
                        let computed = blake3::hash(&raw).to_hex().to_string();
                        if computed != q.b3
                            || mock.reject_blob_hash.lock().unwrap().as_deref() == Some(q.b3.as_str())
                        {
                            return Err((
                                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                                "hash mismatch",
                            )
                                .into_response());
                        }
                        mock.blobs.lock().unwrap().insert(q.b3);
                        *mock.blob_upload_count.lock().unwrap() += 1;
                        Ok(axum::http::StatusCode::OK.into_response())
                    },
                ),
            )
            .route(
                "/mock-r2/{b3}",
                put(
                    |State(mock): State<Arc<MockOrchestrator>>,
                     AxPath(b3): AxPath<String>,
                     body: axum::body::Bytes| async move {
                        let computed = blake3::hash(&body).to_hex().to_string();
                        if computed != b3 {
                            return axum::http::StatusCode::UNPROCESSABLE_ENTITY;
                        }
                        mock.blobs.lock().unwrap().insert(b3);
                        axum::http::StatusCode::OK
                    },
                ),
            )
            .route(
                "/api/v1/projects/{id}/push/commit",
                post(
                    |State(mock): State<Arc<MockOrchestrator>>,
                     axum::Json(req): axum::Json<serde_json::Value>| async move {
                        let manifest: crate::export::Manifest =
                            serde_json::from_value(req.get("manifest").unwrap().clone()).unwrap();
                        let store = mock.blobs.lock().unwrap();
                        for file in &manifest.files {
                            let blob_refs: Vec<&str> = match &file.chunks {
                                Some(chunks) => chunks.iter().map(|c| c.b3.as_str()).collect(),
                                None => vec![file.b3.as_str()],
                            };
                            for b3 in blob_refs {
                                if !store.contains(b3) {
                                    return (
                                        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                                        format!("blob {b3} missing at commit"),
                                    )
                                        .into_response();
                                }
                            }
                        }
                        *mock.commit_count.lock().unwrap() += 1;
                        axum::Json(serde_json::json!({
                            "preview_url": "https://proj.preview.example",
                            "immutable_url": "https://proj-abcd1234.preview.example"
                        }))
                        .into_response()
                    },
                ),
            )
            .with_state(mock.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        *mock.base.lock().unwrap() = format!("http://{addr}");
        let app = app.layer(axum::extract::DefaultBodyLimit::disable());
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), mock)
    }

    fn write_test_site(dir: &Path, extra_files: &[(&str, Vec<u8>)]) {
        std::fs::write(
            dir.join("index.html"),
            "<html><head></head><body><h1>Hi</h1></body></html>",
        )
        .unwrap();
        std::fs::write(dir.join("index.st"), "").unwrap();
        for (path, bytes) in extra_files {
            std::fs::write(dir.join(path), bytes).unwrap();
        }
    }

    /// Periodic bytes defeat CDC (no cut points -> one big chunk); that
    /// deterministically produces a >= 4 MiB single-chunk blob, which
    /// is exactly what exercises the presigned path in the mock.
    fn periodic_bytes(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    #[tokio::test]
    async fn push_full_flow_small_blob_and_presigned_chunk_then_commit() {
        unsafe { std::env::set_var("SPACETIME_PROBE_ATTEMPTS", "1") };
        let (base, mock) = spawn_orchestrator().await;
        let site = tempfile::tempdir().unwrap();
        write_test_site(site.path(), &[("big.bin", periodic_bytes(5 * 1024 * 1024))]);
        let mut events = Vec::new();
        let resp = push(site.path(), &base, "proj", "token", "cli", |p| {
            events.push(p)
        })
        .await
        .expect("push should succeed");
        assert_eq!(resp.preview_url, "https://proj.preview.example");
        assert_eq!(*mock.commit_count.lock().unwrap(), 1);
        // Every blob the artifact declared is now server-side.
        let artifact = export::build_artifact(site.path()).unwrap();
        let store = mock.blobs.lock().unwrap();
        for file in &artifact.manifest.files {
            match &file.chunks {
                Some(chunks) => chunks.iter().for_each(|c| assert!(store.contains(&c.b3))),
                None => assert!(store.contains(&file.b3)),
            }
        }
        drop(store);
        // Progress sequence: building -> uploading... -> committing -> done.
        let types: Vec<String> = events
            .iter()
            .map(|e| {
                serde_json::to_value(e).unwrap()["type"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(types.first().unwrap(), "building");
        assert_eq!(types.last().unwrap(), "done");
        assert!(types.contains(&"committing".to_string()));
        assert!(types.contains(&"uploading".to_string()));
        // Cache written for the next push.
        assert!(site.path().join(".spacetime/host-manifest.json").exists());
    }

    #[tokio::test]
    async fn push_resume_after_blob_rejection_only_reuploads_missing() {
        let (base, mock) = spawn_orchestrator().await;
        let site = tempfile::tempdir().unwrap();
        write_test_site(site.path(), &[]);
        let artifact = export::build_artifact(site.path()).unwrap();
        // Reject index.html's blob: the deduped missing list is sorted,
        // and af13… (spacetime.css/.js, identical content) sorts before
        // it — so the push stores exactly one blob before failing, and
        // the resume must re-upload ONLY this one.
        let rejected_b3 = artifact
            .manifest
            .files
            .iter()
            .find(|f| f.path == "index.html")
            .unwrap()
            .b3
            .clone();
        *mock.reject_blob_hash.lock().unwrap() = Some(rejected_b3);

        let err = push(site.path(), &base, "proj", "token", "cli", |_| {})
            .await
            .expect_err("rejected blob must fail the push");
        assert!(matches!(err, PushError::Server(_)));
        assert_eq!(
            *mock.commit_count.lock().unwrap(),
            0,
            "no commit on failure"
        );

        // "Fixed" the blob source: already-uploaded blobs are NOT
        // re-uploaded (init diff), the rejected one is retried.
        mock.reject_blob_hash.lock().unwrap().take();
        let uploads_before = *mock.blob_upload_count.lock().unwrap();
        assert_eq!(uploads_before, 1, "one blob landed before the rejection");
        push(site.path(), &base, "proj", "token", "cli", |_| {})
            .await
            .expect("resume push should succeed");
        let uploaded_on_resume = *mock.blob_upload_count.lock().unwrap() - uploads_before;
        assert_eq!(uploaded_on_resume, 1, "only the failed blob is retried");
        assert_eq!(*mock.commit_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn push_idempotent_repush_uploads_nothing() {
        unsafe { std::env::set_var("SPACETIME_PROBE_ATTEMPTS", "1") };
        let (base, mock) = spawn_orchestrator().await;
        let site = tempfile::tempdir().unwrap();
        write_test_site(site.path(), &[]);
        push(site.path(), &base, "proj", "token", "cli", |_| {})
            .await
            .unwrap();
        let uploads_after_first = *mock.blob_upload_count.lock().unwrap();
        push(site.path(), &base, "proj", "token", "cli", |_| {})
            .await
            .expect("re-push should succeed (commit is idempotent server-side)");
        assert_eq!(
            *mock.blob_upload_count.lock().unwrap(),
            uploads_after_first,
            "second push uploads zero blobs"
        );
        assert_eq!(*mock.commit_count.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn push_unsupported_when_init_404s() {
        let app: axum::Router = axum::Router::new(); // no routes -> 404 everything
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let site = tempfile::tempdir().unwrap();
        write_test_site(site.path(), &[]);
        let err = push(
            site.path(),
            &format!("http://{addr}"),
            "proj",
            "token",
            "cli",
            |_| {},
        )
        .await
        .expect_err("404 init must surface as Unsupported for the v1 fallback");
        assert!(matches!(err, PushError::Unsupported));
    }

    #[tokio::test]
    async fn cache_hit_skips_rebuild_when_nothing_changed() {
        let site = tempfile::tempdir().unwrap();
        write_test_site(site.path(), &[]);
        let artifact = export::build_artifact(site.path()).unwrap();
        let fp = export::fingerprint(site.path()).unwrap();
        persist_cache(site.path(), &fp, &artifact.manifest).unwrap();
        let loaded =
            load_cached_manifest(site.path(), &fp.hash).expect("cache hit on matching fingerprint");
        assert_eq!(loaded, artifact.manifest);
        // A source edit invalidates the cache.
        std::fs::write(site.path().join("index.html"), "<html>changed</html>").unwrap();
        let fp2 = export::fingerprint(site.path()).unwrap();
        assert!(load_cached_manifest(site.path(), &fp2.hash).is_none());
    }

    /// The widget state machine is process-global (one in-flight push
    /// per dev server) and the grace period is env-driven, so the two
    /// widget tests must not interleave.
    static WIDGET_TEST_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> =
        std::sync::OnceLock::new();

    #[tokio::test]
    async fn widget_flow_pending_then_complete_commits() {
        unsafe { std::env::set_var("SPACETIME_PROBE_ATTEMPTS", "1") };
        unsafe { std::env::set_var("SPACETIME_WIDGET_GRACE_SECS", "3600") };
        let _serial = WIDGET_TEST_LOCK
            .get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await;
        let (base, mock) = spawn_orchestrator().await;
        let site = tempfile::tempdir().unwrap();
        write_test_site(site.path(), &[("big.bin", periodic_bytes(5 * 1024 * 1024))]);
        let status = widget_start(site.path(), &base, "proj", "token")
            .await
            .expect("widget start");
        let pending = match &status {
            WidgetPushStatus::Uploading { pending, .. } => pending.clone(),
            WidgetPushStatus::Done { .. } => {
                panic!("a >=4MiB chunk must leave presigned work for the widget")
            }
        };
        assert!(!pending.is_empty());
        assert_eq!(widget_pending().await.unwrap().len(), pending.len());
        let first = pending.first().expect("pending upload");
        let widget_bytes = widget_blob_bytes(&first.b3)
            .await
            .expect("pending blob bytes are available to the widget");
        let artifact = export::build_artifact(site.path()).unwrap();
        let blobs = artifact_blobs(&artifact.files, &artifact.manifest);
        assert_eq!(widget_bytes, blobs.get(&first.b3).unwrap().1);
        assert!(widget_blob_bytes("not-a-pending-b3").await.is_none());
        // A second start while in flight is rejected.
        let conflict = widget_start(site.path(), &base, "proj", "token").await;
        assert!(matches!(conflict, Err(PushError::AlreadyRunning)));
        // The widget performs the presigned PUTs itself.
        let client = http_client();
        for info in &pending {
            let bytes = &blobs.get(&info.b3).unwrap().1;
            let resp = client
                .put(&info.url)
                .body(bytes.clone())
                .send()
                .await
                .unwrap();
            assert!(resp.status().is_success());
        }
        let done: Vec<String> = pending.iter().map(|p| p.b3.clone()).collect();
        let final_status = widget_complete(&done).await.expect("complete");
        match final_status {
            WidgetPushStatus::Done { response } => {
                assert_eq!(response.preview_url, "https://proj.preview.example")
            }
            WidgetPushStatus::Uploading { .. } => panic!("all uploads done -> commit"),
        }
        assert_eq!(*mock.commit_count.lock().unwrap(), 1);
        assert!(matches!(widget_pending().await, Err(PushError::NotRunning)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn widget_grace_timer_finishes_without_the_widget() {
        let _serial = WIDGET_TEST_LOCK
            .get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await;
        unsafe {
            std::env::set_var("SPACETIME_WIDGET_GRACE_SECS", "0");
        }
        let (base, mock) = spawn_orchestrator().await;
        let site = tempfile::tempdir().unwrap();
        write_test_site(site.path(), &[("big.bin", periodic_bytes(5 * 1024 * 1024))]);
        let status = widget_start(site.path(), &base, "proj", "token")
            .await
            .expect("widget start");
        assert!(matches!(status, WidgetPushStatus::Uploading { .. }));
        // First push: every source file is "changed" (no prior fingerprint).
        if let WidgetPushStatus::Uploading { changed, .. } = &status {
            assert!(changed.contains(&"index.html".to_string()));
        }
        // The widget never completes anything; the grace task must
        // upload the remainder itself and commit.
        let mut committed = false;
        for _ in 0..50 {
            if *mock.commit_count.lock().unwrap() == 1 {
                committed = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        unsafe {
            std::env::remove_var("SPACETIME_WIDGET_GRACE_SECS");
        }
        assert!(committed, "grace fallback committed without the widget");
        assert!(matches!(widget_pending().await, Err(PushError::NotRunning)));
    }
}
