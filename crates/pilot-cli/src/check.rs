//! Running the self-hosted type checker (`selfhost/check/`) over a program.
//!
//! The checker is Pilot. It needs the whole linked program as the parser
//! produced it: the program's files in link order, then the core
//! declarations the CLI names (`cli_checker_decls()` in
//! selfhost/cli/commands.pilot), which give the types of literals, the
//! members of the primitives and the root interfaces. This module links those
//! files into one arena and serves the checker a faithful snapshot of it
//! through the host operation `chk_snapshot`:
//!
//! | index | content |
//! | --- | --- |
//! | 0 | nodes, `NODE_COLUMNS` integers each (see `node_row`) |
//! | 1 | the extra-children list |
//! | 2 | symbol strings, indexed by symbol id |
//! | 3 | type parameters, `[owner node, name symbol]` each |
//! | 4 | base types, `[owner node, type node]` each |
//! | 5 | attributes, `[target node, name symbol, [argument nodes]]` each |
//! | 6 | files, `[path, is_user, base offset, source text]` each, in unit order |
//! | 7 | `for` clauses, `[iterable node, then node]` each |
//! | 8 | generic bounds, `[owner node, parameter symbol, bound type node]` each |
//! | 9 | splice modifiers, `[splice node, modifier sub-slot, member symbol, alias symbol]` each |
//! | 10 | the enumeration protocol `for` uses: `[get, move next, current]` member names, or empty |
//! | 11 | properties, `[field node, number of accessor bodies]` each |
//! | 12 | the receiver words: `[receiver, accessor backing storage, setter value]` (`bind_receiver_ident`), empty strings when unbound |
//! | 13 | declarations, `[node, is partial, is static, visibility tier or -1 when none is written, is package declaration]` each |
//!
//! Absent node references are `-1`. The checker returns its diagnostics as
//! `[code, message, start, end, severity]` with program-wide offsets; only
//! those inside the user's own files (not the toolchain's `stdlib/`) are kept.

use crate::link::LinkedSource;
use crate::run::{self, Diag};
use crate::tool;
use pilot_ast::{
    AstArena, NodeId, NodeKind, DECL_FLAG_PARTIAL, DECL_FLAG_PKG, DECL_FLAG_STATIC,
    DECL_FLAG_VIS_WRITTEN, DECL_VIS_MASK,
};
use pilot_span::ByteSpan;
use pilot_vm::HostValue;
use std::fs;
use std::path::{Path, PathBuf};

/// Integers per node in the snapshot: kind, lhs, rhs, flags, span start,
/// span end, type annotation, literal value, unit index.
const NODE_COLUMNS: usize = 9;

fn reference(raw: u32) -> i64 {
    if raw == u32::MAX {
        -1
    } else {
        i64::from(raw)
    }
}

/// The value a literal node carries: the integer, the float's bits, or the
/// boolean. Zero for other nodes.
fn literal_value(arena: &AstArena, kind: NodeKind, lhs: u32) -> i64 {
    match kind {
        NodeKind::IntLit => arena.i64_pool.get(lhs as usize).copied().unwrap_or(0),
        NodeKind::FloatLit => arena
            .f64_pool
            .get(lhs as usize)
            .map_or(0, |f| f.to_bits() as i64),
        NodeKind::BoolLit => i64::from(lhs != 0),
        _ => 0,
    }
}

/// The unit (file) index of every node, from the arena's unit table.
fn node_units(arena: &AstArena) -> Vec<i64> {
    let mut out = vec![-1; arena.nodes.len()];
    let mut starts: Vec<(usize, i64)> = arena
        .units
        .iter()
        .enumerate()
        .map(|(i, u)| (u.first_node as usize, i as i64))
        .collect();
    starts.sort();
    for (k, &(first, unit)) in starts.iter().enumerate() {
        let end = starts.get(k + 1).map_or(arena.nodes.len(), |s| s.0);
        for slot in out.iter_mut().take(end).skip(first) {
            *slot = unit;
        }
    }
    out
}

fn ints(values: impl IntoIterator<Item = i64>) -> HostValue {
    HostValue::List(values.into_iter().map(HostValue::Int).collect())
}

/// Whether nodes of `kind` declare something that carries modifiers: the
/// declaration kinds, and `let` / `var` bindings.
fn is_declaration(kind: NodeKind) -> bool {
    (kind as u16) < (NodeKind::LetStmt as u16)
        || matches!(kind, NodeKind::LetStmt | NodeKind::VarStmt)
}

