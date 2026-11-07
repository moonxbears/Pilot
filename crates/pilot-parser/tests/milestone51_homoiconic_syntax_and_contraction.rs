//! ==============================================================================
//! crates/pilot-parser/tests/milestone51_homoiconic_syntax_and_contraction.rs
//! Milestone 51 — Uncheatable Adversarial Verification Gate
//!
//! Required `.pilot` & Contraction Contracts (all enforced simultaneously):
//!   1. `stdlib/core/syntax/type_construct.pilot`:
//!      - Full Pratt precedence (`Postfix 30 > Intersection 20 > Union 15 > Arrow 10 > Fallible 5`)
//!      - Zero hardcoded composite names in Rust; arbitrary runtime-renamed composites
//!        splice cleanly via `syntax type_construct { ... => quote { ... } }`.
//!   2. `stdlib/core/syntax/{visibility,scope_header,attribute,lifecycle,properties,
//!      module_ext,for,while,until,switch,guard,try_catch,expr_sugar,unit_decl}.pilot`
//!      & `stdlib/{db/sql_syntax,polyglot/polyglot_syntax,realtime/rt_syntax,
//!      asyncio/select_syntax,contracts/contracts_syntax}.pilot`:
//!      - Every syntax module must be `<= 1,500` lines, parse with 0 diagnostics,
//!        and use `nameof(...)` (never raw quoted identifier strings like `"database"`,
//!        `"from"`, `"select"`, `"union"`, `"intersect"`, `"except"`) when binding
//!        valid identifier keywords in `bind_receptor` / `register_pratt_op`.
//!      - Expanded `while`, `until`, and multi-parameter user `syntax` macros must
//!        produce semantically executable `AstArena` control-flow & arithmetic trees
//!        verified against an independent oracle over randomized trials.
//!   3. Canonical Syntax & Contraction Gates:
//!      - Collection-only trailing commas strictly enforced.
//!      - Unchecked postfix `x!` emits `NULL001`.
//!      - Prohibited legacy keywords (`def`, `let mut`, `match`, `class`, `record`,
//!        `elif`, `lambda`) emit diagnostics.
//!      - `crates/pilot-parser/src/lib.rs <= 1,100` LOC, `crates/pilot-ast/src/lib.rs <= 450` LOC,
//!        zero `HashMap`/`HashSet` on `AstArena`, and `selfhost/scope_header.pilot` deleted.
//! ==============================================================================

use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_ast::{
    AstArena, AstNode, BinaryOp, CallQuoteKind, NodeId, NodeKind, DECL_FLAG_PARTIAL,
    DECL_FLAG_PURE, DECL_FLAG_STATIC,
};
use pilot_intern::SymbolId;
use pilot_parser::{parse, parse_and_ingest, ParserTables};

#[path = "../../pilot-analysis/tests/common/anticheat.rs"]
mod anticheat;

