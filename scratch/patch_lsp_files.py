import json

# ==========================================
# 1. Update vscode-pilot/package.json
# ==========================================
with open('vscode-pilot/package.json', 'r', encoding='utf-8') as f:
    pkg = json.load(f)

# Ensure .pchor is supported
for lang in pkg.get("contributes", {}).get("languages", []):
    if lang.get("id") == "pilot":
        if ".pchor" not in lang["extensions"]:
            lang["extensions"].append(".pchor")
            print("Added .pchor to package.json languages")

with open('vscode-pilot/package.json', 'w', encoding='utf-8') as f:
    json.dump(pkg, f, indent=4)
print("Updated vscode-pilot/package.json")

# ==========================================
# 2. Update vscode-pilot/syntaxes/pilot.tmLanguage.json
# ==========================================
with open('vscode-pilot/syntaxes/pilot.tmLanguage.json', 'r', encoding='utf-8') as f:
    tm = json.load(f)

# Update types
support_types = tm["repository"]["types"]["patterns"][0]["match"]
old_types = "Memory|SliceBuffer|SharedBuffer"
new_types = "Memory|Slice|SliceBuffer|SliceView|StridedSlice|View|SliceMap|SharedBuffer"
if old_types in support_types and "SliceMap" not in support_types:
    tm["repository"]["types"]["patterns"][0]["match"] = support_types.replace(old_types, new_types)
    print("Updated tmLanguage types pattern")

# Update declarations modifiers
decl_pat = tm["repository"]["declarations"]["patterns"][0]["match"]
old_mods = "public|private|protected|internal|static|mut|partial|comptime|async|pure|override|virtual|abstract|sealed|final|readonly"
new_mods = "public|private|priv|pub|prot|file|fileprivate|pkg|package|int|open|export|lvl\\d+|lvl\\([0-9A-Za-z_, ]+\\)|protected|internal|static|mut|immut|iso|steal|partial|comptime|async|pure|override|virtual|abstract|sealed|final|readonly"
if old_mods in decl_pat:
    tm["repository"]["declarations"]["patterns"][0]["match"] = decl_pat.replace(old_mods, new_mods)
    print("Updated tmLanguage declarations modifier pattern")

# Update operators to include deep flatten *..
ops = tm["repository"]["operators"]["patterns"]
has_deep_flatten = any("\\*\\.\\." in p.get("match", "") for p in ops)
if not has_deep_flatten:
    ops.insert(0, {
        "name": "keyword.operator.flatten.pilot",
        "match": "(\\*\\.\\.)"
    })
    print("Added *.. deep flatten operator to tmLanguage")

with open('vscode-pilot/syntaxes/pilot.tmLanguage.json', 'w', encoding='utf-8') as f:
    json.dump(tm, f, indent=2)
print("Updated vscode-pilot/syntaxes/pilot.tmLanguage.json")
