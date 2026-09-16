use spacetime::test_runner::compile_test_html;
fn main() {
    let html = compile_test_html(&[std::path::PathBuf::from("tests/unit/animations/on-sig-change-mutation.test.st")])
        .expect("compile");
    std::fs::write("/tmp/cdp_page.html", &html).unwrap();
    eprintln!("written {} bytes", html.len());
}
