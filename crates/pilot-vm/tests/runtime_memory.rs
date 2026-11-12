//! Memory the seed VM keeps while a program runs.
//!
//! - Strings the VM makes at run time are freed once nothing refers to them
//!   (`pilot_vm::strings`): a loop that builds a string one piece at a time
//!   keeps the result, not every intermediate string.
//! - A string still reachable (from a register, a saved frame, a list or a
//!   record) survives every collection with its text intact.
//! - Lists, records and closure environments nothing reaches are freed, and
//!   their slots are reused; a block's locals stop holding values when the
//!   block ends.
//! - Reading a freed slot is an internal error, never a read of another value.
//! - Loading a program starts from an empty record heap.
//! - A module header is read again once its file changes.
//!
//! Expected values are worked out by hand in each test.

use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

/// Compiles and runs `src`, returning the VM (still holding the result's
/// strings) and the value of the program's last expression.
fn run(src: &str) -> (PilVirtualMachine, PilotVal) {
    let arena = pilot_parser::parse(src);
    assert!(arena.diagnostics.is_empty(), "program must parse: {:?}", arena.diagnostics);
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    let msgs: Vec<&str> = bin.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert!(msgs.is_empty(), "program must compile: {msgs:?}");
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(bin);
    let out = vm.run();
    if let Some(fault) = vm.fault.take() {
        panic!("program faulted: {}", fault.message);
    }
    (vm, out)
}

fn text(v: PilotVal) -> String {
    v.with_str(str::to_string).unwrap_or_else(|| panic!("not a string: {v}"))
}

/// Appending one byte 20,000 times makes strings of every length from 15 to
/// 20,000 bytes (shorter ones are stored inline): 200,009,895 bytes in all
/// (20,000 * 20,001 / 2 - 14 * 15 / 2). Only the last one is still
/// reachable, so the VM must hold far less than that.
#[test]
fn building_a_string_keeps_only_what_is_reachable() {
    let (vm, out) = run(
        "var out = \"\";\n\
         var i = 0;\n\
         while i < 20000 { out = out + \"x\"; i = i + 1; }\n\
         out;\n",
    );
    assert_eq!(text(out).len(), 20_000);
    let held = vm.strings.bytes();
    assert!(held < 16 << 20, "the VM still holds {held} bytes of strings");
}

/// About 25 MB of garbage (strings of 2 to 10,000 bytes, step 2: 25,005,000
/// bytes) forces collections while strings are held in a list, a record and
/// a caller's saved registers; all of them read back unchanged.
#[test]
fn reachable_strings_survive_collection() {
    let (_vm, out) = run(
        "fn churn() -> i64 {\n\
             var g = \"\";\n\
             var j = 0;\n\
             while j < 5000 { g = g + \"yy\"; j = j + 1; }\n\
             return 1;\n\
         }\n\
         fn hold() -> str {\n\
             let s = \"held-across-a-call-\" + 42;\n\
             churn();\n\
             return s;\n\
         }\n\
         let xs = __engine(\"list_new\");\n\
         var i = 0;\n\
         while i < 300 { __engine(\"list_push\", xs, \"long-string-number-\" + i); i = i + 1; }\n\
         let m = __engine(\"map_new\");\n\
         __engine(\"map_set\", m, \"k\", \"kept-in-a-record-\" + 7);\n\
         let held = hold();\n\
         var all = \"\";\n\
         var k = 0;\n\
         while k < 300 { all = all + __engine(\"list_get\", xs, k) + \",\"; k = k + 1; }\n\
         all + __engine(\"map_get\", m, \"k\") + \"|\" + held;\n",
    );
    let mut expected = String::new();
    for i in 0..300 {
        expected.push_str(&format!("long-string-number-{i},"));
    }
    expected.push_str("kept-in-a-record-7|held-across-a-call-42");
    assert_eq!(text(out), expected);
}

/// A VM reused for a second program (the FFI's `pilot_vm_execute_bytes`)
/// keeps none of the first program's records, and the first program's
/// result stays readable.
#[test]
fn loading_a_program_starts_with_an_empty_record_heap() {
    let first = pilot_parser::parse(
        "var i = 0;\n\
         while i < 50 { let m = __engine(\"map_new\"); __engine(\"map_set\", m, \"n\", i); i = i + 1; }\n\
         \"first-program-result-\" + i;\n",
    );
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&first, first.root));
    let kept = vm.run();
    assert!(vm.heap_maps.len() > 50, "the first program builds 50 records");
    let second = pilot_parser::parse("1 + 1;\n");
    vm.load_binary(BytecodeCompiler::compile(&second, second.root));
    assert_eq!(vm.heap_maps.len(), 1, "only the environment record is left");
    assert_eq!(vm.run().as_int(), Some(2));
    assert_eq!(text(kept), "first-program-result-50");
}

