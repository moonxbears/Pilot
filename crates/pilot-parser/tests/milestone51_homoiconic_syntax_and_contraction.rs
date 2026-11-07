//! Milestone 51 Verification Gate:
//! Pure Pilot `syntax` Desugaring & Contraction of `pilot-parser` (<= 1,100 LOC) & `pilot-ast` (<= 450 LOC).

use pilot_ast::{AstNode, CallQuoteKind, NodeId, NodeKind, DECL_FLAG_PURE, DECL_FLAG_STATIC};
use pilot_intern::SymbolId;
use pilot_parser::{parse, parse_and_ingest, ParserTables};

fn resolve_type_str(arena: &pilot_ast::AstArena, id: NodeId) -> String {
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
                let inner: Vec<String> = args.iter().map(|&c| resolve_type_str(arena, NodeId(c))).collect();
                format!("{}<{}>", base, inner.join(", "))
            }
        }
        NodeKind::TypeTuple => {
            let n = arena.node(id);
            let inner: Vec<String> = arena.children_slice(n.lhs, n.rhs).iter().map(|&c| resolve_type_str(arena, NodeId(c))).collect();
            format!("({})", inner.join(", "))
        }
        NodeKind::IntLit => arena.i64_pool.get(n.lhs as usize).copied().unwrap_or(0).to_string(),
        _ => format!("{:?}", n.node_kind()),
    }
}

#[test]
fn test_51_1_general_syntax_and_type_construct_universal_rename_litmus() {
    // 1. Canonical stdlib/core/syntax/type_construct.pilot precedence & lowering
    let src = r#"
        let a: i64[]!str;
        let b: f64[4]?;
        let c: (i32, i32) => bool;
        let d: f32@8;
        let e: i32[:];
        let f: u8[..];
        let g: *void;
    "#;
    let arena = parse(src);
    assert!(arena.diagnostics.is_empty(), "Expected 0 diagnostics, got: {:?}", arena.diagnostics);
    let root_ch = arena.children_slice(arena.node(arena.root).lhs, arena.node(arena.root).rhs);
    assert_eq!(root_ch.len(), 7);
    assert_eq!(resolve_type_str(&arena, arena.type_annot(NodeId(root_ch[0])).unwrap()), "result<slice<i64>, str>");
    assert_eq!(resolve_type_str(&arena, arena.type_annot(NodeId(root_ch[1])).unwrap()), "option<array<f64, 4>>");
    assert_eq!(resolve_type_str(&arena, arena.type_annot(NodeId(root_ch[2])).unwrap()), "fn<(i32, i32), bool>");
    assert_eq!(resolve_type_str(&arena, arena.type_annot(NodeId(root_ch[3])).unwrap()), "simd<f32, 8>");
    assert_eq!(resolve_type_str(&arena, arena.type_annot(NodeId(root_ch[4])).unwrap()), "ring<i32>");
    assert_eq!(resolve_type_str(&arena, arena.type_annot(NodeId(root_ch[5])).unwrap()), "stride<u8>");
    assert_eq!(resolve_type_str(&arena, arena.type_annot(NodeId(root_ch[6])).unwrap()), "ptr<void>");

    // 2. Universal Rename Litmus Test: custom composite names via `syntax type_construct` with zero Rust changes
    let mut custom_tables = ParserTables::new();
    let custom_grammar = r#"
        comptime {
            __engine("bind_receptor", "syntax", 4, 0);
            __engine("bind_receptor", "type_construct", 5, 0);
        }
        syntax type_construct {
            (T[])    => quote { window<$T> };
            (T[:])   => quote { dyn_buf<$T> };
            (T[N])   => quote { fixed_vec<$T, $N> };
            (T?)     => quote { maybe<$T> };
            (T ! E)  => quote { outcome<$T, $E> };
            (*T)     => quote { raw_addr<$T> };
        }
        let x: c32[]!err_code;
        let y: f64[16]?;
        let z: (*u8)[:];
    "#;
    let renamed_arena = parse_and_ingest(custom_grammar, &mut custom_tables);
    assert!(renamed_arena.diagnostics.is_empty(), "Expected 0 diagnostics: {:?}", renamed_arena.diagnostics);
    let r_ch = renamed_arena.children_slice(renamed_arena.node(renamed_arena.root).lhs, renamed_arena.node(renamed_arena.root).rhs);
    let x_node = NodeId(r_ch[r_ch.len() - 3]);
    let y_node = NodeId(r_ch[r_ch.len() - 2]);
    let z_node = NodeId(r_ch[r_ch.len() - 1]);
    assert_eq!(resolve_type_str(&renamed_arena, renamed_arena.type_annot(x_node).unwrap()), "outcome<window<c32>, err_code>");
    assert_eq!(resolve_type_str(&renamed_arena, renamed_arena.type_annot(y_node).unwrap()), "maybe<fixed_vec<f64, 16>>");
    assert_eq!(resolve_type_str(&renamed_arena, renamed_arena.type_annot(z_node).unwrap()), "dyn_buf<raw_addr<u8>>");

    // 3. Live mid-file user `syntax` macro expansion with `$param` AST splicing
    let macro_src = r#"
        pub syntax {
            unless(cond, body) => quote {
                if !($cond) {
                    $body
                }
            };
        }
        fn check(flag: bool) -> i64 {
            unless flag {
                return 99;
            }
            return 0;
        }
    "#;
    let mac_arena = parse(macro_src);
    assert!(mac_arena.diagnostics.is_empty(), "Expected 0 diagnostics on user macro: {:?}", mac_arena.diagnostics);
    let has_if = mac_arena.nodes.iter().any(|n| n.node_kind() == NodeKind::IfExpr);
    assert!(has_if, "`unless` macro must splice into a canonical `IfExpr` node");
}