macro_rules! syntax_rule {
    ($path:expr, $req:expr) => {
        anticheat::FileAuditRule {
            path: $path,
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[],
            required_code_markers: $req,
            forbidden_code_markers: &[],
            forbidden_string_substrings: &["*void", "0b00 (IValue)"],
        }
    };
}

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 51,
    test_file_rel: "crates/pilot-parser/tests/milestone51_homoiconic_syntax_and_contraction.rs",
    expected_test_count: 4,
    files: &[
        syntax_rule!("stdlib/core/syntax/type_construct.pilot", &["syntax", "type_construct", "quote"]),
        syntax_rule!("stdlib/core/syntax/visibility.pilot", &["register_modifier", "nameof("]),
        syntax_rule!("stdlib/core/syntax/scope_header.pilot", &["nameof("]),
        syntax_rule!("stdlib/core/syntax/attribute.pilot", &["quote"]),
        syntax_rule!("stdlib/core/syntax/lifecycle.pilot", &["nameof("]),
        syntax_rule!("stdlib/core/syntax/properties.pilot", &["nameof("]),
        syntax_rule!("stdlib/core/syntax/module_ext.pilot", &["nameof("]),
        syntax_rule!("stdlib/core/syntax/for.pilot", &["nameof("]),
        syntax_rule!("stdlib/core/syntax/while.pilot", &["syntax", "quote", "loop"]),
        syntax_rule!("stdlib/core/syntax/until.pilot", &["syntax", "quote", "loop"]),
        syntax_rule!("stdlib/core/syntax/switch.pilot", &["nameof("]),
        syntax_rule!("stdlib/core/syntax/guard.pilot", &["nameof("]),
        syntax_rule!("stdlib/core/syntax/try_catch.pilot", &["nameof("]),
        syntax_rule!("stdlib/core/syntax/expr_sugar.pilot", &["__engine"]),
        syntax_rule!("stdlib/core/syntax/unit_decl.pilot", &["unit"]),
        syntax_rule!("stdlib/db/sql_syntax.pilot", &["nameof("]),
        syntax_rule!("stdlib/polyglot/polyglot_syntax.pilot", &["nameof("]),
        syntax_rule!("stdlib/realtime/rt_syntax.pilot", &["nameof("]),
        syntax_rule!("stdlib/asyncio/select_syntax.pilot", &["nameof("]),
        syntax_rule!("stdlib/contracts/contracts_syntax.pilot", &["nameof("]),
    ],
    required_group_pub_fns: &[],
};

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5151_5151_DEAD_BEEF);
        let s = nanos ^ ((std::process::id() as u64).rotate_left(19)) ^ salt;
        Self(if s == 0 { 0x1234_5678_9ABC_DEF0 } else { s })
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        lo + (self.next_u64() % ((hi_inclusive - lo + 1) as u64)) as i64
    }

    fn ident(&mut self, prefix: &str) -> String {
        const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
        let suffix: String = (0..6)
            .map(|_| LOWER[(self.next_u64() as usize) % LOWER.len()] as char)
            .collect();
        format!("{}_{}", prefix, suffix)
    }
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

fn resolve_type_str(arena: &AstArena, id: NodeId) -> String {
    if id.is_none() {
        return "none".to_string();
    }
    let n = arena.node(id);
    match n.node_kind() {
        NodeKind::TypeNamed => {
            let base = arena.interner.resolve(SymbolId(n.lhs)).to_string();
            let args = arena.type_args(id);
            if args.is_empty() {
                base
            } else {
                let inner: Vec<String> = args
                    .iter()
                    .map(|&c| resolve_type_str(arena, NodeId(c)))
                    .collect();
                format!("{}<{}>", base, inner.join(", "))
            }
        }
        NodeKind::TypeTuple => {
            let inner: Vec<String> = arena
                .children_slice(n.lhs, n.rhs)
                .iter()
                .map(|&c| resolve_type_str(arena, NodeId(c)))
                .collect();
            format!("({})", inner.join(", "))
        }
        NodeKind::IntLit => arena
            .i64_pool
            .get(n.lhs as usize)
            .copied()
            .unwrap_or(0)
            .to_string(),
        _ => format!("{:?}", n.node_kind()),
    }
}

enum Flow {
    Val(i64),
    Return(i64),
    Break,
}

fn eval_ast_fn(arena: &AstArena, fn_name: &str, args: &[i64]) -> i64 {
    let fn_id = (0..arena.nodes.len())
        .map(|i| NodeId(i as u32))
        .find(|&id| {
            arena.node(id).node_kind() == NodeKind::FnDecl
                && arena.interner.resolve(SymbolId(arena.node(id).lhs)) == fn_name
        })
        .unwrap_or_else(|| panic!("FnDecl '{}' not found in expanded AST", fn_name));
    let fn_node = arena.node(fn_id);
    let ch = arena.children_slice(fn_node.rhs, fn_node.flags as u32);
    let body = NodeId(ch[0]);
    let mut env = HashMap::new();
    for (idx, &pid) in ch[1..].iter().enumerate() {
        let pname = arena
            .interner
            .resolve(SymbolId(arena.node(NodeId(pid)).lhs))
            .to_string();
        env.insert(pname, args[idx]);
    }
    match eval_node(arena, body, &mut env) {
        Flow::Val(v) | Flow::Return(v) => v,
        Flow::Break => 0,
    }
}

