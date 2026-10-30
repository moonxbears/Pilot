//! Milestone 42.2: Universal Bidirectional Polyglot Type Bridging, Variable Injection & Resumable Yield-Bridges.

use std::collections::HashMap;
use std::io::BufRead;
use pilot_rt::value::{PilotVal, TAG_BOOL, TAG_FLOAT, TAG_HEAP_STR, TAG_INT, TAG_NULL, TAG_SSO_STR, TAG_UINT};

/// Universally representable polyglot value for cross-language marshaling.
#[derive(Debug, Clone, PartialEq)]
pub enum PolyglotVal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<PolyglotVal>),
    Map(HashMap<String, PolyglotVal>),
}

impl PolyglotVal {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(n) => Some(*n),
            Self::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[PolyglotVal]> {
        match self {
            Self::List(l) => Some(l.as_slice()),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<String, PolyglotVal>> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn get_field(&self, key: &str) -> Option<&PolyglotVal> {
        match self {
            Self::Map(m) => m.get(key),
            _ => None,
        }
    }

    pub fn get_index(&self, idx: usize) -> Option<&PolyglotVal> {
        match self {
            Self::List(l) => l.get(idx),
            _ => None,
        }
    }

    /// Convert from native 16-byte `PilotVal` register to `PolyglotVal`.
    pub fn from_pilot_val(val: PilotVal) -> Self {
        match val.tag() {
            TAG_NULL => Self::Null,
            TAG_BOOL => Self::Bool(val.payload != 0),
            TAG_INT => Self::Int(val.payload as i64),
            TAG_UINT => Self::Int(val.payload as i64),
            TAG_FLOAT => Self::Float(f64::from_bits(val.payload)),
            TAG_SSO_STR | TAG_HEAP_STR => {
                val.with_str(|s| Self::String(s.to_string())).unwrap_or(Self::Null)
            }
            _ => Self::String(val.to_string_repr()),
        }
    }

    /// Convert to native 16-byte `PilotVal` register.
    pub fn to_pilot_val(&self) -> PilotVal {
        match self {
            Self::Null => PilotVal::NULL,
            Self::Bool(b) => PilotVal::from_bool(*b),
            Self::Int(n) => PilotVal::from_int(*n),
            Self::Float(f) => PilotVal::from_float(*f),
            Self::String(s) => PilotVal::from_str(s),
            Self::List(_) => PilotVal::from_str(&self.to_json()),
            Self::Map(_) => PilotVal::from_str(&self.to_json()),
        }
    }

    /// Serialize value to JSON representation without external dependencies.
    pub fn to_json(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            Self::Int(n) => n.to_string(),
            Self::Float(f) => {
                let s = f.to_string();
                if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                    format!("{}.0", s)
                } else {
                    s
                }
            }
            Self::String(s) => {
                let mut out = String::with_capacity(s.len() + 2);
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
                out
            }
            Self::List(items) => {
                let parts: Vec<String> = items.iter().map(|item| item.to_json()).collect();
                format!("[{}]", parts.join(","))
            }
            Self::Map(map) => {
                let mut parts = Vec::with_capacity(map.len());
                for (k, v) in map {
                    parts.push(format!("\"{}\":{}", k, v.to_json()));
                }
                format!("{{{}}}", parts.join(","))
            }
        }
    }

    /// Parse value from JSON representation.
    pub fn from_json(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();
        let mut chars = trimmed.chars().peekable();
        parse_json_value(&mut chars)
    }
}

