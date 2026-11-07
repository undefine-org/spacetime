//! The shape of a film's score — what the film studio draws (PLAN-155 F3).
//!
//! A film is its source: a `@score` places clips and sounds in time, each
//! clip's `@on &.clip { … }` body moves things over its window, `@form
//! instrument` / `@form pattern` say what the sounds are, and a `@stage`'s
//! parts are what the camera sees. [`score`] reads all of it from the REAL
//! parser's form matches — every part with the byte span it came from (the
//! metasystem keeps a span for every capture, nested, under its grammar path:
//! `entries.3.line.head.places.1.for.for`). Nothing here re-reads the language:
//! a keyframe chain is split into stops by the compiler's own stop parser
//! (`metasystem::expand::parse_stops_spanned`), a time by the score's
//! (`pipeline::score::time_ms`), and a pattern row into cells the way the
//! synthesizer reads it (whitespace).
//!
//! The studio edits by these spans (PLAN-154's guarded `decl` rail): moving a
//! clip rewrites its `at` measure, a keyframe its stop, a drum cell its word.

use serde_json::{Map, Value, json};

use super::outline::Site;
use crate::syntax::{CapturedValue, FormMatch};

/// The film's shape, for `GET /__spacetime/studio/score.json`.
pub fn score(site: &Site) -> Value {
    let mut out = Shape::default();
    for (fi, f) in site.files.iter().enumerate() {
        let Ok(ast) = crate::parser::parse(&f.text) else { continue };
        let file = File { index: fi, text: &f.text };
        // a match inside a selector is visited with its selector (a clip body
        // needs it); the file's flat list holds every match, so it adds only
        // the file-level ones
        let mut seen = std::collections::HashSet::new();
        for scope in &ast.scopes {
            for m in &scope.matches {
                seen.insert(m.span.start);
                out.visit(&file, Some(&scope.selector), m);
            }
            for nested in &scope.nested_scopes {
                for m in &nested.matches {
                    seen.insert(m.span.start);
                    out.visit(&file, Some(&nested.selector), m);
                }
            }
        }
        for m in &ast.matches {
            if seen.insert(m.span.start) {
                out.visit(&file, None, m);
            }
        }
    }
    json!({
        "entry": site.entry,
        "files": site.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>(),
        // the code pane edits a whole file, guarded by the hash it was read at
        "sources": site.files.iter().map(|f| json!({
            "path": f.path, "text": f.text, "hash": crate::studio::ops::source_hash(&f.text),
        })).collect::<Vec<_>>(),
        "scores": out.scores,
        "clips": out.clips,
        "forms": out.forms,
        "parts": out.parts,
    })
}

/// The synth program of every score on the page (what plays and what
/// `render` mixes), from the compiler's merged tree.
pub fn programs(dir: &std::path::Path, entry: &str) -> Value {
    let Ok(compiler) = crate::compiler::Compiler::from_file(&dir.join(entry), dir) else {
        return Value::Array(Vec::new());
    };
    let merged = compiler.merged_ast();
    let mut matches = merged.matches.clone();
    crate::render::collect_scope_matches(&merged.scopes, &mut matches);
    let registry = crate::compiler::cached_stdlib_registry().0;
    Value::Array(crate::pipeline::score::declared_soundtracks(&matches, &registry))
}

#[derive(Default)]
struct Shape {
    scores: Vec<Value>,
    clips: Map<String, Value>,
    forms: Map<String, Value>,
    parts: Vec<Value>,
}

struct File<'a> {
    index: usize,
    text: &'a str,
}

impl File<'_> {
    /// `{file, start, end, text}` — the address an edit carries back.
    fn at(&self, span: (usize, usize)) -> Value {
        json!({ "file": self.index, "start": span.0, "end": span.1, "text": &self.text[span.0..span.1] })
    }
}

fn span(m: &FormMatch, path: &str) -> Option<(usize, usize)> {
    m.capture_spans.get(path).map(|s| (s.start as usize, s.end as usize))
}

fn has(m: &FormMatch, path: &str) -> bool {
    m.capture_spans.contains_key(path)
}

/// `.shot` / `div.shot` → `shot`: the element a score's `&shot` names.
fn subject_of(selector: &str) -> String {
    selector.rsplit('.').next().unwrap_or(selector).trim().to_string()
}

