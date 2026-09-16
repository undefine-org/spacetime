//! WARNING: this test pushes REAL content to the project in
//! SPACETIME_PROJECT_ID — it MOVES that project's preview pointer
//! (and audit history) to the test deploy. Point it at a throwaway
//! project unless you deliberately want the dogfood site refreshed.
//! DEPLOYED-rung live proof (W5-1 / PLAN-005): a REAL push of the real
//! ora-ventures site through Deploy Protocol v2 against the PRODUCTION
//! orchestrator, then HTTP verification against the live edge:
//! preview 200, /spacetime.js 200, /spacetime.css 200, /index.st 404
//! (the source-leak class stays dead), x-spacetime-deploy == the pushed
//! artifact hash, and the immutable URL serves the same bytes.
//!
//! `#[ignore]`d — production side effects (a real deploy happens).
//! Run via:
//!   cargo test --test live_deploy_rung -- --ignored --nocapture
//!
//! Required env / files:
//!   SPACETIME_HOST_SERVER_URL (defaults to https://spacetime.undefine.org)
//!   SPACETIME_LIVE_SITE_DIR   (defaults to the ora-ventures site path)
//!   SPACETIME_LIVE_PROJECT_ID (the site's host project id)
//!   ~/.local/share/spacetime-host/credentials (whole file = bearer token)

use spacetime::host_push_v2;

fn api_base() -> String {
    std::env::var("SPACETIME_HOST_SERVER_URL")
        .unwrap_or_else(|_| "https://spacetime.undefine.org".to_string())
}

fn site_dir() -> std::path::PathBuf {
    // NO default — require an explicit SPACETIME_LIVE_SITE_DIR.
    // A default here is dangerous: this test pushes to a REAL project's
    // preview pointer on PRODUCTION infra. An outdated default silently
    // deployed a stale stub scaffold ("Ora Ventures Articles") as the
    // dogfood project's preview AND got promoted to production (2026-07-21
    // incident). Forcing the operator to name the dir makes them verify it.
    std::env::var("SPACETIME_LIVE_SITE_DIR")
        .unwrap_or_else(|_| {
            panic!(
                "SPACETIME_LIVE_SITE_DIR must be set explicitly — this test \
                 pushes real content to production infra. Point it at the real \
                 site source (e.g. verse/projects/ora-ventures.com), NOT a \
                 stub scaffold."
            )
        })
        .into()
}

fn project_id() -> String {
    std::env::var("SPACETIME_LIVE_PROJECT_ID")
        .unwrap_or_else(|_| "od8ibBpzFn6OxqwQwwSa7".to_string())
}

fn token() -> String {
    let path = std::env::var("SPACETIME_HOST_CREDENTIALS").unwrap_or_else(|_| {
        format!(
            "{}/.local/share/spacetime-host/credentials",
            std::env::var("HOME").unwrap()
        )
    });
    std::fs::read_to_string(path)
        .expect("credentials file")
        .trim()
        .to_string()
}

#[tokio::test]
#[ignore = "production deploy — run explicitly"]
async fn live_deployed_rung_push_and_edge_verification() {
    let base = api_base();
    let site = site_dir();
    let project = project_id();
    let token = token();
    let http = reqwest::Client::new();

    // ── Push via protocol v2 (build → init → blobs → commit) ─────────
    let mut events = Vec::new();
    let commit = host_push_v2::push(&site, &base, &project, &token, "cli", |p| events.push(p))
        .await
        .expect("v2 push against production must succeed");
    // immutable URL: https://{project}-{hash8}.preview.{base}
    let hash = commit
        .immutable_url
        .rsplit('-')
        .next()
        .and_then(|tail| tail.split('.').next())
        .expect("immutable URL embeds hash8")
        .to_string();
    println!("push committed; immutable_url={}", commit.immutable_url);

    // ── Edge verification (KV propagation: poll briefly) ─────────────
    let preview_url = commit.preview_url.clone();
    let mut last_status = 0u16;
    let mut headers = reqwest::header::HeaderMap::new();
    for attempt in 0..20 {
        let resp = http.get(&preview_url).send().await.unwrap();
        last_status = resp.status().as_u16();
        headers = resp.headers().clone();
        if last_status == 200
            && headers
                .get("x-spacetime-deploy")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|h| h != "unknown")
        {
            break;
        }
        if attempt == 19 {
            panic!("preview never came up: last status {last_status}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
    let deploy_header = headers
        .get("x-spacetime-deploy")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        deploy_header.starts_with(&hash),
        "x-spacetime-deploy ({deploy_header}) must match the pushed hash ({hash}…)"
    );

    // Assets serve; source leak stays dead.
    for (path, want) in [
        ("/spacetime.js", 200u16),
        ("/spacetime.css", 200u16),
        ("/index.st", 404u16),
    ] {
        let url = format!("{}{}", preview_url.trim_end_matches('/'), path);
        let status = http.get(&url).send().await.unwrap().status().as_u16();
        assert_eq!(status, want, "{url}");
    }

    // Immutable URL serves the same deploy (independent of the pointer).
    // KV reads are cached at the edge (~60s): poll through propagation.
    let mut immutable_status = 0u16;
    let mut immutable_root = None;
    for attempt in 0..30 {
        let resp = http.get(&commit.immutable_url).send().await.unwrap();
        immutable_status = resp.status().as_u16();
        if immutable_status == 200 {
            immutable_root = Some(resp);
            break;
        }
        if attempt == 29 {
            panic!("immutable URL never served: last status {immutable_status}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
    let immutable_root = immutable_root.unwrap();
    let immutable_deploy = immutable_root
        .headers()
        .get("x-spacetime-deploy")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert_eq!(immutable_deploy, deploy_header);

    // Progress narration covered the pinned sequence.
    let types: Vec<String> = events
        .iter()
        .map(|e| {
            serde_json::to_value(e).unwrap()["type"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(types.first().map(String::as_str), Some("building"));
    assert_eq!(types.last().map(String::as_str), Some("done"));

    println!("DEPLOYED rung green: {preview_url} (deploy {deploy_header})");
}
