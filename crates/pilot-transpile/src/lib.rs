//! Milestone 35.2: Native Rust Bidirectional Transpilers (`pilot-transpile`).
//!
//! Emits clean C, C++, C#, Rust, Python, JavaScript, and Lua code with zero external GC dependencies.

pub mod ir;
pub mod emitters;
pub mod licensing;

pub use ir::{IrExpr, IrFunction, IrModule, IrStmt, IrType, TargetLanguage};
pub use emitters::Transpiler;
pub use licensing::LicenseVerifier;

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_module() -> IrModule {
        IrModule {
            name: "MathRoutines".to_string(),
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
}
