//! Phase 3 Verification Gate Tests:
//! 1. `test_2bit_lattice_bitwise_or_and_auto_traits`
//! 2. `test_lattice_contract_mismatch_diagnostics`
//! 3. `test_4tier_method_table_and_bundle_modifiers`
//! 4. `test_partial_struct_layout_seal_enforcement`
//! 5. `test_enforce_core_naming_lint`

use pilot_analysis::{
    classify_perceus_action, verify_core_naming_rules, BundleDef, BundleId, Diagnostic, FieldDef,
    InterfaceId, LatticeBits, MethodEntry, MethodSignature, MethodTier, OriginId, OwnershipAction,
    PackageAst, SpliceDirectives, TypeArena, TypeDef, TypeId, TypeKind,
};
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_parser::{FunctionId, PackageId};
use pilot_span::ByteSpan;

#[test]
fn test_2bit_lattice_bitwise_or_and_auto_traits() {
    let mut arena = TypeArena::new();
    let mut interner = pilot_intern::Interner::new();
    let span = ByteSpan::new(0, 10);
    let pkg_id = PackageId(0);

    // 1. Verify `bool` (0b00) and `usize` (0b00) auto-implement `IAnyValue` and `IValue`
    let bool_sym = interner.intern("bool");
    let mut bool_td = TypeDef::new(TypeId::BOOL, bool_sym, pkg_id);
    bool_td.is_lang_primitive = true;
    bool_td.size = 1;
    bool_td.align = 1;
    arena.register_type_def(bool_td, &interner);

    let mut diags = Vec::new();
    arena.seal_primary_struct_layout(TypeId::BOOL, &interner, &mut diags);
    assert!(diags.is_empty());
    let bool_sealed = arena.type_defs.get(&TypeId::BOOL).unwrap();
    assert_eq!(bool_sealed.lattice, LatticeBits::Value);
    assert!(arena.implements_interface(TypeId::BOOL, InterfaceId::IANY_VALUE));
    assert!(arena.implements_interface(TypeId::BOOL, InterfaceId::IVALUE));
    assert!(!arena.implements_interface(TypeId::BOOL, InterfaceId::IREFERENCE));
    assert!(!arena.implements_interface(TypeId::BOOL, InterfaceId::IOBJECT));
    assert!(!arena.implements_interface(TypeId::BOOL, InterfaceId::IGUARD));

    let usize_ty = arena.alloc(TypeKind::Primitive("usize".to_string()));
    let usize_sym = interner.intern("usize");
    let mut usize_td = TypeDef::new(usize_ty, usize_sym, pkg_id);
    usize_td.is_lang_primitive = true;
    usize_td.size = 8;
    usize_td.align = 8;
    arena.register_type_def(usize_td, &interner);
    arena.seal_primary_struct_layout(usize_ty, &interner, &mut diags);
    assert!(diags.is_empty());
    assert_eq!(arena.type_lattice(usize_ty), LatticeBits::Value);
    assert!(arena.implements_interface(usize_ty, InterfaceId::IANY_VALUE));
    assert!(arena.implements_interface(usize_ty, InterfaceId::IVALUE));

    // Define a 0b01 (`IReference`) primitive/type `RefSlice`
    let ref_slice_ty = arena.alloc(TypeKind::Collection {
        kind: pilot_analysis::types::CollectionKind::Slice,
        element: usize_ty,
    });
    assert_eq!(arena.type_lattice(ref_slice_ty), LatticeBits::Reference);

    // 2. Verify a struct containing a `0b00` field and a `0b01` (`IReference`) field computes `0b01`
    // and auto-implements `IAnyValue` + `IReference` (dropping `IValue`).
    let view_ty = arena.alloc(TypeKind::Adt {
        name: "TokenView".to_string(),
        variants: vec![],
    });
    let view_td = TypeDef::new(view_ty, interner.intern("TokenView"), pkg_id);
    arena.register_type_def(view_td, &interner);
    arena.add_struct_field(view_ty, interner.intern("len"), usize_ty, 8, span);
    arena.add_struct_field(view_ty, interner.intern("slice"), ref_slice_ty, 8, span);
    arena.seal_primary_struct_layout(view_ty, &interner, &mut diags);
    assert!(diags.is_empty());
    assert_eq!(arena.type_lattice(view_ty), LatticeBits::Reference);
    assert!(arena.implements_interface(view_ty, InterfaceId::IANY_VALUE));
    assert!(arena.implements_interface(view_ty, InterfaceId::IREFERENCE));
    assert!(!arena.implements_interface(view_ty, InterfaceId::IVALUE));
    assert!(!arena.implements_interface(view_ty, InterfaceId::IOBJECT));
    assert!(!arena.implements_interface(view_ty, InterfaceId::IGUARD));

    // Define a 0b10 (`IObject`) type `HeapBuf` (with destructor `~self()`)
    let heap_buf_ty = arena.alloc(TypeKind::Adt {
        name: "HeapBuf".to_string(),
        variants: vec![],
    });
    let mut heap_buf_td = TypeDef::new(heap_buf_ty, interner.intern("HeapBuf"), pkg_id);
    heap_buf_td.destructor_fn = Some(FunctionId(10));
    arena.register_type_def(heap_buf_td, &interner);
    arena.add_struct_field(heap_buf_ty, interner.intern("cap"), usize_ty, 8, span);
    arena.seal_primary_struct_layout(heap_buf_ty, &interner, &mut diags);
    assert!(diags.is_empty());
    assert_eq!(arena.type_lattice(heap_buf_ty), LatticeBits::Object);
    assert!(arena.implements_interface(heap_buf_ty, InterfaceId::IANY_VALUE));
    assert!(arena.implements_interface(heap_buf_ty, InterfaceId::IOBJECT));
    assert!(!arena.implements_interface(heap_buf_ty, InterfaceId::IVALUE));

    // 3. Verify a struct combining a `0b01` (`IReference`) field and a `0b10` (`IObject`) field
    // computes `0b11` (`IGuard`) and auto-implements `IAnyValue` + `IReference` + `IObject` + `IGuard`,
    // and verify `classify_perceus_action(0b11) == OwnershipAction::ScopedGuardMoveAndDrop`.
    let guard_ty = arena.alloc(TypeKind::Adt {
        name: "ScopedBufferGuard".to_string(),
        variants: vec![],
    });
    let guard_td = TypeDef::new(guard_ty, interner.intern("ScopedBufferGuard"), pkg_id);
    arena.register_type_def(guard_td, &interner);
    arena.add_struct_field(guard_ty, interner.intern("slice"), ref_slice_ty, 8, span);
    arena.add_struct_field(guard_ty, interner.intern("buf"), heap_buf_ty, 8, span);
    arena.seal_primary_struct_layout(guard_ty, &interner, &mut diags);
    assert!(diags.is_empty());

    let guard_lattice = arena.type_lattice(guard_ty);
    assert_eq!(guard_lattice, LatticeBits::Guard);
    assert_eq!(guard_lattice.raw(), 0b11);
    assert!(arena.implements_interface(guard_ty, InterfaceId::IANY_VALUE));
    assert!(arena.implements_interface(guard_ty, InterfaceId::IREFERENCE));
    assert!(arena.implements_interface(guard_ty, InterfaceId::IOBJECT));
    assert!(arena.implements_interface(guard_ty, InterfaceId::IGUARD));
    assert!(!arena.implements_interface(guard_ty, InterfaceId::IVALUE));

    assert_eq!(
        classify_perceus_action(guard_lattice.raw()),
        OwnershipAction::ScopedGuardMoveAndDrop
    );
    assert_eq!(classify_perceus_action(0b00), OwnershipAction::TrivialBitCopy);
    assert_eq!(
        classify_perceus_action(0b01),
        OwnershipAction::LexicalBorrowCheckOnly
    );
    assert_eq!(
        classify_perceus_action(0b10),
        OwnershipAction::MoveAndDropAtScopeExit
    );
}