impl Shape {
    fn visit(&mut self, f: &File, selector: Option<&str>, m: &FormMatch) {
        match m.matched_macro.as_deref() {
            Some("score-block") => self.score(f, selector, m),
            Some(mm) if mm.starts_with("on-driver-body") => self.clip(f, selector, m),
            Some(mm) if mm.starts_with("form-") => self.form(f, m),
            Some("stage") => {
                self.part(f, m);
                if let Some(CapturedValue::Block(kids)) = m.captures.get("children") {
                    for k in kids {
                        self.part(f, k);
                    }
                }
            }
            _ => {}
        }
    }

    fn score(&mut self, f: &File, selector: Option<&str>, m: &FormMatch) {
        let total = span(m, "driver.driver_params.0.value").map(|s| f.at(s));
        let total_ms = total.as_ref().and_then(|t| t["text"].as_str()).and_then(crate::pipeline::score::time_ms);
        let mut entries = Vec::new();
        let mut i = 0;
        while let Some(whole) = span(m, &format!("entries.{i}")) {
            let p = format!("entries.{i}");
            let mut e = Map::new();
            e.insert("span".into(), f.at(whole));
            if has(m, &format!("{p}.line.head.subject")) {
                let head = format!("{p}.line.head");
                e.insert("kind".into(), "clip".into());
                let subject = span(m, &format!("{head}.subject")).map(|s| f.text[s.0..s.1].trim_start_matches('&').to_string());
                e.insert("subject".into(), subject.into());
                let mut j = 0;
                while has(m, &format!("{head}.places.{j}")) {
                    let place = format!("{head}.places.{j}");
                    for (word, path) in [("at", format!("{place}.at.at")), ("for", format!("{place}.for.for"))] {
                        if let Some(s) = span(m, &path) {
                            e.insert(word.into(), f.at(s));
                            e.insert(format!("{word}_ms"), crate::pipeline::score::time_ms(&f.text[s.0..s.1]).into());
                        }
                    }
                    j += 1;
                }
                e.insert("steps".into(), has(m, &format!("{p}.line.steps")).into());
            } else if has(m, &format!("{p}.sound")) {
                let snd = format!("{p}.sound");
                e.insert("kind".into(), "sound".into());
                for (key, path) in [
                    ("form", format!("{snd}.form")),
                    ("at", format!("{snd}.at")),
                    ("for", format!("{snd}.for.len")),
                    ("notes", format!("{snd}.notes.notes")),
                    ("name", format!("{snd}.named.name")),
                ] {
                    if let Some(s) = span(m, &path) {
                        e.insert(key.into(), f.at(s));
                        if key == "at" || key == "for" {
                            e.insert(format!("{key}_ms"), crate::pipeline::score::time_ms(&f.text[s.0..s.1]).into());
                        }
                    }
                }
            } else {
                e.insert("kind".into(), "other".into());
            }
            entries.push(Value::Object(e));
            i += 1;
        }
        let name = span(m, "as.name").map(|s| f.text[s.0..s.1].trim_start_matches('&').to_string());
        self.scores.push(json!({
            "name": name,
            "host": selector,
            "total": total,
            "total_ms": total_ms,
            "span": f.at((m.span.start as usize, m.span.end as usize)),
            "entries": entries,
        }));
    }

    /// An `@on &.clip { … }` body: the clip's channels, each a chain of stops.
    fn clip(&mut self, f: &File, selector: Option<&str>, m: &FormMatch) {
        let member = span(m, "driver.member").map(|s| &f.text[s.0..s.1]);
        if member != Some("clip") {
            return;
        }
        let Some(selector) = selector else { return };
        let mut channels = Vec::new();
        let mut i = 0;
        while has(m, &format!("body.{i}")) {
            let item = format!("body.{i}");
            if let Some(sel) = span(m, &format!("{item}.sel")) {
                let scope = f.text[sel.0..sel.1].to_string();
                let mut j = 0;
                while has(m, &format!("{item}.scope.{j}")) {
                    if let Some(c) = channel(f, m, &format!("{item}.scope.{j}.line"), Some(&scope)) {
                        channels.push(c);
                    }
                    j += 1;
                }
            } else if let Some(c) = channel(f, m, &format!("{item}.line"), None) {
                channels.push(c);
            }
            i += 1;
        }
        self.clips.insert(
            subject_of(selector),
            json!({ "selector": selector, "span": f.at((m.span.start as usize, m.span.end as usize)), "channels": channels }),
        );
    }

