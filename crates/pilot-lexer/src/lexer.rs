//! The seed lexer: runs the table Pilot builds from its token declarations
//! (PROP-021 §3). `stdlib/core/lex/toolchain.lex` is written by
//! `pilot lex --write-table` (`stdlib/pilot/lex/seed.pilot`); this file reads
//! it and steps it. Nothing here names a character, a word or a symbol: each
//! family's `TokenKind`, flags and emission come from the table.
//!
//! The steps are those of `stdlib/pilot/lex/build.pilot` (`match_at`,
//! `context_at`, `context_after`) and `stdlib/pilot/lex/modes.pilot`
//! (`lex_modal`): the longest match from the start state of the current
//! context, in the mode on top of the mode stack. A token that enters a mode
//! from the base mode (an f-string's opening) starts one token that runs
//! until the stack is back to the base mode, as the seed parser reads it.

use crate::token::{TokenFlags, TokenKind, TokenTape};
use pilot_span::ByteSpan;

/// The table format this runner reads (`CACHE_FORMAT` in cache.pilot).
const FORMAT: &str = "pilot-lex-cache 4";
/// The start contexts, by their position in `LEX_CONTEXT_KINDS`.
const CTX_OTHER: usize = 0;
const CTX_SPACED: usize = 1;
const CTX_GLUED: usize = 2;
const CTX_START: usize = 3;
const CONTEXTS: usize = 4;
/// The token roles, by their position in cache.pilot's `ROLES`.
const ROLE_OPERAND: u8 = 1;
const ROLE_SPACE: u8 = 2;
const ROLE_TRIVIA: u8 = 3;
/// The emissions, by their position in seed.pilot's `SEED_EMITS`.
const EMIT_SKIP: u8 = 1;
/// No next state.
const DEAD: u16 = u16::MAX;

struct Automaton {
    class_of: [u16; 256],
    classes: usize,
    starts: [u16; CONTEXTS],
    next: Vec<u16>,
    accept: Vec<i32>,
}

#[derive(Clone, Copy)]
struct Action {
    role: u8,
    leaves: bool,
    trails: u32,
    enters: i32,
}

struct Mode {
    automaton: usize,
    base: usize,
    actions: Vec<Action>,
}

#[derive(Clone, Copy)]
struct Family {
    kind: TokenKind,
    flags: TokenFlags,
    emit: u8,
}

/// A table loaded from `toolchain.lex`.
pub struct SeedTable {
    automata: Vec<Automaton>,
    modes: Vec<Mode>,
    families: Vec<Family>,
    eof: TokenKind,
    error: TokenKind,
}

/// The words of a table file, read in order.
struct Reader<'a> {
    src: &'a str,
    at: usize,
}

impl<'a> Reader<'a> {
    fn word(&mut self) -> Result<&'a str, String> {
        let b = self.src.as_bytes();
        while self.at < b.len() && (b[self.at] == b' ' || b[self.at] == b'\n') {
            self.at += 1;
        }
        let from = self.at;
        while self.at < b.len() && b[self.at] != b' ' && b[self.at] != b'\n' {
            self.at += 1;
        }
        if from == self.at {
            return Err("the file ends early".into());
        }
        Ok(&self.src[from..self.at])
    }
    fn expect(&mut self, label: &str) -> Result<(), String> {
        let w = self.word()?;
        if w != label {
            return Err(format!("`{label}` expected, found `{w}`"));
        }
        Ok(())
    }
    fn int(&mut self) -> Result<i64, String> {
        let w = self.word()?;
        w.parse().map_err(|_| format!("`{w}` is not a number"))
    }
    /// A line written by `ints_line`: `label count`, then the values, a run
    /// written `value*times`.
    fn ints(&mut self, label: &str) -> Result<Vec<i64>, String> {
        self.expect(label)?;
        let n = usize::try_from(self.int()?).map_err(|_| format!("`{label}`: bad count"))?;
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            let w = self.word()?;
            let (v, times) = match w.split_once('*') {
                Some((v, t)) => (v, t.parse::<usize>().map_err(|_| format!("`{w}`"))?),
                None => (w, 1),
            };
            let v: i64 = v.parse().map_err(|_| format!("`{w}` is not a number"))?;
            if out.len() + times > n {
                return Err(format!("`{label}` holds more than {n} values"));
            }
            out.resize(out.len() + times, v);
        }
        Ok(out)
    }
    /// A line written by `words_line`: `label count`, then `=name` words.
    fn skip_words(&mut self, label: &str) -> Result<usize, String> {
        self.expect(label)?;
        let n = self.int()?;
        for _ in 0..n {
            self.word()?;
        }
        Ok(n as usize)
    }
}

