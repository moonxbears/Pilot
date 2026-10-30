//! Lightweight, zero-dependency JSON parser and serializer for DAP communication.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(entries) => {
                entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
            }
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_f64().map(|n| n as u64)
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn serialize(&self) -> String {
        let mut out = String::with_capacity(128);
        self.write_json(&mut out);
        out
    }

    fn write_json(&self, out: &mut String) {
        match self {
            JsonValue::Null => out.push_str("null"),
            JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 {
                    out.push_str(&format!("{:.0}", n));
                } else {
                    out.push_str(&format!("{}", n));
                }
            }
            JsonValue::String(s) => {
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        _ => out.push(c),
                    }
                }
                out.push('"');
            }
            JsonValue::Array(arr) => {
                out.push('[');
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write_json(out);
                }
                out.push(']');
            }
            JsonValue::Object(entries) => {
                out.push('{');
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('"');
                    out.push_str(k);
                    out.push_str("\":");
                    v.write_json(out);
                }
                out.push('}');
            }
        }
    }
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.serialize())
    }
}

pub struct JsonParser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> JsonParser<'a> {
    pub fn parse(src: &'a str) -> Result<JsonValue, String> {
        let mut parser = Self { src, pos: 0 };
        parser.skip_whitespace();
        let val = parser.parse_value()?;
        parser.skip_whitespace();
        Ok(val)
    }

    fn skip_whitespace(&mut self) {
        let bytes = self.src.as_bytes();
        while self.pos < bytes.len() {
            match bytes[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.as_bytes().get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b't') | Some(b'f') => self.parse_bool(),
            Some(b'n') => self.parse_null(),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(format!("Unexpected character '{}' at offset {}", c as char, self.pos)),
            None => Err("Unexpected end of JSON input".to_string()),
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.advance(); // skip opening quote
        let mut out = String::new();
        while let Some(b) = self.advance() {
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    match self.advance() {
                        Some(b'"') => out.push('"'),
                        Some(b'\\') => out.push('\\'),
                        Some(b'/') => out.push('/'),
                        Some(b'b') => out.push('\x08'),
                        Some(b'f') => out.push('\x0C'),
                        Some(b'n') => out.push('\n'),
                        Some(b'r') => out.push('\r'),
                        Some(b't') => out.push('\t'),
                        Some(b'u') => {
                            let mut code = 0u32;
                            for _ in 0..4 {
                                let h = self.advance().ok_or("Incomplete unicode escape")?;
                                let val = match h {
                                    b'0'..=b'9' => (h - b'0') as u32,
                                    b'a'..=b'f' => (h - b'a' + 10) as u32,
                                    b'A'..=b'F' => (h - b'A' + 10) as u32,
                                    _ => return Err("Invalid hex digit in unicode escape".to_string()),
                                };
                                code = (code << 4) | val;
                            }
                            if let Some(ch) = char::from_u32(code) {
                                out.push(ch);
                            }
                        }
                        _ => return Err("Invalid escape sequence".to_string()),
                    }
                }
                _ => out.push(b as char),
            }
        }
        Err("Unterminated string literal".to_string())
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.advance(); // skip '{'
        self.skip_whitespace();
        let mut entries = Vec::new();

        if let Some(b'}') = self.peek() {
            self.advance();
            return Ok(JsonValue::Object(entries));
        }

        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(format!("Expected object key string at offset {}", self.pos));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.advance() != Some(b':') {
                return Err(format!("Expected ':' after object key at offset {}", self.pos));
            }
            let val = self.parse_value()?;
            entries.push((key, val));

            self.skip_whitespace();
            match self.advance() {
                Some(b',') => {
                    self.skip_whitespace();
                    if self.peek() == Some(b'}') {
                        self.advance();
                        break;
                    }
                }
                Some(b'}') => break,
                _ => return Err(format!("Expected ',' or '}}' at offset {}", self.pos)),
            }
        }

        Ok(JsonValue::Object(entries))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.advance(); // skip '['
        self.skip_whitespace();
        let mut items = Vec::new();

        if let Some(b']') = self.peek() {
            self.advance();
            return Ok(JsonValue::Array(items));
        }

        loop {
            let item = self.parse_value()?;
            items.push(item);
            self.skip_whitespace();
            match self.advance() {
                Some(b',') => {
                    self.skip_whitespace();
                    if self.peek() == Some(b']') {
                        self.advance();
                        break;
                    }
                }
                Some(b']') => break,
                _ => return Err(format!("Expected ',' or ']' at offset {}", self.pos)),
            }
        }

        Ok(JsonValue::Array(items))
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.src[self.pos..].starts_with("true") {
            self.pos += 4;
            Ok(JsonValue::Bool(true))
        } else if self.src[self.pos..].starts_with("false") {
            self.pos += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err(format!("Invalid boolean at offset {}", self.pos))
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.src[self.pos..].starts_with("null") {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err(format!("Invalid null literal at offset {}", self.pos))
        }
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;

        if self.peek() == Some(b'-') {
            self.advance();
        }

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        if self.peek() == Some(b'.') {
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        if let Some(c) = self.peek() {
            if c == b'e' || c == b'E' {
                self.advance();
                if let Some(sign) = self.peek() {
                    if sign == b'+' || sign == b'-' {
                        self.advance();
                    }
                }
                while let Some(d) = self.peek() {
                    if d.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        let slice = &self.src[start..self.pos];
        slice.parse::<f64>()
            .map(JsonValue::Number)
            .map_err(|e| format!("Invalid number '{}': {}", slice, e))
    }
}