    /// `@form instrument --kick { … }` / `@form pattern --beat(…) { … }`.
    fn form(&mut self, f: &File, m: &FormMatch) {
        let Some(name) = span(m, "name").map(|s| f.text[s.0..s.1].to_string()) else { return };
        let kind = m.matched_macro.as_deref().unwrap_or("form").trim_start_matches("form-").to_string();
        let mut props = Vec::new();
        let mut i = 0;
        while has(m, &format!("body.{i}")) {
            let item = format!("body.{i}");
            if let (Some(p), Some(v)) = (span(m, &format!("{item}.property")), span(m, &format!("{item}.values"))) {
                let mut prop = json!({ "prop": &f.text[p.0..p.1], "value": f.at(v) });
                if kind == "pattern" {
                    prop["cells"] = cells(f, v);
                } else {
                    prop["stops"] = stops(f, v);
                }
                props.push(prop);
            }
            i += 1;
        }
        self.forms.insert(name, json!({ "kind": kind, "span": f.at((m.span.start as usize, m.span.end as usize)), "props": props }));
    }

    /// A stage part (`@object &orb (…)`, `@light`, `@gltf`, …) and its params:
    /// the ones written (with their spans) and the ones left at their default.
    fn part(&mut self, f: &File, m: &FormMatch) {
        let kind = m.matched_macro.clone().unwrap_or_default();
        let named = span(m, "named").map(|s| &f.text[s.0..s.1]);
        let name = named.and_then(|t| t.strip_prefix('&')).map(|t| {
            t.chars().take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect::<String>()
        });
        let mut params = Vec::new();
        let mut keys: Vec<&String> = m.captures.keys().collect();
        keys.sort();
        for key in keys {
            if matches!(key.as_str(), "named" | "children" | "body") {
                continue;
            }
            let written = span(m, key).map(|s| f.at(s));
            params.push(json!({
                "name": key,
                "written": written,
                "value": captured_text(&m.captures[key]),
            }));
        }
        self.parts.push(json!({
            "kind": kind,
            "name": name,
            "span": f.at((m.span.start as usize, m.span.end as usize)),
            "params": params,
        }));
    }
}

/// One `prop: a -> b at 40% -> c;` line: its value split into stops.
fn channel(f: &File, m: &FormMatch, line: &str, scope: Option<&str>) -> Option<Value> {
    let p = span(m, &format!("{line}.prop"))?;
    let v = span(m, &format!("{line}.value"))?;
    Some(json!({
        "scope": scope,
        "prop": &f.text[p.0..p.1],
        "value": f.at(v),
        "stops": stops(f, v),
    }))
}

/// A keyframe chain's stops, by the compiler's own stop parser.
fn stops(f: &File, v: (usize, usize)) -> Value {
    let text = &f.text[v.0..v.1];
    crate::metasystem::expand::parse_stops_spanned(text)
        .into_iter()
        .map(|(stop, (a, b))| {
            json!({
                "value": stop.value,
                "at": stop.at,
                "easing": stop.easing,
                "span": f.at((v.0 + a, v.0 + b)),
            })
        })
        .collect()
}

/// A pattern row's cells — whitespace-separated, as the synthesizer reads them.
fn cells(f: &File, v: (usize, usize)) -> Value {
    let text = &f.text[v.0..v.1];
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                out.push(f.at((v.0 + s, v.0 + i)));
                start = None;
            }
            _ => {}
        }
    }
    Value::Array(out)
}