/// State `v` of an automaton of `states` states.
fn state(v: i64, states: usize) -> Result<u16, String> {
    u16::try_from(v)
        .ok()
        .filter(|&x| x != DEAD && (x as usize) < states)
        .ok_or_else(|| format!("state {v} is out of range"))
}

fn read_automaton(r: &mut Reader) -> Result<Automaton, String> {
    let names = r.skip_words("names")?;
    let class_of_list = r.ints("class_of")?;
    r.expect("size")?;
    let classes = r.int()? as usize;
    let states = r.int()? as usize;
    let starts_list = r.ints("starts")?;
    let next_list = r.ints("next")?;
    let accept_list = r.ints("accept")?;
    if class_of_list.len() != 256
        || starts_list.len() != CONTEXTS
        || next_list.len() != classes * states
        || accept_list.len() != states
    {
        return Err("an automaton's lists do not fit its size".into());
    }
    let mut class_of = [0u16; 256];
    for (i, &c) in class_of_list.iter().enumerate() {
        if c < 0 || c as usize >= classes {
            return Err(format!("byte class {c} is out of range"));
        }
        class_of[i] = c as u16;
    }
    let mut starts = [0u16; CONTEXTS];
    for (i, &s) in starts_list.iter().enumerate() {
        starts[i] = state(s, states)?;
    }
    let mut next = Vec::with_capacity(next_list.len());
    for &s in &next_list {
        next.push(if s < 0 { DEAD } else { state(s, states)? });
    }
    let mut accept = Vec::with_capacity(states);
    for &a in &accept_list {
        if a >= names as i64 {
            return Err(format!("accepted token {a} is out of range"));
        }
        accept.push(a as i32);
    }
    Ok(Automaton {
        class_of,
        classes,
        starts,
        next,
        accept,
    })
}

fn kind_of(v: i64) -> Result<TokenKind, String> {
    u16::try_from(v)
        .ok()
        .and_then(TokenKind::from_u16)
        .ok_or_else(|| format!("{v} is no seed token kind"))
}

impl SeedTable {
    /// Reads a table written by `pilot lex --write-table`.
    pub fn parse(text: &str) -> Result<SeedTable, String> {
        let first = text.lines().next().unwrap_or("");
        if first != FORMAT {
            return Err(format!("the format is `{first}`, not `{FORMAT}`"));
        }
        let mut r = Reader {
            src: text,
            at: first.len(),
        };
        r.expect("key")?;
        let key_len = r.int()? as usize;
        r.at += 1 + key_len;
        if r.at > text.len() {
            return Err("the key runs past the end".into());
        }
        let n_modes = r.skip_words("modes")?;
        let bases = r.ints("bases")?;
        let mode_of = r.ints("mode_of")?;
        if bases.len() != n_modes {
            return Err("`bases` does not list every mode".into());
        }
        let mut automata = Vec::new();
        let mut modes = Vec::with_capacity(n_modes);
        for (i, &base) in bases.iter().enumerate() {
            let enters = r.ints("enters")?;
            r.expect("shares")?;
            let shares = r.int()?;
            let automaton = if shares < 0 {
                automata.push(read_automaton(&mut r)?);
                automata.len() - 1
            } else {
                let j = shares as usize;
                if j >= i {
                    return Err(format!("mode {i} shares mode {j}, which is not before it"));
                }
                modes.get(j).map(|m: &Mode| m.automaton).unwrap_or(0)
            };
            let acts = r.ints("actions")?;
            r.skip_words("enters_mode")?;
            if acts.len() != enters.len() * 3 {
                return Err(format!("mode {i}: `actions` and `enters` disagree"));
            }
            let mut actions = Vec::with_capacity(enters.len());
            for (d, &to) in enters.iter().enumerate() {
                if to >= n_modes as i64 {
                    return Err(format!("mode {i} enters mode {to}, which is not declared"));
                }
                actions.push(Action {
                    role: acts[d * 3] as u8,
                    leaves: acts[d * 3 + 1] != 0,
                    trails: acts[d * 3 + 2].max(0) as u32,
                    enters: to as i32,
                });
            }
            modes.push(Mode {
                automaton,
                base: base as usize,
                actions,
            });
        }
        r.expect("seed_special")?;
        let eof = kind_of(r.int()?)?;
        let error = kind_of(r.int()?)?;
        let kinds = r.ints("seed_kinds")?;
        if kinds.len() != mode_of.len() * 3 {
            return Err("`seed_kinds` does not list every family".into());
        }
        let mut families = Vec::with_capacity(mode_of.len());
        for k in kinds.chunks(3) {
            families.push(Family {
                kind: kind_of(k[0])?,
                flags: TokenFlags(k[1] as u8),
                emit: k[2] as u8,
            });
        }
        for m in &modes {
            if m.base + m.actions.len() > families.len() {
                return Err("a mode's families run past `seed_kinds`".into());
            }
        }
        Ok(SeedTable {
            automata,
            modes,
            families,
            eof,
            error,
        })
    }
}

