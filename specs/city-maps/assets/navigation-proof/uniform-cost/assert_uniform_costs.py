import json,sys
for line in open(sys.argv[1]):
 row=json.loads(line)
 if 'attribution' not in row:continue
 counts=row['attribution']['search']
 limit=10*counts['nonuniform_stencil']+2
 assert counts['cell_cost_calls']<=limit,(row['side'],row['mover'],row['policy'],counts['cell_cost_calls'],limit)
print('Uniform cost work is bounded by two per plan plus ordinary exceptional expansion work')
