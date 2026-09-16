//! W4/t5 — `&.clip`, the video pole of the thesis.
//!
//! # What a clip driver IS
//!
//! `&.clip` declares `domain: inherited` — a clip does not compute a window,
//! it RECEIVES one. That is R3 seen from the inside: the parent score already
//! derives a progress signal per clip (`score-window`), so a clip driver is
//! the thing that READS that signal and republishes it as its own progress,
//! letting a nested score treat it as a driver like any other.
//!
//! ```st
//! .film  { @score &.time(12s) { &shot at 0s for 6s; } }   // parent assigns
//! .shot  { @score &.clip      { &title for 50%; } }        // clip receives
//! ```
//!
//! The inner score never mentions time. It is written once and runs at
//! whatever scale its parent hands it — which is precisely the claim SIP-001b
//! §5 says must be falsifiable: a website and a video differ only in which
//! signal drives the timeline.
//!
//! # Why this file exists separately from score_window_map_test
//!
//! That file asserts the parent's arithmetic. This one asserts the JOIN: that
//! a clip's progress is wired to the window its parent computed, rather than
//! to a second clock. A clip that starts its own timer would satisfy every
//! assertion about windows and still break the thesis, because live and
//! rendered would diverge the moment the two clocks drifted.

use spacetime::compiler::Compiler;

fn compile_js(source: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, source).expect("write");
    Compiler::from_file(&entry, dir.path())
        .unwrap_or_else(|e| panic!("compile failed: {e}"))
        .compile()
        .js
}

fn check_ok(source: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, source).expect("write");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(&entry)
        .output()
        .expect("run check");
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

const PAGE: &str = r#"@version 2026-06-09;

<div class="film">
  <div class="shot">
    <div class="title">Title</div>
  </div>
</div>
"#;

/// `&.clip` must no longer be `planned:`.
///
/// The honest-dead pattern (a `planned:` primitive raising E0948) is the right
/// state for a word that is reserved but unimplemented — it is loud, and it is
/// why `.clip` was never silently wrong the way the five event drivers were.
/// Promoting it is exactly this: the word starts working.
#[test]
fn a_clip_driven_score_compiles() {
    let source = format!(
        r#"{PAGE}
.shot {{
  @score &.clip {{
    &title for 50%;
  }}
}}
"#
    );

    let (ok, output) = check_ok(&source);
    assert!(
        ok,
        "W4/t5: `&.clip` must drive a score. While it stays `planned:` this is \
         E0948, and the thesis test (one body, two projections) cannot run at \
         all.\nOutput:\n{output}"
    );
}

/// The JOIN, and the point of the whole file: a clip's progress must be READ
/// from its parent, not generated.
///
/// Asserted on emitted JS because the difference between "reads a signal" and
/// "starts a timer" is invisible to a green check and decides whether live and
/// rendered agree.
#[test]
fn a_clip_reads_its_parents_window_rather_than_starting_a_clock() {
    let source = format!(
        r#"{PAGE}
.film {{
  @score &.time(12s) as &reel {{
    &shot at 0s for 6s;
  }}
}}
.shot {{
  @score &.clip {{
    &title for 50%;
  }}
}}
"#
    );

    let js = compile_js(&source);

    assert!(
        js.contains("_swMap"),
        "W4/R3: the parent score must still emit its window map.\n\
         (bundle length {})",
        js.len()
    );

    // THE JOIN. The parent computes a window for `&shot`; `.shot`'s own
    // `@score &.clip` must READ that exact signal. Both halves are asserted
    // because either alone is satisfiable by an accident:
    //   - the parent publishing `__clip_shot` with nobody reading it is a
    //     score whose inner body never runs;
    //   - the inner score watching `__clip_shot` with nobody publishing it is
    //     a clip frozen at 0 forever — which is what this code did before the
    //     join existed, silently and with a green check.
    assert!(
        js.contains("_swName = \"__clip_shot\""),
        "W4/t5: the PARENT must publish a window named for its subject \
         (`__clip_shot`), which is the address the nested clip resolves \
         against. Without a deterministic name the two passes cannot join \
         without a side channel."
    );
    assert!(
        js.contains("ST.watchScoped(el, \"__clip_shot\""),
        "W4/t5: the CLIP-driven score must watch the window its parent \
         assigned. `domain: inherited` means exactly this — the clip receives \
         its span rather than deriving one, so the inner body never mentions \
         time and runs at whatever scale the parent hands it.\n\
         NB W5b/BUG-266: this is watchScoped, not ST.watch — the parent score \
         publishes `__clip_shot` on an ANCESTOR (the `.film` node), and a bare \
         ST.watch on our own node resolves the INITIAL value but never fires \
         reactively (the nested clip sits at 0 forever). watchScoped mirrors \
         ST.resolve's parent-chain walk (FUP-094 doctrine)."
    );

    // And the inner body's own window must be derived from that, not from a
    // second score name nothing publishes.
    assert!(
        js.contains("_swName = \"__clip_title\""),
        "W4/R3: the clip's own child window must exist — a nested score is just \
         a window map over a window map, which is what makes nesting free."
    );
    assert!(
        !js.contains("_clipRaf"),
        "W4/t5: a clip must NOT drive itself off rAF. Its progress is the \
         window its parent assigned — a clip with its own clock makes live and \
         rendered diverge the moment the two drift, which is the divergence \
         SIP-001b §8.6 warns about and R3's one-evaluator rule exists to \
         prevent."
    );
}

