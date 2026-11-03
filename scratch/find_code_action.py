with open('crates/pilot-lsp/src/server.rs', 'r', encoding='utf-8') as f:
    for i, line in enumerate(f, 1):
        if 'handle_code_action' in line or 'CodeAction' in line:
            print(f'{i}: {line.strip()}')
