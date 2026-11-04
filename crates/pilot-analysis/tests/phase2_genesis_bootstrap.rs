use pilot_analysis::{
    bootstrap_core_file, eval_file_with_tables, BootstrapDiagnostic, ComptimeEngine, ComptimeValue,
    LangItem, RegisterClass,
};
use pilot_ast::{BinaryOp, NodeId, NodeKind};
use pilot_parser::{parse_with_tables, ModifierSpec, ModuleKwKind, ParserTables};

const CORE_PILOT_SRC: &str = include_str!("../../../stdlib/core/core.pilot");

#[test]
fn test_line1_cfg_ast_bundle_evaluation() {
    let mut tables = ParserTables::new();
    let mut vm = ComptimeEngine::new();
    let arena = bootstrap_core_file(CORE_PILOT_SRC, &mut tables, &mut vm)
        .expect("stdlib/core/core.pilot should bootstrap cleanly");

    // 1. Cfg.Hooks.bind_attr -> "register_bracket_attribute"
    assert_eq!(
        vm.resolve_cfg_str(&["Hooks", "bind_attr"], &arena.interner)
            .as_deref(),
        Some("register_bracket_attribute")
    );

    // 2. Cfg.Lattice.guard -> 0b11 (3)
    assert_eq!(
        vm.resolve_cfg_uint(&["Lattice", "guard"], &arena.interner),
        Some(0b11)
    );

    // 3. Cfg.Ops.prec_add -> 60
    assert_eq!(
        vm.resolve_cfg_uint(&["Ops", "prec_add"], &arena.interner),
        Some(60)
    );

    // Additional sanity checks on nested Cfg AstBundle fields
    assert_eq!(
        vm.resolve_cfg_str(&["Hooks", "seal_bridge"], &arena.interner)
            .as_deref(),
        Some("seal_bootstrap_bridge")
    );
    assert_eq!(
        vm.resolve_cfg_uint(&["Tokens", "ring_genesis"], &arena.interner),
        Some(100)
    );
}

#[test]
fn test_interleaved_top_level_bootstrap() {
    let mut tables = ParserTables::new();
    let mut vm = ComptimeEngine::new();
    let _arena = bootstrap_core_file(CORE_PILOT_SRC, &mut tables, &mut vm)
        .expect("stdlib/core/core.pilot should bootstrap cleanly");

    // 1. 8 bracket attributes: Lang, Bits, Layout, Operator, Inline, Lattice, EnforceCoreNaming, SyntaxMacro
    let expected_attrs = [
        "Lang",
        "Bits",
        "Layout",
        "Operator",
        "Inline",
        "Lattice",
        "EnforceCoreNaming",
        "SyntaxMacro",
    ];
    assert_eq!(tables.bracket_attributes_by_name.len(), 8);
    for attr in expected_attrs {
        assert!(
            tables.bracket_attributes_by_name.contains_key(attr),
            "Missing bracket attribute '{}'",
            attr
        );
    }

    // 2. 6 visibility rings (priv..=export) and 7 semantic modifiers (pure, static, const, var, iso, partial, ext)
    let expected_vis = [
        ("priv", 0u8),
        ("file", 1u8),
        ("pkg", 2u8),
        ("int", 3u8),
        ("pub", 4u8),
        ("export", 5u8),
    ];
    for (vname, ring) in expected_vis {
        assert_eq!(
            tables.modifiers_by_name.get(vname),
            Some(&ModifierSpec::VisibilityRing(ring)),
            "Visibility modifier '{}' mismatch",
            vname
        );
    }

    let expected_sem = ["pure", "static", "const", "var", "iso", "partial", "ext"];
    for sname in expected_sem {
        assert!(
            matches!(
                tables.modifiers_by_name.get(sname),
                Some(ModifierSpec::SemanticFlag(_))
            ),
            "Missing semantic modifier '{}'",
            sname
        );
    }
    assert_eq!(tables.modifiers_by_name.len(), 13);

    // 3. lvl, pkg, use, and self
    assert_eq!(
        tables.param_modifiers_by_name.get("lvl").map(|s| s.as_str()),
        Some("set_clearance_ring")
    );
    assert_eq!(
        tables.module_keywords_by_name.get("pkg"),
        Some(&ModuleKwKind::DeclarePkg)
    );
    assert_eq!(
        tables.module_keywords_by_name.get("use"),
        Some(&ModuleKwKind::ImportPath)
    );
    assert_eq!(tables.receiver_ident_name.as_deref(), Some("self"));

    // 4. syntax, break, continue, and op syntax macros
    for kw in ["syntax", "break", "continue", "op"] {
        assert!(
            tables.syntax_macros_by_name.contains_key(kw),
            "Missing syntax macro '{}'",
            kw
        );
    }
}