fn eval_node(arena: &AstArena, id: NodeId, env: &mut HashMap<String, i64>) -> Flow {
    if id.is_none() {
        return Flow::Val(0);
    }
    let n = *arena.node(id);
    match n.node_kind() {
        NodeKind::BlockExpr | NodeKind::Program => {
            let mut last = 0;
            for &c in arena.children_slice(n.lhs, n.rhs) {
                match eval_node(arena, NodeId(c), env) {
                    Flow::Val(v) => last = v,
                    other => return other,
                }
            }
            Flow::Val(last)
        }
        NodeKind::IntLit => Flow::Val(arena.i64_pool[n.lhs as usize]),
        NodeKind::BoolLit => Flow::Val(i64::from(n.lhs != 0)),
        NodeKind::IdentExpr => {
            let nm = arena.interner.resolve(SymbolId(n.lhs));
            Flow::Val(*env.get(nm).unwrap_or(&0))
        }
        NodeKind::LetStmt | NodeKind::VarStmt => {
            let nm = arena.interner.resolve(SymbolId(n.lhs)).to_string();
            let v = match eval_node(arena, NodeId(n.rhs), env) {
                Flow::Val(x) => x,
                other => return other,
            };
            env.insert(nm, v);
            Flow::Val(v)
        }
        NodeKind::AssignStmt => {
            let lhs_nm = arena
                .interner
                .resolve(SymbolId(arena.node(NodeId(n.lhs)).lhs))
                .to_string();
            let v = match eval_node(arena, NodeId(n.rhs), env) {
                Flow::Val(x) => x,
                other => return other,
            };
            env.insert(lhs_nm, v);
            Flow::Val(v)
        }
        NodeKind::ExprStmt => eval_node(arena, NodeId(n.lhs), env),
        NodeKind::ReturnStmt => match eval_node(arena, NodeId(n.lhs), env) {
            Flow::Val(v) | Flow::Return(v) => Flow::Return(v),
            Flow::Break => Flow::Return(0),
        },
        NodeKind::BreakStmt => Flow::Break,
        NodeKind::UnaryExpr => {
            let Flow::Val(v) = eval_node(arena, NodeId(n.lhs), env) else {
                return Flow::Val(0);
            };
            Flow::Val(if n.flags == 5 { i64::from(v == 0) } else { -v })
        }
        NodeKind::BinaryExpr => {
            let Flow::Val(l) = eval_node(arena, NodeId(n.lhs), env) else {
                return Flow::Val(0);
            };
            let Flow::Val(r) = eval_node(arena, NodeId(n.rhs), env) else {
                return Flow::Val(0);
            };
            let res = match n.flags {
                x if x == BinaryOp::Add as u16 => l.wrapping_add(r),
                x if x == BinaryOp::Sub as u16 => l.wrapping_sub(r),
                x if x == BinaryOp::Mul as u16 => l.wrapping_mul(r),
                x if x == BinaryOp::Div as u16 => l / r,
                x if x == BinaryOp::Rem as u16 => l % r,
                x if x == BinaryOp::Eq as u16 => i64::from(l == r),
                x if x == BinaryOp::Ne as u16 => i64::from(l != r),
                x if x == BinaryOp::Lt as u16 => i64::from(l < r),
                x if x == BinaryOp::Le as u16 => i64::from(l <= r),
                x if x == BinaryOp::Gt as u16 => i64::from(l > r),
                x if x == BinaryOp::Ge as u16 => i64::from(l >= r),
                _ => 0,
            };
            Flow::Val(res)
        }
        NodeKind::IfExpr => {
            let Flow::Val(c) = eval_node(arena, NodeId(n.lhs), env) else {
                return Flow::Val(0);
            };
            if n.flags == 2 {
                let ch = arena.children_slice(n.rhs, 2);
                eval_node(arena, NodeId(if c != 0 { ch[0] } else { ch[1] }), env)
            } else if c != 0 {
                eval_node(arena, NodeId(n.rhs), env)
            } else {
                Flow::Val(0)
            }
        }
        NodeKind::WhileStmt => {
            let mut guard = 0;
            loop {
                guard += 1;
                assert!(guard < 10_000, "Expanded loop did not terminate");
                let Flow::Val(c) = eval_node(arena, NodeId(n.lhs), env) else {
                    break;
                };
                if c == 0 {
                    break;
                }
                match eval_node(arena, NodeId(n.rhs), env) {
                    Flow::Val(_) => {}
                    Flow::Break => break,
                    Flow::Return(v) => return Flow::Return(v),
                }
            }
            Flow::Val(0)
        }
        _ => Flow::Val(0),
    }
}

