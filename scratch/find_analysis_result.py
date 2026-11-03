with open('crates/pilot-analysis/src/lib.rs', 'r', encoding='utf-8') as f:
    for i, line in enumerate(f, 1):
        if 'struct AnalysisResult' in line:
            print(f'{i}: {line.strip()}')