/// A param's value as the author would write it (a default the form filled in).
fn captured_text(v: &CapturedValue) -> Value {
    match v {
        CapturedValue::String(s) | CapturedValue::Ident(s) | CapturedValue::Color(s) | CapturedValue::Expr(s) => {
            Value::from(s.clone())
        }
        CapturedValue::Number(n) => json!(n),
        CapturedValue::Bool(b) => json!(b),
        other => Value::from(format!("{other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn robot_day() -> Value {
        let text = std::fs::read_to_string("demos/film-robot-day/index.st").expect("robot day");
        score(&Site::from_files(&[("index.st", &text)]))
    }

    #[test]
    fn the_score_entries_carry_their_at_and_for_words() {
        let s = robot_day();
        let entries = s["scores"][0]["entries"].as_array().expect("entries");
        let clips: Vec<&Value> = entries.iter().filter(|e| e["kind"] == "clip").collect();
        assert_eq!(clips.len(), 7, "robot day places 7 clips");
        let card = clips.iter().find(|e| e["subject"] == "card").expect("&card");
        assert_eq!(card["at"]["text"], "0.4s");
        assert_eq!(card["for"]["text"], "5.2s");
        assert_eq!(card["at_ms"], 400.0);
        assert_eq!(card["for_ms"], 5200.0);
        assert_eq!(s["scores"][0]["name"], "film");
        assert_eq!(s["scores"][0]["total_ms"], 24000.0);
        assert_eq!(s["scores"].as_array().unwrap().len(), 1, "one score, listed once");
        assert_eq!(s["parts"].as_array().unwrap().iter().filter(|p| p["name"] == "robot").count(), 1, "each part once");
        let sounds: Vec<&Value> = entries.iter().filter(|e| e["kind"] == "sound").collect();
        assert_eq!(sounds.len(), 6, "robot day places 6 sounds");
        assert_eq!(sounds[0]["form"]["text"], "--pad");
        assert_eq!(sounds[0]["notes"]["text"], "\"C3 Eb3 G3 Bb3\"");
    }

    #[test]
    fn every_span_is_the_text_it_names() {
        let text = std::fs::read_to_string("demos/film-robot-day/index.st").unwrap();
        let s = robot_day();
        fn walk(v: &Value, text: &str, n: &mut usize) {
            match v {
                Value::Object(o) => {
                    if let (Some(a), Some(b), Some(t)) = (o.get("start"), o.get("end"), o.get("text")) {
                        let (a, b) = (a.as_u64().unwrap() as usize, b.as_u64().unwrap() as usize);
                        assert_eq!(&text[a..b], t.as_str().unwrap());
                        *n += 1;
                    }
                    for x in o.values() {
                        walk(x, text, n);
                    }
                }
                Value::Array(a) => a.iter().for_each(|x| walk(x, text, n)),
                _ => {}
            }
        }
        let mut n = 0;
        walk(&s, &text, &mut n);
        assert!(n > 300, "robot day has hundreds of addressable words, saw {n}");
    }

    #[test]
    fn clip_bodies_are_channels_of_stops() {
        let s = robot_day();
        let shot = &s["clips"]["shot"]["channels"];
        let robot_clip = shot.as_array().unwrap().iter().find(|c| c["scope"] == "&robot" && c["prop"] == "clip").expect("&robot clip");
        let stops = robot_clip["stops"].as_array().unwrap();
        assert_eq!(stops.len(), 7);
        assert_eq!(stops[1]["value"], "\"Walking\"");
        assert_eq!(stops[1]["at"], 0.25);
        assert_eq!(stops[1]["easing"], "--steps(1)");
        assert_eq!(stops[1]["span"]["text"], "\"Walking\" at 25% --steps(1)");
        let card = &s["clips"]["card"]["channels"][0];
        assert_eq!(card["prop"], "opacity");
        assert_eq!(card["stops"][1]["span"]["text"], "1 at 18%");
    }

    #[test]
    fn instruments_patterns_and_stage_parts() {
        let s = robot_day();
        let kick = &s["forms"]["--kick"];
        assert_eq!(kick["kind"], "instrument");
        let pitch = kick["props"].as_array().unwrap().iter().find(|p| p["prop"] == "pitch").unwrap();
        assert_eq!(pitch["stops"][1]["span"]["text"], "42hz at 25%");
        let beat = &s["forms"]["--beat"];
        assert_eq!(beat["kind"], "pattern");
        let kick_row = beat["props"].as_array().unwrap().iter().find(|p| p["prop"] == "--kick").unwrap();
        assert_eq!(kick_row["cells"].as_array().unwrap().len(), 16);
        assert_eq!(kick_row["cells"][0]["text"], "x");
        let robot = s["parts"].as_array().unwrap().iter().find(|p| p["name"] == "robot").expect("&robot");
        assert_eq!(robot["kind"], "gltf");
        let scale = robot["params"].as_array().unwrap().iter().find(|p| p["name"] == "scale").unwrap();
        assert_eq!(scale["written"]["text"], "0.42");
        let stage = s["parts"].as_array().unwrap().iter().find(|p| p["kind"] == "stage").unwrap();
        let near = stage["params"].as_array().unwrap().iter().find(|p| p["name"] == "near").unwrap();
        assert!(near["written"].is_null(), "an unwritten param is shown at its default");
    }

    /// The edit a gesture sends: one `score.json` address, a new word.
    fn span_op(addr: &Value, value: &str) -> crate::studio::ops::Op {
        crate::studio::ops::Op::Span {
            file: "index.st".into(),
            start: addr["start"].as_u64().unwrap() as usize,
            end: addr["end"].as_u64().unwrap() as usize,
            expect: addr["text"].as_str().unwrap().into(),
            value: value.into(),
        }
    }

    fn apply(text: &str, op: &crate::studio::ops::Op) -> Result<String, String> {
        let site = Site::from_files(&[("index.st", text)]);
        crate::studio::ops::apply(&site, op).map(|c| c["index.st"].clone())
    }

    #[test]
    fn a_gesture_rewrites_exactly_its_word() {
        let text = std::fs::read_to_string("demos/film-robot-day/index.st").unwrap();
        let s = robot_day();
        // drag the &card clip to start at 1 s
        let card = s["scores"][0]["entries"].as_array().unwrap().iter().find(|e| e["subject"] == "card").unwrap();
        let moved = apply(&text, &span_op(&card["at"], "1s")).unwrap();
        assert!(moved.contains("&card at 1s for 5.2s;"));
        assert_eq!(moved.len(), text.len() - "0.4s".len() + "1s".len(), "nothing else changed");
        let again = score(&Site::from_files(&[("index.st", &moved)]));
        let card2 = again["scores"][0]["entries"].as_array().unwrap().iter().find(|e| e["subject"] == "card").unwrap();
        assert_eq!(card2["at_ms"], 1000.0);
        // a stale address is refused, never guessed
        assert!(apply(&moved, &span_op(&card["at"], "2s")).is_err());
        // a drum cell
        let row = s["forms"]["--beat"]["props"].as_array().unwrap().iter().find(|p| p["prop"] == "--kick").unwrap();
        let toggled = apply(&text, &span_op(&row["cells"][1], "x")).unwrap();
        assert!(toggled.contains("--kick:  x x . . . . . . x . x . . . . . ;"));
        // a keyframe stop
        let card_opacity = &s["clips"]["card"]["channels"][0]["stops"][1]["span"];
        let stop = apply(&text, &span_op(card_opacity, "1 at 25%")).unwrap();
        assert!(stop.contains("opacity: 0 -> 1 at 25% -> 1 at 78% -> 0;"));
    }

    #[test]
    fn a_param_is_added_and_a_file_is_saved_whole() {
        use crate::studio::ops::{Op, source_hash};
        let text = std::fs::read_to_string("demos/film-robot-day/index.st").unwrap();
        let s = robot_day();
        let robot = s["parts"].as_array().unwrap().iter().find(|p| p["name"] == "robot").unwrap();
        let scale = robot["params"].as_array().unwrap().iter().find(|p| p["name"] == "scale").unwrap();
        let at = scale["written"]["end"].as_u64().unwrap() as usize;
        let added = apply(&text, &Op::Insert { file: "index.st".into(), at, before: "0.42".into(), text: ", rate: 1.5".into() }).unwrap();
        assert!(added.contains("scale: 0.42, rate: 1.5"));
        assert!(apply(&text, &Op::Insert { file: "index.st".into(), at, before: "0.43".into(), text: "x".into() }).is_err());
        let saved = apply(&text, &Op::Source { file: "index.st".into(), base: source_hash(&text), text: "// new\n".into() }).unwrap();
        assert_eq!(saved, "// new\n");
        assert!(apply(&added, &Op::Source { file: "index.st".into(), base: source_hash(&text), text: "x".into() }).is_err(), "a save on a stale base is refused");
    }
}
