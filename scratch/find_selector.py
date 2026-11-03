with open('vscode-pilot/src/extension.ts', 'r', encoding='utf-8') as f:
    for i, line in enumerate(f, 1):
        if 'selector' in line:
            print(f'{i}: {line.strip()}')
