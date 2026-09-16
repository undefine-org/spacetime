//! BUG-263 item 4 — behavioral gate: `@pose-track` must bind its PRECEDING
//! `@object` by SOURCE-ORDER interleaving (the compiler emits sibling
//! primitive inits in source order), so `objectIndex` is unnecessary.
//!
//! Mechanism-level proof (deterministic, no CDP/BUG-150 rAF timing): in the
//! emitted page JS, each `three-object` init block (`canvas._stObject = mesh`)
//! and each `three-pose-track` init block (`const target = … canvas._stObject`)
//! must INTERLEAVE in author order — obj0, track0, obj1, track1 — so that when
//! a track resolves `canvas._stObject` it sees ITS OWN preceding object, not
//! the last object of the whole scene.
//!
//! RED today (pre-fix): the compiler groups ALL `three-object` inits first,
//! then ALL `three-pose-track` inits, so every track resolves the LAST-created
//! object regardless of which @object it followed.

use spacetime::compiler::Compiler;
use std::path::Path;

#[test]
fn gate_pt_interleaving() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    let source = r#"@import "stdlib/3d";

.canvas.m {
  @stage(fov: 36, camZ: 8) {
    @object(shape: "box", size: 0.5)
    @pose-track(ease: 1, stops: [ { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() } ])
    @object(shape: "sphere", size: 0.5)
    @pose-track(ease: 1, stops: [ { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() } ])
  }
}
"#;
    std::fs::write(&entry, source).expect("write");
    let js = Compiler::from_file(&entry, root).unwrap().compile().js;

    // Object-creation line: the @object primitive registering its mesh.
    let create: Vec<usize> = js
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("canvas._stObject = mesh;"))
        .map(|(i, _)| i)
        .collect();
    // Track-binding line: the @pose-track primitive resolving its target.
    let bind: Vec<usize> = js
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("const target =") && l.contains("canvas._stObject"))
        .map(|(i, _)| i)
        .collect();

    assert_eq!(create.len(), 2, "expected 2 @object inits, got {create:?}");
    assert_eq!(bind.len(), 2, "expected 2 @pose-track inits, got {bind:?}");

    // Each track must resolve AFTER its own object but BEFORE the next object:
    // create[0] < bind[0] < create[1] < bind[1].
    assert!(
        create[0] < bind[0] && bind[0] < create[1] && create[1] < bind[1],
        "pose-tracks not interleaved with their objects in source order: \
         create@{create:?} bind@{bind:?} — every track binds the LAST object"
    );
}

/// The retired `objectIndex:` parameter is a DEAD NO-OP: it is not captured
/// by the `%form` (the parser drops unknown params — the compiler has no
/// general unknown-parameter diagnostic, verified: even `@stage(banana: 42)`
/// compiles silently), so it cannot override the source-order binding. This
/// gate proves the deletion is sound: a scene written with the OLD `objectIndex:`
/// spelling still interleaves correctly (each track binds its PRECEDING object),
/// i.e. `objectIndex` has zero effect on which object a track drives.
#[test]
fn gate_pt_object_index_is_a_dead_noop() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    let source = r#"@import "stdlib/3d";

.canvas.m {
  @stage(fov: 36, camZ: 8) {
    @object(shape: "box", size: 0.5)
    @pose-track(ease: 1, objectIndex: 1, stops: [ { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() } ])
    @object(shape: "sphere", size: 0.5)
    @pose-track(ease: 1, objectIndex: 0, stops: [ { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() } ])
  }
}
"#;
    std::fs::write(&entry, source).expect("write");
    let js = Compiler::from_file(&entry, root).expect("compile file").compile().js;

    let create: Vec<usize> = js
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("canvas._stObject = mesh;"))
        .map(|(i, _)| i)
        .collect();
    let bind: Vec<usize> = js
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("const target =") && l.contains("canvas._stObject"))
        .map(|(i, _)| i)
        .collect();

    assert_eq!(create.len(), 2, "expected 2 @object inits, got {create:?}");
    assert_eq!(bind.len(), 2, "expected 2 @pose-track inits, got {bind:?}");
    // objectIndex is intentionally inverted (1 on the FIRST track) yet the
    // binding MUST still follow SOURCE ORDER — the parameter has zero effect.
    assert!(
        create[0] < bind[0] && bind[0] < create[1] && create[1] < bind[1],
        "objectIndex must be a dead no-op (source-order pairing must win):          create@{create:?} bind@{bind:?}"
    );
}