#[test]
fn test_51_1_randomized_type_construct_precedence_and_executable_homoiconic_macros() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_51_1_randomized_type_construct_precedence_and_executable_homoiconic_macros",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5101_ABCD);

    // 1. Verify full Pratt type precedence (`Postfix 30 > Intersection 20 > Union 15 > Arrow 10 > Fallible 5`)
    let prec_src = r#"
        let t1: i64[]? & u32[8] | f64[..] -> bool ! str;
        let t2: (*u8)[:] @ 16;
        let t3: @i32[4, 4]^;
        let t4: (i64[]) => bool ! str;
    "#;
    let prec_arena = parse(prec_src);
    assert!(
        prec_arena.diagnostics.is_empty(),
        "Type precedence diagnostics: {:?}",
        prec_arena.diagnostics
    );
    let r_ch = prec_arena.children_slice(
        prec_arena.node(prec_arena.root).lhs,
        prec_arena.node(prec_arena.root).rhs,
    );
    assert_eq!(
        resolve_type_str(&prec_arena, prec_arena.type_annot(NodeId(r_ch[0])).unwrap()),
        "result<fn<union<intersect<option<slice<i64>>, array<u32, 8>>, stride<f64>>, bool>, str>"
    );
    assert_eq!(
        resolve_type_str(&prec_arena, prec_arena.type_annot(NodeId(r_ch[3])).unwrap()),
        "result<fn<slice<i64>, bool>, str>"
    );
    assert_eq!(
        resolve_type_str(&prec_arena, prec_arena.type_annot(NodeId(r_ch[1])).unwrap()),
        "simd<ring<ptr<u8>>, 16>"
    );
    assert_eq!(
        resolve_type_str(&prec_arena, prec_arena.type_annot(NodeId(r_ch[2])).unwrap()),
        "pin<signal<grid<i32, 4, 4>>>"
    );

    // 2. Randomized Universal Rename Litmus on `syntax type_construct` (8 independent trials)
    for _ in 0..8 {
        let w_name = rng.ident("win");
        let a_name = rng.ident("arr");
        let o_name = rng.ident("opt");
        let r_name = rng.ident("out");
        let p_name = rng.ident("ptr");
        let u_name = rng.ident("sum");
        let i_name = rng.ident("both");
        let scalar_a = rng.ident("sc");
        let err_t = rng.ident("err");
        let n_len = rng.range(2, 64);

        let mut custom_tables = ParserTables::new();
        let grammar = format!(
            r#"
            comptime {{
                __engine("bind_receptor", nameof(syntax), 4, 0);
                __engine("bind_receptor", nameof(type_construct), 5, 0);
            }}
            syntax type_construct {{
                (T[])     => quote {{ {w_name}<$T> }};
                (T[N])    => quote {{ {a_name}<$T, $N> }};
                (T?)      => quote {{ {o_name}<$T> }};
                (T ! E)   => quote {{ {r_name}<$T, $E> }};
                (*T)      => quote {{ {p_name}<$T> }};
                (T1 & T2) => quote {{ {i_name}<$T1, $T2> }};
                (T1 | T2) => quote {{ {u_name}<$T1, $T2> }};
            }}
            let target: (*{scalar_a})[{n_len}]? & {scalar_a}[] | {scalar_a} ! {err_t};
            "#
        );
        let a = parse_and_ingest(&grammar, &mut custom_tables);
        assert!(a.diagnostics.is_empty(), "Diagnostics: {:?}", a.diagnostics);
        let ch = a.children_slice(a.node(a.root).lhs, a.node(a.root).rhs);
        let last = NodeId(*ch.last().unwrap());
        let expected = format!(
            "{r_name}<{u_name}<{i_name}<{o_name}<{a_name}<{p_name}<{scalar_a}>, {n_len}>>, {w_name}<{scalar_a}>>, {scalar_a}>, {err_t}>"
        );
        assert_eq!(resolve_type_str(&a, a.type_annot(last).unwrap()), expected);
    }

    // 3. Randomized executable semantic test of `while`, `until`, and multi-param user `syntax` macros
    for _ in 0..8 {
        let mac_clamp = rng.ident("clamp_macro");
        let mac_unless = rng.ident("unless_macro");
        let fn_nm = rng.ident("exec_macro_sem");
        let lo = rng.range(10, 30);
        let hi = rng.range(60, 95);
        let step = rng.range(2, 7);
        let target_n = rng.range(5, 18);

        let src = format!(
            r#"
            pub syntax {{
                {mac_unless}(cond, body) => quote {{
                    if !($cond) {{
                        $body
                    }}
                }};
                {mac_clamp}(val, min_v, max_v) => quote {{
                    if ($val) < ($min_v) {{
                        $min_v
                    }} else if ($val) > ($max_v) {{
                        $max_v
                    }} else {{
                        $val
                    }}
                }};
            }}

            pub fn {fn_nm}(x: i64) -> i64 {{
                var acc = 0;
                var i = 0;
                while i < {target_n} {{
                    acc = acc + {step};
                    i = i + 1;
                }}
                until acc >= {hi} {{
                    acc = acc + 3;
                }}
                {mac_unless} x > 0 {{
                    return {mac_clamp}(x, {lo}, {hi});
                }}
                return {mac_clamp}(acc - x, {lo}, {hi});
            }}
            "#
        );

        let arena = parse(&src);
        assert!(
            arena.diagnostics.is_empty(),
            "Macro semantic trial diagnostics: {:?}",
            arena.diagnostics
        );

        for probe_x in [-50i64, 0, 5, 45, 200] {
            let mut oracle_acc = 0i64;
            let mut oi = 0i64;
            while oi < target_n {
                oracle_acc += step;
                oi += 1;
            }
            while oracle_acc < hi {
                oracle_acc += 3;
            }
            let raw = if !(probe_x > 0) {
                probe_x
            } else {
                oracle_acc - probe_x
            };
            let expected = raw.max(lo).min(hi);
            let actual = eval_ast_fn(&arena, &fn_nm, &[probe_x]);
            assert_eq!(
                actual, expected,
                "Expanded AST semantics mismatch for x={}",
                probe_x
            );
        }
    }
    guard.finish();
}

