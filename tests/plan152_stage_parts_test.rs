//! PLAN-152 contract gates for the stage's scene parts (the behavior gates are
//! `tests/render/raea_render_test.rs`: a named part MOVES in a rendered film).
//!
//! 1. NAMED ARGS. `@object &orb (shape: …, size: …)` — the documented spelling,
//!    a space between the name and its parens — compiled with EVERY argument at
//!    its default: the ARG_LIST began with that space, the matcher skipped only a
//!    leading `(`, so the whole call became one unnamed group and no named
//!    argument bound. Silent: the object still appeared, as a unit icosahedron.
//!    The W4 named-object gate only checked that the object moved. `@light &sun
//!    (…)` exposed it, because the responsive `@light { … }` overload made the
//!    lenient match lose loudly (E0946).
//! 2. THE CAMERA RIG STEPS ASIDE for a stage: the DOM rig transform falls back
//!    to `--cam-rig-off`, which a stage sets, so a camera form moves the real 3D
//!    camera rather than sliding the canvas as a flat picture.

use spacetime::compiler::Compiler;

fn compile_js(source: &str) -> String {
    let temp = tempfile::tempdir().expect("tempdir");
    let file = temp.path().join("probe.st");
    std::fs::write(&file, source).expect("write probe file");
    let compiled = Compiler::from_file(&file, temp.path()).expect("parse").compile();
    assert!(
        compiled.pipeline_errors.is_empty(),
        "compile errors: {:?}",
        compiled.pipeline_errors.iter().map(|e| format!("{}: {}", e.code, e.message)).collect::<Vec<_>>()
    );
    compiled.js
}

#[test]
fn a_named_object_keeps_its_arguments() {
    let js = compile_js(
        "@import \"stdlib/3d\"\n<canvas class=\"w\"></canvas>\n\
         canvas.w { @stage { @object &orb (shape: \"torusknot\", size: 3.25, posY: 7.5) } }\n",
    );
    assert!(js.contains("\"torusknot\""), "the named object's shape was dropped");
    assert!(js.contains("3.25"), "the named object's size was dropped (it fell back to 1)");
}

#[test]
fn a_named_light_compiles_beside_the_light_mode_overload_and_keeps_its_arguments() {
    let js = compile_js(
        "@import \"stdlib/3d\"\n<canvas class=\"w\"></canvas>\n\
         canvas.w { @stage { @light &sun (type: \"spot\", intensity: 2.75) } }\n",
    );
    assert!(js.contains("\"spot\""), "the named light's type was dropped");
    assert!(js.contains("2.75"), "the named light's intensity was dropped");
}

#[test]
fn a_named_model_keeps_its_clip_and_scale() {
    let js = compile_js(
        "@import \"stdlib/3d\"\n<canvas class=\"w\"></canvas>\n\
         canvas.w { @stage { @gltf &robot (src: \"models/robot.glb\", clip: \"Wave\", scale: 0.45) } }\n",
    );
    assert!(js.contains("\"Wave\""), "the named model's clip was dropped");
    assert!(js.contains("0.45"), "the named model's scale was dropped");
}

#[test]
fn the_dom_camera_rig_falls_back_to_the_stage_switch() {
    let js = compile_js(
        "@import \"stdlib/film\"\n<div class=\"reel\"><div class=\"shot\"></div></div>\n\
         .reel { @score &.time(2s) { &shot at 0s for 2s; } }\n\
         .shot { @on &.clip --push-in; }\n",
    );
    assert!(js.contains("var(--cam-rig-off,"), "the camera rig must step aside when a stage sets --cam-rig-off");
}
