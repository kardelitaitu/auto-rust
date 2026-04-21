import sys
import json
from graphify.extract import collect_files, extract
from pathlib import Path

code_files = []
for p in ['src', 'task']:
    path = Path(p)
    if path.is_dir():
        for f in path.rglob('*.rs'):
            code_files.append(f)

print(f"Processing {len(code_files)} Rust files...")

if code_files:
    result = extract(code_files)
    Path('.graphify_ast.json').write_text(json.dumps(result, indent=2))
    print(f'AST: {len(result["nodes"])} nodes, {len(result["edges"])} edges')
else:
    Path('.graphify_ast.json').write_text(json.dumps({'nodes': [], 'edges': []}))
    print('No code files - skipping AST extraction')