#[test]
fn test_51_2_surface_syntax_distributive_headers_call_quotes_and_diagnostics() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_51_2_surface_syntax_distributive_headers_call_quotes_and_diagnostics",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5102_DEAD);
    let fn_fast = rng.ident("fast_op");
    let struct_w = format!(
        "Wallet{}",
        rng.ident("x").replace('_', "").to_ascii_uppercase()
    );

    let src = format!(
        r#"
        pub partial static pure:
        fn {fn_fast}(a: i64, b: i64) -> i64 => a + b;

        let List<i32> {{
            arr1 = [1, 2, 3,],
            arr2 = [4, 5, 6,],
        }};

        pub struct {struct_w} : IValue {{
            pub balance: f64 {{
                get => field;
                set => {{ field = value; }}
            }}
            self(init: f64) {{
                .balance = init;
            }}
            ~self() {{}}
            scope self {{
                enter: {{ .balance += 1.0; }}
                exit:  {{ .balance -= 1.0; }}
            }}
            pub op +(rhs: {struct_w}) -> {struct_w} => {struct_w}(.balance + rhs.balance);
        }}

        fn exercise(opt: i64?) -> i64 {{
            guard some v = opt else {{
                return -1;
            }}
            let found = for i in 0 .. 10 {{
                if i == v {{
                    break i;
                }}
            }} then 0;
            let f = (x: i64) => x * 2;
            let q1 = f!(10);
            let q2 = f!!(~v);
            let q3 = f!&(10);
            let q4 = f!^(10);
            let q5 = f!@(10);
            let q6 = f!?(10);
            return switch found {{
                case 0: 100,
                default: found,
            }};
        }}
        "#
    );

    let arena = parse(&src);
    assert!(arena.diagnostics.is_empty(), "Got: {:?}", arena.diagnostics);

    let fn_id = (0..arena.nodes.len())
        .map(|i| NodeId(i as u32))
        .find(|&id| {
            arena.node(id).node_kind() == NodeKind::FnDecl
                && arena.interner.resolve(SymbolId(arena.node(id).lhs)) == fn_fast
        })
        .expect("distributive FnDecl");
    let flags = arena.decl_flags[fn_id.0 as usize];
    assert_ne!(flags & DECL_FLAG_PARTIAL, 0);
    assert_ne!(flags & DECL_FLAG_STATIC, 0);
    assert_ne!(flags & DECL_FLAG_PURE, 0);

    for mode in [
        CallQuoteKind::ShallowQuote,
        CallQuoteKind::DeepSplice,
        CallQuoteKind::AbiFrame,
        CallQuoteKind::ReactiveSignal,
        CallQuoteKind::SymbolicAdjoint,
        CallQuoteKind::EffectDryRun,
    ] {
        assert!(arena
            .nodes
            .iter()
            .any(|n| n.node_kind() == NodeKind::CallExpr && n.flags == mode as u16));
    }

    // Postfix `x!` must emit NULL001
    let bad_bang = parse(&format!(
        "fn {}(x: i64?) -> i64 {{ return x!; }}",
        rng.ident("bad")
    ));
    assert!(bad_bang
        .diagnostics
        .iter()
        .any(|(_, m)| m.contains("NULL001")));

    // Disallowed trailing comma contexts
    for invalid_comma in [
        "[Clamp(0, 1),] struct S {}",
        "fn f(a: i32, b: i32,) {}",
        "fn g() { f(1, 2,); }",
        "struct Box<T,> {}",
        "struct V : IValue, {}",
    ] {
        let a = parse(invalid_comma);
        assert!(
            !a.diagnostics.is_empty(),
            "Expected trailing-comma error on: {}",
            invalid_comma
        );
    }

    // Prohibited legacy syntax keywords
    for legacy in [
        "def legacy_fn() {}",
        "fn f() { let mut x = 1; }",
        "fn f(x: i64) { match x { 1 => 2 } }",
        "class Foo {}",
        "record Bar {}",
        "fn f(b: bool) { if b { } elif !b { } }",
    ] {
        let a = parse(legacy);
        assert!(
            !a.diagnostics.is_empty(),
            "Legacy construct must be rejected with diagnostic: {}",
            legacy
        );
    }
    guard.finish();
}

