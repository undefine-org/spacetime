//! PLAN-155 instant edits: a page's hot binds and its fingerprint.
//!
//! A studio edit is taken by the running page IN PLACE when only hot-keyed
//! arguments moved (a clip window, a keyframe table, a synth program): the
//! compiler hands out each hot call's arguments under a stable key, and a
//! fingerprint of everything else. These gates pin the contract on a real film.

use spacetime::compiler::Compiler;
use serde_json::Value;

fn hot(source: &str) -> (Vec<Value>, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let film = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("demos/film-robot-day");
    for e in std::fs::read_dir(&film).unwrap().flatten() {
        let to = dir.path().join(e.file_name());
        if e.path().is_dir() {
            std::os::unix::fs::symlink(e.path(), &to).unwrap();
        }
    }
    let file = dir.path().join("index.st");
    std::fs::write(&file, source).unwrap();
    let mut binds = Compiler::from_file(&file, dir.path())
        .expect("parse")
        .with_site_dir(Some(dir.path().to_path_buf()))
        .hot_only()
        .compile()
        .hot_binds;
    let at = binds.iter().position(|b| b.get("fingerprint").is_some()).expect("a fingerprint row");
    let fp = binds.remove(at)["fingerprint"].as_str().unwrap().to_string();
    (binds, fp)
}

fn robot_day() -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("demos/film-robot-day/index.st")).unwrap()
}

fn find<'a>(binds: &'a [Value], key: &str) -> &'a Value {
    binds.iter().find(|b| b["key"] == key).unwrap_or_else(|| {
        panic!("no hot bind {key}; have {:?}", binds.iter().map(|b| b["key"].clone()).collect::<Vec<_>>())
    })
}

#[test]
fn a_film_s_windows_keyframes_and_sound_are_hot() {
    let (binds, _) = hot(&robot_day());
    let prims: std::collections::BTreeSet<&str> = binds.iter().filter_map(|b| b["primitive"].as_str()).collect();
    assert!(prims.contains("score-window"), "clip windows are hot: {prims:?}");
    assert!(prims.contains("apply-animations"), "clip bodies are hot: {prims:?}");
    assert!(prims.contains("score-synth"), "the score's sound is hot: {prims:?}");
    let card = find(&binds, "apply-animations@.card#0");
    assert!(card["args"]["animations"].as_str().unwrap().contains("opacity"));
}

#[test]
fn retiming_a_clip_moves_only_its_window() {
    let src = robot_day();
    let (before, fp0) = hot(&src);
    let (after, fp1) = hot(&src.replace("&card at 0.4s for 5.2s;", "&card at 1s for 5.2s;"));
    assert_eq!(fp0, fp1, "a retimed clip changes nothing but hot arguments");
    let changed: Vec<&str> = before
        .iter()
        .zip(after.iter())
        .filter(|(a, b)| a["args"] != b["args"])
        .map(|(a, _)| a["key"].as_str().unwrap())
        .collect();
    assert_eq!(before.len(), after.len());
    assert!(changed.iter().any(|k| k.starts_with("score-window@")), "its window moved: {changed:?}");
    assert!(changed.iter().all(|k| k.starts_with("score-window@")), "nothing else did: {changed:?}");
}

#[test]
fn a_keyframe_and_a_drum_cell_are_hot_too() {
    let src = robot_day();
    let (_, fp0) = hot(&src);
    let (_, fp1) = hot(&src.replace("opacity: 0 -> 1 at 18%", "opacity: 0 -> 1 at 30%"));
    assert_eq!(fp0, fp1, "a keyframe edit is hot");
    let (b2, fp2) = hot(&src.replace("--kick:  x . . . .", "--kick:  x x . . ."));
    assert_eq!(fp0, fp2, "a drum cell is hot");
    let synth = b2.iter().find(|b| b["primitive"] == "score-synth").unwrap();
    assert!(synth["args"]["program"].as_str().unwrap().contains("x x . . ."));
}

#[test]
fn anything_else_changes_the_fingerprint() {
    let src = robot_day();
    let (_, fp0) = hot(&src);
    let (_, markup) = hot(&src.replace("Every evening it crossed the plaza.", "Every morning it crossed the plaza."));
    assert_ne!(fp0, markup, "markup is not hot: the page must reload");
    let (_, style) = hot(&src.replace("bottom: 7vh;", "bottom: 9vh;"));
    assert_ne!(fp0, style, "a style declaration is not hot");
    let (_, length) = hot(&src.replace("@score &.time(24s)", "@score &.time(26s)"));
    assert_ne!(fp0, length, "the score's length is its driver's: not hot");
}

#[test]
fn hot_arguments_are_the_values_the_body_is_given() {
    let (binds, _) = hot(&robot_day());
    for b in &binds {
        let args = b["args"].as_object().unwrap();
        assert!(!args.contains_key("self") && !args.contains_key("el"), "an element is not data: {}", b["key"]);
    }
    // a `string` param arrives as the string the body sees, not raw source text
    let ground = binds
        .iter()
        .find(|b| b["primitive"] == "three-object" && b["args"]["name"].as_str().is_some_and(|n| n.contains("ground")))
        .expect("the &ground object");
    let name = ground["args"]["name"].as_str().unwrap();
    assert!(name.starts_with('"') && name.ends_with('"'), "name must be a JS string literal, got {name}");
}