/// The snapshot of the linked program described in the module documentation.
fn snapshot(linked: &LinkedSource, files: &[(String, bool)]) -> HostValue {
    let arena = &linked.arena;
    let units = node_units(arena);
    let mut nodes = Vec::with_capacity(arena.nodes.len() * NODE_COLUMNS);
    for (i, n) in arena.nodes.iter().enumerate() {
        let id = NodeId(i as u32);
        let span = arena.span(id);
        let kind = n.node_kind();
        nodes.extend([
            i64::from(n.kind),
            reference(n.lhs),
            reference(n.rhs),
            i64::from(n.flags),
            i64::from(span.start),
            i64::from(span.end),
            arena.type_annot(id).map_or(-1, |t| i64::from(t.0)),
            literal_value(arena, kind, n.lhs),
            units[i],
        ]);
    }
    let extra = arena.extra_children.iter().map(|&c| reference(c));
    let symbols = (0..arena.interner.symbol_count())
        .map(|s| {
            HostValue::Str(
                arena
                    .interner
                    .resolve(pilot_intern::SymbolId(s as u32))
                    .to_string(),
            )
        })
        .collect();
    let pair = |a: i64, b: i64| ints([a, b]);
    let type_params = arena
        .type_params
        .iter()
        .map(|(owner, sym)| pair(i64::from(owner.0), i64::from(sym.0)))
        .collect();
    let bases = arena
        .base_types
        .iter()
        .map(|(owner, ty)| pair(i64::from(owner.0), reference(ty.0)))
        .collect();
    let attrs = arena
        .attrs
        .iter()
        .map(|a| {
            let args = arena
                .children_slice(a.args_start, u32::from(a.args_len))
                .iter()
                .map(|&c| reference(c));
            HostValue::List(vec![
                HostValue::Int(reference(a.target.0)),
                HostValue::Int(i64::from(a.name.0)),
                ints(args),
            ])
        })
        .collect();
    let file_rows = files
        .iter()
        .zip(linked.units())
        .map(|((path, user), (_, text, base))| {
            HostValue::List(vec![
                HostValue::Str(path.clone()),
                HostValue::Bool(*user),
                HostValue::Int(i64::from(base)),
                HostValue::Str(text.to_string()),
            ])
        })
        .collect();
    let for_clauses = arena
        .for_clauses
        .iter()
        .map(|(it, then)| pair(reference(it.0), reference(then.0)))
        .collect();
    let bounds = arena
        .type_param_bounds
        .iter()
        .map(|(owner, sym, ty)| ints([i64::from(owner.0), i64::from(sym.0), reference(ty.0)]))
        .collect();
    let splice_mods = arena
        .splice_mods
        .iter()
        .map(|(splice, sub, member, alias)| {
            ints([i64::from(splice.0), i64::from(*sub), i64::from(member.0), i64::from(alias.0)])
        })
        .collect();
    let accessors = arena
        .property_accessors
        .iter()
        .map(|(field, block)| {
            // A list node keeps its child count in `rhs`.
            let bodies = i64::from(arena.node(*block).rhs);
            ints([i64::from(field.0), bodies])
        })
        .collect();
    let protocol = pilot_parser::bootstrap_tables()
        .enumerator()
        .map(|e| {
            [&e.get_enumerator, &e.move_next, &e.current]
                .into_iter()
                .map(|n| HostValue::Str(n.clone()))
                .collect()
        })
        .unwrap_or_default();
    let modifiers = arena
        .decl_flags
        .iter()
        .enumerate()
        .filter(|(i, _)| is_declaration(arena.nodes[*i].node_kind()))
        .map(|(i, &f)| {
            ints([
                i as i64,
                i64::from(f & DECL_FLAG_PARTIAL != 0),
                i64::from(f & DECL_FLAG_STATIC != 0),
                if f & DECL_FLAG_VIS_WRITTEN != 0 {
                    i64::from(f & DECL_VIS_MASK)
                } else {
                    -1
                },
                i64::from(f & DECL_FLAG_PKG != 0),
            ])
        })
        .collect();
    let tables = pilot_parser::bootstrap_tables();
    let receiver_words = [&tables.receiver_ident, &tables.backing_ident, &tables.setter_value_ident]
        .into_iter()
        .map(|w| HostValue::Str(w.clone()))
        .collect();
    HostValue::List(vec![
        ints(nodes),
        ints(extra),
        HostValue::List(symbols),
        HostValue::List(type_params),
        HostValue::List(bases),
        HostValue::List(attrs),
        HostValue::List(file_rows),
        HostValue::List(for_clauses),
        HostValue::List(bounds),
        HostValue::List(splice_mods),
        HostValue::List(protocol),
        HostValue::List(accessors),
        HostValue::List(receiver_words),
        HostValue::List(modifiers),
    ])
}

