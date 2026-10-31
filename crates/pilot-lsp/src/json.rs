//! Lightweight, zero-dependency JSON parser and serializer for LSP communication.

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
            JsonValue::Array(items) => {
                out.push('[');
                for (idx, item) in items.iter().enumerate() {
                    if idx > 0 {
                        out.push(',');
                    }
                    item.write_json(out);
                }
                out.push(']');
            }
            JsonValue::Object(entries) => {
                out.push('{');
                for (idx, (k, v)) in entries.iter().enumerate() {
                    if idx > 0 {
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

/// Parses a JSON string slice into a `JsonValue`.
pub fn parse_json(input: &str) -> Result<JsonValue, String> {
    let mut chars = input.char_indices().peekable();
    skip_whitespace(&mut chars);
    let val = parse_value(&mut chars, input)?;
    Ok(val)
}

fn skip_whitespace<'a>(chars: &mut std::iter::Peekable<std::str::CharIndices<'a>>) {
    while let Some(&(_, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn parse_value<'a>(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'a>>,
    input: &'a str,
) -> Result<JsonValue, String> {
    skip_whitespace(chars);
    match chars.peek() {
        Some(&(_, '{')) => parse_object(chars, input),
        Some(&(_, '[')) => parse_array(chars, input),
        Some(&(_, '"')) => parse_string(chars, input),
        Some(&(_, 't')) | Some(&(_, 'f')) => parse_bool(chars, input),
        Some(&(_, 'n')) => parse_null(chars, input),
        Some(&(_, c)) if c == '-' || c.is_ascii_digit() => parse_number(chars, input),
        Some(&(pos, c)) => Err(format!("Unexpected character '{}' at pos {}", c, pos)),
        None => Err("Unexpected end of JSON input".to_string()),
    }
}

fn parse_object<'a>(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'a>>,
    input: &'a str,
) -> Result<JsonValue, String> {
    chars.next(); // consume '{'
    skip_whitespace(chars);
    let mut entries = Vec::new();

    if let Some(&(_, '}')) = chars.peek() {
        chars.next();
        return Ok(JsonValue::Object(entries));
    }

    loop {
        skip_whitespace(chars);
        let key = match chars.peek() {
            Some(&(_, '"')) => match parse_string(chars, input)? {
                JsonValue::String(s) => s,
                _ => unreachable!(),
            },
            _ => return Err("Expected string key in object".to_string()),
        };

        skip_whitespace(chars);
        match chars.next() {
            Some((_, ':')) => {}
            _ => return Err("Expected ':' after key in object".to_string()),
        }

        let val = parse_value(chars, input)?;
        entries.push((key, val));

        skip_whitespace(chars);
        match chars.peek() {
            Some(&(_, ',')) => {
                chars.next();
            }
            Some(&(_, '}')) => {
                chars.next();
                break;
            }
            _ => return Err("Expected ',' or '}' in object".to_string()),
        }
    }

    Ok(JsonValue::Object(entries))
}

fn parse_array<'a>(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'a>>,
    input: &'a str,
) -> Result<JsonValue, String> {
    chars.next(); // consume '['
    skip_whitespace(chars);
    let mut items = Vec::new();

    if let Some(&(_, ']')) = chars.peek() {
        chars.next();
        return Ok(JsonValue::Array(items));
    }

    loop {
        let val = parse_value(chars, input)?;
        items.push(val);
        skip_whitespace(chars);

        match chars.peek() {
            Some(&(_, ',')) => {
                chars.next();
            }
            Some(&(_, ']')) => {
                chars.next();
                break;
            }
            _ => return Err("Expected ',' or ']' in array".to_string()),
        }
    }

    Ok(JsonValue::Array(items))
}

fn parse_string<'a>(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'a>>,
    _input: &'a str,
) -> Result<JsonValue, String> {
    chars.next(); // consume '"'
    let mut s = String::new();
    let mut escaped = false;

    while let Some((_, c)) = chars.next() {
        if escaped {
            match c {
                '"' => s.push('"'),
                '\\' => s.push('\\'),
                '/' => s.push('/'),
                'b' => s.push('\x08'),
                'f' => s.push('\x0c'),
                'n' => s.push('\n'),
                'r' => s.push('\r'),
                't' => s.push('\t'),
                _ => s.push(c),
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Ok(JsonValue::String(s));
        } else {
            s.push(c);
        }
    }

    Err("Unclosed string in JSON".to_string())
}

fn parse_number<'a>(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'a>>,
    input: &'a str,
) -> Result<JsonValue, String> {
    let start = chars.peek().map(|&(pos, _)| pos).unwrap();
    let mut end = start;

    while let Some(&(pos, c)) = chars.peek() {
        if c == '-' || c == '+' || c == '.' || c == 'e' || c == 'E' || c.is_ascii_digit() {
            end = pos + c.len_utf8();
            chars.next();
        } else {
            break;
        }
    }

    let num_str = &input[start..end];
    let num: f64 = num_str.parse().map_err(|e| format!("Invalid number '{}': {}", num_str, e))?;
    Ok(JsonValue::Number(num))
}

fn parse_bool<'a>(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'a>>,
    input: &'a str,
) -> Result<JsonValue, String> {
    let start = chars.peek().map(|&(pos, _)| pos).unwrap();
    if input[start..].starts_with("true") {
        for _ in 0..4 { chars.next(); }
        Ok(JsonValue::Bool(true))
    } else if input[start..].starts_with("false") {
        for _ in 0..5 { chars.next(); }
        Ok(JsonValue::Bool(false))
    } else {
        Err("Expected boolean in JSON".to_string())
    }
}

fn parse_null<'a>(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'a>>,
    input: &'a str,
) -> Result<JsonValue, String> {
    let start = chars.peek().map(|&(pos, _)| pos).unwrap();
    if input[start..].starts_with("null") {
        for _ in 0..4 { chars.next(); }
        Ok(JsonValue::Null)
    } else {
        Err("Expected null in JSON".to_string())
    }
}