#[test]
fn test_lattice_contract_mismatch_diagnostics() {
    let mut arena = TypeArena::new();
    let mut interner = pilot_intern::Interner::new();
    let span = ByteSpan::new(10, 40);
    let pkg_id = PackageId(0);

    // Register `HeapBuf` with lattice `0b10` (`IObject`)
    let heap_buf_ty = arena.alloc(TypeKind::Adt {
        name: "HeapBuf".to_string(),
        variants: vec![],
    });
    let mut heap_buf_td = TypeDef::new(heap_buf_ty, interner.intern("HeapBuf"), pkg_id);
    heap_buf_td.destructor_fn = Some(FunctionId(1));
    arena.register_type_def(heap_buf_td, &interner);
    let mut setup_diags = Vec::new();
    arena.seal_primary_struct_layout(heap_buf_ty, &interner, &mut setup_diags);
    assert!(setup_diags.is_empty());
    assert_eq!(arena.type_lattice(heap_buf_ty), LatticeBits::Object);

    // Register `RefSlice` with lattice `0b01` (`IReference`)
    let ref_slice_ty = arena.alloc(TypeKind::Collection {
        kind: pilot_analysis::types::CollectionKind::Slice,
        element: TypeId::I64,
    });
    assert_eq!(arena.type_lattice(ref_slice_ty), LatticeBits::Reference);

    // 1. Verify `struct BadCopy : IValue { var buf: HeapBuf }` (where `HeapBuf` has lattice `0b10`)
    // emits `LatticeContractMismatch`.
    let bad_copy_ty = arena.alloc(TypeKind::Adt {
        name: "BadCopy".to_string(),
        variants: vec![],
    });
    let mut bad_copy_td = TypeDef::new(bad_copy_ty, interner.intern("BadCopy"), pkg_id);
    bad_copy_td.declared_interfaces.push(InterfaceId::IVALUE);
    arena.register_type_def(bad_copy_td, &interner);
    arena.add_struct_field(bad_copy_ty, interner.intern("buf"), heap_buf_ty, 8, span);

    let mut diags1: Vec<Diagnostic> = Vec::new();
    arena.seal_primary_struct_layout(bad_copy_ty, &interner, &mut diags1);
    assert_eq!(diags1.len(), 1);
    assert_eq!(diags1[0].code, "LatticeContractMismatch");
    assert!(diags1[0].message.contains("IValue"));

    // 2. Verify `struct BadSend : IObject { var slice: RefSlice }` (where `RefSlice` has lattice `0b01`)
    // emits `LatticeContractMismatch`.
    let bad_send_ty = arena.alloc(TypeKind::Adt {
        name: "BadSend".to_string(),
        variants: vec![],
    });
    let mut bad_send_td = TypeDef::new(bad_send_ty, interner.intern("BadSend"), pkg_id);
    bad_send_td.declared_interfaces.push(InterfaceId::IOBJECT);
    arena.register_type_def(bad_send_td, &interner);
    arena.add_struct_field(bad_send_ty, interner.intern("slice"), ref_slice_ty, 8, span);

    let mut diags2: Vec<Diagnostic> = Vec::new();
    arena.seal_primary_struct_layout(bad_send_ty, &interner, &mut diags2);
    assert_eq!(diags2.len(), 1);
    assert_eq!(diags2[0].code, "LatticeContractMismatch");
    assert!(diags2[0].message.contains("IObject"));
}

