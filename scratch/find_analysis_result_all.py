import os
for root, dirs, files in os.walk('crates/pilot-analysis'):
    for file in files:
        if file.endswith('.rs'):
            p = os.path.join(root, file)
            with open(p, 'r', encoding='utf-8') as f:
                for i, line in enumerate(f, 1):
                    if 'struct AnalysisResult' in line:
                        print(f"{p}:{i}: {line.strip()}")
