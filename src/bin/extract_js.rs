use spacetime::test_runner::{CompileOptions, compile_test_html_with_options};
use std::path::PathBuf;

fn main() {
    let file = std::env::args().nth(1).expect("Usage: extract_js <file>");
    let paths = vec![PathBuf::from(&file)];
    let html = compile_test_html_with_options(&paths, &CompileOptions::default()).unwrap();

    // Extract the second <script> block (compiled test code)
    let scripts: Vec<&str> = html.split("<script>").collect();
    if scripts.len() >= 3 {
        // scripts[2] is the compiled test code script (0=before first script, 1=runtime, 2=compiled, 3=runner)
        let compiled_script = scripts[2].split("</script>").next().unwrap_or("");
        std::fs::write("/tmp/compiled_test.js", compiled_script).unwrap();
        eprintln!("Extracted {} bytes of compiled JS", compiled_script.len());
    } else {
        eprintln!(
            "Unexpected HTML structure, found {} script blocks",
            scripts.len()
        );
    }
}
