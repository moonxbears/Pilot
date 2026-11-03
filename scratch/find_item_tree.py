with open('crates/pilot-db/src/lib.rs', 'r', encoding='utf-8') as f:
    for i, line in enumerate(f, 1):
        if 'module_item_tree' in line or 'ItemTree' in line:
            print(f'{i}: {line.strip()}')