#[test]
fn test_4tier_method_table_and_bundle_modifiers() {
    let mut ast_arena = AstArena::with_capacity(64);
    let span = ByteSpan::new(0, 30);
    let pkg_id = PackageId(0);

    let sym_bundle_a = ast_arena.interner.intern("BundleA");
    let sym_bundle_b = ast_arena.interner.intern("BundleB");
    let sym_count = ast_arena.interner.intern("count");
    let sym_node_count = ast_arena.interner.intern("node_count");
    let sym_reset = ast_arena.interner.intern("reset");
    let sym_self = ast_arena.interner.intern("self");

    // Build an AST method body for `BundleA.reset()` that accesses both `.count` (`self.count`)
    // and assigns `self.count = self.count + 1`:
    let self_ident_1 = ast_arena.alloc_node(NodeKind::IdentExpr, 0, span, sym_self.0, 0);
    let member_self_count_lhs = ast_arena.alloc_node(
        NodeKind::MemberExpr,
        0,
        span,
        self_ident_1.0,
        sym_count.0,
    );
    let self_ident_2 = ast_arena.alloc_node(NodeKind::IdentExpr, 0, span, sym_self.0, 0);
    let member_self_count_rhs = ast_arena.alloc_node(
        NodeKind::MemberExpr,
        0,
        span,
        self_ident_2.0,
        sym_count.0,
    );
    let one_lit = ast_arena.alloc_int(1, span);
    let add_expr = ast_arena.alloc_node(
        NodeKind::BinaryExpr,
        pilot_ast::BinaryOp::Add as u16,
        span,
        member_self_count_rhs.0,
        one_lit.0,
    );
    let assign_stmt = ast_arena.alloc_node(
        NodeKind::AssignStmt,
        0,
        span,
        member_self_count_lhs.0,
        add_expr.0,
    );
    let reset_body = ast_arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &[assign_stmt]);
    let (fn_start, fn_len) = ast_arena.alloc_child_slice(&[reset_body]);
    let reset_fn_ast = ast_arena.alloc_node(
        NodeKind::FnDecl,
        fn_len as u16,
        span,
        sym_reset.0,
        fn_start,
    );

    let mut type_arena = TypeArena::new();
    let fn_id_a = FunctionId(101);
    let fn_id_b = FunctionId(102);
    type_arena.method_ast_nodes.insert(fn_id_a, reset_fn_ast);
    type_arena.method_ast_nodes.insert(fn_id_b, reset_fn_ast);

    let reset_sig = MethodSignature {
        sym: sym_reset,
        params: vec![],
        ret: TypeId::VOID,
        is_static: false,
        is_mutating_var: true,
        is_pure: false,
        span,
    };
    type_arena.method_signatures.insert(fn_id_a, reset_sig.clone());
    type_arena.method_signatures.insert(fn_id_b, reset_sig.clone());

    let bundle_a = BundleDef {
        id: BundleId(0),
        name: sym_bundle_a,
        fields: vec![FieldDef {
            name: sym_count,
            ty: TypeId::I64,
            byte_offset: 0,
            visibility: 8,
            source_bundle: Some(BundleId(0)),
            span,
        }],
        methods: vec![MethodEntry {
            sym: sym_reset,
            fn_id: fn_id_a,
            tier: MethodTier::Tier2BundleOrFacetSplice,
            source_origin: OriginId::Bundle(BundleId(0)),
            is_static: false,
            is_mutating_var: true,
            is_pure: false,
            span,
        }],
        span,
    };

    let bundle_b = BundleDef {
        id: BundleId(1),
        name: sym_bundle_b,
        fields: vec![FieldDef {
            name: sym_count,
            ty: TypeId::I64,
            byte_offset: 0,
            visibility: 8,
            source_bundle: Some(BundleId(1)),
            span,
        }],
        methods: vec![MethodEntry {
            sym: sym_reset,
            fn_id: fn_id_b,
            tier: MethodTier::Tier2BundleOrFacetSplice,
            source_origin: OriginId::Bundle(BundleId(1)),
            is_static: false,
            is_mutating_var: true,
            is_pure: false,
            span,
        }],
        span,
    };

    let interner_snap = ast_arena.interner.clone();
    type_arena.register_bundle_def(bundle_a, &interner_snap);
    type_arena.register_bundle_def(bundle_b, &interner_snap);

    // 1. Verify splicing both `*BundleA` and `*BundleB` without modifiers emits
    // `BundleFieldCollision` and `AmbiguousBundleMethodCollision`.
    let host_ty_1 = type_arena.alloc(TypeKind::Adt {
        name: "CollidingCounter".to_string(),
        variants: vec![],
    });
    let host_td_1 = TypeDef::new(
        host_ty_1,
        ast_arena.interner.intern("CollidingCounter"),
        pkg_id,
    );
    let interner_snap = ast_arena.interner.clone();
    type_arena.register_type_def(host_td_1, &interner_snap);

    let mut diags_collision = Vec::new();
    let splice_a = SpliceDirectives::new(BundleId(0), span);
    let splice_b = SpliceDirectives::new(BundleId(1), span);
    type_arena.apply_bundle_splice(
        host_ty_1,
        &splice_a,
        &interner_snap,
        Some(&mut ast_arena),
        &mut diags_collision,
    );
    type_arena.apply_bundle_splice(
        host_ty_1,
        &splice_b,
        &interner_snap,
        Some(&mut ast_arena),
        &mut diags_collision,
    );
    assert!(
        diags_collision.iter().any(|d| d.code == "BundleFieldCollision"),
        "Expected BundleFieldCollision, got: {:?}",
        diags_collision
    );
    assert!(
        diags_collision.iter().any(|d| d.code == "AmbiguousBundleMethodCollision"),
        "Expected AmbiguousBundleMethodCollision, got: {:?}",
        diags_collision
    );

    // 2. Verify splicing with `coalesce { count }` and `without { reset }` (or providing a Tier 1
    // `fn reset()` override on the struct) resolves cleanly with 0 errors and populates
    // `static_delegates` for `BundleA.reset(var ref self)` and `BundleB.reset(var ref self)`.
    let host_ty_2 = type_arena.alloc(TypeKind::Adt {
        name: "CleanCounter".to_string(),
        variants: vec![],
    });
    let host_td_2 = TypeDef::new(host_ty_2, ast_arena.interner.intern("CleanCounter"), pkg_id);
    let interner_snap = ast_arena.interner.clone();
    type_arena.register_type_def(host_td_2, &interner_snap);

    let mut coalesce_only = SpliceDirectives::new(BundleId(1), span);
    coalesce_only.coalesce_fields.insert(sym_count);

    let mut diags_clean = Vec::new();
    type_arena.apply_bundle_splice(
        host_ty_2,
        &splice_a,
        &interner_snap,
        Some(&mut ast_arena),
        &mut diags_clean,
    );
    type_arena.apply_bundle_splice(
        host_ty_2,
        &coalesce_only,
        &interner_snap,
        Some(&mut ast_arena),
        &mut diags_clean,
    );
    // Now provide a Tier 1 override `fn reset()` on the struct, which resolves the Tier 2 method collision!
    type_arena.insert_tier1_method(
        host_ty_2,
        MethodEntry {
            sym: sym_reset,
            fn_id: FunctionId(999),
            tier: MethodTier::Tier1ExplicitStructMember,
            source_origin: OriginId::Struct(host_ty_2),
            is_static: false,
            is_mutating_var: true,
            is_pure: false,
            span,
        },
        Some(reset_sig),
        &interner_snap,
        &mut diags_clean,
    );
    assert!(
        diags_clean.is_empty(),
        "Expected 0 diagnostics after coalesce + Tier 1 override, got: {:?}",
        diags_clean
    );
    let td2 = type_arena.type_defs.get(&host_ty_2).unwrap();
    assert_eq!(td2.fields.len(), 1);
    assert_eq!(
        td2.method_table.get(&sym_reset).unwrap().tier,
        MethodTier::Tier1ExplicitStructMember
    );
    assert_eq!(
        td2.static_delegates.get(&(sym_bundle_a, sym_reset)),
        Some(&fn_id_a)
    );
    assert_eq!(
        td2.static_delegates.get(&(sym_bundle_b, sym_reset)),
        Some(&fn_id_b)
    );

    // 3. Verify `with { count as node_count }` rewrites `.count` and `self.count` inside the
    // cloned bundle methods to `node_count`.
    let host_ty_3 = type_arena.alloc(TypeKind::Adt {
        name: "RenamedCounter".to_string(),
        variants: vec![],
    });
    let host_td_3 = TypeDef::new(
        host_ty_3,
        ast_arena.interner.intern("RenamedCounter"),
        pkg_id,
    );
    let interner_snap = ast_arena.interner.clone();
    type_arena.register_type_def(host_td_3, &interner_snap);

    let mut rename_directives = SpliceDirectives::new(BundleId(0), span);
    rename_directives
        .renamed_symbols
        .insert(sym_count, sym_node_count);

    let mut diags_rename = Vec::new();
    type_arena.apply_bundle_splice(
        host_ty_3,
        &rename_directives,
        &interner_snap,
        Some(&mut ast_arena),
        &mut diags_rename,
    );
    assert!(diags_rename.is_empty());
    let td3 = type_arena.type_defs.get(&host_ty_3).unwrap();
    assert_eq!(td3.fields.len(), 1);
    assert_eq!(td3.fields[0].name, sym_node_count);

    let cloned_reset_entry = td3.method_table.get(&sym_reset).unwrap();
    assert_ne!(cloned_reset_entry.fn_id, fn_id_a);
    let cloned_ast_id = *type_arena
        .method_ast_nodes
        .get(&cloned_reset_entry.fn_id)
        .expect("Expected cloned AST node");
    assert_ne!(
        cloned_ast_id, reset_fn_ast,
        "Expected cloned AST subtree for renamed bundle method"
    );

    let fn_node = *ast_arena.node(cloned_ast_id);
    let fn_children = ast_arena.children_slice(fn_node.rhs, fn_node.flags as u32);
    let block_node = *ast_arena.node(NodeId(fn_children[0]));
    let block_children = ast_arena.children_slice(block_node.lhs, block_node.rhs);
    let assign_node = *ast_arena.node(NodeId(block_children[0]));
    let lhs_member = *ast_arena.node(NodeId(assign_node.lhs));
    assert_eq!(lhs_member.node_kind(), NodeKind::MemberExpr);
    assert_eq!(
        lhs_member.rhs, sym_node_count.0,
        "Expected `.count` / `self.count` to be rewritten to `node_count`"
    );
}

