with open('crates/pilot-lsp/src/server.rs', 'r', encoding='utf-8') as f:
    for i, line in enumerate(f, 1):
        if 'semantic_tokens' in line or 'SemanticToken' in line:
            print(f'{i}: {line.strip()}')