#[test]
fn test_51_2_core_surface_syntax_modules_and_collection_only_trailing_commas() {
    let src = r#"
        pub static pure:
        fn fast_add(a: i64, b: i64) -> i64 => a + b;

        let List<i32> {
            arr1 = [1, 2, 3,],
            arr2 = [4, 5, 6,],
        };

        pub struct Wallet : IValue {
            pub balance: f64 {
                get => field;
                set => { field = value; }
            }
            self(init: f64) {
                .balance = init;
            }
            ~self() {}
            scope self {
                enter: { .balance += 1.0; }
                exit:  { .balance -= 1.0; }
            }
            pub op +(rhs: Wallet) -> Wallet => Wallet(.balance + rhs.balance);
        }

        fn demo(opt: i64?) -> i64 {
            guard some v = opt else {
                return -1;
            }
            var acc = 0;
            while acc < 5 {
                acc += 1;
            }
            until acc >= 10 {
                acc += 1;
            }
            let found = for i in 0 .. 10 {
                if i == v {
                    break i;
                }
            } then 0;
            let f = (x: i64) => x * 2;
            let q1 = f!(10);
            let q2 = f!!(~v);
            let q3 = f!&(10);
            let q4 = f!^(10);
            let q5 = f!@(10);
            let q6 = f!?(10);
            return switch found {
                case 0: 100,
                default: found,
            };
        }
    "#;
    let arena = parse(src);
    assert!(arena.diagnostics.is_empty(), "Expected 0 diagnostics, got: {:?}", arena.diagnostics);

    // Verify `fast_add` inherited `pub static pure:` distributive section header
    let fast_add_id = (0..arena.nodes.len())
        .map(|i| NodeId(i as u32))
        .find(|&id| arena.node(id).node_kind() == NodeKind::FnDecl && arena.interner.resolve(SymbolId(arena.node(id).lhs)) == "fast_add")
        .expect("fast_add FnDecl must exist");
    let flags = arena.decl_flags[fast_add_id.0 as usize];
    assert_ne!(flags & DECL_FLAG_STATIC, 0);
    assert_ne!(flags & DECL_FLAG_PURE, 0);

    // Verify all 6 homoiconic call-quote modes are preserved
    for mode in [
        CallQuoteKind::ShallowQuote,
        CallQuoteKind::DeepSplice,
        CallQuoteKind::AbiFrame,
        CallQuoteKind::ReactiveSignal,
        CallQuoteKind::SymbolicAdjoint,
        CallQuoteKind::EffectDryRun,
    ] {
        assert!(
            arena.nodes.iter().any(|n| n.node_kind() == NodeKind::CallExpr && n.flags == mode as u16),
            "Missing CallQuoteKind::{:?}",
            mode
        );
    }

    // Verify NULL001 diagnostic on unchecked postfix `x!`
    let bad_bang = parse("fn bad(x: i64?) -> i64 { return x!; }");
    assert!(
        bad_bang.diagnostics.iter().any(|(_, m)| m.contains("NULL001")),
        "Expected NULL001 diagnostic on postfix `x!`, got: {:?}",
        bad_bang.diagnostics
    );

    // Verify collection-only trailing commas: disallowed in attributes, params, args, generics, inheritance
    for invalid_comma in [
        "[Clamp(0, 1),] struct S {}",
        "fn f(a: i32, b: i32,) {}",
        "fn g() { f(1, 2,); }",
        "struct Box<T,> {}",
        "struct V : IValue, {}",
    ] {
        let a = parse(invalid_comma);
        assert!(!a.diagnostics.is_empty(), "Expected trailing comma diagnostic for: {}", invalid_comma);
    }

    // Verify prohibited legacy keywords (`def`, `let mut`, `match`, `class`) are rejected with diagnostics
    for legacy in [
        "def legacy_fn() {}",
        "fn f() { let mut x = 1; }",
        "fn f(x: i64) { match x { 1 => 2 } }",
        "class Foo {}",
    ] {
        let a = parse(legacy);
        assert!(!a.diagnostics.is_empty(), "Prohibited legacy syntax must emit diagnostic: {}", legacy);
    }
}

