with open('crates/pilot-analysis/src/infer.rs', 'r', encoding='utf-8') as f:
    for i, line in enumerate(f, 1):
        if 'fn_visibility' in line or 'node_visibility' in line:
            print(f'{i}: {line.strip()}')
