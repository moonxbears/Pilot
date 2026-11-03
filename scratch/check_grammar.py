with open('vscode-pilot/syntaxes/pilot.tmLanguage.json', 'r', encoding='utf-8') as f:
    text = f.read()

keywords_to_check = ["self", "op", "view", "*..", "priv", "pub", "file", "pkg", "open", "export", "lvl", "SliceView", "SliceMap", "StridedSlice"]
for kw in keywords_to_check:
    print(f"{kw}: {'FOUND' if kw in text else 'MISSING'}")