#[test]
fn test_partial_struct_layout_seal_enforcement() {
    let mut type_arena = TypeArena::new();
    let mut interner = pilot_intern::Interner::new();
    let span = ByteSpan::new(0, 25);
    let pkg_id = PackageId(0);

    // Primary struct declaration for `i32` (4 bytes, Value `0b00`, layout sealed)
    let mut i32_td = TypeDef::new(TypeId::I32, interner.intern("i32"), pkg_id);
    i32_td.is_lang_primitive = true;
    i32_td.size = 4;
    i32_td.align = 4;
    type_arena.register_type_def(i32_td, &interner);

    let mut diags = Vec::new();
    type_arena.seal_primary_struct_layout(TypeId::I32, &interner, &mut diags);
    assert!(diags.is_empty());
    assert!(type_arena.type_defs.get(&TypeId::I32).unwrap().layout_sealed);
    assert_eq!(type_arena.type_defs.get(&TypeId::I32).unwrap().size, 4);
    assert_eq!(type_arena.type_lattice(TypeId::I32), LatticeBits::Value);

    // 1. Attempting to declare a field in a subsequent `partial struct` block must emit
    // `PartialStructCannotSpliceFields`.
    let sym_extra = interner.intern("extra");
    type_arena.add_struct_field_checked(
        TypeId::I32,
        sym_extra,
        TypeId::I32,
        8,
        span,
        &interner,
        &mut diags,
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "PartialStructCannotSpliceFields");
    assert_eq!(type_arena.type_defs.get(&TypeId::I32).unwrap().size, 4);

    // 2. Attempting to splice a stateful `*Bundle` (fields > 0) after layout seal must emit
    // `PartialStructCannotSpliceFields`.
    diags.clear();
    let stateful_bundle = BundleDef {
        id: BundleId(10),
        name: interner.intern("StatefulCounterBundle"),
        fields: vec![FieldDef {
            name: interner.intern("counter"),
            ty: TypeId::I64,
            byte_offset: 0,
            visibility: 8,
            source_bundle: Some(BundleId(10)),
            span,
        }],
        methods: vec![],
        span,
    };
    assert!(!stateful_bundle.is_method_only_facet());
    type_arena.register_bundle_def(stateful_bundle, &interner);
    type_arena.apply_bundle_splice(
        TypeId::I32,
        &SpliceDirectives::new(BundleId(10), span),
        &interner,
        None,
        &mut diags,
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "PartialStructCannotSpliceFields");
    assert_eq!(type_arena.type_defs.get(&TypeId::I32).unwrap().size, 4);
    assert_eq!(type_arena.type_lattice(TypeId::I32), LatticeBits::Value);

    // 3. Splicing a zero-field `*Facet` (`fields.is_empty()`) inside a `partial struct` succeeds
    // without changing `size` or `lattice`.
    diags.clear();
    let sym_abs = interner.intern("abs");
    let signed_math_facet = BundleDef {
        id: BundleId(11),
        name: interner.intern("SignedMathFacet"),
        fields: vec![],
        methods: vec![MethodEntry {
            sym: sym_abs,
            fn_id: FunctionId(500),
            tier: MethodTier::Tier2BundleOrFacetSplice,
            source_origin: OriginId::Bundle(BundleId(11)),
            is_static: false,
            is_mutating_var: false,
            is_pure: true,
            span,
        }],
        span,
    };
    assert!(signed_math_facet.is_method_only_facet());
    type_arena.register_bundle_def(signed_math_facet, &interner);
    type_arena.apply_bundle_splice(
        TypeId::I32,
        &SpliceDirectives::new(BundleId(11), span),
        &interner,
        None,
        &mut diags,
    );
    assert!(
        diags.is_empty(),
        "Expected zero-field *Facet splice into sealed partial struct to succeed, got: {:?}",
        diags
    );
    let i32_after = type_arena.type_defs.get(&TypeId::I32).unwrap();
    assert_eq!(i32_after.size, 4);
    assert_eq!(i32_after.lattice, LatticeBits::Value);
    assert!(i32_after.method_table.contains_key(&sym_abs));
}

