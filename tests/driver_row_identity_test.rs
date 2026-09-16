//! STRUCTURAL GUARD (BUG-253 / PLAN-127): a driver row's identity MUST reach
//! its emitted logic.
//!
//! Five driver rows — `.hover` `.click` `.focus` `.submit` `.key` — all declare
//! `primitive: event-driver`. They compiled to BYTE-IDENTICAL logic: every one
//! emitted `const trig = "hover"`, the primitive's parameter default. So
//! `@on &.click:` listened for *mouseenter*, and `@on &.submit:` emitted no
//! submit listener at all. All five built clean, with zero diagnostics.
//!
//! The cause was not a missing argument. `driver_param_args` looked its
//! registry row up by PRIMITIVE NAME (`find(entry_field("primitive") ==
//! primitive)`), and `.find` returns the FIRST match — so all five rows read
//! *hover's* row. Per-row data was unreachable by construction.
//!
//! WHY THIS GATE AND NOT PER-ROW ASSERTIONS: a test asserting
//! `trig == "submit"` for the submit row would have caught this instance, and
//! nothing else — it must be written again, correctly, for every future row,
//! and its absence is invisible. This gate states the INVARIANT instead: two
//! driver rows that compile to the same logic are indistinguishable to the
//! author, and that is a bug regardless of which rows they are or what the
//! mechanism was. It needs no per-row expectations, so it cannot rot as rows
//! are added, and it generalizes to any registry where N entries share one
//! implementation.
//!
//! A deliberate alias (`.mouse` / `.pointer` genuinely ARE the same driver)
//! is exempted by DECLARING `alias_of:` in the registry row — data, not an
//! allowlist in this file. That way declaring an alias is a deliberate act,
//! rather than an accident that merely looks like one.

use std::collections::HashMap;
use std::process::Command;

