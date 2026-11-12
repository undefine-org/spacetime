use super::*;

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(p, t)| (p.to_string(), t.to_string())).collect()
}

fn edit(file: &str, text: &str, word: &str, new: &str) -> Edit {
    let start = text.find(word).expect("word");
    Edit { file: file.into(), start, end: start + word.len(), expect: word.into(), text: new.into() }
}

const PAGE: &str = ".card { @fade-in(duration: 400ms) }\n<div class=\"card\">hi</div>\n";

#[test]
fn edits_apply_back_to_front_in_the_coordinates_read() {
    let f = files(&[("index.st", PAGE)]);
    let tx = Transaction {
        edits: vec![edit("index.st", PAGE, "400ms", "1200ms"), edit("index.st", PAGE, "hi", "hello there")],
        base: BTreeMap::new(),
    };
    let out = apply(&f, &tx).unwrap();
    assert_eq!(out["index.st"], ".card { @fade-in(duration: 1200ms) }\n<div class=\"card\">hello there</div>\n");
}

#[test]
fn a_stale_edit_is_refused() {
    let f = files(&[("index.st", PAGE)]);
    let mut e = edit("index.st", PAGE, "400ms", "1s");
    e.expect = "300ms".into();
    let err = apply(&f, &Transaction { edits: vec![e], base: BTreeMap::new() }).unwrap_err();
    assert!(matches!(err, Error::Stale { .. }), "{err:?}");
    assert!(err.to_string().starts_with("STALE"));
}

#[test]
fn a_stale_base_hash_is_refused() {
    let f = files(&[("index.st", PAGE)]);
    let tx = Transaction {
        edits: vec![edit("index.st", PAGE, "400ms", "1s")],
        base: [("index.st".to_string(), "0000000000000000".to_string())].into(),
    };
    assert!(matches!(apply(&f, &tx), Err(Error::Stale { .. })));
}

#[test]
fn overlapping_edits_and_unknown_files_are_refused() {
    let f = files(&[("index.st", PAGE)]);
    let a = edit("index.st", PAGE, "400ms", "1s");
    let mut b = a.clone();
    b.start += 1;
    b.expect = PAGE[b.start..b.end].into();
    assert!(matches!(apply(&f, &Transaction { edits: vec![a.clone(), b], base: BTreeMap::new() }), Err(Error::BadSpan { .. })));
    let mut c = a;
    c.file = "other.st".into();
    assert!(matches!(apply(&f, &Transaction { edits: vec![c], base: BTreeMap::new() }), Err(Error::UnknownFile(_))));
}

#[test]
fn an_edit_that_breaks_the_parse_is_never_written() {
    let f = files(&[("index.st", PAGE)]);
    // drop the closing brace of the rule
    let start = PAGE.find(") }").unwrap() + 2;
    let e = Edit { file: "index.st".into(), start, end: start + 1, expect: "}".into(), text: "".into() };
    match apply(&f, &Transaction { edits: vec![e], base: BTreeMap::new() }) {
        Err(Error::Unparseable { .. }) => {}
        other => {
            // the parser is lenient about some shapes; then the result must at least parse
            let out = other.expect("either refused or still parseable");
            assert!(crate::parser::parse(&out["index.st"]).is_ok());
        }
    }
}

#[test]
fn a_commit_is_written_whole_and_undone() {
    let dir = std::env::temp_dir().join(format!("rewrite-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.st"), PAGE).unwrap();
    let before = files(&[("index.st", PAGE)]);
    let changed = apply(&before, &Transaction { edits: vec![edit("index.st", PAGE, "400ms", "900ms")], base: BTreeMap::new() }).unwrap();
    commit(&dir, &before, &changed).unwrap();
    assert!(std::fs::read_to_string(dir.join("index.st")).unwrap().contains("900ms"));
    assert!(!dir.join("index.st.rewrite-tmp").exists());
    assert_eq!(undo(&dir).unwrap(), vec!["index.st".to_string()]);
    assert_eq!(std::fs::read_to_string(dir.join("index.st")).unwrap(), PAGE);
    // nothing left to undo here
    assert!(undo(&dir).unwrap().is_empty());
    // an undo after someone else changed the file is refused
    commit(&dir, &before, &changed).unwrap();
    std::fs::write(dir.join("index.st"), "/* changed */\n").unwrap();
    assert!(matches!(undo(&dir), Err(Error::Stale { .. })));
    std::fs::remove_dir_all(&dir).ok();
}
