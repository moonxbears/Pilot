with open('crates/pilot-lsp/src/server.rs', 'r', encoding='utf-8') as f:
    for i, line in enumerate(f, 1):
        if 'fn handle_hover' in line:
            print(f'{i}: {line.strip()}')