fn skip_whitespace<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn parse_json_value<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Result<PolyglotVal, String> {
    skip_whitespace(chars);
    match chars.peek() {
        Some('n') => {
            let lit: String = chars.take(4).collect();
            if lit == "null" {
                Ok(PolyglotVal::Null)
            } else {
                Err(format!("Expected null, found {}", lit))
            }
        }
        Some('t') => {
            let lit: String = chars.take(4).collect();
            if lit == "true" {
                Ok(PolyglotVal::Bool(true))
            } else {
                Err(format!("Expected true, found {}", lit))
            }
        }
        Some('f') => {
            let lit: String = chars.take(5).collect();
            if lit == "false" {
                Ok(PolyglotVal::Bool(false))
            } else {
                Err(format!("Expected false, found {}", lit))
            }
        }
        Some('"') => {
            chars.next(); // consume opening quote
            let mut s = String::new();
            while let Some(c) = chars.next() {
                if c == '"' {
                    return Ok(PolyglotVal::String(s));
                } else if c == '\\' {
                    if let Some(esc) = chars.next() {
                        match esc {
                            '"' => s.push('"'),
                            '\\' => s.push('\\'),
                            '/' => s.push('/'),
                            'n' => s.push('\n'),
                            'r' => s.push('\r'),
                            't' => s.push('\t'),
                            _ => s.push(esc),
                        }
                    }
                } else {
                    s.push(c);
                }
            }
            Err("Unterminated string".to_string())
        }
        Some('[') => {
            chars.next(); // consume '['
            let mut items = Vec::new();
            skip_whitespace(chars);
            if let Some(&']') = chars.peek() {
                chars.next();
                return Ok(PolyglotVal::List(items));
            }
            loop {
                let val = parse_json_value(chars)?;
                items.push(val);
                skip_whitespace(chars);
                match chars.peek() {
                    Some(',') => {
                        chars.next();
                    }
                    Some(']') => {
                        chars.next();
                        break;
                    }
                    other => return Err(format!("Expected ',' or ']', found {:?}", other)),
                }
            }
            Ok(PolyglotVal::List(items))
        }
        Some('{') => {
            chars.next(); // consume '{'
            let mut map = HashMap::new();
            skip_whitespace(chars);
            if let Some(&'}') = chars.peek() {
                chars.next();
                return Ok(PolyglotVal::Map(map));
            }
            loop {
                skip_whitespace(chars);
                let key = match parse_json_value(chars)? {
                    PolyglotVal::String(k) => k,
                    _ => return Err("Expected string key in object".to_string()),
                };
                skip_whitespace(chars);
                if chars.next() != Some(':') {
                    return Err("Expected ':' after object key".to_string());
                }
                let val = parse_json_value(chars)?;
                map.insert(key, val);
                skip_whitespace(chars);
                match chars.peek() {
                    Some(',') => {
                        chars.next();
                    }
                    Some('}') => {
                        chars.next();
                        break;
                    }
                    other => return Err(format!("Expected ',' or '}}', found {:?}", other)),
                }
            }
            Ok(PolyglotVal::Map(map))
        }
        Some(&c) if c == '-' || c.is_ascii_digit() => {
            let mut num_str = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_ascii_digit() || ch == '-' || ch == '+' || ch == '.' || ch == 'e' || ch == 'E' {
                    num_str.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if num_str.contains('.') || num_str.contains('e') || num_str.contains('E') {
                num_str.parse::<f64>()
                    .map(PolyglotVal::Float)
                    .map_err(|e| format!("Invalid float {}: {}", num_str, e))
            } else {
                num_str.parse::<i64>()
                    .map(PolyglotVal::Int)
                    .map_err(|e| format!("Invalid integer {}: {}", num_str, e))
            }
        }
        other => Err(format!("Unexpected token in JSON: {:?}", other)),
    }
}

/// Execution output result containing return value and any captured variable mutations.
#[derive(Debug, Clone)]
pub struct PolyglotResult {
    pub return_val: PolyglotVal,
    pub captured_vars: HashMap<String, PolyglotVal>,
    pub stdout: String,
}

/// Bridge for variable injection, snippet wrapping, and execution result extraction.
pub struct PolyglotTypeBridge;

impl PolyglotTypeBridge {
    /// Generates preamble source code to inject Pilot variables into foreign language scripts.
    pub fn inject_variables(language: &str, vars: &HashMap<String, PolyglotVal>) -> String {
        let mut preamble = String::new();
        match language.to_ascii_lowercase().as_str() {
            "python" | "py" => {
                for (name, val) in vars {
                    match val {
                        PolyglotVal::Null => preamble.push_str(&format!("{} = None\n", name)),
                        PolyglotVal::Bool(b) => preamble.push_str(&format!("{} = {}\n", name, if *b { "True" } else { "False" })),
                        PolyglotVal::Int(n) => preamble.push_str(&format!("{} = {}\n", name, n)),
                        PolyglotVal::Float(f) => preamble.push_str(&format!("{} = {}\n", name, f)),
                        PolyglotVal::String(_s) => preamble.push_str(&format!("{} = {}\n", name, val.to_json())),
                        PolyglotVal::List(_) | PolyglotVal::Map(_) => {
                            preamble.push_str(&format!("import json\n{} = json.loads({})\n", name, PolyglotVal::String(val.to_json()).to_json()));
                        }
                    }
                }
            }
            "js" | "javascript" | "node" => {
                for (name, val) in vars {
                    preamble.push_str(&format!("let {} = {};\n", name, val.to_json()));
                }
            }
            "lua" => {
                for (name, val) in vars {
                    match val {
                        PolyglotVal::Null => preamble.push_str(&format!("local {} = nil\n", name)),
                        PolyglotVal::Bool(b) => preamble.push_str(&format!("local {} = {}\n", name, b)),
                        PolyglotVal::Int(n) => preamble.push_str(&format!("local {} = {}\n", name, n)),
                        PolyglotVal::Float(f) => preamble.push_str(&format!("local {} = {}\n", name, f)),
                        PolyglotVal::String(_s) => preamble.push_str(&format!("local {} = {}\n", name, val.to_json())),
                        _ => preamble.push_str(&format!("local {} = {}\n", name, val.to_json())),
                    }
                }
            }
            "c" | "cpp" => {
                for (name, val) in vars {
                    match val {
                        PolyglotVal::Int(n) => preamble.push_str(&format!("int64_t {} = {}LL;\n", name, n)),
                        PolyglotVal::Float(f) => preamble.push_str(&format!("double {} = {};\n", name, f)),
                        PolyglotVal::Bool(b) => preamble.push_str(&format!("bool {} = {};\n", name, if *b { "true" } else { "false" })),
                        PolyglotVal::String(_s) => preamble.push_str(&format!("const char* {} = {};\n", name, val.to_json())),
                        _ => {}
                    }
                }
            }
            "rust" | "rs" => {
                for (name, val) in vars {
                    match val {
                        PolyglotVal::Int(n) => preamble.push_str(&format!("let mut {}: i64 = {};\n", name, n)),
                        PolyglotVal::Float(f) => preamble.push_str(&format!("let mut {}: f64 = {};\n", name, f)),
                        PolyglotVal::Bool(b) => preamble.push_str(&format!("let mut {}: bool = {};\n", name, b)),
                        PolyglotVal::String(_s) => preamble.push_str(&format!("let mut {}: &str = {};\n", name, val.to_json())),
                        _ => {}
                    }
                }
            }
            "csharp" | "cs" | "dotnet" => {
                for (name, val) in vars {
                    match val {
                        PolyglotVal::Int(n) => preamble.push_str(&format!("long {} = {}L;\n", name, n)),
                        PolyglotVal::Float(f) => preamble.push_str(&format!("double {} = {};\n", name, f)),
                        PolyglotVal::Bool(b) => preamble.push_str(&format!("bool {} = {};\n", name, if *b { "true" } else { "false" })),
                        PolyglotVal::String(_s) => preamble.push_str(&format!("string {} = {};\n", name, val.to_json())),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        preamble
    }

    /// Dedents a multi-line code block by stripping the common leading whitespace.
    pub fn dedent(code: &str) -> String {
        let lines: Vec<&str> = code.lines().collect();
        let min_indent = lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
            .min()
            .unwrap_or(0);

        lines
            .iter()
            .map(|line| {
                if line.len() >= min_indent {
                    &line[min_indent..]
                } else {
                    line.trim_start()
                }
            })
            .collect::<Vec<&str>>()
            .join("\n")
    }

    /// Wraps foreign code with variable injection and result extraction sentinel epilogues.
    pub fn wrap_script(language: &str, code: &str, vars: &HashMap<String, PolyglotVal>) -> String {
        let dedented = Self::dedent(code);
        let trimmed = dedented.trim();
        let preamble = Self::inject_variables(language, vars);

        match language.to_ascii_lowercase().as_str() {
            "python" | "py" => {
                let mut script = String::new();
                script.push_str("import sys, json\n");
                script.push_str(&preamble);
                script.push_str("\n# --- BEGIN USER CODE ---\n");
                script.push_str(trimmed);
                script.push_str("\n# --- END USER CODE ---\n");

                // Check and print mutated variables
                for name in vars.keys() {
                    script.push_str(&format!(
                        "if '{name}' in locals():\n    print('__PILOT_VAR__:{name}:json:' + json.dumps({name}))\n",
                        name = name
                    ));
                }
                script
            }
            "js" | "javascript" | "node" => {
                let mut script = String::new();
                script.push_str("(function() {\n");
                script.push_str(&preamble);
                script.push_str("\n// --- BEGIN USER CODE ---\n");

                // If user code starts with "return ", capture into a function
                if trimmed.starts_with("return ") {
                    script.push_str(&format!("let __ret = (function() {{ {} }})();\n", trimmed));
                    script.push_str("console.log('__PILOT_RET__:json:' + JSON.stringify(__ret));\n");
                } else {
                    script.push_str(trimmed);
                    script.push('\n');
                }

                // Check and print mutated variables
                for name in vars.keys() {
                    script.push_str(&format!(
                        "if (typeof {name} !== 'undefined') console.log('__PILOT_VAR__:{name}:json:' + JSON.stringify({name}));\n",
                        name = name
                    ));
                }
                script.push_str("})();\n");
                script
            }
            "lua" => {
                let mut script = String::new();
                script.push_str(&preamble);
                script.push_str("\n-- --- BEGIN USER CODE ---\n");
                script.push_str(trimmed);
                script.push_str("\n-- --- END USER CODE ---\n");
                for name in vars.keys() {
                    script.push_str(&format!(
                        "if {name} ~= nil then print('__PILOT_VAR__:{name}:str:' .. tostring({name})) end\n",
                        name = name
                    ));
                }
                script
            }
            "c" => {
                if trimmed.contains("int main") {
                    trimmed.to_string()
                } else {
                    let mut src = String::new();
                    src.push_str("#include <stdio.h>\n#include <stdlib.h>\n#include <stdint.h>\n#include <stdbool.h>\n#include <string.h>\n");
                    src.push_str("int main(int argc, char** argv) {\n");
                    src.push_str(&preamble);
                    if trimmed.starts_with("return ") {
                        let ret_expr = trimmed["return ".len()..].trim().trim_end_matches(';');
                        src.push_str(&format!("int64_t __ret = (int64_t)({});\n", ret_expr));
                        src.push_str("printf(\"__PILOT_RET__:int:%lld\\n\", (long long)__ret);\n");
                        src.push_str("return 0;\n");
                    } else {
                        src.push_str(trimmed);
                        src.push_str("\nreturn 0;\n");
                    }
                    src.push_str("}\n");
                    src
                }
            }
            "cpp" | "c++" => {
                if trimmed.contains("int main") {
                    trimmed.to_string()
                } else {
                    let mut src = String::new();
                    src.push_str("#include <iostream>\n#include <cstdint>\n#include <string>\n#include <vector>\n");
                    src.push_str("int main(int argc, char** argv) {\n");
                    src.push_str(&preamble);
                    if trimmed.starts_with("return ") {
                        let ret_expr = trimmed["return ".len()..].trim().trim_end_matches(';');
                        src.push_str(&format!("int64_t __ret = (int64_t)({});\n", ret_expr));
                        src.push_str("std::cout << \"__PILOT_RET__:int:\" << __ret << std::endl;\n");
                        src.push_str("return 0;\n");
                    } else {
                        src.push_str(trimmed);
                        src.push_str("\nreturn 0;\n");
                    }
                    src.push_str("}\n");
                    src
                }
            }
            "rust" | "rs" => {
                if trimmed.contains("fn main") {
                    trimmed.to_string()
                } else {
                    let mut src = String::new();
                    src.push_str("fn main() {\n");
                    src.push_str(&preamble);
                    if trimmed.starts_with("return ") {
                        let ret_expr = trimmed["return ".len()..].trim().trim_end_matches(';');
                        src.push_str(&format!("let __ret = {};\n", ret_expr));
                        src.push_str("println!(\"__PILOT_RET__:int:{}\", __ret);\n");
                    } else {
                        src.push_str(trimmed);
                        src.push('\n');
                    }
                    src.push_str("}\n");
                    src
                }
            }
            _ => trimmed.to_string(),
        }
    }

    /// Parses execution output, extracting return value and variable mutations.
    pub fn parse_execution_result(
        _language: &str,
        stdout: &str,
        initial_vars: &HashMap<String, PolyglotVal>,
    ) -> PolyglotResult {
        let mut return_val = PolyglotVal::Null;
        let mut captured_vars = initial_vars.clone();
        let mut output_lines = Vec::new();
        let mut has_explicit_return = false;

        for line in stdout.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.starts_with("__PILOT_RET__:") {
                let rest = &line_trimmed["__PILOT_RET__:".len()..];
                return_val = parse_typed_sentinel(rest);
                has_explicit_return = true;
            } else if line_trimmed.starts_with("__PILOT_VAR__:") {
                let rest = &line_trimmed["__PILOT_VAR__:".len()..];
                if let Some((var_name, val)) = parse_var_sentinel(rest) {
                    captured_vars.insert(var_name, val);
                }
            } else {
                output_lines.push(line.to_string());
            }
        }

        // If no explicit __PILOT_RET__ was captured, parse the trailing output line
        if !has_explicit_return && !output_lines.is_empty() {
            let last_line = output_lines.last().unwrap().trim();
            if let Ok(i) = last_line.parse::<i64>() {
                return_val = PolyglotVal::Int(i);
            } else if let Ok(f) = last_line.parse::<f64>() {
                return_val = PolyglotVal::Float(f);
            } else if last_line == "true" {
                return_val = PolyglotVal::Bool(true);
            } else if last_line == "false" {
                return_val = PolyglotVal::Bool(false);
            } else if last_line == "null" || last_line == "None" || last_line == "nil" {
                return_val = PolyglotVal::Null;
            } else if !last_line.is_empty() {
                return_val = PolyglotVal::String(last_line.to_string());
            }
        }

        PolyglotResult {
            return_val,
            captured_vars,
            stdout: output_lines.join("\n"),
        }
    }
}

fn parse_typed_sentinel(s: &str) -> PolyglotVal {
    // format: <type>:<payload>
    if let Some(idx) = s.find(':') {
        let ty = &s[..idx];
        let payload = &s[idx + 1..];
        match ty {
            "null" => PolyglotVal::Null,
            "bool" => PolyglotVal::Bool(payload == "1" || payload == "true"),
            "int" => payload.parse::<i64>().map(PolyglotVal::Int).unwrap_or(PolyglotVal::Null),
            "float" => payload.parse::<f64>().map(PolyglotVal::Float).unwrap_or(PolyglotVal::Null),
            "str" => PolyglotVal::String(payload.to_string()),
            "json" => PolyglotVal::from_json(payload).unwrap_or_else(|_| PolyglotVal::String(payload.to_string())),
            _ => PolyglotVal::String(payload.to_string()),
        }
    } else {
        PolyglotVal::String(s.to_string())
    }
}

fn parse_var_sentinel(s: &str) -> Option<(String, PolyglotVal)> {
    // format: <name>:<type>:<payload>
    let first_colon = s.find(':')?;
    let var_name = s[..first_colon].to_string();
    let val = parse_typed_sentinel(&s[first_colon + 1..]);
    Some((var_name, val))
}

/// Resumable Yield-Bridge stream for foreign generator and coroutine stepping.
pub struct YieldStream<R> {
    reader: R,
}

impl<R: BufRead> YieldStream<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: BufRead> Iterator for YieldStream<R> {
    type Item = PolyglotVal;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        while self.reader.read_line(&mut line).unwrap_or(0) > 0 {
            let trimmed = line.trim();
            if trimmed.starts_with("__PILOT_YIELD__:") {
                let rest = &trimmed["__PILOT_YIELD__:".len()..];
                return Some(parse_typed_sentinel(rest));
            }
            line.clear();
        }
        None
    }
}

/// In-process resumable coroutine state tracker (for callable blocks with yield statements).
pub struct ResumableCoroutine {
    pub name: String,
    steps: Vec<PolyglotVal>,
    current_index: usize,
}

impl ResumableCoroutine {
    pub fn new(name: &str, steps: Vec<PolyglotVal>) -> Self {
        Self {
            name: name.to_string(),
            steps,
            current_index: 0,
        }
    }

    pub fn resume(&mut self, _arg: Option<PolyglotVal>) -> Option<PolyglotVal> {
        if self.current_index < self.steps.len() {
            let item = self.steps[self.current_index].clone();
            self.current_index += 1;
            Some(item)
        } else {
            None
        }
    }

    pub fn is_completed(&self) -> bool {
        self.current_index >= self.steps.len()
    }

    pub fn reset(&mut self) {
        self.current_index = 0;
    }
}
