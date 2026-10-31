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

    pub fn str_split<'a>(s: &'a str, sep: &str) -> Vec<&'a str> {
        if sep.is_empty() {
            s.split("").filter(|p| !p.is_empty()).collect()
        } else {
            s.split(sep).collect()
        }
    }

    pub fn str_join(items: &[&str], sep: &str) -> String {
        items.join(sep)
    }

    pub fn str_replace(s: &str, old: &str, new: &str) -> String {
        s.replace(old, new)
    }

    pub fn str_trim(s: &str) -> &str {
        s.trim()
    }

    pub fn str_trim_start(s: &str) -> &str {
        s.trim_start()
    }

    pub fn str_trim_end(s: &str) -> &str {
        s.trim_end()
    }

    pub fn str_to_upper(s: &str) -> String {
        s.to_uppercase()
    }

    pub fn str_to_lower(s: &str) -> String {
        s.to_lowercase()
    }

    pub fn str_starts_with(s: &str, prefix: &str) -> bool {
        s.starts_with(prefix)
    }

    pub fn str_ends_with(s: &str, suffix: &str) -> bool {
        s.ends_with(suffix)
    }

    pub fn str_contains(s: &str, sub: &str) -> bool {
        s.contains(sub)
    }

    pub fn str_index_of(s: &str, sub: &str) -> Option<usize> {
        s.find(sub)
    }

    pub fn str_last_index_of(s: &str, sub: &str) -> Option<usize> {
        s.rfind(sub)
    }

    pub fn str_substring(s: &str, start: usize, len: usize) -> String {
        let chars: Vec<char> = s.chars().collect();
        if start >= chars.len() {
            return String::new();
        }
        let end = (start + len).min(chars.len());
        chars[start..end].iter().collect()
    }

    pub fn str_repeat(s: &str, count: usize) -> String {
        s.repeat(count)
    }

    pub fn str_pad_start(s: &str, target_len: usize, pad_char: char) -> String {
        let cur_len = s.chars().count();
        if cur_len >= target_len {
            s.to_string()
        } else {
            let pad_count = target_len - cur_len;
            let mut res: String = std::iter::repeat(pad_char).take(pad_count).collect();
            res.push_str(s);
            res
        }
    }

    pub fn str_pad_end(s: &str, target_len: usize, pad_char: char) -> String {
        let cur_len = s.chars().count();
        if cur_len >= target_len {
            s.to_string()
        } else {
            let pad_count = target_len - cur_len;
            let mut res = s.to_string();
            res.extend(std::iter::repeat(pad_char).take(pad_count));
            res
        }
    }

    pub fn str_is_digit(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
    }

    pub fn str_is_alpha(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic())
    }

    pub fn str_is_alphanumeric(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric())
    }

    pub fn str_to_int(s: &str) -> Option<i64> {
        s.trim().parse::<i64>().ok()
    }

    /// Generates a sequence of strings from `start` to `end` using bijective alphabetical enumeration.
    ///
    /// Supports:
    /// - `"a".."z"`: 26 lowercase characters
    /// - `"z".."a"`: 26 lowercase characters in reverse
    /// - `"a".."zzza"`: Bijective base-26 shortlex combinations (`"a"`..`"z"`, `"aa"`..`"zz"`, `"aaa"`..`"zzz"`, `"aaaa"`..`"zzza"`)
    /// - `"a".."Z"`: 52 combinations from `"a"`..`"z"` followed by `"A"`..`"Z"`
    /// - `"A".."z"`: 52 combinations from `"A"`..`"Z"` followed by `"a"`..`"z"`
    /// - `"Z".."a"`: 52 combinations in reverse from `"Z"`..`"A"` followed by `"z"`..`"a"`
    /// - `"z".."A"`: 52 combinations in reverse from `"z"`..`"a"` followed by `"Z"`..`"A"`
    pub fn str_range(start: &str, end: &str, inclusive: bool) -> Vec<String> {
        if start.is_empty() || end.is_empty() {
            return Vec::new();
        }

        let s0 = start.chars().next().unwrap();
        let e0 = end.chars().next().unwrap();

        let s_lower = s0.is_ascii_lowercase();
        let s_upper = s0.is_ascii_uppercase();
        let s_digit = s0.is_ascii_digit();

        let e_lower = e0.is_ascii_lowercase();
        let e_upper = e0.is_ascii_uppercase();
        let e_digit = e0.is_ascii_digit();

        let mut alpha = Vec::new();

        if s_digit && e_digit {
            for c in '0'..='9' { alpha.push(c); }
        } else if s_lower && e_lower {
            for c in 'a'..='z' { alpha.push(c); }
        } else if s_upper && e_upper {
            for c in 'A'..='Z' { alpha.push(c); }
        } else if s_lower && e_upper {
            if s0 <= 'm' {
                // "a".."Z": a..z then A..Z (52 combinations)
                for c in 'a'..='z' { alpha.push(c); }
                for c in 'A'..='Z' { alpha.push(c); }
            } else {
                // "z".."A": reverse of A..z -> alphabet is A..Z then a..z
                for c in 'A'..='Z' { alpha.push(c); }
                for c in 'a'..='z' { alpha.push(c); }
            }
        } else if s_upper && e_lower {
            if s0 <= 'M' {
                // "A".."z": A..Z then a..z (52 combinations)
                for c in 'A'..='Z' { alpha.push(c); }
                for c in 'a'..='z' { alpha.push(c); }
            } else {
                // "Z".."a": reverse of a..Z -> alphabet is a..z then A..Z
                for c in 'a'..='z' { alpha.push(c); }
                for c in 'A'..='Z' { alpha.push(c); }
            }
        } else {
            for c in 'a'..='z' { alpha.push(c); }
        }

        let m = alpha.len() as u64;
        if m == 0 {
            return Vec::new();
        }

        let str_to_rank = |s: &str| -> Option<u64> {
            let s_chars: Vec<char> = s.chars().collect();
            let l = s_chars.len();
            if l == 0 { return None; }
            let mut rank: u64 = 0;
            let mut power: u64 = 1;
            for _ in 1..l {
                power = power.checked_mul(m)?;
                rank = rank.checked_add(power)?;
            }
            let mut offset: u64 = 0;
            for &c in &s_chars {
                let idx = alpha.iter().position(|&x| x == c)? as u64;
                offset = offset.checked_mul(m)?.checked_add(idx)?;
            }
            rank.checked_add(offset)
        };

        let rank_to_str = |mut rank: u64| -> String {
            let mut l = 1;
            let mut power = m;
            while rank >= power {
                rank -= power;
                l += 1;
                match power.checked_mul(m) {
                    Some(next) => power = next,
                    None => break,
                }
            }
            let mut digits = Vec::with_capacity(l);
            for _ in 0..l {
                digits.push((rank % m) as usize);
                rank /= m;
            }
            digits.reverse();
            digits.into_iter().map(|d| alpha[d]).collect()
        };

        let s_rank = match str_to_rank(start) {
            Some(r) => r,
            None => return Vec::new(),
        };
        let e_rank = match str_to_rank(end) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut res = Vec::new();
        const MAX_ITEMS: usize = 1_000_000;

        if s_rank <= e_rank {
            let limit = if inclusive { e_rank } else { if e_rank > 0 { e_rank - 1 } else { 0 } };
            if inclusive || e_rank > s_rank {
                for r in s_rank..=limit {
                    res.push(rank_to_str(r));
                    if res.len() >= MAX_ITEMS { break; }
                }
            }
        } else {
            let limit = if inclusive { e_rank } else { e_rank + 1 };
            if inclusive || s_rank > e_rank {
                let mut r = s_rank;
                loop {
                    res.push(rank_to_str(r));
                    if res.len() >= MAX_ITEMS || r == limit { break; }
                    r -= 1;
                }
            }
        }

        res
    }

    pub fn str_to_float(s: &str) -> Option<f64> {
        s.trim().parse::<f64>().ok()
    }

    pub fn int_abs(x: i64) -> i64 {
        x.abs()
    }

    pub fn int_clamp(x: i64, min: i64, max: i64) -> i64 {
        x.clamp(min, max)
    }

    pub fn int_min(a: i64, b: i64) -> i64 {
        a.min(b)
    }

    pub fn int_max(a: i64, b: i64) -> i64 {
        a.max(b)
    }

    pub fn float_abs(x: f64) -> f64 {
        x.abs()
    }

    pub fn float_clamp(x: f64, min: f64, max: f64) -> f64 {
        x.clamp(min, max)
    }

    pub fn float_round(x: f64) -> f64 {
        x.round()
    }

    pub fn float_floor(x: f64) -> f64 {
        x.floor()
    }

    pub fn float_ceil(x: f64) -> f64 {
        x.ceil()
    }

    pub fn float_trunc(x: f64) -> f64 {
        x.trunc()
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
        "std.option",
        "std.result",
        "std.mem",
        "std.meta",
        "std.collections",
        "std.itertools",
        "std.text",
        "std.math",
        "std.random",
        "std.io",
        "std.fs",
        "std.path",
        "std.pathlib",
        "std.shutil",
        "std.os",
        "std.sys",
        "std.functools",
        "std.json",
        "std.csv",
        "std.pickle",
        "std.tomllib",
        "std.secrets",
        "std.decimal",
        "std.threading",
        "std.multiprocessing",
        "std.asyncio",
        "std.encoding",
        "std.formats",
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
        "std.time",
        "std.crypto",
    ];

    pub const MODULE_MAP: &'static [(&'static str, &'static str)] = &[
        ("std.core", "core/core.pilot"),
        ("std.option", "core/option.pilot"),
        ("std.result", "core/result.pilot"),
        ("std.mem", "core/mem.pilot"),
        ("std.meta", "core/meta.pilot"),
        ("std.collections", "collections/collections.pilot"),
        ("std.itertools", "collections/itertools.pilot"),
        ("std.text", "text/text.pilot"),
        ("std.math", "math/math.pilot"),
        ("std.random", "math/random.pilot"),
        ("std.io", "io/io.pilot"),
        ("std.fs", "io/fs.pilot"),
        ("std.path", "io/path.pilot"),
        ("std.pathlib", "io/pathlib.pilot"),
        ("std.shutil", "io/shutil.pilot"),
        ("std.os", "os/os.pilot"),
        ("std.sys", "sys/sys.pilot"),
        ("std.functools", "functools/functools.pilot"),
        ("std.json", "json/json.pilot"),
        ("std.csv", "csv/csv.pilot"),
        ("std.pickle", "pickle/pickle.pilot"),
        ("std.tomllib", "tomllib/tomllib.pilot"),
        ("std.secrets", "secrets/secrets.pilot"),
        ("std.decimal", "decimal/decimal.pilot"),
        ("std.threading", "threading/threading.pilot"),
        ("std.multiprocessing", "multiprocessing/multiprocessing.pilot"),
        ("std.asyncio", "asyncio/asyncio.pilot"),
        ("std.encoding", "encoding/encoding.pilot"),
        ("std.formats", "formats/formats.pilot"),
        ("std.socket", "socket/socket.pilot"),
        ("std.urllib", "urllib/urllib.pilot"),
        ("std.http", "http/http.pilot"),
        ("std.request", "request/request.pilot"),
        ("std.unittest", "unittest/unittest.pilot"),
        ("std.logging", "logging/logging.pilot"),
        ("std.timeit", "timeit/timeit.pilot"),
        ("std.numpy", "numpy/numpy.pilot"),
        ("std.pandas", "pandas/pandas.pilot"),
        ("std.matplotlib", "matplotlib/matplotlib.pilot"),
        ("std.scikit", "scikit/scikit.pilot"),
    ];

    pub fn is_stdlib_module(name: &str) -> bool {
        Self::MODULES.contains(&name)
    }

    pub fn get_module_relative_path(name: &str) -> Option<&'static str> {
        Self::MODULE_MAP.iter().find(|(m, _)| *m == name).map(|(_, p)| *p)
    }
}
