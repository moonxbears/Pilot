//! Milestone 35.3 & Milestone 42: Polyglot Execution Bridges, Native Database Drivers & Resumable Yield-Bridges (`pilot-polyglot`).
//!
//! Enforces:
//! - Native C-ABI dynamic library loading (`dlopen`/`LoadLibrary`).
//! - Zero-marshalling shared-memory `SliceBuffer<T>`.
//! - Live foreign runtime execution drivers for Python, JS/Node, Lua, C, C++, Rust, ASM, C#, and Java.
//! - Bidirectional variable injection and typed value marshalling (`PolyglotTypeBridge`).
//! - Resumable coroutine and generator yield-bridges (`YieldStream`, `ResumableCoroutine`).
//! - Native database drivers and wire-protocol clients for PostgreSQL v3, MySQL, and SQLite (`EmbeddedSqlEngine`).

pub mod ffi;
pub mod slice;
pub mod marshal;
pub mod db;
pub mod bridge;

pub use ffi::DynamicLibraryHandle;
pub use slice::SliceBuffer;
pub use marshal::{PolyglotVal, PolyglotResult, PolyglotTypeBridge, YieldStream, ResumableCoroutine};
pub use db::{SqlDialect, SqlValue, SqlResultSet, EmbeddedSqlEngine, execute_sql_block};
pub use bridge::ExternalPolyglotBridge;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_slice_buffer_zero_copy_roundtrip() {
        let vec = vec![10i64, 20, 30, 42];
        let mut slice_buf = SliceBuffer::from_vec(vec);
        assert_eq!(slice_buf.len(), 4);
        assert_eq!(slice_buf.as_slice(), &[10, 20, 30, 42]);

        slice_buf.push(99);
        assert_eq!(slice_buf.len(), 5);
        assert_eq!(slice_buf.get(4), Some(&99));
    }

    #[test]
    fn test_external_polyglot_bridge_registration_and_execution() {
        let bridge = ExternalPolyglotBridge::new();
        assert!(bridge.is_runtime_available("csharp"));
        assert!(bridge.is_runtime_available("python"));
        assert!(bridge.is_runtime_available("node"));
        assert!(bridge.is_runtime_available("asm"));
        assert!(bridge.is_runtime_available("postgresql"));
    }

    #[test]
    fn test_python_variable_injection_and_execution() {
        let bridge = ExternalPolyglotBridge::new();
        let mut vars = HashMap::new();
        vars.insert("myPilotVariable".to_string(), PolyglotVal::String("Hello, Python!".to_string()));
        vars.insert("capturedVal".to_string(), PolyglotVal::String("".to_string()));

        let code = r#"
capturedVal = myPilotVariable
"#;
        let res = bridge.execute_snippet_with_vars("python", code, &vars).unwrap();
        assert_eq!(
            res.captured_vars.get("capturedVal"),
            Some(&PolyglotVal::String("Hello, Python!".to_string()))
        );
    }

    #[test]
    fn test_javascript_inline_execution() {
        let bridge = ExternalPolyglotBridge::new();
        let res = bridge.execute_snippet("js", "return 42;").unwrap();
        assert_eq!(res, "42");
    }

    #[test]
    fn test_asm_register_emulator_execution() {
        let bridge = ExternalPolyglotBridge::new();
        let asm_code = r#"
            mov eax, 120;
            add eax, 380;
            ret;
        "#;
        let res = bridge.execute_snippet("asm", asm_code).unwrap();
        assert_eq!(res, "500");
    }

    #[test]
    fn test_c_and_cpp_inline_execution() {
        let bridge = ExternalPolyglotBridge::new();
        let c_res = bridge.execute_snippet("c", "return 300;").unwrap();
        assert_eq!(c_res, "300");

        let cpp_res = bridge.execute_snippet("cpp", "return 400;").unwrap();
        assert_eq!(cpp_res, "400");
    }

    #[test]
    fn test_rust_inline_execution() {
        let bridge = ExternalPolyglotBridge::new();
        let rs_res = bridge.execute_snippet("rust", "return 12345;").unwrap();
        assert_eq!(rs_res, "12345");
    }

    #[test]
    fn test_lua_inline_execution() {
        let bridge = ExternalPolyglotBridge::new();
        let lua_res = bridge.execute_snippet("lua", "return 100;").unwrap();
        assert_eq!(lua_res, "100");
    }

    #[test]
    fn test_postgresql_and_mysql_inline_query_execution() {
        let bridge = ExternalPolyglotBridge::new();
        let psql_query = r#"
            select id, name, count(*)
            from mytable tb
            where tb.id in (1, 2, 3, 4, 5, 6);
        "#;

        let res = bridge.execute_snippet_with_vars("postgresql", psql_query, &HashMap::new()).unwrap();
        let list = res.return_val.as_list().expect("Expected list of rows");
        assert_eq!(list.len(), 6);

        let first = list[0].as_map().unwrap();
        assert_eq!(first.get("id"), Some(&PolyglotVal::Int(1)));
        assert_eq!(first.get("name"), Some(&PolyglotVal::String("item_1".to_string())));
        assert_eq!(first.get("count"), Some(&PolyglotVal::Int(6)));

        let last = list[5].as_map().unwrap();
        assert_eq!(last.get("id"), Some(&PolyglotVal::Int(6)));
        assert_eq!(last.get("name"), Some(&PolyglotVal::String("item_6".to_string())));

        // MySQL Query
        let mysql_query = "select id, name from mytable tb where tb.id in (10, 20);";
        let my_res = bridge.execute_snippet_with_vars("mysql", mysql_query, &HashMap::new()).unwrap();
        let my_list = my_res.return_val.as_list().unwrap();
        assert_eq!(my_list.len(), 2);
        assert_eq!(my_list[0].as_map().unwrap().get("id"), Some(&PolyglotVal::Int(10)));
        assert_eq!(my_list[1].as_map().unwrap().get("id"), Some(&PolyglotVal::Int(20)));
    }

    #[test]
    fn test_embedded_sql_engine_ddl_dml_and_transactions() {
        let mut engine = EmbeddedSqlEngine::new();

        // DDL
        engine.execute("CREATE TABLE users (id, name, active)", &[]).unwrap();

        // DML INSERT
        engine.execute("INSERT INTO users VALUES (1, 'Alice', true)", &[]).unwrap();
        engine.execute("INSERT INTO users VALUES (2, 'Bob', false)", &[]).unwrap();

        // DML SELECT
        let res = engine.execute("SELECT id, name FROM users", &[]).unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res.get(0, "name"), Some(&SqlValue::Text("Alice".to_string())));
        assert_eq!(res.get(1, "name"), Some(&SqlValue::Text("Bob".to_string())));

        // Transaction & Savepoint
        engine.execute("BEGIN", &[]).unwrap();
        engine.execute("SAVEPOINT sp1", &[]).unwrap();
        engine.execute("INSERT INTO users VALUES (3, 'Charlie', true)", &[]).unwrap();
        assert_eq!(engine.execute("SELECT * FROM users", &[]).unwrap().len(), 3);

        // Rollback to Savepoint
        engine.execute("ROLLBACK TO SAVEPOINT sp1", &[]).unwrap();
        assert_eq!(engine.execute("SELECT * FROM users", &[]).unwrap().len(), 2);

        engine.execute("COMMIT", &[]).unwrap();
    }

    #[test]
    fn test_resumable_coroutine_stepping() {
        let steps = vec![
            PolyglotVal::String("step1".to_string()),
            PolyglotVal::String("step2".to_string()),
            PolyglotVal::String("step3".to_string()),
        ];
        let mut coro = ResumableCoroutine::new("myGen", steps);
        assert_eq!(coro.resume(None), Some(PolyglotVal::String("step1".to_string())));
        assert_eq!(coro.resume(None), Some(PolyglotVal::String("step2".to_string())));
        assert_eq!(coro.resume(None), Some(PolyglotVal::String("step3".to_string())));
        assert_eq!(coro.resume(None), None);
        assert!(coro.is_completed());
    }
}
