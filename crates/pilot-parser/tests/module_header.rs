//! `parse_header` reads the `pkg` and `use` declarations at the head of a
//! file and stops at the first other item.

use pilot_parser::parse_header;

#[test]
fn header_lists_package_and_imports_in_order() {
    let src = "[EnforceCoreNaming]\npub lvl(0) pkg Demo.Shapes;\n\nuse Demo.Core.*;\nuse std.col as col;\n\nfn area() -> i64 => 1;\nuse Late.Import;\n";
    let decls = parse_header(src);
    let got: Vec<(bool, &str, &str)> = decls.iter().map(|d| (d.is_package, d.path.as_str(), d.alias.as_str())).collect();
    assert_eq!(got, vec![(true, "Demo.Shapes", ""), (false, "Demo.Core.*", ""), (false, "std.col", "col")]);
    assert_eq!(&src[decls[1].offset as usize..decls[1].offset as usize + 3], "use");
}

#[test]
fn header_of_a_file_without_module_declarations_is_empty() {
    assert!(parse_header("fn main() { println(1); }\n").is_empty());
}

#[test]
fn selected_names_stay_in_one_declaration() {
    let decls = parse_header("use Demo.Shapes.{Circle, Square};\nuse std.col;\n");
    let paths: Vec<&str> = decls.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(paths, vec!["Demo.Shapes.{Circle,Square}", "std.col"]);
}
