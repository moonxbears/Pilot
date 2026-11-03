with open('vscode-pilot/syntaxes/pilot.tmLanguage.json', 'r', encoding='utf-8') as f:
    import json
    data = json.load(f)

print(list(data.get("repository", {}).keys()))
