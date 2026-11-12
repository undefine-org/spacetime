//! A value is checked against the file it names (PLAN-156, `%facts`): a
//! model's clip against the model.

use std::path::Path;

fn compile_page(clip: &str) -> Vec<String> {
    let dir = std::env::temp_dir().join(format!("st-facts-{}-{clip}", std::process::id()));
    std::fs::create_dir_all(dir.join("models")).unwrap();
    std::fs::copy("demos/film-robot-day/models/robot.glb", dir.join("models/robot.glb")).unwrap();
    let page = format!(
        r#"@import "stdlib/3d"
canvas.world {{
  @stage(camZ: 6) {{
    @light (type: directional, intensity: 2)
    @gltf &robot (src: "models/robot.glb", clip: "{clip}", scale: 0.42)
  }}
}}
<canvas class="world"></canvas>
"#
    );
    std::fs::write(dir.join("index.st"), page).unwrap();
    let out = spacetime::compiler::Compiler::from_file(&dir.join("index.st"), Path::new("."))
        .expect("parses")
        .compile();
    std::fs::remove_dir_all(&dir).ok();
    out.pipeline_errors.iter().map(|e| format!("{}: {} {:?}", e.code, e.message, e.hint)).collect()
}

#[test]
fn a_clip_the_model_has_compiles() {
    let errs = compile_page("Walking");
    assert!(errs.iter().all(|e| !e.starts_with("E0971")), "{errs:?}");
}

#[test]
fn a_clip_the_model_lacks_is_refused_with_its_clips() {
    let errs = compile_page("Walkng");
    let e = errs.iter().find(|e| e.starts_with("E0971")).unwrap_or_else(|| panic!("E0971 expected: {errs:?}"));
    assert!(e.contains("robot.glb") && e.contains("Walking") && e.contains("did you mean"), "{e}");
}
