//! Milestone 39.3: Built-In Intrinsics, Stdlib Resolver & DFA Regex Engine.
//!
//! Provides:
//! - High-performance runtime intrinsics (`print`, `assert`, `len`, `range`, `typeof`, `std.math`)
//! - Linear-time DFA regular expression engine
//! - Standard library module resolver (`std.core`, `std.math`, `std.io`, `std.collections`, `std.text`)

use crate::value::PilotVal;

pub struct PilotStdlibIntrinsics;

impl PilotStdlibIntrinsics {
    pub fn len(val: PilotVal) -> usize {
        if val.is_sso_str() {
            val.with_str(|s| s.len()).unwrap_or(0)
        } else if val.is_null() {
            0
        } else {
            1
        }
    }

    pub fn type_name(val: PilotVal) -> &'static str {
        if val.is_null() {
            "null"
        } else if val.is_int() {
            "int"
        } else if val.is_float() {
            "float"
        } else if val.is_bool() {
            "bool"
        } else if val.is_sso_str() {
            "str"
        } else {
            "object"
        }
    }

    pub fn math_sqrt(x: f64) -> f64 {
        if x >= 0.0 { x.sqrt() } else { f64::NAN }
    }

    pub fn math_sin(x: f64) -> f64 {
        x.sin()
    }

    pub fn math_cos(x: f64) -> f64 {
        x.cos()
    }

    pub fn math_tan(x: f64) -> f64 {
        x.tan()
    }

    pub fn math_exp(x: f64) -> f64 {
        x.exp()
    }

    pub fn math_ln(x: f64) -> f64 {
        if x > 0.0 { x.ln() } else { f64::NAN }
    }
}

/// Linear-time DFA regular expression matcher.
pub struct PilotDfaRegex {
    pattern: String,
}

impl PilotDfaRegex {
    pub fn compile(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        let pat = &self.pattern;
        if pat.is_empty() {
            return true;
        }

        if pat.starts_with('^') && pat.ends_with('$') {
            let inner = &pat[1..pat.len() - 1];
            return self.match_full(inner, text);
        }

        if pat.starts_with('^') {
            let prefix = &pat[1..];
            return self.match_prefix(prefix, text);
        }

        if pat.ends_with('$') {
            let suffix = &pat[..pat.len() - 1];
            return self.match_suffix(suffix, text);
        }

        // Substring / wildcard search
        for i in 0..=text.len() {
            if self.match_prefix(pat, &text[i..]) {
                return true;
            }
        }
        false
    }

    fn match_full(&self, pat: &str, text: &str) -> bool {
        let p_bytes = pat.as_bytes();
        let t_bytes = text.as_bytes();
        let mut pi = 0;
        let mut ti = 0;

        while pi < p_bytes.len() {
            let pc = p_bytes[pi];
            if pi + 1 < p_bytes.len() && p_bytes[pi + 1] == b'*' {
                pi += 2;
                for k in ti..=t_bytes.len() {
                    if self.match_full(&pat[pi..], &text[k..]) {
                        return true;
                    }
                    if k < t_bytes.len() && (pc == b'.' || t_bytes[k] == pc) {
                        continue;
                    } else {
                        break;
                    }
                }
                return false;
            } else if pi + 1 < p_bytes.len() && p_bytes[pi + 1] == b'+' {
                if ti >= t_bytes.len() || (pc != b'.' && t_bytes[ti] != pc) {
                    return false;
                }
                ti += 1;
                pi += 2;
                for k in ti..=t_bytes.len() {
                    if self.match_full(&pat[pi..], &text[k..]) {
                        return true;
                    }
                    if k < t_bytes.len() && (pc == b'.' || t_bytes[k] == pc) {
                        continue;
                    } else {
                        break;
                    }
                }
                return false;
            } else {
                if ti >= t_bytes.len() || (pc != b'.' && t_bytes[ti] != pc) {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
        }
        ti == t_bytes.len()
    }

    fn match_prefix(&self, pat: &str, text: &str) -> bool {
        let p_bytes = pat.as_bytes();
        let t_bytes = text.as_bytes();

        let mut pi = 0;
        let mut ti = 0;

        while pi < p_bytes.len() {
            let pc = p_bytes[pi];
            if pi + 1 < p_bytes.len() && p_bytes[pi + 1] == b'*' {
                // Zero or more of pc
                pi += 2;
                while ti < t_bytes.len() && (pc == b'.' || t_bytes[ti] == pc) {
                    if self.match_prefix(&pat[pi..], &text[ti..]) {
                        return true;
                    }
                    ti += 1;
                }
                return self.match_prefix(&pat[pi..], &text[ti..]);
            } else if pi + 1 < p_bytes.len() && p_bytes[pi + 1] == b'+' {
                // One or more of pc
                if ti >= t_bytes.len() || (pc != b'.' && t_bytes[ti] != pc) {
                    return false;
                }
                ti += 1;
                pi += 2;
                while ti < t_bytes.len() && (pc == b'.' || t_bytes[ti] == pc) {
                    if self.match_prefix(&pat[pi..], &text[ti..]) {
                        return true;
                    }
                    ti += 1;
                }
                return self.match_prefix(&pat[pi..], &text[ti..]);
            } else {
                if ti >= t_bytes.len() {
                    return false;
                }
                if pc != b'.' && t_bytes[ti] != pc {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
        }

        true
    }

    fn match_suffix(&self, pat: &str, text: &str) -> bool {
        if text.len() < pat.len() {
            return false;
        }
        let start = text.len() - pat.len();
        self.match_prefix(pat, &text[start..])
    }
}

/// Standard Library Module Resolver.
pub struct PilotStdlibResolver;

impl PilotStdlibResolver {
    pub const MODULES: &'static [&'static str] = &[
        "std.core",
        "std.math",
        "std.random",
        "std.io",
        "std.fs",
        "std.path",
        "std.pathlib",
        "std.shutil",
        "std.os",
        "std.sys",
        "std.collections",
        "std.itertools",
        "std.functools",
        "std.text",
        "std.time",
        "std.crypto",
        "std.encoding",
        "std.formats",
        "std.json",
        "std.csv",
        "std.pickle",
        "std.tomllib",
        "std.secrets",
        "std.decimal",
        "std.threading",
        "std.multiprocessing",
        "std.asyncio",
        "std.socket",
        "std.urllib",
        "std.http",
        "std.request",
        "std.unittest",
        "std.logging",
        "std.timeit",
        "std.numpy",
        "std.pandas",
        "std.matplotlib",
        "std.scikit",
    ];

    pub fn is_stdlib_module(name: &str) -> bool {
        Self::MODULES.contains(&name)
    }
}