#[test]
fn test_51_3_domain_syntax_packages_and_strict_nameof_hygiene() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_51_3_domain_syntax_packages_and_strict_nameof_hygiene",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();

    let domain_src = r#"
        database AppDb {
            pub users: i64;
        }
        adapter LegacyUser -> ModernUser {
            pub id: i64;
        }
        extern "C" fn c_puts(s: *u8) -> i32;

        fn domain_demo(users: i64[]) -> i64
            requires len(users) >= 0
            ensures true
        {
            transaction {
                commit;
            }
            noalloc {
                let q = from u in users where u > 0 select u;
            }
            deadline(50) {
                let sq = sql("SELECT 1");
            }
            return 0;
        }
    "#;
    let arena = parse(domain_src);
    assert!(
        arena.diagnostics.is_empty(),
        "Domain syntax diagnostics: {:?}",
        arena.diagnostics
    );

    let syntax_modules = [
        "stdlib/core/syntax/type_construct.pilot",
        "stdlib/core/syntax/visibility.pilot",
        "stdlib/core/syntax/scope_header.pilot",
        "stdlib/core/syntax/attribute.pilot",
        "stdlib/core/syntax/lifecycle.pilot",
        "stdlib/core/syntax/properties.pilot",
        "stdlib/core/syntax/module_ext.pilot",
        "stdlib/core/syntax/for.pilot",
        "stdlib/core/syntax/while.pilot",
        "stdlib/core/syntax/until.pilot",
        "stdlib/core/syntax/switch.pilot",
        "stdlib/core/syntax/guard.pilot",
        "stdlib/core/syntax/try_catch.pilot",
        "stdlib/core/syntax/expr_sugar.pilot",
        "stdlib/core/syntax/unit_decl.pilot",
        "stdlib/db/sql_syntax.pilot",
        "stdlib/polyglot/polyglot_syntax.pilot",
        "stdlib/realtime/rt_syntax.pilot",
        "stdlib/asyncio/select_syntax.pilot",
        "stdlib/contracts/contracts_syntax.pilot",
    ];

    for rel in syntax_modules {
        let src = std::fs::read_to_string(root.join(rel))
            .unwrap_or_else(|e| panic!("Missing {}: {}", rel, e));
        let lines = src.lines().count();
        assert!(lines >= 15 && lines <= 1500, "{} lines={}", rel, lines);
        let parsed = parse(&src);
        assert!(
            parsed.diagnostics.is_empty(),
            "{} parse diagnostics: {:?}",
            rel,
            parsed.diagnostics
        );

        // Enforce `nameof(kw)` instead of quoted `"kw"` on identifier receptors & Pratt ops!
        for (ln_no, line) in src.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.contains("\"bind_receptor\"") || trimmed.contains("\"register_pratt_op\"") {
                if let Some(after) = trimmed
                    .split("\"bind_receptor\",")
                    .nth(1)
                    .or_else(|| trimmed.split("\"register_pratt_op\",").nth(1))
                {
                    let arg = after.trim_start();
                    if let Some(rest) = arg.strip_prefix('"') {
                        if let Some(end_q) = rest.find('"') {
                            let lit = &rest[..end_q];
                            let is_ident = !lit.is_empty()
                                && lit.bytes().next().unwrap().is_ascii_alphabetic()
                                && lit.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
                            assert!(
                                !is_ident,
                                "{}:{} must use `nameof({})` instead of hardcoded string literal `\"{}\"` in receptor/operator registration!",
                                rel,
                                ln_no + 1,
                                lit,
                                lit
                            );
                        }
                    }
                }
            }
        }
    }
    guard.finish();
}

