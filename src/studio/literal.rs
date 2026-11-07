//! A lossless reader for `@data` values: JSON with the leniencies authors use
//! in `.st` (comments, trailing commas, single quotes, bare keys).
//!
//! Lossless means every node keeps the byte span it was read from, in the
//! coordinates of the WHOLE file, so an edit replaces exactly those bytes and
//! leaves the author's spacing, order and comments alone.

use super::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    Array { span: Span, items: Vec<Lit> },
    Object { span: Span, entries: Vec<Entry> },
    Str { span: Span, value: String },
    /// Numbers, booleans, null, bare identifiers: kept as their raw text.
    Other { span: Span, raw: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub key: String,
    pub key_span: Span,
    pub value: Lit,
}

impl Lit {
    pub fn span(&self) -> Span {
        match self {
            Lit::Array { span, .. }
            | Lit::Object { span, .. }
            | Lit::Str { span, .. }
            | Lit::Other { span, .. } => *span,
        }
    }

    /// The value as display text: a string's content, anything else's raw text.
    pub fn text(&self, src: &str) -> String {
        match self {
            Lit::Str { value, .. } => value.clone(),
            Lit::Other { raw, .. } => raw.clone(),
            other => src[other.span().start..other.span().end].to_string(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Lit> {
        match self {
            Lit::Object { entries, .. } => entries.iter().find(|e| e.key == key).map(|e| &e.value),
            _ => None,
        }
    }
}

/// Read ONE value starting at byte `start` of `src` (leading whitespace and
/// comments skipped). Returns the value; its span says where it ended.
pub fn read(src: &str, start: usize) -> Result<Lit, String> {
    let mut r = Reader { src, b: src.as_bytes(), i: start };
    r.skip_trivia();
    r.value()
}

struct Reader<'a> {
    src: &'a str,
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn skip_trivia(&mut self) {
        loop {
            while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
                self.i += 1;
            }
            if self.b[self.i..].starts_with(b"//") {
                while self.peek().is_some_and(|c| c != b'\n') {
                    self.i += 1;
                }
            } else if self.b[self.i..].starts_with(b"/*") {
                match self.src[self.i + 2..].find("*/") {
                    Some(end) => self.i += 2 + end + 2,
                    None => self.i = self.b.len(),
                }
            } else {
                return;
            }
        }
    }

    fn value(&mut self) -> Result<Lit, String> {
        match self.peek() {
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'"') | Some(b'\'') => self.string(),
            Some(_) => self.other(),
            None => Err("unexpected end of value".to_string()),
        }
    }

    fn array(&mut self) -> Result<Lit, String> {
        let start = self.i;
        self.i += 1;
        let mut items = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some(b']') => {
                    self.i += 1;
                    return Ok(Lit::Array { span: Span::new(start, self.i), items });
                }
                Some(b',') => self.i += 1,
                Some(_) => items.push(self.value()?),
                None => return Err(format!("unterminated array opened at byte {start}")),
            }
        }
    }

    fn object(&mut self) -> Result<Lit, String> {
        let start = self.i;
        self.i += 1;
        let mut entries = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Lit::Object { span: Span::new(start, self.i), entries });
                }
                Some(b',') => self.i += 1,
                Some(_) => {
                    let key_lit = self.value()?;
                    let key = match &key_lit {
                        Lit::Str { value, .. } => value.clone(),
                        Lit::Other { raw, .. } => raw.clone(),
                        _ => return Err(format!("object key must be a string at byte {}", key_lit.span().start)),
                    };
                    self.skip_trivia();
                    if self.peek() != Some(b':') {
                        return Err(format!("expected ':' after key '{key}' at byte {}", self.i));
                    }
                    self.i += 1;
                    self.skip_trivia();
                    let value = self.value()?;
                    entries.push(Entry { key, key_span: key_lit.span(), value });
                }
                None => return Err(format!("unterminated object opened at byte {start}")),
            }
        }
    }

    fn string(&mut self) -> Result<Lit, String> {
        let start = self.i;
        let quote = self.b[self.i];
        self.i += 1;
        let mut value = String::new();
        loop {
            let Some(c) = self.src[self.i..].chars().next() else {
                return Err(format!("unterminated string opened at byte {start}"));
            };
            self.i += c.len_utf8();
            if c as u32 == quote as u32 {
                return Ok(Lit::Str { span: Span::new(start, self.i), value });
            }
            if c != '\\' {
                value.push(c);
                continue;
            }
            let Some(e) = self.src[self.i..].chars().next() else {
                return Err(format!("unterminated escape in string at byte {start}"));
            };
            self.i += e.len_utf8();
            match e {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                'r' => value.push('\r'),
                'b' => value.push('\u{8}'),
                'f' => value.push('\u{c}'),
                'u' => {
                    let hex = self.src.get(self.i..self.i + 4).ok_or("short \\u escape")?;
                    let code = u32::from_str_radix(hex, 16).map_err(|_| "bad \\u escape")?;
                    value.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    self.i += 4;
                }
                other => value.push(other),
            }
        }
    }

    fn other(&mut self) -> Result<Lit, String> {
        let start = self.i;
        while self
            .peek()
            .is_some_and(|c| !c.is_ascii_whitespace() && !matches!(c, b',' | b':' | b']' | b'}' | b'[' | b'{' | b';'))
        {
            self.i += 1;
        }
        if self.i == start {
            return Err(format!("unexpected '{}' at byte {start}", self.b[start] as char));
        }
        Ok(Lit::Other { span: Span::new(start, self.i), raw: self.src[start..self.i].to_string() })
    }
}

/// Encode a string as a literal for the source (JSON escaping, double quotes).
pub fn encode_str(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rows_with_exact_spans() {
        let src = r#"x : [ { "n": "01", "line": "AI isn't" }, /* two */ { n: 2, "ok": true }, ];"#;
        let start = src.find('[').unwrap();
        let lit = read(src, start).unwrap();
        let Lit::Array { items, span } = &lit else { panic!() };
        assert_eq!(&src[span.start..span.end], &src[start..src.rfind(']').unwrap() + 1]);
        assert_eq!(items.len(), 2);
        let line = items[0].get("line").unwrap();
        assert_eq!(line.text(src), "AI isn't");
        assert_eq!(&src[line.span().start..line.span().end], "\"AI isn't\"");
        assert_eq!(items[1].get("n").unwrap().text(src), "2");
        assert_eq!(items[1].get("ok").unwrap().text(src), "true");
    }

    #[test]
    fn decodes_escapes_and_keeps_unicode_spans() {
        let src = r#"["a \"q\" é — b"]"#;
        let lit = read(src, 0).unwrap();
        let Lit::Array { items, .. } = lit else { panic!() };
        assert_eq!(items[0].text(src), "a \"q\" é — b");
        assert_eq!(items[0].span().end, src.len() - 1);
    }

    #[test]
    fn unterminated_input_is_an_error_not_a_panic() {
        assert!(read("[ {\"a\": \"b\" ", 0).is_err());
        assert!(read("\"open", 0).is_err());
    }
}