#[test]
fn test_enforce_core_naming_lint() {
    let mut interner = pilot_intern::Interner::new();
    let mut type_arena = TypeArena::new();
    let span = ByteSpan::new(0, 20);
    let pkg_id = PackageId(0);

    // 1. `[Lang("i32")] struct Int32` (not lowercase -> violates Rule 1)
    let sym_int32 = interner.intern("Int32");
    let ty_int32 = type_arena.alloc(TypeKind::Primitive("Int32".to_string()));
    let mut td_int32 = TypeDef::new(ty_int32, sym_int32, pkg_id);
    td_int32.is_lang_primitive = true;
    type_arena.register_type_def(td_int32, &interner);

    // 2. `interface Enumerable` (missing `I` prefix -> violates Rule 2)
    let sym_enumerable = interner.intern("Enumerable");
    let iface_id = type_arena.alloc_interface_id();
    type_arena.register_interface_def(
        pilot_analysis::InterfaceDef {
            id: iface_id,
            name: sym_enumerable,
            home_pkg: pkg_id,
            super_interfaces: vec![],
            explicit_lattice: None,
            required_contracts: vec![],
            default_methods: vec![],
            span,
        },
        &interner,
    );

    // 3. `struct LockBundle` with 0 fields (must end with `Facet` -> violates Rule 3)
    let sym_lock_bundle = interner.intern("LockBundle");
    type_arena.register_bundle_def(
        BundleDef {
            id: BundleId(1),
            name: sym_lock_bundle,
            fields: vec![],
            methods: vec![],
            span,
        },
        &interner,
    );

    // 4. `struct MathFacet` with 1 field (must end with `Bundle` -> violates Rule 3)
    let sym_math_facet = interner.intern("MathFacet");
    let sym_val_field = interner.intern("val");
    type_arena.register_bundle_def(
        BundleDef {
            id: BundleId(2),
            name: sym_math_facet,
            fields: vec![FieldDef {
                name: sym_val_field,
                ty: TypeId::I64,
                byte_offset: 0,
                visibility: 8,
                source_bundle: Some(BundleId(2)),
                span,
            }],
            methods: vec![],
            span,
        },
        &interner,
    );

    // 5. `struct Inline : IAttribute` (missing `Attr` suffix -> violates Rule 4)
    let sym_inline_bad = interner.intern("Inline");
    let ty_inline_bad = type_arena.alloc(TypeKind::Adt {
        name: "Inline".to_string(),
        variants: vec![],
    });
    let td_inline_bad = TypeDef::new(ty_inline_bad, sym_inline_bad, pkg_id);
    type_arena.register_type_def(td_inline_bad, &interner);

    // 6. `struct InlineAttr : IAttribute` (valid Rule 4 -> must pass and register `[Inline]`)
    let sym_inline_good = interner.intern("InlineAttr");
    let ty_inline_good = type_arena.alloc(TypeKind::Adt {
        name: "InlineAttr".to_string(),
        variants: vec![],
    });
    let td_inline_good = TypeDef::new(ty_inline_good, sym_inline_good, pkg_id);
    type_arena.register_type_def(td_inline_good, &interner);

    let mut pkg = PackageAst::new("Pilot.Core", true);
    pkg.attribute_structs.push(sym_inline_bad);
    pkg.attribute_structs.push(sym_inline_good);

    let mut diags: Vec<Diagnostic> = Vec::new();
    verify_core_naming_rules(&pkg, &mut type_arena, &interner, &mut diags);

    // Should have exactly 5 violations (for Int32, Enumerable, LockBundle, MathFacet, Inline)
    // and 0 violations for InlineAttr
    assert_eq!(
        diags.len(),
        5,
        "Expected 5 EnforceCoreNaming diagnostics, got: {:?}",
        diags
    );
    assert!(diags.iter().all(|d| d.code == "EnforceCoreNaming"));
    assert!(diags.iter().any(|d| d.message.contains("Int32")));
    assert!(diags.iter().any(|d| d.message.contains("Enumerable")));
    assert!(diags.iter().any(|d| d.message.contains("LockBundle")));
    assert!(diags.iter().any(|d| d.message.contains("MathFacet")));
    assert!(diags.iter().any(|d| d.message.contains("'Inline'")));

    // Verify `InlineAttr` registered short name `Inline` in `type_arena.registered_attributes`
    assert!(
        type_arena.registered_attributes.contains("Inline"),
        "Expected `struct InlineAttr : IAttribute` to register short attribute `[Inline]`"
    );
    assert!(
        type_arena.registered_attributes.contains("InlineAttr"),
        "Expected `struct InlineAttr : IAttribute` to register full attribute `[InlineAttr]`"
    );
}

