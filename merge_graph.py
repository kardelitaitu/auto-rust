import json
from pathlib import Path

ast = json.loads(Path('.graphify_ast.json').read_text())
sem = {'nodes': [], 'edges': [], 'hyperedges': []}

seen = {n['id'] for n in ast['nodes']}
merged_nodes = list(ast['nodes'])
merged_edges = ast['edges']
merged_hyperedges = []

merged = {'nodes': merged_nodes, 'edges': merged_edges, 'hyperedges': merged_hyperedges}
Path('.graphify_extract.json').write_text(json.dumps(merged, indent=2))
print(f"Merged: {len(merged_nodes)} nodes, {len(merged_edges)} edges from AST")