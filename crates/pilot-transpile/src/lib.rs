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

    #[test]
    fn test_transpile_struct_and_class_with_methods() {
        let code = r#"
struct Point {
    x: int
    y: int
}

class Counter {
    val: int

    def inc() -> int {
        return self.val + 1;
    }
}
"#;
        let arena = pilot_parser::parse(code);
        let root = arena.root;
        let analysis = pilot_analysis::pipeline::AnalysisPipeline::analyze(&arena, root);
        let module = IrModule::from_ast(&arena, &analysis, "CounterMod");

        assert_eq!(module.type_decls.len(), 2);

        // 1. Rust
        let rs = Transpiler::emit_module(&module, TargetLanguage::Rust);
        assert!(rs.contains("pub struct Point {"));
        assert!(rs.contains("pub x: i64,"));
        assert!(rs.contains("pub y: i64,"));
        assert!(rs.contains("impl Counter {"));
        assert!(rs.contains("pub fn inc(&mut self) -> i64 {"));

        // 2. Python
        let py = Transpiler::emit_module(&module, TargetLanguage::Python);
        assert!(py.contains("class Point:"));
        assert!(py.contains("def __init__(self, x=None, y=None):"));
        assert!(py.contains("class Counter:"));
        assert!(py.contains("def inc(self):"));

        // 3. C#
        let cs = Transpiler::emit_module(&module, TargetLanguage::CSharp);
        assert!(cs.contains("public struct Point {"));
        assert!(cs.contains("public long x;"));
        assert!(cs.contains("public class Counter {"));
        assert!(cs.contains("public long inc() {"));

        // 4. C++
        let cpp = Transpiler::emit_module(&module, TargetLanguage::Cpp);
        assert!(cpp.contains("struct Point {"));
        assert!(cpp.contains("int64_t x;"));
        assert!(cpp.contains("class Counter {"));
        assert!(cpp.contains("int64_t inc() {"));

        // 5. JavaScript
        let js = Transpiler::emit_module(&module, TargetLanguage::JavaScript);
        assert!(js.contains("export class Point {"));
        assert!(js.contains("constructor(x, y) {"));
        assert!(js.contains("this.x = x;"));
        assert!(js.contains("export class Counter {"));
        assert!(js.contains("inc() {"));

        // 6. C
        let c = Transpiler::emit_module(&module, TargetLanguage::C);
        assert!(c.contains("typedef struct Point {"));
        assert!(c.contains("int64_t x;"));
        assert!(c.contains("} Point;"));

        // 7. Lua
        let lua = Transpiler::emit_module(&module, TargetLanguage::Lua);
        assert!(lua.contains("local Point = {}"));
        assert!(lua.contains("function Point.new(x, y)"));
    }

    #[test]
    fn test_transpile_modern_pilot_expressions() {
        let code = r#"
def process_data(limit: int) -> int {
    let items = [10, 20, 30];
    var sum = 0;
    for i in 0..<limit {
        sum = sum + i;
    }
    return sum;
}
"#;
        let arena = pilot_parser::parse(code);
        let root = arena.root;
        let analysis = pilot_analysis::pipeline::AnalysisPipeline::analyze(&arena, root);
        let module = IrModule::from_ast(&arena, &analysis, "DataMod");

        // Rust emitter
        let rs = Transpiler::emit_module(&module, TargetLanguage::Rust);
        assert!(rs.contains("vec![10, 20, 30]"));
        assert!(rs.contains("for i in 0..limit {"));

        // Python emitter
        let py = Transpiler::emit_module(&module, TargetLanguage::Python);
        assert!(py.contains("[10, 20, 30]"));
        assert!(py.contains("for i in range(0, limit):"));

        // C# emitter
        let cs = Transpiler::emit_module(&module, TargetLanguage::CSharp);
        assert!(cs.contains("[10, 20, 30]"));
        assert!(cs.contains("for (long i = 0; i < limit; i++) {"));

        // C++ emitter
        let cpp = Transpiler::emit_module(&module, TargetLanguage::Cpp);
        assert!(cpp.contains("std::vector{10, 20, 30}"));
        assert!(cpp.contains("for (int64_t i = 0; i < limit; i++) {"));

        // JS emitter
        let js = Transpiler::emit_module(&module, TargetLanguage::JavaScript);
        assert!(js.contains("[10, 20, 30]"));
        assert!(js.contains("for (let i = 0; i < limit; i++) {"));
    }

    #[test]
    fn test_ingest_classes_and_methods() {
        let opts = IngestOptions {
            strict_pilot: true,
            viable_license_only: true,
            license_spdx: Some("Apache-2.0".to_string()),
        };

        // 1. Python class ingestion
        let py_cls = "class Counter:\n    def __init__(self, val):\n        self.val = val\n\n    def inc(self):\n        self.val = self.val + 1\n        return self.val\n";
        let pilot_py = ForeignAstIngester::ingest_to_pilot_source(py_cls, TargetLanguage::Python, &opts).unwrap();
        assert!(pilot_py.contains("class Counter {"));
        assert!(pilot_py.contains("def init(val) {"));
        assert!(pilot_py.contains("def inc() {"));

        // 2. JavaScript class ingestion
        let js_cls = "class Vector {\n    constructor(x, y) {\n        this.x = x;\n        this.y = y;\n    }\n}\n";
        let pilot_js = ForeignAstIngester::ingest_to_pilot_source(js_cls, TargetLanguage::JavaScript, &opts).unwrap();
        assert!(pilot_js.contains("class Vector {"));
        assert!(pilot_js.contains("def init(x, y) {"));

        // 3. C# class ingestion
        let cs_cls = "public class Greeter {\n    public static void SayHello(string name) {\n        Console.WriteLine(name);\n    }\n}\n";
        let pilot_cs = ForeignAstIngester::ingest_to_pilot_source(cs_cls, TargetLanguage::CSharp, &opts).unwrap();
        assert!(pilot_cs.contains("class Greeter {"));
        assert!(pilot_cs.contains("def SayHello(string name) {"));
        assert!(pilot_cs.contains("println(name);"));
    }
}