#[test]
fn test_universal_rename_litmus_gate() {
    use pilot_analysis::{
        bootstrap_core_file, eval_file_with_tables, ComptimeEngine,
        ForLoopAnalyzer, ForLoweringStrategy, HardwarePhysics, LoopReturnMode,
        LoopScheduleMode, RegClass, StructuralShape, TypeInferenceEngine,
    };
    use pilot_codegen::{
        c_target::CTargetEmitter, gpu::GpuEmitter, llvm::LlvmEmitter, wasm::WasmEmitter,
    };
    use pilot_parser::{parse, ParserTables};

    // 1. Bootstrap core.pilot first so the 14-keyword syntax/Pratt/attribute tables are active,
    // then evaluate a self-contained module where EVERY primitive and composite type is renamed:
    // `bitflag` (`bool`), `word64` (`usize`), `sint64` (`i64`), `c32` (`char`), `addr<T>` (`ptr<T>`),
    // `window<T>` (`slice<T>`), `utf8view` (`str`), `maybe<T>` (`option<T>`),
    // `outcome<T, E>` (`result<T, E>`), and `interval<T>` (`range<T>`).
    let mut tables = ParserTables::new();
    let mut vm = ComptimeEngine::new();
    let core_src = include_str!("../../../stdlib/core/core.pilot");
    bootstrap_core_file(core_src, &mut tables, &mut vm)
        .expect("core.pilot must bootstrap with 0 errors");

    let renamed_src = r#"
        pub syntax type_construct {
            (T: type) []            => quote { window<$T> };
            (T: type) ?             => quote { maybe<$T> };
            (T: type) ! (E: type)   => quote { outcome<$T, $E> };
        }

        pub partial struct {
            [Bits(1,  3)] bitflag : IValue {
                pub pure op == (rhs: bitflag) -> bitflag => __ssa(12, self, rhs);
            }
            [Bits(64, 3)] word64  : IValue {
                pub pure op +  (rhs: word64)  -> word64  => __ssa(1, self, rhs);
            }
            [Bits(64, 2)] sint64  : IValue {
                pub pure op +  (rhs: sint64)  -> sint64  => __ssa(1, self, rhs);
                pub pure op -  (rhs: sint64)  -> sint64  => __ssa(2, self, rhs);
                pub pure op *  (rhs: sint64)  -> sint64  => __ssa(3, self, rhs);
            }
            [Bits(32, 3)] c32     : IValue {
                pub pure op == (rhs: c32)     -> bitflag => __ssa(12, self, rhs);
            }
            [Bits(64, 6)] addr<T: IAnyValue> : IReference {}

            [Layout(c_abi)]
            window<T: IAnyValue> : IReference {
                pub data: addr<T>;
                pub len:  word64;
            }

            [Layout(c_abi)]
            utf8view : IReference {
                pub data: addr<word64>;
                pub len:  word64;
            }

            [Layout("niche")]
            maybe<T: IAnyValue> : IValue {
                pub val:       T;
                pub has_value: bitflag;
            }

            [Layout("register_pair")]
            outcome<T: IAnyValue, E: IAnyValue> : IValue {
                pub ok_val:  T;
                pub err_val: E;
                pub is_ok:   bitflag;
            }

            [Layout(c_abi)]
            interval<T: IValue> : IValue {
                pub start: T;
                pub end:   T;
            }
        }

        pub struct SymbolHolder {
            pub w: sint64[];
            pub m: sint64?;
            pub o: sint64 ! word64;
        }
    "#;

    let renamed_arena = eval_file_with_tables(renamed_src, &mut tables, &mut vm)
        .expect("Renamed primitive/composite module must parse and evaluate with 0 errors");

    // Verify `syntax type_construct` re-bound `T[] -> window<T>`, `T? -> maybe<T>`, `T ! E -> outcome<T, E>`
    let holder_layout = vm
        .type_layouts
        .get("SymbolHolder")
        .expect("SymbolHolder must be registered");
    assert_eq!(
        holder_layout.field_types,
        vec![
            "window<sint64>".to_string(),
            "maybe<sint64>".to_string(),
            "outcome<sint64, word64>".to_string(),
        ]
    );

    // 2. Verify size, alignment, and 2-bit hardware lattice of renamed types via ComptimeEngine & TypeArena
    let (win_sz, win_align, win_lat) = vm.query_type_size_align_lattice("window<sint64>");
    assert_eq!(win_sz, 16);
    assert_eq!(win_align, 8);
    assert_eq!(win_lat, 0b01);

    let (u8v_sz, u8v_align, u8v_lat) = vm.query_type_size_align_lattice("utf8view");
    assert_eq!(u8v_sz, 16);
    assert_eq!(u8v_align, 8);
    assert_eq!(u8v_lat, 0b01);

    let (iv_sz, iv_align, iv_lat) = vm.query_type_size_align_lattice("interval<sint64>");
    assert_eq!(iv_sz, 16);
    assert_eq!(iv_align, 8);
    assert_eq!(iv_lat, 0b00);

    // Populate TypeArena with the renamed types and seal their layouts to deduce `StructuralShape`
    let mut interner = renamed_arena.interner.clone();
    let mut type_arena = TypeArena::new();
    let pkg_id = PackageId(0);
    let span = ByteSpan::new(0, 10);

    let ty_bitflag = type_arena.alloc(TypeKind::Primitive("bitflag".to_string()));
    let mut td_bitflag = TypeDef::new(ty_bitflag, interner.intern("bitflag"), pkg_id);
    td_bitflag.is_lang_primitive = true;
    td_bitflag.physics = HardwarePhysics {
        reg_class: RegClass::GprUnsigned,
        bit_width: 1,
    };
    td_bitflag.size = 1;
    td_bitflag.align = 1;
    type_arena.register_type_def(td_bitflag, &interner);

    let ty_word64 = type_arena.alloc(TypeKind::Primitive("word64".to_string()));
    let mut td_word64 = TypeDef::new(ty_word64, interner.intern("word64"), pkg_id);
    td_word64.is_lang_primitive = true;
    td_word64.physics = HardwarePhysics {
        reg_class: RegClass::GprUnsigned,
        bit_width: 64,
    };
    td_word64.size = 8;
    td_word64.align = 8;
    type_arena.register_type_def(td_word64, &interner);

    let ty_sint64 = type_arena.alloc(TypeKind::Primitive("sint64".to_string()));
    let mut td_sint64 = TypeDef::new(ty_sint64, interner.intern("sint64"), pkg_id);
    td_sint64.is_lang_primitive = true;
    td_sint64.physics = HardwarePhysics {
        reg_class: RegClass::GprSigned,
        bit_width: 64,
    };
    td_sint64.size = 8;
    td_sint64.align = 8;
    type_arena.register_type_def(td_sint64, &interner);

    let ty_c32 = type_arena.alloc(TypeKind::Primitive("c32".to_string()));
    let mut td_c32 = TypeDef::new(ty_c32, interner.intern("c32"), pkg_id);
    td_c32.is_lang_primitive = true;
    td_c32.physics = HardwarePhysics {
        reg_class: RegClass::GprUnsigned,
        bit_width: 32,
    };
    td_c32.size = 4;
    td_c32.align = 4;
    type_arena.register_type_def(td_c32, &interner);

    let ty_addr_sint64 = type_arena.alloc(TypeKind::Adt {
        name: "addr<sint64>".to_string(),
        variants: vec![],
    });
    let mut td_addr = TypeDef::new(ty_addr_sint64, interner.intern("addr"), pkg_id);
    td_addr.is_lang_primitive = true;
    td_addr.physics = HardwarePhysics {
        reg_class: RegClass::RawPtr,
        bit_width: 64,
    };
    td_addr.size = 8;
    td_addr.align = 8;
    td_addr.lattice = LatticeBits::Reference;
    td_addr.declared_interfaces.push(InterfaceId::IREFERENCE);
    type_arena.register_type_def(td_addr, &interner);

    // Seal `window<sint64>` and verify `StructuralShape::ContiguousView`
    let ty_window = type_arena.alloc(TypeKind::Adt {
        name: "window<sint64>".to_string(),
        variants: vec![],
    });
    let mut td_window = TypeDef::new(ty_window, interner.intern("window"), pkg_id);
    td_window.declared_interfaces.push(InterfaceId::IREFERENCE);
    type_arena.register_type_def(td_window, &interner);
    type_arena.add_struct_field(ty_window, interner.intern("data"), ty_addr_sint64, 8, span);
    type_arena.add_struct_field(ty_window, interner.intern("len"), ty_word64, 8, span);
    let mut diags = Vec::new();
    type_arena.seal_primary_struct_layout(ty_window, &interner, &mut diags);
    assert!(diags.is_empty());
    let win_def = type_arena.type_defs.get(&ty_window).unwrap();
    assert_eq!(win_def.size, 16);
    assert_eq!(win_def.align, 8);
    assert_eq!(win_def.lattice, LatticeBits::Reference);
    assert_eq!(
        win_def.shape,
        StructuralShape::ContiguousView {
            elem_ty: ty_addr_sint64
        }
    );

    // 3. Seal `maybe<sint64>` and `outcome<sint64, word64>` and verify `StructuralShape::TaggedOutcome` + `?` operator
    let ty_maybe = type_arena.alloc(TypeKind::Adt {
        name: "maybe<sint64>".to_string(),
        variants: vec![],
    });
    let mut td_maybe = TypeDef::new(ty_maybe, interner.intern("maybe"), pkg_id);
    td_maybe.layout_mode = Some("niche".to_string());
    td_maybe.declared_interfaces.push(InterfaceId::IVALUE);
    type_arena.register_type_def(td_maybe, &interner);
    type_arena.add_struct_field(ty_maybe, interner.intern("val"), ty_sint64, 8, span);
    type_arena.add_struct_field(ty_maybe, interner.intern("has_value"), ty_bitflag, 8, span);
    type_arena.seal_primary_struct_layout(ty_maybe, &interner, &mut diags);
    assert!(diags.is_empty());
    assert_eq!(
        type_arena.type_defs.get(&ty_maybe).unwrap().shape,
        StructuralShape::TaggedOutcome {
            ok_ty: ty_sint64,
            err_ty: None,
            niche_optimized: true,
        }
    );

    let ty_outcome = type_arena.alloc(TypeKind::Adt {
        name: "outcome<sint64, word64>".to_string(),
        variants: vec![],
    });
    let mut td_outcome = TypeDef::new(ty_outcome, interner.intern("outcome"), pkg_id);
    td_outcome.layout_mode = Some("register_pair".to_string());
    td_outcome.declared_interfaces.push(InterfaceId::IVALUE);
    type_arena.register_type_def(td_outcome, &interner);
    type_arena.add_struct_field(ty_outcome, interner.intern("ok_val"), ty_sint64, 8, span);
    type_arena.add_struct_field(ty_outcome, interner.intern("err_val"), ty_word64, 8, span);
    type_arena.add_struct_field(ty_outcome, interner.intern("is_ok"), ty_bitflag, 8, span);
    type_arena.seal_primary_struct_layout(ty_outcome, &interner, &mut diags);
    assert!(diags.is_empty());
    assert_eq!(
        type_arena.type_defs.get(&ty_outcome).unwrap().shape,
        StructuralShape::TaggedOutcome {
            ok_ty: ty_sint64,
            err_ty: Some(ty_word64),
            niche_optimized: false,
        }
    );

    // 4. Seal `interval<sint64>` and verify `StructuralShape::BoundsPair` + 3-tier `for` loop schedule
    let ty_interval = type_arena.alloc(TypeKind::Adt {
        name: "interval<sint64>".to_string(),
        variants: vec![],
    });
    let mut td_interval = TypeDef::new(ty_interval, interner.intern("interval"), pkg_id);
    td_interval.declared_interfaces.push(InterfaceId::IVALUE);
    type_arena.register_type_def(td_interval, &interner);
    type_arena.add_struct_field(ty_interval, interner.intern("start"), ty_sint64, 8, span);
    type_arena.add_struct_field(ty_interval, interner.intern("end"), ty_sint64, 8, span);
    type_arena.seal_primary_struct_layout(ty_interval, &interner, &mut diags);
    assert!(diags.is_empty());
    assert_eq!(
        type_arena.type_defs.get(&ty_interval).unwrap().shape,
        StructuralShape::BoundsPair {
            bound_ty: ty_sint64,
            inclusive: false,
        }
    );
    assert_eq!(
        ForLoopAnalyzer::worklist_frame_size_align(ty_sint64, &type_arena),
        (16, 8)
    );

    let p_bitflag = type_arena.type_physics(ty_bitflag);
    let p_word64 = type_arena.type_physics(ty_word64);
    let p_sint64 = type_arena.type_physics(ty_sint64);
    let p_c32 = type_arena.type_physics(ty_c32);
    let p_addr = type_arena.type_physics(ty_addr_sint64);
    let p_win = type_arena.type_physics(ty_window);

    let for_arena = parse("for x in iv { let y = x; }");
    let for_root = for_arena.node(for_arena.root);
    let for_id = NodeId(for_arena.children_slice(for_root.lhs, for_root.rhs)[0]);
    let iter_id = *for_arena.for_iter_nodes.get(&for_id.0).unwrap();
    let mut infer = TypeInferenceEngine::new();
    infer.types = type_arena;
    infer.inferred_node_types.insert(iter_id.0, ty_interval);
    let mut loop_analyzer = ForLoopAnalyzer::new();
    let strategy = loop_analyzer.analyze_for_expr(&for_arena, for_id, false, false, &mut infer);
    assert_eq!(
        strategy,
        ForLoweringStrategy::ThreeTierDynamicSchedule {
            schedule: LoopScheduleMode::FixedScalar,
            ret_mode: LoopReturnMode::StatementVoid,
            can_preallocate_yield_buffer: true,
        }
    );

    assert_eq!(LlvmEmitter::lower_physics_to_llvm(p_bitflag, &[]), "i1");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(p_word64, &[]), "i64");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(p_sint64, &[]), "i64");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(p_c32, &[]), "i32");
    assert_eq!(LlvmEmitter::lower_physics_to_llvm(p_addr, &["i64".to_string()]), "ptr");
    assert_eq!(
        LlvmEmitter::lower_physics_to_llvm(p_win, &["ptr".to_string(), "i64".to_string()]),
        "{ ptr, i64 }"
    );

    assert_eq!(CTargetEmitter::lower_physics_to_c(p_bitflag, &[]), "bool");
    assert_eq!(CTargetEmitter::lower_physics_to_c(p_word64, &[]), "uint64_t");
    assert_eq!(CTargetEmitter::lower_physics_to_c(p_sint64, &[]), "int64_t");
    assert_eq!(CTargetEmitter::lower_physics_to_c(p_c32, &[]), "uint32_t");
    assert_eq!(
        CTargetEmitter::lower_physics_to_c(
            p_win,
            &[("data", "int64_t*".to_string()), ("len", "uint64_t".to_string())]
        ),
        "struct { int64_t* data; uint64_t len; }"
    );

    assert_eq!(WasmEmitter::lower_physics_to_wasm(p_bitflag, &[]), "i32");
    assert_eq!(WasmEmitter::lower_physics_to_wasm(p_sint64, &[]), "i64");
    assert_eq!(
        WasmEmitter::lower_physics_to_wasm(p_win, &["i32".to_string(), "i32".to_string()]),
        "(i32, i32)"
    );

    assert_eq!(GpuEmitter::lower_physics_to_gpu(p_sint64, &[]), "long");
    assert_eq!(GpuEmitter::lower_physics_to_gpu(p_word64, &[]), "ulong");
    assert_eq!(
        GpuEmitter::lower_physics_to_gpu(
            p_win,
            &[("d", "long*".to_string()), ("l", "ulong".to_string())]
        ),
        "struct { long* d; ulong l; }"
    );
}
