import json
with open('vscode-pilot/syntaxes/pilot.tmLanguage.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

for k in ['types', 'operators', 'keywords-control', 'declarations']:
    print(f"=== {k} ===")
    print(json.dumps(data['repository'][k], indent=2)[:500])
