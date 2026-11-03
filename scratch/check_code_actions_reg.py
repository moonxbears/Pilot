with open('vscode-pilot/src/extension.ts', 'r', encoding='utf-8') as f:
    text = f.read()

print("registerCodeActionsProvider in extension.ts:", "registerCodeActionsProvider" in text)