/// `unit_header` lists a file's `pkg` and `use` declarations; after the file
/// gains a `use`, the next read sees three declarations, not the two read
/// before.
#[test]
fn a_module_header_is_read_again_after_its_file_changes() {
    let path = std::env::temp_dir().join(format!("pilot_vm_header_{}.pilot", std::process::id()));
    let path_text = path.to_string_lossy().replace('\\', "/");
    let count = |vm_src: &str| -> i64 {
        let (_vm, out) = run(vm_src);
        out.as_int().expect("an integer")
    };
    let program = format!("__engine(\"str_len\", __engine(\"unit_header\", \"{path_text}\"));\n");
    std::fs::write(&path, "pkg Probe.Header;\nuse std.col;\n\nfn f() {}\n").expect("write");
    assert_eq!(count(&program), 2);
    std::fs::write(&path, "pkg Probe.Header;\nuse std.col;\nuse std.text;\n\nfn f() {}\n").expect("write");
    assert_eq!(count(&program), 3);
    let _ = std::fs::remove_file(&path);
}

/// Each of 200,000 iterations makes a record holding a 3-element list: 400,000
/// slots without freeing. Only the current iteration's pair is reachable, so
/// the VM must end with far fewer slots than that.
#[test]
fn records_and_lists_out_of_scope_are_freed() {
    let (vm, out) = run(
        "fn make(i: i64) -> i64 {\n\
             let m = __engine(\"map_new\");\n\
             __engine(\"map_set\", m, \"items\", [i, i + 1, i + 2]);\n\
             let xs = __engine(\"map_get\", m, \"items\");\n\
             return __engine(\"list_get\", xs, 2);\n\
         }\n\
         var total = 0;\n\
         var i = 0;\n\
         while i < 200000 { total = total + make(i); i = i + 1; }\n\
         total;\n",
    );
    // The sum of i + 2 for i below 200,000: 199,999 * 200,000 / 2 + 400,000.
    assert_eq!(out.as_int(), Some(20_000_300_000));
    let slots = vm.heap_lists.len() + vm.heap_maps.len();
    assert!(slots < 150_000, "the VM still has {slots} list and record slots");
    assert!(vm.heap.collections() > 0);
}

/// A list built in a block is unreachable once the block ends, even though
/// the code around it goes on: after enough garbage to force a collection,
/// no 100,000-element list is left.
#[test]
fn a_block_local_is_freed_after_its_block_ends() {
    let (vm, out) = run(
        "fn churn() -> i64 {\n\
             var j = 0;\n\
             while j < 100000 { let t = __engine(\"list_new\"); __engine(\"list_push\", t, j); j = j + 1; }\n\
             return j;\n\
         }\n\
         var kept = 0;\n\
         if true {\n\
             let big = __engine(\"list_new\");\n\
             var k = 0;\n\
             while k < 100000 { __engine(\"list_push\", big, k); k = k + 1; }\n\
             kept = __engine(\"list_len\", big);\n\
         }\n\
         churn();\n\
         kept;\n",
    );
    assert_eq!(out.as_int(), Some(100_000));
    let longest = vm.heap_lists.iter().map(Vec::len).max().unwrap_or(0);
    assert!(longest < 100_000, "a list of {longest} elements is still held");
}

/// Lists reachable only through a record (filled in a call whose registers
/// are gone), a closure's environment and a caller's saved registers survive
/// collections whole.
#[test]
fn reachable_lists_and_records_survive_collection() {
    let (_vm, out) = run(
        "fn churn() -> i64 {\n\
             var j = 0;\n\
             while j < 150000 { let t = __engine(\"map_new\"); __engine(\"map_set\", t, \"j\", [j]); j = j + 1; }\n\
             return j;\n\
         }\n\
         fn hold() -> i64 {\n\
             let mine = [7, 8, 9];\n\
             churn();\n\
             return __engine(\"list_get\", mine, 0) * 100 + __engine(\"list_get\", mine, 2);\n\
         }\n\
         fn fill(r: i64) -> i64 { __engine(\"map_set\", r, \"inner\", [1, 2, 3]); return 0; }\n\
         let rec = __engine(\"map_new\");\n\
         fill(rec);\n\
         let captured = [40, 2];\n\
         let f = (x) => x + __engine(\"list_get\", captured, 0) + __engine(\"list_get\", captured, 1);\n\
         let held = hold();\n\
         churn();\n\
         let inner = __engine(\"map_get\", rec, \"inner\");\n\
         held * 1000 + f(__engine(\"list_get\", inner, 2));\n",
    );
    // hold() gives 7 * 100 + 9 = 709; f(3) = 3 + 40 + 2 = 45.
    assert_eq!(out.as_int(), Some(709_045));
}

/// A handle the roots do not reach reads as an internal error once its slot
/// is freed, not as an empty list.
#[test]
fn reading_a_freed_list_is_an_internal_error() {
    let mut vm = PilVirtualMachine::new();
    let orphan = vm.new_list(vec![PilotVal::from_int(1)]);
    vm.collect();
    let err = vm.intrinsic("list_len", &[orphan]).expect_err("the slot was freed");
    assert!(err.contains("freed"), "error: {err}");
}
