//! Comment extraction from source text

#[derive(Debug, Clone)]
pub struct Comment {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub is_block: bool,
}

/// Extract all comments from source text
pub fn extract_comments(source: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut line = 1;

    while i < bytes.len() {
        // Track line numbers
        if bytes[i] == b'\n' {
            line += 1;
            i += 1;
            continue;
        }

        // Check for line comment
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            let comment_line = line;
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            comments.push(Comment {
                text: source[start..i].to_string(),
                start,
                end: i,
                line: comment_line,
                is_block: false,
            });
            continue;
        }

        // Check for block comment
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            let comment_line = line;
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                if bytes[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2; // Skip */
            comments.push(Comment {
                text: source[start..i.min(bytes.len())].to_string(),
                start,
                end: i.min(bytes.len()),
                line: comment_line,
                is_block: true,
            });
            continue;
        }

        // Skip double-quoted strings to avoid false positives.
        //
        // DOUBLE QUOTES ONLY, deliberately. `'` is an apostrophe in ordinary
        // prose across 185+ stdlib files ("element's", "container's") and the
        // language has no single-quoted string form; treating it as a
        // delimiter would swallow every comment after any contraction.
        // Backticks are THE hole form, not a string delimiter, for the same
        // reason.
        //
        // Newlines ARE counted while skipping: a multiline string that did not
        // advance `line` would misreport the line of every comment after it —
        // the number a structured comment persists as its anchor and shows in
        // diagnostics.
        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    if bytes[i + 1] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                } else if bytes[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    comments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_comments() {
        let source = "// comment 1\n.foo { }";
        let comments = extract_comments(source);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "// comment 1");
        assert_eq!(comments[0].line, 1);
        assert!(!comments[0].is_block);
    }

    #[test]
    fn test_block_comments() {
        let source = ".foo { /* block */ }";
        let comments = extract_comments(source);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "/* block */");
        assert!(comments[0].is_block);
    }

    #[test]
    fn test_multiple_comments() {
        let source = "// line 1\n// line 2\n/* block */";
        let comments = extract_comments(source);
        assert_eq!(comments.len(), 3);
    }
}