/// The files `decls` names, under the toolchain root: each file itself, and
/// each directory's `.pilot` files in name order.
fn decl_files(decls: &[String]) -> Vec<String> {
    let root = PathBuf::from(tool::toolchain_root());
    let mut out = Vec::new();
    for d in decls {
        let path = root.join(d);
        if path.is_dir() {
            let mut files: Vec<PathBuf> = fs::read_dir(&path)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "pilot"))
                .collect();
            files.sort();
            out.extend(files);
        } else {
            out.push(path);
        }
    }
    out.into_iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect()
}

/// Whether `path` lies inside the toolchain's own `stdlib/`.
pub(crate) fn in_stdlib(path: &str) -> bool {
    let stdlib = Path::new(&tool::toolchain_root()).join("stdlib");
    match (fs::canonicalize(path), fs::canonicalize(stdlib)) {
        (Ok(p), Ok(s)) => p.starts_with(s),
        _ => false,
    }
}

/// Runs the checker made of `checker_files` over the program at `path`,
/// linked with the declarations `decls` names, and returns its diagnostics
/// in the user's files. A program that does not link gives none here:
/// `cli_check` reports why it does not.
pub fn check(path: &str, checker_files: &[String], decls: &[String]) -> Vec<Diag> {
    let Some((linked, files, snap)) = program_snapshot(path, decls) else {
        return Vec::new();
    };
    let ops: pilot_vm::HostOpHandler = Box::new(move |op, _| match op {
        "chk_snapshot" => Some(Ok(snap.clone())),
        _ => None,
    });
    let tool_paths: Vec<&str> = checker_files.iter().map(String::as_str).collect();
    let out = tool::eval_to_host(&tool_paths, "check_program();", ops);
    let user: Vec<&str> = files
        .iter()
        .filter(|(_, u)| *u)
        .map(|(p, _)| p.as_str())
        .collect();
    out.as_list()
        .unwrap_or_default()
        .iter()
        .filter_map(|d| {
            let f = d.as_list()?;
            let at = |i: usize| f.get(i).and_then(HostValue::as_int).unwrap_or(0);
            let span = ByteSpan::new(at(2) as u32, at(3) as u32);
            let (file, start, end) = linked.resolve(span);
            if !user.contains(&file.as_str()) {
                return None;
            }
            Some(Diag {
                code: f.first()?.as_str()?.to_string(),
                message: f.get(1)?.as_str()?.to_string(),
                file,
                start,
                end,
                severity: at(4).clamp(1, 4) as u8,
            })
        })
        .collect()
}

/// Runs the tool made of `tool_files`, calling `entry`, with the snapshot of
/// the program at `path` (linked with `decls`) served as `chk_snapshot`.
/// The tool's result is its exit code; a program that does not link gives
/// `EXIT_FAILURE`.
pub fn run_on_program(path: &str, tool_files: &[String], decls: &[String], entry: &str) -> i32 {
    let Some((_, _, snap)) = program_snapshot(path, decls) else {
        return run::EXIT_FAILURE;
    };
    let ops: pilot_vm::HostOpHandler = Box::new(move |op, _| match op {
        "chk_snapshot" => Some(Ok(snap.clone())),
        _ => None,
    });
    let tool_paths: Vec<&str> = tool_files.iter().map(String::as_str).collect();
    tool::eval_cli(&tool_paths, entry, ops)
}

/// The program at `path` linked with the declaration files `decls`: its
/// linked source, its files (`[path, is the user's]`) and its snapshot.
fn program_snapshot(path: &str, decls: &[String]) -> Option<(LinkedSource, Vec<(String, bool)>, HostValue)> {
    let Ok(mut paths) = run::link_plan(path) else {
        return None;
    };
    for p in decl_files(decls) {
        if !paths.contains(&p) {
            paths.push(p);
        }
    }
    let mut sources = Vec::with_capacity(paths.len());
    let mut files = Vec::with_capacity(paths.len());
    for p in paths {
        let Ok(src) = fs::read_to_string(&p) else {
            return None;
        };
        files.push((p.clone(), !in_stdlib(&p)));
        sources.push((p, src));
    }
    let linked = LinkedSource::parse(sources);
    let snap = snapshot(&linked, &files);
    Some((linked, files, snap))
}