/// Build a one-element page driven by `member` and return the emitted JS.
///
/// Returns `None` when the row cannot drive an element page at all (signal
/// facets like `.done`, and `planned:` rows, which are compile errors by
/// design — those are covered by the W3.6 facet gates, not here).
fn build_driver_page(member: &str) -> Option<String> {
    let dir = std::env::temp_dir().join(format!(
        "drv-identity-{}-{}-{}",
        std::process::id(),
        member,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    let src = format!(
        r#"@version 2026-06-09;
.t {{ width: 100px; height: 100px; background: red; }}
<div class="t"></div>
@form motion --grow {{ from {{ opacity: 0; }} to {{ opacity: 1; }} }}
.t {{ @on &.{member}: --grow; }}
"#
    );
    std::fs::write(&file, src).expect("write fixture");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&file)
        .output()
        .expect("run spacetime build");
    let js = std::fs::read_to_string(dir.join("spacetime.js")).ok();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let _ = std::fs::remove_dir_all(&dir);
    if !out.status.success() || stderr.contains("E0948") {
        return None;
    }
    js
}

/// Strip identity that is COSMETIC — a name derived from the row, which proves
/// only that the row's *label* travelled, not its behavior.
///
/// This normalization is the sharp edge of the gate: `__drive_click_1` vs
/// `__drive_submit_1` was the ONLY difference between the five broken rows.
/// Leaving it in would let a label alone satisfy the invariant, which is
/// exactly the failure being guarded against.
fn normalize(js: &str) -> String {
    let mut out = String::with_capacity(js.len());
    let mut rest = js;
    // `__drive_<member>_<n>` → `__drive_X`; `_st_init_<hash>` → `_st_init_X`.
    loop {
        let drive = rest.find("__drive_");
        let init = rest.find("_st_init_");
        let (at, tag, len) = match (drive, init) {
            (Some(d), Some(i)) if d < i => (d, "__drive_X", "__drive_".len()),
            (Some(_), Some(i)) => (i, "_st_init_X", "_st_init_".len()),
            (Some(d), None) => (d, "__drive_X", "__drive_".len()),
            (None, Some(i)) => (i, "_st_init_X", "_st_init_".len()),
            (None, None) => break,
        };
        out.push_str(&rest[..at]);
        out.push_str(tag);
        let after = &rest[at + len..];
        let end = after
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(after.len());
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Rows that drive an ELEMENT (`@on &.<member>`). Signal facets (`.change`,
/// `.done`, `.prev`, `.text-change`) drive a signal subject and are gated by
/// the W3.6 facet suite.
const ELEMENT_DRIVER_ROWS: &[&str] = &[
    "visible", "hover", "click", "focus", "submit", "key", "scroll", "mouse", "pointer", "time",
    "loop", "load",
];

/// Deliberate aliases, read from the REGISTRY (`alias_of: <row>`), never from
/// an allowlist in this file.
///
/// The distinction the gate is drawing is intent: two rows compiling to the
/// same logic is a bug *unless someone said it should be*. Keeping the
/// exemption in the registry means declaring an alias is a deliberate act in
/// the same file the rows live in — and means this test needs no edit when a
/// future alias is added, so it cannot drift out of date with the rows it
/// guards.
fn declared_aliases() -> Vec<(String, String)> {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("stdlib/macros/drivers.st"),
    )
    .expect("read the driver registry");
    let mut out = Vec::new();
    let mut row: Option<String> = None;
    for line in src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("%registers driver(") {
            row = rest.split(')').next().map(str::to_string);
        } else if let Some(rest) = t.strip_prefix("alias_of:")
            && let Some(r) = &row
        {
            out.push((r.clone(), rest.trim().to_string()));
        }
    }
    out
}

fn declared_alias(a: &str, b: &str) -> bool {
    declared_aliases()
        .iter()
        .any(|(row, target)| (row == a && target == b) || (row == b && target == a))
}

#[test]
fn every_driver_row_compiles_to_distinguishable_logic() {
    let mut by_hash: HashMap<String, Vec<&str>> = HashMap::new();
    for member in ELEMENT_DRIVER_ROWS {
        let Some(js) = build_driver_page(member) else {
            continue;
        };
        by_hash.entry(normalize(&js)).or_default().push(member);
    }
    assert!(
        by_hash.values().map(Vec::len).sum::<usize>() >= ELEMENT_DRIVER_ROWS.len(),
        "some driver rows failed to build — the gate cannot see what it cannot compile"
    );

    let mut collisions: Vec<String> = Vec::new();
    for rows in by_hash.values() {
        if rows.len() < 2 {
            continue;
        }
        // A group of size 2 that is a declared alias is fine; anything larger,
        // or any undeclared pair, is a collision.
        if rows.len() == 2 && declared_alias(rows[0], rows[1]) {
            continue;
        }
        let mut names: Vec<&str> = rows.clone();
        names.sort_unstable();
        collisions.push(format!("  .{}", names.join("  .")));
    }

    assert!(
        collisions.is_empty(),
        "these driver rows compile to IDENTICAL logic — their registry identity \
         never reaches the emitted code, so the author's choice between them has \
         no effect:\n{}\n\n\
         A driver row's data (its trigger, its policy) must reach its primitive. \
         If two rows are genuinely the same driver, declare the alias in the \
         registry; if they are not, the row's distinguishing data is being \
         dropped between `%registers driver(<name>)` and the emitted bundle \
         (BUG-253: `driver_param_args` looked the row up by PRIMITIVE, so every \
         row sharing a primitive read the FIRST such row's data).",
        collisions.join("\n")
    );
}

/// The narrow, human-readable form of the same fact: an event row's trigger is
/// its OWN name, not the primitive's default.
///
/// Kept alongside the invariant gate because it names the mechanism — when this
/// fails, the message points straight at the trigger, where the invariant gate
/// can only say "these two are identical".
#[test]
fn an_event_driver_listens_for_its_own_event() {
    // The expected value is the REAL DOM EVENT the row listens for — `.hover`
    // is spelled `mouseenter` because that is the event, and the primitive no
    // longer knows any driver's name (its `if (trig === 'hover')` branch was
    // replaced by `untrigger`/`toggle` row data in the same wave).
    for (member, expected) in [
        ("click", "click"),
        ("focus", "focus"),
        ("submit", "submit"),
        ("key", "keydown"),
        ("hover", "mouseenter"),
    ] {
        let js = build_driver_page(member).unwrap_or_else(|| panic!("`.{member}` failed to build"));
        let needle = format!("const trig = \"{expected}\"");
        assert!(
            js.contains(&needle),
            "`@on &.{member}:` must emit `{needle}` — the row's own trigger. \
             Emitted instead: {:?}. A row that emits another row's trigger \
             listens for the wrong event entirely (BUG-253: `.click` listened \
             for mouseenter).",
            js.split("const trig = ")
                .nth(1)
                .map(|s| s.split(';').next().unwrap_or("").to_string())
                .unwrap_or_else(|| "<no trigger emitted at all>".to_string())
        );
    }
}

/// The same invariant stated at the REGISTRY level, where it is cheap and
/// instant.
///
/// The compile-and-compare gate above is the ground truth, but it costs a
/// build per row. This one reads the registry directly: if two rows share a
/// `primitive` and neither declares `alias_of`, at least one of them must
/// carry `primitive_args` — otherwise there is, by inspection, nothing that
/// could ever tell them apart.
///
/// It fires the moment a row is WRITTEN rather than when its output is
/// diffed, which is the difference between "your new driver is a duplicate"
/// and a puzzling behavior report weeks later.
#[test]
fn rows_sharing_a_primitive_carry_distinguishing_data() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("stdlib/macros/drivers.st"),
    )
    .expect("read the driver registry");

    #[derive(Default)]
    struct Row {
        primitive: String,
        has_args: bool,
        alias: bool,
    }
    let mut rows: Vec<(String, Row)> = Vec::new();
    for line in src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("%registers driver(") {
            let name = rest.split(')').next().unwrap_or_default().to_string();
            rows.push((name, Row::default()));
        } else if let Some((_, row)) = rows.last_mut() {
            if let Some(p) = t.strip_prefix("primitive:") {
                row.primitive = p.trim().to_string();
            } else if t.starts_with("primitive_args:") {
                row.has_args = true;
            } else if t.starts_with("alias_of:") {
                row.alias = true;
            }
        }
    }
    assert!(
        rows.len() > 10,
        "registry did not parse — got {} rows",
        rows.len()
    );

    let mut by_primitive: HashMap<&str, Vec<&str>> = HashMap::new();
    for (name, row) in &rows {
        // `planned:` rows have no primitive to share yet.
        if row.primitive.is_empty() || row.primitive.starts_with("planned:") || row.alias {
            continue;
        }
        by_primitive
            .entry(row.primitive.as_str())
            .or_default()
            .push(name.as_str());
    }

    let lookup = |n: &str| rows.iter().find(|(name, _)| name == n).map(|(_, r)| r);
    let mut bad: Vec<String> = Vec::new();
    for (primitive, names) in &by_primitive {
        if names.len() < 2 {
            continue;
        }
        let undistinguished: Vec<&str> = names
            .iter()
            .copied()
            .filter(|n| !lookup(n).map(|r| r.has_args).unwrap_or(false))
            .collect();
        // One row may legitimately ride the primitive's own defaults; two or
        // more with nothing to tell them apart cannot both be right.
        if undistinguished.len() > 1 {
            bad.push(format!(
                "  `{primitive}` is shared by .{} with no `primitive_args` on any of them",
                undistinguished.join(", .")
            ));
        }
    }

    assert!(
        bad.is_empty(),
        "these driver rows share a primitive with nothing to distinguish them:\n{}\n\n\
         Give each row its own `primitive_args: \"name:value\"`, or declare the \
         duplicate deliberate with `alias_of: <row>`. A row that carries neither \
         silently inherits whatever the primitive defaults to (BUG-253: five \
         event rows all emitted `trigger: \"hover\"`, so `@on &.click:` listened \
         for mouseenter).",
        bad.join("\n")
    );
}
