//! Minimal JSON (RFC 8259 subset) — enough for Flow's own config, receipts and
//! journal records. No external crate is used, so the CLI stays dependency-free.

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn obj(pairs: Vec<(&str, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }
}

impl std::fmt::Display for Json {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        self.write(&mut out, None, 0);
        f.write_str(&out)
    }
}

impl Json {
    pub fn to_string_pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, Some(2), 0);
        out
    }

    fn write(&self, out: &mut String, indent: Option<usize>, level: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 9.0e15 {
                    out.push_str(&format!("{}", *n as i64));
                } else {
                    out.push_str(&format!("{}", n));
                }
            }
            Json::Str(s) => write_string(out, s),
            Json::Arr(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, indent, level + 1);
                    item.write(out, indent, level + 1);
                }
                newline(out, indent, level);
                out.push(']');
            }
            Json::Obj(pairs) => {
                if pairs.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, indent, level + 1);
                    write_string(out, k);
                    out.push(':');
                    if indent.is_some() {
                        out.push(' ');
                    }
                    v.write(out, indent, level + 1);
                }
                newline(out, indent, level);
                out.push('}');
            }
        }
    }
}

fn newline(out: &mut String, indent: Option<usize>, level: usize) {
    if let Some(width) = indent {
        out.push('\n');
        for _ in 0..width * level {
            out.push(' ');
        }
    }
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

// ------------------------------------------------------------------ parser
pub fn parse(src: &str) -> Result<Json, String> {
    let mut p = Parser {
        bytes: src.as_bytes(),
        pos: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.pos != p.bytes.len() {
        return Err(format!("trailing data at byte {}", p.pos));
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn ws(&mut self) {
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r')
        {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn value(&mut self) -> Result<Json, String> {
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            Some(c) => Err(format!("unexpected byte {:?} at {}", c as char, self.pos)),
            None => Err("unexpected end of input".into()),
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(format!("invalid literal at {}", self.pos))
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut pairs = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Obj(pairs));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return Err(format!("expected key at {}", self.pos));
            }
            let key = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return Err(format!("expected ':' at {}", self.pos));
            }
            self.pos += 1;
            self.ws();
            let value = self.value()?;
            // Duplicate names are ambiguous: two readers can disagree about
            // which value wins. Flow refuses them instead of picking one.
            if pairs.iter().any(|(k, _)| k == &key) {
                return Err(format!("duplicate object key {key:?} at {}", self.pos));
            }
            pairs.push((key, value));
            self.ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or '}}' at {}", self.pos)),
            }
        }
        Ok(Json::Obj(pairs))
    }

    fn array(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            self.ws();
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or ']' at {}", self.pos)),
            }
        }
        Ok(Json::Arr(items))
    }

    fn string(&mut self) -> Result<String, String> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            let b = self.peek().ok_or("unterminated string")?;
            self.pos += 1;
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = self.peek().ok_or("unterminated escape")?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = self
                                .bytes
                                .get(self.pos..self.pos + 4)
                                .ok_or("truncated \\u escape")?;
                            let s = std::str::from_utf8(hex).map_err(|_| "bad \\u escape")?;
                            let code = u32::from_str_radix(s, 16).map_err(|_| "bad \\u escape")?;
                            self.pos += 4;
                            out.push(char::from_u32(code).ok_or("invalid code point")?);
                        }
                        other => return Err(format!("bad escape \\{}", other as char)),
                    }
                }
                _ => {
                    // copy the UTF-8 sequence as-is
                    let start = self.pos - 1;
                    let len = utf8_len(b);
                    if start + len > self.bytes.len() {
                        return Err("truncated UTF-8".into());
                    }
                    let s = std::str::from_utf8(&self.bytes[start..start + len])
                        .map_err(|_| "invalid UTF-8")?;
                    out.push_str(s);
                    self.pos = start + len;
                }
            }
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| "bad number")?;
        let value: f64 = s.parse().map_err(|_| format!("invalid number {:?}", s))?;
        // JSON has no infinity and no NaN. Accepting `1e999` would make `7` print
        // `inf`, i.e. output that is not JSON — a machine-contract break found by
        // the seeded fuzzer.
        if !value.is_finite() {
            return Err(format!(
                "number {s:?} is not finite; JSON has no infinity, so it is refused"
            ));
        }
        Ok(Json::Num(value))
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let original = Json::obj(vec![
            ("a", Json::Num(1.0)),
            ("b", Json::str("x\"y")),
            ("c", Json::Arr(vec![Json::Bool(true), Json::Null])),
        ]);
        let text = original.to_string();
        let parsed = parse(&text).unwrap();
        assert_eq!(parsed.to_string(), text);
        assert_eq!(parsed.get("b").and_then(Json::as_str), Some("x\"y"));
    }

    #[test]
    fn rejects_trailing_data() {
        assert!(parse("{\"a\":1} garbage").is_err());
        assert!(parse("{\"a\":").is_err());
    }

    #[test]
    fn rejects_duplicate_keys() {
        // Two readers could disagree about which value wins; a policy is far too
        // important for that ambiguity.
        assert!(parse(r#"{"required":true,"required":false}"#).is_err());
        assert!(parse(r#"{"a":{"b":1,"b":2}}"#).is_err());
        assert!(parse(r#"{"a":1,"b":2}"#).is_ok());
    }
    #[test]
    fn a_non_finite_number_is_refused_not_printed_as_inf() {
        // 1e999 overflows f64 to infinity; JSON cannot represent it, and printing
        // `inf` would produce output that is not JSON.
        assert!(parse("1e999").is_err());
        assert!(parse("-1e999").is_err());
        assert!(parse("1e308").is_ok());
        let value = parse("1e308").unwrap();
        let text = value.to_string();
        assert!(
            parse(&text).is_ok(),
            "printed output must still be JSON: {text}"
        );
    }

    #[test]
    fn an_accessor_never_panics_on_another_shape() {
        // Receipts and journals are read from disk; a field of an unexpected
        // shape must answer "not this shape", never panic and never guess.
        let null = Json::Null;
        let boolean = Json::Bool(false);
        let number = Json::Num(2.0);
        let text = Json::str("text");
        let array = Json::Arr(vec![Json::Num(1.0)]);
        let object = Json::Obj(vec![("k".into(), Json::Bool(true))]);

        assert_eq!(
            (null.as_str(), null.as_bool(), null.as_num(), null.as_arr()),
            (None, None, None, None)
        );
        assert_eq!(null.get("k"), None);

        assert_eq!(boolean.as_bool(), Some(false));
        assert_eq!(
            (boolean.as_str(), boolean.as_num(), boolean.as_arr()),
            (None, None, None)
        );
        assert_eq!(boolean.get("k"), None);

        assert_eq!(number.as_num(), Some(2.0));
        assert_eq!(
            (number.as_str(), number.as_bool(), number.as_arr()),
            (None, None, None)
        );
        assert_eq!(number.get("k"), None);

        assert_eq!(text.as_str(), Some("text"));
        assert_eq!(
            (text.as_bool(), text.as_num(), text.as_arr()),
            (None, None, None)
        );
        assert_eq!(text.get("k"), None);

        assert_eq!(array.as_arr().map(<[Json]>::len), Some(1));
        assert_eq!(
            (array.as_str(), array.as_bool(), array.as_num()),
            (None, None, None)
        );
        assert_eq!(array.get("k"), None);

        assert_eq!(object.get("k").and_then(Json::as_bool), Some(true));
        assert_eq!(
            (
                object.as_str(),
                object.as_bool(),
                object.as_num(),
                object.as_arr()
            ),
            (None, None, None, None)
        );
    }

    #[test]
    fn the_writer_emits_every_shape_as_text_that_parses_back() {
        // An empty container is `{}`/`[]`, not a broken pair of delimiters.
        assert_eq!(Json::Obj(vec![]).to_string(), "{}");
        assert_eq!(Json::Arr(vec![]).to_string(), "[]");
        // Integral numbers keep the integer spelling; anything else keeps its
        // own, because a receipt's numbers are part of what a digest covers.
        assert_eq!(Json::Num(-2.0).to_string(), "-2");
        assert_eq!(Json::Num(1.5).to_string(), "1.5");
        assert_eq!(Json::Num(1.0e16).to_string(), "10000000000000000");
        // A control character has no escaped spelling of its own and must become
        // a `\u` escape; emitting it raw would produce unparsable text.
        assert_eq!(Json::str("a\u{1}b").to_string(), "\"a\\u0001b\"");
        let pretty = Json::obj(vec![("only", Json::Num(1.0))]).to_string_pretty();
        assert!(pretty.contains("\"only\": 1"), "{pretty}");
        for text in [Json::Bool(true).to_string(), Json::Null.to_string(), pretty] {
            assert!(parse(&text).is_ok(), "{text} must parse back");
        }
    }

    #[test]
    fn the_parser_refuses_shapes_json_does_not_allow() {
        // A leading `+` is not a JSON number, even though Rust parses it.
        assert!(parse("+1").is_err());
        assert!(parse("@").is_err());
        // A truncated literal never becomes `true`/`null`.
        assert!(parse("tru").is_err());
        assert!(parse("nul").is_err());
        // A colon and a closing delimiter are required where JSON requires them.
        assert!(parse(r#"{"a" 1}"#).is_err());
        assert!(parse(r#"{"a":1"#).is_err());
        assert!(parse(r#"{"a":1,}"#).is_err());
        // The two empty containers are valid and carry no member.
        assert_eq!(parse("{}").unwrap(), Json::Obj(vec![]));
        assert_eq!(parse("[]").unwrap(), Json::Arr(vec![]));
    }
}