#[test]
fn test_pratt_precedence_registration() {
    let mut tables = ParserTables::new();
    let mut vm = ComptimeEngine::new();
    let _ = bootstrap_core_file(CORE_PILOT_SRC, &mut tables, &mut vm)
        .expect("stdlib/core/core.pilot should bootstrap cleanly");

    // Verify precedences in ParserTables match Cfg.Ops:
    // prec_mul (70) > prec_add (60) > prec_eq (45) > prec_coal (10)
    assert_eq!(tables.infix_precedence("*"), Some(70));
    assert_eq!(tables.infix_precedence("+"), Some(60));
    assert_eq!(tables.infix_precedence("=="), Some(45));
    assert_eq!(tables.infix_precedence("??"), Some(10));

    // Parse `let x = a + b * c == d ?? e;` using the populated ParserTables
    let expr_arena = parse_with_tables("let x = a + b * c == d ?? e;", &tables);
    assert!(
        expr_arena.nodes.len() > 1,
        "Expected parsed AST for expression"
    );

    // Find the LetStmt and inspect its RHS binary expression tree:
    // Top-level must be `??` (BinaryOp::Coalesce):
    //   lhs: `(a + (b * c)) == d` (BinaryOp::Eq)
    //     lhs: `a + (b * c)` (BinaryOp::Add)
    //       lhs: `a`
    //       rhs: `b * c` (BinaryOp::Mul)
    //     rhs: `d`
    //   rhs: `e`
    let let_id = (0..expr_arena.nodes.len())
        .map(|i| NodeId(i as u32))
        .find(|&id| expr_arena.node(id).node_kind() == NodeKind::LetStmt)
        .expect("Should find LetStmt");
    let root_expr = NodeId(expr_arena.node(let_id).rhs);
    let root_node = *expr_arena.node(root_expr);
    assert_eq!(root_node.node_kind(), NodeKind::BinaryExpr);
    assert_eq!(root_node.flags, BinaryOp::Coalesce as u16);

    let eq_expr = NodeId(root_node.lhs);
    let eq_node = *expr_arena.node(eq_expr);
    assert_eq!(eq_node.node_kind(), NodeKind::BinaryExpr);
    assert_eq!(eq_node.flags, BinaryOp::Eq as u16);

    let add_expr = NodeId(eq_node.lhs);
    let add_node = *expr_arena.node(add_expr);
    assert_eq!(add_node.node_kind(), NodeKind::BinaryExpr);
    assert_eq!(add_node.flags, BinaryOp::Add as u16);

    let mul_expr = NodeId(add_node.rhs);
    let mul_node = *expr_arena.node(mul_expr);
    assert_eq!(mul_node.node_kind(), NodeKind::BinaryExpr);
    assert_eq!(mul_node.flags, BinaryOp::Mul as u16);
}

#[test]
fn test_seed_bool_and_usize_ssa_lowering() {
    let mut tables = ParserTables::new();
    let mut vm = ComptimeEngine::new();
    let _arena = bootstrap_core_file(CORE_PILOT_SRC, &mut tables, &mut vm)
        .expect("stdlib/core/core.pilot should bootstrap cleanly");

    // 1. Assert bool is bound to LangItem::Bool (8-bit gpr_unsigned)
    assert!(vm.lang_item_node(LangItem::Bool).is_some());
    let bool_layout = vm.type_layout("bool").expect("bool layout registered");
    assert_eq!(bool_layout.bit_width, 8);
    assert_eq!(bool_layout.reg_class, RegisterClass::GprUnsigned);

    // 2. Assert usize is bound to LangItem::Usize (64-bit gpr_unsigned)
    assert!(vm.lang_item_node(LangItem::Usize).is_some());
    let usize_layout = vm.type_layout("usize").expect("usize layout registered");
    assert_eq!(usize_layout.bit_width, 64);
    assert_eq!(usize_layout.reg_class, RegisterClass::GprUnsigned);

    // 3. Evaluate sizeof(usize) -> 8, sizeof(bool) -> 1, 10 + 20 -> 30, 10 < 20 -> true, !true -> false
    let probe_src = r#"
        comptime {
            let sz_usize = sizeof(usize);
            let sz_bool = sizeof(bool);
            let sum_val = 10 + 20;
            let cmp_val = 10 < 20;
            let not_val = !true;
        }
    "#;
    let _ = eval_file_with_tables(probe_src, &mut tables, &mut vm)
        .expect("sizeof and operator evaluation should succeed through genesis comptime wrappers");
    assert_eq!(vm.globals_by_name.get("sz_usize"), Some(&ComptimeValue::Uint(8)));
    assert_eq!(vm.globals_by_name.get("sz_bool"), Some(&ComptimeValue::Uint(1)));
    assert_eq!(vm.globals_by_name.get("sum_val"), Some(&ComptimeValue::Uint(30)));
    assert_eq!(vm.globals_by_name.get("cmp_val"), Some(&ComptimeValue::Bool(true)));
    assert_eq!(vm.globals_by_name.get("not_val"), Some(&ComptimeValue::Bool(false)));

    // 4. Verify SSA operator lowering for `10 + 20` (`add`), `10 < 20` (`ult`), and `!true` (`xor`)
    assert_eq!(vm.ssa_opcode_for_operator("usize", "+"), Some("add"));
    assert_eq!(vm.ssa_opcode_for_operator("usize", "<"), Some("ult"));
    assert_eq!(vm.ssa_opcode_for_operator("bool", "!"), Some("xor"));
}

#[test]
fn test_seal_bootstrap_bridge_enforcement() {
    let mut tables = ParserTables::new();
    let mut vm = ComptimeEngine::new();
    let _ = bootstrap_core_file(CORE_PILOT_SRC, &mut tables, &mut vm)
        .expect("stdlib/core/core.pilot should bootstrap cleanly");

    // Verify the bridge is sealed after `core.pilot` Step 6
    assert!(
        tables.bootstrap_sealed,
        "ParserTables::bootstrap_sealed must be true after core.pilot finishes"
    );

    // Attempt to invoke `__engine("bind_lang_item", Foo, "bar");` in a downstream user file
    let illegal_user_src = r#"
        struct Foo {}
        comptime {
            __engine("bind_lang_item", Foo, "bar");
        }
    "#;
    let result = eval_file_with_tables(illegal_user_src, &mut tables, &mut vm);
    assert!(
        matches!(
            result,
            Err(BootstrapDiagnostic::BootstrapBridgeSealedError { ref hook, .. }) if hook == "bind_lang_item"
        ),
        "Expected BootstrapBridgeSealedError for direct __engine call after seal, got: {:?}",
        result
    );
}
