//! Register layouts reach the bootstrap tables from the `[Bits(..)]`
//! attributes in the Pilot sources, through the `Bits` attribute function.

use pilot_parser::bootstrap_tables;
use std::fs;
use std::path::Path;

/// `(type, width token, class token)` for each `[.., Bits(w, c)] .. struct T` in `src`.
fn declared_layouts(src: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let Some(at) = line.find("Bits(") else { continue };
        if !line.trim_start().starts_with('[') {
            continue;
        }
        let inner = &line[at + 5..];
        let Some(end) = inner.find(')') else { continue };
        let mut parts = inner[..end].split(',').map(|p| p.trim().rsplit('.').next().unwrap_or("").to_string());
        let (Some(w), Some(c)) = (parts.next(), parts.next()) else { continue };
        let decl = lines[i + 1..].iter().find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('['));
        let Some(decl) = decl else { continue };
        let words: Vec<&str> = decl.split(|ch: char| !ch.is_alphanumeric() && ch != '_').filter(|w| !w.is_empty()).collect();
        let name = match words.iter().position(|w| *w == "struct") {
            Some(p) => words.get(p + 1),
            None => words.first(),
        };
        if let Some(name) = name {
            out.push((name.to_string(), w, c));
        }
    }
    out
}

#[test]
fn every_bits_attribute_sets_a_register_layout() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../stdlib/core");
    let mut sources = vec![fs::read_to_string(root.join("core.pilot")).unwrap()];
    for e in fs::read_dir(root.join("primitives")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "pilot") {
            sources.push(fs::read_to_string(p).unwrap());
        }
    }
    let tables = bootstrap_tables();
    let mut checked = 0;
    for src in &sources {
        for (name, w, c) in declared_layouts(src) {
            let layout = tables.reg_layout(&name).unwrap_or_else(|| panic!("no layout for `{name}`"));
            assert!(w.trim_start_matches('b').parse::<i64>().is_ok_and(|n| n == layout.width), "`{name}` width");
            assert!(layout.class.starts_with(c.split('_').next().unwrap()), "`{name}` class {} vs {c}", layout.class);
            checked += 1;
        }
    }
    assert!(checked >= 20, "only {checked} layouts found");
}
