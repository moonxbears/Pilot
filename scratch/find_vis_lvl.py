with open('crates/pilot-analysis/src/infer.rs', 'r', encoding='utf-8') as f:
    for i, line in enumerate(f, 1):
        if 'VIS001' in line or 'LVL001' in line:
            print(f'{i}: {line.strip()}')