#[test]
fn test_51_4_rust_parser_and_ast_contraction_and_name_blind_purity() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_51_4_rust_parser_and_ast_contraction_and_name_blind_purity",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    assert_eq!(std::mem::size_of::<AstNode>(), 16);

    let parser_rs =
        std::fs::read_to_string(root.join("crates/pilot-parser/src/lib.rs")).unwrap();
    let parser_loc = parser_rs.lines().count();
    assert!(
        parser_loc <= 1100,
        "crates/pilot-parser/src/lib.rs must be <= 1,100 LOC, got {}",
        parser_loc
    );

    let ast_rs = std::fs::read_to_string(root.join("crates/pilot-ast/src/lib.rs")).unwrap();
    let ast_loc = ast_rs.lines().count();
    assert!(
        ast_loc <= 450,
        "crates/pilot-ast/src/lib.rs must be <= 450 LOC, got {}",
        ast_loc
    );
    assert!(!ast_rs.contains("collections::HashMap"));
    assert!(!ast_rs.contains("collections::HashSet"));

    for forbidden_type in [
        "\"slice\"",
        "\"option\"",
        "\"result\"",
        "\"SliceBuffer\"",
        "\"RangeWindow\"",
        "\"*void\"",
        "\"0b00 (IValue)\"",
    ] {
        assert!(
            !parser_rs.contains(forbidden_type) && !ast_rs.contains(forbidden_type),
            "Forbidden hardcoded Pilot type name {} found in pilot-parser or pilot-ast",
            forbidden_type
        );
    }

    assert!(
        !root.join("selfhost/scope_header.pilot").exists(),
        "Superseded selfhost/scope_header.pilot must remain deleted"
    );
    guard.finish();
}
