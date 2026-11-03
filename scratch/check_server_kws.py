with open('crates/pilot-lsp/src/server.rs', 'r', encoding='utf-8') as f:
    text = f.read()

for kw in ["SliceView", "SliceMap", "StridedSlice", "export", "open", "pkg", "fileprivate"]:
    print(f"{kw}: {'FOUND' if kw in text else 'MISSING'}")
