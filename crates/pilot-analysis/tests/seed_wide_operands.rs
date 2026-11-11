//! The seed VM addresses programs past 65,535 instructions and 65,535
//! constants: jump targets, call entries and constant indices are 32-bit
//! operands (`.pilc` version 5), so a program that size runs instead of
//! being refused.
//!
//! The expected values are worked by hand from the generated source.

use pilot_parser::parse;
use pilot_vm::pilc::PilcBinary;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

/// `parts` functions of `per_part` statements `x = x + k;` with `k` counting
/// from 1, called from the end of the script, then a loop and a call to a
/// function declared last (so both sit past the old 16-bit limit).
fn generated_script(parts: usize, per_part: usize, distinct: bool) -> String {
    let mut src = String::new();
    let mut k = 1u64;
    for j in 0..parts {
        src.push_str(&format!("fn part{j}() -> i64 {{\n    var x = 0;\n"));
        for _ in 0..per_part {
            let add = if distinct { k } else { 2 };
            src.push_str(&format!("    x = x + {add};\n"));
            k += 1;
        }
        src.push_str("    return x;\n}\n");
    }
    src.push_str("fn late(a: i64) -> i64 => a + 1;\n");
    src.push_str("var total = 0;\n");
    for j in 0..parts {
        src.push_str(&format!("total = total + part{j}();\n"));
    }
    src.push_str("var i = 0;\nwhile i < 3 { i = i + 1; }\n");
    src.push_str("total + i * 1000000000000 + late(6) * 10000000000000;\n");
    src
}

fn compile(src: &str) -> PilcBinary {
    let arena = parse(src);
    let binary = BytecodeCompiler::compile(&arena, arena.root);
    assert!(binary.diagnostics.is_empty(), "compile errors: {:?}", binary.diagnostics);
    binary
}

fn run(binary: PilcBinary) -> String {
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(binary);
    vm.run().to_string_repr()
}

#[test]
fn jumps_and_calls_past_sixty_five_thousand_instructions() {
    // 4 parts x 9,000 statements, each adding 2: total 72,000.
    let binary = compile(&generated_script(4, 9_000, false));
    assert!(binary.instructions.len() > 65_535, "only {} instructions", binary.instructions.len());
    // total + 3 * 10^12 + 7 * 10^13
    assert_eq!(run(binary), "73000000072000");
}

#[test]
fn constant_indices_past_sixty_five_thousand() {
    // 2 parts x 33,000 statements adding 1..=66,000: 66,000 * 66,001 / 2.
    let binary = compile(&generated_script(2, 33_000, true));
    assert!(binary.constants.len() > 65_535, "only {} constants", binary.constants.len());
    assert_eq!(run(binary), "73002178033000");
}

#[test]
fn wide_programs_survive_the_pilc_round_trip() {
    let binary = compile(&generated_script(4, 9_000, false));
    let bytes = binary.serialize();
    let back = PilcBinary::load_from_bytes(&bytes).expect("version 5 image loads");
    assert_eq!(back.instructions.len(), binary.instructions.len());
    assert!(back.instructions.iter().zip(&binary.instructions).all(|(a, b)| a == b));
    assert_eq!(run(back), "73000000072000");
}
