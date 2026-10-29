//! Milestone 35.2: Native Rust Bidirectional Transpilers (`pilot-transpile`).
//!
//! Emits clean C, C++, C#, Rust, Python, JavaScript, and Lua code with zero external GC dependencies.

pub mod ir;
pub mod emitters;
pub mod licensing;
pub mod ingest;

pub use ir::{IrExpr, IrFunction, IrModule, IrStmt, IrType, TargetLanguage};
pub use emitters::Transpiler;
pub use licensing::LicenseVerifier;
pub use ingest::{ForeignAstIngester, IngestOptions};

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_module() -> IrModule {
        IrModule {
            name: "MathRoutines".to_string(),
            type_decls: Vec::new(),
            functions: vec![IrFunction {
                name: "calculate".to_string(),
                ret_type: IrType::Int,
                params: vec![("a".to_string(), IrType::Int), ("b".to_string(), IrType::Int)],
                body: vec![
                    IrStmt::Let {
                        name: "sum".to_string(),
                        ty: IrType::Int,
                        init: IrExpr::Binary {
                            op: "+".to_string(),
                            lhs: Box::new(IrExpr::Var("a".to_string())),
                            rhs: Box::new(IrExpr::Var("b".to_string())),
                        },
                    },
                    IrStmt::Return(Some(IrExpr::Binary {
                        op: "*".to_string(),
                        lhs: Box::new(IrExpr::Var("sum".to_string())),
                        rhs: Box::new(IrExpr::LitInt(2)),
                    })),
                ],
            }],
        }
    }

    #[test]
    fn test_transpile_to_all_seven_languages() {
        let module = sample_module();

        // 1. Rust
        let rs = Transpiler::emit_module(&module, TargetLanguage::Rust);
        assert!(rs.contains("pub fn calculate(a: i64, b: i64) -> i64 {"));
        assert!(rs.contains("let sum = (a + b);"));

        // 2. C
        let c = Transpiler::emit_module(&module, TargetLanguage::C);
        assert!(c.contains("int64_t calculate(int64_t a, int64_t b) {"));
        assert!(c.contains("int64_t sum = (a + b);"));

        // 3. C++
        let cpp = Transpiler::emit_module(&module, TargetLanguage::Cpp);
        assert!(cpp.contains("int64_t calculate(int64_t a, int64_t b) {"));
        assert!(cpp.contains("auto sum = (a + b);"));

        // 4. C#
        let cs = Transpiler::emit_module(&module, TargetLanguage::CSharp);
        assert!(cs.contains("public static class MathRoutines {"));
        assert!(cs.contains("public static long calculate(long a, long b) {"));

        // 5. Python
        let py = Transpiler::emit_module(&module, TargetLanguage::Python);
        assert!(py.contains("def calculate(a, b):"));
        assert!(py.contains("sum = (a + b)"));

        // 6. JavaScript
        let js = Transpiler::emit_module(&module, TargetLanguage::JavaScript);
        assert!(js.contains("export function calculate(a, b) {"));

        // 7. Lua
        let lua = Transpiler::emit_module(&module, TargetLanguage::Lua);
        assert!(lua.contains("local function calculate(a, b)"));
        assert!(lua.contains("end"));
    }

    #[test]
    fn test_spdx_license_verification() {
        assert!(LicenseVerifier::is_viable_license("MIT"));
        assert!(LicenseVerifier::is_viable_license("Apache-2.0"));
        assert!(LicenseVerifier::is_viable_license("BSD-3-Clause"));
        assert!(!LicenseVerifier::is_viable_license("GPL-3.0"));
        assert!(!LicenseVerifier::is_viable_license("Proprietary"));
    }

    #[test]
    fn test_ast_to_transpile_ir_lowerer() {
        let code = "def add_nums(x: int, y: int) -> int { return x + y; }";
        let arena = pilot_parser::parse(code);
        let root = arena.root;
        let analysis = pilot_analysis::pipeline::AnalysisPipeline::analyze(&arena, root);

        let module = IrModule::from_ast(&arena, &analysis, "test_mod");
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "add_nums");
        assert_eq!(module.functions[0].params.len(), 2);

        // Verify lowering to Python
        let py = Transpiler::emit_module(&module, TargetLanguage::Python);
        assert!(py.contains("def add_nums(x, y):"));
        assert!(py.contains("return (x + y)"));
    }

    #[test]
    fn test_foreign_ast_ingesters_and_strict_pilot() {
        let opts = IngestOptions {
            strict_pilot: true,
            viable_license_only: true,
            license_spdx: Some("MIT".to_string()),
        };

        // 1. Rust ingestion
        let rs_src = "pub fn compute(val: i64) -> Option<i64> {\n    let mut x = val;\n    x\n}";
        let pilot_rs = ForeignAstIngester::ingest_to_pilot_source(rs_src, TargetLanguage::Rust, &opts).unwrap();
        assert!(pilot_rs.contains("def compute"));
        assert!(pilot_rs.contains("var x = val;"));

        // 2. Python ingestion
        let py_src = "def process(item):\n    x = item\n    return x\n";
        let pilot_py = ForeignAstIngester::ingest_to_pilot_source(py_src, TargetLanguage::Python, &opts).unwrap();
        assert!(pilot_py.contains("def process(item) {"));
        assert!(pilot_py.contains("let x = item"));

        // 3. JavaScript ingestion
        let js_src = "function greet(name) {\n    const msg = name;\n    console.log(msg);\n}";
        let pilot_js = ForeignAstIngester::ingest_to_pilot_source(js_src, TargetLanguage::JavaScript, &opts).unwrap();
        assert!(pilot_js.contains("def greet(name)"));
        assert!(pilot_js.contains("let msg = name;"));
        assert!(pilot_js.contains("println(msg);"));
    }
}