#[test]
fn test_51_3_domain_syntax_packages_and_stdlib_selfhost_zero_error_parse() {
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
            let sq = sql("SELECT 1");
            return 0;
        }
    "#;
    let arena = parse(domain_src);
    assert!(arena.diagnostics.is_empty(), "Expected 0 diagnostics on domain syntax, got: {:?}", arena.diagnostics);

    // Verify all 20 syntax modules and selfhost files parse with 0 diagnostics
    for rel_path in [
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
        "selfhost/analyzer.pilot",
        "selfhost/parser.pilot",
        "selfhost/preprocessor.pilot",
    ] {
        let full_path = format!("../../{}", rel_path);
        let src = std::fs::read_to_string(&full_path).unwrap_or_else(|e| panic!("Failed to read {}: {}", full_path, e));
        let lines = src.lines().count();
        assert!(lines <= 1500, "{} exceeds 1,500 lines ({} lines)", rel_path, lines);
        let parsed = parse(&src);
        assert!(parsed.diagnostics.is_empty(), "{} had parse diagnostics: {:?}", rel_path, parsed.diagnostics);
    }
}

#[test]
fn test_51_4_rust_parser_and_ast_contraction_and_legacy_cleanup() {
    assert_eq!(std::mem::size_of::<AstNode>(), 16, "AstNode must remain strictly 16 bytes");

    let parser_rs = std::fs::read_to_string("../../crates/pilot-parser/src/lib.rs").expect("read pilot-parser/src/lib.rs");
    let parser_loc = parser_rs.lines().count();
    assert!(parser_loc <= 1100, "crates/pilot-parser/src/lib.rs must be <= 1,100 LOC, got {}", parser_loc);

    let ast_rs = std::fs::read_to_string("../../crates/pilot-ast/src/lib.rs").expect("read pilot-ast/src/lib.rs");
    let ast_loc = ast_rs.lines().count();
    assert!(ast_loc <= 450, "crates/pilot-ast/src/lib.rs must be <= 450 LOC, got {}", ast_loc);
    assert!(!ast_rs.contains("collections::HashMap"), "crates/pilot-ast/src/lib.rs must have zero HashMap side-tables");
    assert!(!ast_rs.contains("collections::HashSet"), "crates/pilot-ast/src/lib.rs must have zero HashSet side-tables");

    assert!(
        !std::path::Path::new("../../selfhost/scope_header.pilot").exists(),
        "Legacy duplicate selfhost/scope_header.pilot must be deleted"
    );
}
