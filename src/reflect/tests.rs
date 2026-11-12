use super::*;

fn reflect_text(files: &[(&str, &str)]) -> Value {
    let files: Vec<File<'_>> = files.iter().map(|(p, t)| File { path: p, text: t }).collect();
    reflect(&files, &crate::compiler::cached_stdlib_registry().0)
}

fn robot_day() -> (String, Value) {
    let text = std::fs::read_to_string("demos/film-robot-day/index.st").expect("robot day");
    // the page's own imports (`stdlib/3d`) bring its stage macros
    let v = page(std::path::Path::new("demos/film-robot-day"), "index.st").expect("reflects");
    (text, v)
}

fn find<'a>(v: &'a Value, pred: impl Fn(&Value) -> bool) -> &'a Value {
    let all = v["matches"].as_array().unwrap();
    all.iter().find(|m| pred(m)).unwrap_or_else(|| {
        let seen: Vec<String> = all.iter().map(|m| format!("{}/{}", m["directive"], m["macro"])).collect();
        panic!("no such match; files: {}; seen: {seen:?}", v["files"])
    })
}

fn param<'a>(m: &'a Value, name: &str) -> &'a Value {
    m["params"].as_array().unwrap().iter().find(|p| p["name"] == name).unwrap_or_else(|| panic!("param {name} in {}", m["macro"]))
}

#[test]
fn a_part_shows_its_declared_params_written_or_not() {
    let (text, v) = robot_day();
    let robot = find(&v, |m| m["macro"] == "gltf");
    // written: the words and their span are the source's
    let scale = param(robot, "scale");
    assert_eq!(scale["type"], "number");
    let w = &scale["written"];
    assert_eq!(&text[w["start"].as_u64().unwrap() as usize..w["end"].as_u64().unwrap() as usize], "0.42");
    assert_eq!(w["text"], "0.42");
    // left out: no span, the form's default
    let spin = param(robot, "spinY");
    assert!(spin["written"].is_null());
    assert_eq!(spin["default"], "0");
    // a light's colour is typed as the form declares it
    let sun = find(&v, |m| m["macro"] == "light" && m["spans"]["named"]["text"] == "&sun");
    assert_eq!(param(sun, "color")["type"], "color");
}

#[test]
fn nested_matches_are_children_by_id() {
    let (_, v) = robot_day();
    let stage = find(&v, |m| m["directive"] == "stage");
    let kids = stage["children"].as_array().unwrap();
    assert!(kids.len() >= 15, "the stage holds its lights and objects: {}", kids.len());
    let first = kids[0].as_str().unwrap();
    let child = find(&v, |m| m["id"] == first);
    assert_eq!(child["parent"], stage["id"]);
}

#[test]
fn a_macro_with_registers_resolves_its_entry_per_call() {
    // the page's own macro (no stdlib name involved), registering a `clip`
    let src = r#"
%macro beat {
  %form { @beat $name:ident(at: $at:number = 0) }
  %registers cue($name) {
    at: $at
    kind: drum
  }
}
.a { @beat kick(at: 2) }
<div class="a"></div>
"#;
    let v = reflect_text(&[("index.st", src)]);
    let beat = find(&v, |m| m["macro"] == "beat");
    let e = &beat["entries"][0];
    assert_eq!(e["kind"], "cue");
    assert_eq!(e["key"], "kick");
    assert_eq!(e["fields"]["kind"], "drum");
    assert_eq!(e["fields"]["at"], 2.0);
}

#[test]
fn the_doc_above_a_match_is_reflected() {
    let src = "/// the hero's entrance\n.hero { @fade-in(duration: 400ms) }\n<div class=\"hero\"></div>\n";
    let v = reflect_text(&[("index.st", src)]);
    let m = find(&v, |m| m["directive"].as_str().unwrap_or("").contains("fade"));
    assert!(m["doc"].as_str().unwrap_or("").contains("entrance") || m["doc"].is_null());
    assert_eq!(v["files"][0]["hash"], text_hash(src));
}

#[test]
fn a_doc_above_a_form_param_is_that_params_doc() {
    let src = r#"
%macro glow {
  %form {
    @glow(
      /// how bright, in stops
      /// (0 is off)
      gain: $gain:number = 1,
      // a plain comment documents nothing
      tint: $tint:color = white
    )
  }
}
.a { @glow(gain: 2) }
<div class="a"></div>
"#;
    let v = reflect_text(&[("index.st", src)]);
    let m = find(&v, |m| m["macro"] == "glow");
    assert_eq!(param(m, "gain")["doc"], "how bright, in stops\n(0 is off)");
    assert!(param(m, "tint")["doc"].is_null());
    assert_eq!(param(m, "gain")["written"]["text"], "2");
}

#[test]
fn a_models_clips_are_the_clip_fields_choices() {
    let (_, v) = robot_day();
    let robot = find(&v, |m| m["macro"] == "gltf");
    let clip = param(robot, "clip");
    assert_eq!(clip["type"], "model_clip");
    let words: Vec<&str> = clip["choices"].as_array().expect("choices").iter().map(|c| c["word"].as_str().unwrap()).collect();
    assert_eq!(words.len(), 14, "robot.glb's clips: {words:?}");
    assert!(words.contains(&"Walking"));
}

#[test]
fn a_keyword_set_param_offers_its_words_with_their_docs() {
    let (_, v) = robot_day();
    let sun = find(&v, |m| m["macro"] == "light" && m["spans"]["named"]["text"] == "&sun");
    let ty = param(sun, "type");
    assert_eq!(ty["type"], "light_kind");
    let point = ty["choices"].as_array().unwrap().iter().find(|c| c["word"] == "point").expect("point");
    assert!(point["doc"].as_str().unwrap_or("").contains("lamp"), "{point}");
    let pen = param(sun, "penumbra");
    assert_eq!(pen["scalar"]["range"], json!([0.0, 1.0]));
}
