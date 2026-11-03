with open('vscode-pilot/src/extension.ts', 'r', encoding='utf-8') as f:
    content = f.read()

target = "action.kind ? new vscode.CodeActionKind(action.kind) : vscode.CodeActionKind.QuickFix"
repl = "action.kind ? vscode.CodeActionKind.Empty.append(action.kind) : vscode.CodeActionKind.QuickFix"

content = content.replace(target, repl)

with open('vscode-pilot/src/extension.ts', 'w', encoding='utf-8', newline='') as f:
    f.write(content)
print("Updated CodeActionKind")
