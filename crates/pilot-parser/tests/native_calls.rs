//! The seed's native calls come from `stdlib/core/builtins/native_calls.pilot`.

use pilot_parser::{bootstrap_tables, NativeKind};

#[test]
fn native_bindings_are_ingested_from_pilot() {
    let t = bootstrap_tables();
    assert_eq!(t.native(NativeKind::Function, "print"), Some("rt_print"));
    assert_eq!(t.native(NativeKind::Method, "get"), Some("map_get"));
    assert_eq!(t.native(NativeKind::Method, "set"), Some("map_set"));
    assert_eq!(t.native(NativeKind::Method, "remove"), Some("map_remove"));
    assert_eq!(t.native(NativeKind::Property, "len"), Some("str_len"));
    assert_eq!(t.native(NativeKind::Value, "ENV"), Some("get_env"));
    assert_eq!(t.native(NativeKind::TypeQuery, "sizeof"), Some("byte_size"));
    assert!(t.quotes_symbol("nameof"));
}
