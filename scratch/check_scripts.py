import json
with open('vscode-pilot/package.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

print("scripts:", data.get("scripts", {}))