/// The longest match of `a` at `pos` from context `ctx`: the declaration
/// and the end of its last accepting state, or `None`.
fn match_at(a: &Automaton, bytes: &[u8], pos: usize, ctx: usize) -> Option<(usize, usize)> {
    let mut state = a.starts[ctx] as usize;
    let mut last = None;
    let mut i = pos;
    while i < bytes.len() {
        let nx = a.next[state * a.classes + a.class_of[bytes[i] as usize] as usize];
        if nx == DEAD {
            break;
        }
        state = nx as usize;
        i += 1;
        let acc = a.accept[state];
        if acc >= 0 {
            last = Some((acc as usize, i));
        }
    }
    last
}

/// Tokens pushed onto a tape, with `()`, `[]` and `{}` linked as found.
struct Out {
    tape: TokenTape,
    open: Vec<(TokenKind, u32)>,
}

impl Out {
    fn push(&mut self, kind: TokenKind, flags: TokenFlags, start: usize, end: usize) {
        let idx = self
            .tape
            .push(kind, flags, ByteSpan::new(start as u32, end as u32));
        let opener = match kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                self.open.push((kind, idx));
                return;
            }
            TokenKind::RParen => TokenKind::LParen,
            TokenKind::RBracket => TokenKind::LBracket,
            TokenKind::RBrace => TokenKind::LBrace,
            _ => return,
        };
        if let Some(&(_, at)) = self.open.last().filter(|&&(k, _)| k == opener) {
            self.open.pop();
            self.tape.bracket_links[at as usize] = idx;
            self.tape.bracket_links[idx as usize] = at;
        }
    }
}

/// The token tape of `src`.
pub fn run(t: &SeedTable, src: &str) -> TokenTape {
    let bytes = src.as_bytes();
    let mut out = Out {
        tape: TokenTape::with_capacity((src.len() / 4).max(16)),
        open: Vec::with_capacity(32),
    };
    let mut stack: Vec<usize> = vec![0];
    let (mut spaced, mut operand) = (true, false);
    // The start and family of the token a mode change opened (an f-string).
    let mut merge: Option<(usize, usize)> = None;
    let mut pos = 0;
    while pos < bytes.len() {
        let mode = &t.modes[*stack.last().unwrap_or(&0)];
        let ctx = if pos == 0 {
            CTX_START
        } else if spaced {
            CTX_SPACED
        } else if operand {
            CTX_GLUED
        } else {
            CTX_OTHER
        };
        let found = match_at(&t.automata[mode.automaton], bytes, pos, ctx)
            .map(|(d, end)| (d, end.saturating_sub(mode.actions[d].trails as usize)))
            .filter(|&(_, end)| end > pos);
        let Some((d, end)) = found else {
            if let Some((start, fam)) = merge.take() {
                // An f-string cut short: its token ends here, and the byte
                // is lexed again in the base mode.
                let f = t.families[fam];
                out.push(f.kind, f.flags, start, pos);
                stack.truncate(1);
                continue;
            }
            let width = src
                .get(pos..)
                .and_then(|rest| rest.chars().next())
                .map_or(1, char::len_utf8);
            out.push(t.error, TokenFlags::NONE, pos, pos + width);
            (spaced, operand) = (false, false);
            pos += width;
            continue;
        };
        let act = mode.actions[d];
        let fam = t.families[mode.base + d];
        match act.role {
            ROLE_SPACE => spaced = true,
            ROLE_TRIVIA => {}
            role => (spaced, operand) = (false, role == ROLE_OPERAND),
        }
        if act.leaves && stack.len() > 1 {
            stack.pop();
        }
        if act.enters >= 0 {
            stack.push(act.enters as usize);
        }
        if let Some((start, family)) = merge {
            if fam.kind == t.error {
                let f = t.families[family];
                out.push(f.kind, f.flags, start, pos);
                out.push(t.error, TokenFlags::NONE, pos, end);
                stack.truncate(1);
                merge = None;
            } else if stack.len() == 1 {
                let f = t.families[family];
                out.push(f.kind, f.flags, start, end);
                merge = None;
            }
        } else if stack.len() > 1 {
            merge = Some((pos, mode.base + d));
        } else if fam.emit != EMIT_SKIP {
            out.push(fam.kind, fam.flags, pos, end);
        }
        pos = end;
    }
    if let Some((start, family)) = merge {
        let f = t.families[family];
        out.push(f.kind, f.flags, start, bytes.len());
    }
    out.push(t.eof, TokenFlags::NONE, bytes.len(), bytes.len());
    out.tape
}