/// The thesis, at compile time: ONE score body, both projections.
///
/// The fragment names no driver — the driver is supplied at the `@score` head
/// — which is why R4 rejected sub-typing: fragments are already
/// projection-agnostic, and a type parameter would be ceremony to recover
/// polymorphism we have by omission.
#[test]
fn the_same_fragment_runs_under_both_poles() {
    let source = format!(
        r#"{PAGE}
@form score --story {{
  &title for 50%;
}}

.film {{ @score &.time(12s) {{ --story; }} }}
.shot {{ @score &.clip      {{ --story; }} }}
"#
    );

    let (ok, output) = check_ok(&source);
    assert!(
        ok,
        "W4: the SAME `@form score` body must compile under &.time (the website \
         pole) and &.clip (the video pole). If the two poles need different \
         bodies, the family should be reopened rather than patched \
         (SIP-001b §5).\nOutput:\n{output}"
    );
}

/// EVERY watched score signal must have a publisher.
///
/// This is the invariant behind the specific assertions above, and it is the
/// one that generalizes. Before the driver bind existed, a score emitted its
/// windows but never its DRIVER: the windows watched `__score_1`, nothing ever
/// published `__score_1`, and every clip sat at 0 forever. `check` was green,
/// the bundle was full of plausible code, and nothing moved.
///
/// A per-case assertion would have caught that one instance. This catches the
/// SHAPE — any future score construct that reads a signal it forgot to produce
/// fails here without anyone writing a new test.
#[test]

fn every_watched_score_signal_has_a_publisher() {
    #[derive(Default)]
    struct DriverRow {
        name: String,
        primitive: String,
        domain: String,
    }

    let registry = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("stdlib/macros/drivers.st"),
    )
    .expect("read driver registry");
    let mut rows: Vec<DriverRow> = Vec::new();
    for line in registry.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("%registers driver(") {
            rows.push(DriverRow {
                name: rest.split(')').next().unwrap_or_default().to_string(),
                ..Default::default()
            });
        } else if let Some(row) = rows.last_mut() {
            if let Some(value) = line.strip_prefix("primitive:") {
                row.primitive = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("domain:") {
                row.domain = value.trim().to_string();
            }
        }
    }
    let rows: Vec<DriverRow> = rows
        .into_iter()
        .filter(|row| !row.domain.is_empty())
        .collect();
    assert!(
        !rows.is_empty(),
        "the driver registry has no domain-bearing rows — this gate is vacuous"
    );

    for row in rows {
        assert!(
            !row.primitive.is_empty(),
            "driver .{} declares domain:{} without primitive data",
            row.name,
            row.domain
        );
        let source = match row.domain.as_str() {
            "declared" => format!(
                r#"{PAGE}
.shot {{ @score &.{}(60s) {{ &title at 2s for 3s; }} }}
"#,
                row.name
            ),
            "normalized" => format!(
                r#"{PAGE}
.shot {{ @score &.{}(cover) {{ &title for 30%; }} }}
"#,
                row.name
            ),
            "derived" => format!(
                r#"{PAGE}
.shot {{ @score &.{} {{ &title; }} }}
"#,
                row.name
            ),
            "inherited" => format!(
                r#"{PAGE}
.film {{ @score &.time(12s) {{ &shot at 0s for 6s; }} }}
.shot {{ @score &.{} {{ &title for 50%; }} }}
"#,
                row.name
            ),
            domain => panic!("driver .{} has unknown domain:{domain}", row.name),
        };

        if row.primitive.starts_with("planned:") {
            let (ok, output) = check_ok(&source);
            assert!(
                !ok && output.contains("E0948"),
                "driver .{} is primitive:{} and must be rejected as E0948, not \\
                 silently compiled or rejected for an incidental reason.\\nOutput:\\n{output}",
                row.name,
                row.primitive,
            );
            continue;
        }

        let js = compile_js(&source);
        let watched: std::collections::BTreeSet<String> = js
            .match_indices("ST.watchScoped(el, \"")
            .filter_map(|(i, _)| {
                let rest = &js[i + "ST.watchScoped(el, \"".len()..];
                rest.find('"').map(|end| rest[..end].to_string())
            })
            .collect();
        let mut published: std::collections::BTreeSet<String> = js
            .match_indices("_swName = \"")
            .filter_map(|(i, _)| {
                let rest = &js[i + "_swName = \"".len()..];
                rest.find('"').map(|end| rest[..end].to_string())
            })
            .collect();
        published.extend(js.match_indices("timelineName = \"").filter_map(|(i, _)| {
            let rest = &js[i + "timelineName = \"".len()..];
            rest.find('"').map(|end| rest[..end].to_string())
        }));
        published.extend(js.match_indices("ST.set(el, \"").filter_map(|(i, _)| {
            let rest = &js[i + "ST.set(el, \"".len()..];
            rest.find('"').map(|end| rest[..end].to_string())
        }));
        assert!(
            !watched.is_empty(),
            "driver .{} primitive:{} domain:{} produced no watched score signals; \\
             the probe has drifted and is checking nothing.",
            row.name,
            row.primitive,
            row.domain
        );
        let orphans: Vec<&String> = watched.difference(&published).collect();
        assert!(
            orphans.is_empty(),
            "driver .{} primitive:{} domain:{} watches signals without publishers: \\
             {orphans:?}.\\nwatched={watched:?}\\npublished={published:?}",
            row.name,
            row.primitive,
            row.domain
        );
    }
}
