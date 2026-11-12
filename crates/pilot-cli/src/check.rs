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
//! | 6 | files, `[path, is_user, base offset, source text, linked by the program]` each, in unit order; a file that is not linked is a declaration file the checker reads beside the program |
//! | 7 | `for` clauses, `[iterable node, then node]` each |
//! | 8 | generic bounds, `[owner node, parameter symbol, bound type node]` each |
//! | 9 | splice modifiers, `[splice node, modifier sub-slot, member symbol, alias symbol]` each |
//! | 10 | the enumeration protocol `for` uses: `[get, move next, current]` member names, or empty |
//! | 11 | properties, `[field node, number of accessor bodies, accessor block node]` each; the block lists the getters first, their count in its `flags` |
//! | 12 | the receiver words: `[receiver, accessor backing storage, setter value]` (`bind_receiver_ident`), empty strings when unbound |
//! | 13 | declarations, `[node, is partial, is static, visibility tier or -1 when none is written, is package declaration]` each |
//! | 14 | field defaults and explicit enum values, `[field node, value node]` each |
//! | 15 | switch arm guards (`case x if x < 0`), `[arm node, guard node]` each |
//! | 16 | property setters, `[setter body node, parameter node, guard node]` each (`set(v: T) if cond => ..`) |
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
fn snapshot(linked: &LinkedSource, files: &[(String, bool)], program_files: usize) -> HostValue {
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
    // `[owner, parameter, default type or -1]`.
    let type_params = arena
        .type_params
        .iter()
        .map(|&(owner, sym)| {
            let default = arena
                .type_param_defaults
                .iter()
                .find(|&&(o, s, _)| o == owner && s == sym)
                .map_or(-1, |&(_, _, ty)| reference(ty.0));
            ints([i64::from(owner.0), i64::from(sym.0), default])
        })
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
        .enumerate()
        .map(|(i, ((path, user), (_, text, base)))| {
            HostValue::List(vec![
                HostValue::Str(path.clone()),
                HostValue::Bool(*user),
                HostValue::Int(i64::from(base)),
                HostValue::Str(text.to_string()),
                HostValue::Bool(i < program_files),
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
            ints([i64::from(field.0), bodies, i64::from(block.0)])
        })
        .collect();
    let protocol = pilot_parser::bootstrap_tables()
        .enumerator()
        .map(|e| {
            [&e.get_enumerator, &e.move_next, &e.current, &e.close]
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
    let field_defaults = arena
        .field_defaults
        .iter()
        .map(|(field, value)| pair(i64::from(field.0), reference(value.0)))
        .collect();
    // An arm's `flags` count its guard in `arm_guards` from 1.
    let arm_guards = arena
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.node_kind() == NodeKind::MatchArm && n.flags > 0)
        .filter_map(|(i, n)| {
            let guard = arena.arm_guards.get(usize::from(n.flags) - 1)?;
            Some(pair(i as i64, reference(guard.0)))
        })
        .collect();
    let setters = arena
        .property_setters
        .iter()
        .map(|(body, param, guard)| ints(vec![i64::from(body.0), reference(param.0), reference(guard.0)]))
        .collect();
    let tables = pilot_parser::bootstrap_tables();
    // `[type, frame field, constructor, each-item function]`, or empty.
    let generator = tables
        .generator()
        .map(|g| {
            [&g.type_name, &g.frame_field, &g.construct, &g.each_item]
                .into_iter()
                .map(|n| HostValue::Str(n.clone()))
                .collect()
        })
        .unwrap_or_default();
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
        HostValue::List(field_defaults),
        HostValue::List(arm_guards),
        HostValue::List(setters),
        HostValue::List(generator),
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
/// `cli_check` reports why it does not. With `keep_plan`, the same pass also
/// makes the program's run plan and keeps it for `drop_plan`, for a caller
/// that runs the program next.
pub fn check(path: &str, checker_files: &[String], decls: &[String], keep_plan: bool) -> Vec<Diag> {
    let Some((linked, files, snap)) = program_snapshot(path, decls) else {
        return Vec::new();
    };
    let ops: pilot_vm::HostOpHandler = Box::new(move |op, _| match op {
        "chk_snapshot" => Some(Ok(snap.clone())),
        _ => None,
    });
    let tool_paths: Vec<&str> = checker_files.iter().map(String::as_str).collect();
    let out = if keep_plan {
        let both = tool::eval_to_host(&tool_paths, "check_and_plan();", ops);
        let mut parts = both.as_list().unwrap_or_default().to_vec().into_iter();
        let out = parts.next().unwrap_or(HostValue::Null);
        let plan = parts.next().unwrap_or(HostValue::Null);
        KEPT_PLAN.with(|k| *k.borrow_mut() = Some((plan_key(path, checker_files, decls), plan)));
        out
    } else {
        tool::eval_to_host(&tool_paths, "check_program();", ops)
    };
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

/// One row of a drop plan (selfhost/check/drops.pilot), its node named by
/// file and the node's offsets in it.
#[derive(Clone, Debug)]
pub struct PlannedDrop {
    pub action: pilot_vm::lower_drop::DropAction,
    pub file: String,
    pub start: u32,
    pub end: u32,
    pub kind: u16,
    pub a: String,
    pub b: String,
}

/// One probe of the plan (selfhost/check/probes.pilot), its statement named
/// by file and the node's offsets in it.
#[derive(Clone, Debug)]
pub struct PlannedProbe {
    pub file: String,
    pub start: u32,
    pub end: u32,
    pub kind: u16,
    pub probe: pilot_vm::lower_probe::Probe,
}

/// The drop plan of the program at `path`: its rows, and how to drop a value
/// of each type it names (`[type, destructor owner or "", fields, element
/// type, length op, element op, sequence op, payload type, absent op]`, the
/// element four empty for a type that owns no elements, the payload two for
/// one without a payload); and the probes the checker plans around statements.
#[derive(Clone, Debug, Default)]
pub struct CheckedDropPlan {
    pub rows: Vec<PlannedDrop>,
    pub glue: Vec<(String, pilot_vm::lower_drop::DropGlue)>,
    /// `[value type, fields]`: the value-type fields a copy copies too.
    pub copy_glue: Vec<(String, Vec<(String, String)>)>,
    pub probes: Vec<PlannedProbe>,
}

/// The probe in row `[start, end, kind, capture, call, args, at]` of the
/// checker's `run_plan()`, each argument `[code, value]`.
fn planned_probe(linked: &LinkedSource, row: &HostValue) -> Option<PlannedProbe> {
    let f = row.as_list()?;
    let n = |i: usize| f.get(i).and_then(HostValue::as_int).unwrap_or(0);
    let mut args = Vec::new();
    for a in f.get(5)?.as_list()? {
        let pair = a.as_list()?;
        let code = pair.first()?.as_int()?;
        let text = pair.get(1).and_then(HostValue::as_str).unwrap_or("");
        let flag = matches!(pair.get(1), Some(HostValue::Bool(true)));
        args.push(pilot_vm::lower_probe::ProbeArg::from_code(code, text, flag)?);
    }
    let (file, start, end) = linked.resolve(ByteSpan::new(n(0) as u32, n(1) as u32));
    Some(PlannedProbe {
        file,
        start,
        end,
        kind: n(2) as u16,
        probe: pilot_vm::lower_probe::Probe {
            capture: matches!(f.get(3), Some(HostValue::Bool(true))),
            call: f.get(4)?.as_str()?.to_string(),
            args,
            at: pilot_vm::lower_probe::ProbeAt::from_code(n(6))?,
        },
    })
}

thread_local! {
    /// The run plan `check` last computed, with the program, checker and
    /// declarations it was computed for (`plan_key`).
    static KEPT_PLAN: std::cell::RefCell<Option<(String, HostValue)>> = const { std::cell::RefCell::new(None) };
}

/// What a run plan depends on: the program, the checker and the declarations.
fn plan_key(path: &str, checker_files: &[String], decls: &[String]) -> String {
    format!("{path}\0{}\0{}", checker_files.join("\n"), decls.join("\n"))
}

/// Runs the checker's `run_plan()` over the program at `path`: its drop plan
/// and its probes. A program that does not link has no plan. The plan `check`
/// computed for the same program, checker and declarations in this process
/// is used instead of computing it again.
pub fn drop_plan(path: &str, checker_files: &[String], decls: &[String]) -> CheckedDropPlan {
    let Some((linked, _, snap)) = program_snapshot(path, decls) else {
        return CheckedDropPlan::default();
    };
    let key = plan_key(path, checker_files, decls);
    let kept = KEPT_PLAN.with(|k| k.borrow().as_ref().filter(|(k, _)| *k == key).map(|(_, p)| p.clone()));
    let out = kept.unwrap_or_else(|| {
        let ops: pilot_vm::HostOpHandler = Box::new(move |op, _| match op {
            "chk_snapshot" => Some(Ok(snap.clone())),
            _ => None,
        });
        let tool_paths: Vec<&str> = checker_files.iter().map(String::as_str).collect();
        tool::eval_to_host(&tool_paths, "run_plan();", ops)
    });
    let parts = out.as_list().unwrap_or_default();
    let mut plan = CheckedDropPlan::default();
    for r in parts.get(1).and_then(HostValue::as_list).unwrap_or_default() {
        plan.probes.extend(planned_probe(&linked, r));
    }
    for r in parts.first().and_then(HostValue::as_list).unwrap_or_default() {
        let Some(f) = r.as_list() else { continue };
        let s = |i: usize| f.get(i).and_then(HostValue::as_str).unwrap_or("").to_string();
        let n = |i: usize| f.get(i).and_then(HostValue::as_int).unwrap_or(0);
        let Some(action) = pilot_vm::lower_drop::DropAction::from_code(n(0)) else { continue };
        use pilot_vm::lower_drop::DropAction;
        if matches!(action, DropAction::Glue | DropAction::CopyGlue) {
            let raw: Vec<String> = f
                .get(3)
                .and_then(HostValue::as_list)
                .map(|l| l.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let fields = raw.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0].clone(), c[1].clone())).collect();
            if action == DropAction::Glue {
                let elements = (!s(4).is_empty()).then(|| pilot_vm::lower_drop::DropElements {
                    ty: s(4),
                    len_op: s(5),
                    get_op: s(6),
                    seq_op: (!s(7).is_empty()).then(|| s(7)),
                });
                let owner = s(2);
                let payload = (!s(8).is_empty()).then(|| (s(8), s(9)));
                let glue = pilot_vm::lower_drop::DropGlue {
                    destructor_owner: (!owner.is_empty()).then_some(owner),
                    fields,
                    elements,
                    payload,
                };
                plan.glue.push((s(1), glue));
            } else {
                plan.copy_glue.push((s(1), fields));
            }
            continue;
        }
        let (file, start, end) = linked.resolve(ByteSpan::new(n(1) as u32, n(2) as u32));
        plan.rows.push(PlannedDrop {
            action,
            file,
            start,
            end,
            kind: n(3) as u16,
            a: s(4),
            b: s(5),
        });
    }
    plan
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

/// Runs the tool made of `tool_files`, calling `entry`, with the snapshot of
/// the program at `path` served as `chk_snapshot`, like `run_on_program`;
/// gives the value `entry` returns. A program that does not link gives
/// `HostValue::Null`.
pub fn eval_on_program(path: &str, tool_files: &[String], decls: &[String], entry: &str) -> HostValue {
    let Some((_, _, snap)) = program_snapshot(path, decls) else {
        return HostValue::Null;
    };
    let ops: pilot_vm::HostOpHandler = Box::new(move |op, _| match op {
        "chk_snapshot" => Some(Ok(snap.clone())),
        _ => None,
    });
    let tool_paths: Vec<&str> = tool_files.iter().map(String::as_str).collect();
    tool::eval_to_host(&tool_paths, entry, ops)
}

/// The program at `path` linked with the declaration files `decls`: its
/// linked source, its files (`[path, is the user's]`) and its snapshot.
fn program_snapshot(path: &str, decls: &[String]) -> Option<(LinkedSource, Vec<(String, bool)>, HostValue)> {
    let Ok(mut paths) = run::link_plan(path) else {
        return None;
    };
    let program_files = paths.len();
    // The same file can be spelled two ways (`stdlib/..`, `./stdlib/..`);
    // a declaration file the program already links is not linked again.
    let canonical = |p: &str| fs::canonicalize(p).map_or_else(|_| p.to_string(), |c| c.to_string_lossy().into_owned());
    let mut seen: std::collections::HashSet<String> = paths.iter().map(|p| canonical(p)).collect();
    for p in decl_files(decls) {
        if seen.insert(canonical(&p)) {
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
    let snap = snapshot(&linked, &files, program_files);
    Some((linked, files, snap))
}
