"""Edge wear on the armor_paint primitives of a GLB: the COLOR_0 channel the
paint recipe writes wear into (alpha), as a distribution. usage: wearstat.py a.glb [b.glb]"""
import json, struct, sys
def load(path):
    data = open(path, 'rb').read(); jl, = struct.unpack_from('<I', data, 12)
    g = json.loads(data[20:20 + jl]); off = 20 + jl; bl, = struct.unpack_from('<I', data, off)
    return g, data[off + 8: off + 8 + bl]
FMT = {5126: ('f', 4, 1.0), 5121: ('B', 1, 255.0), 5123: ('H', 2, 65535.0)}
def wear(path):
    g, b = load(path)
    paint = [i for i, m in enumerate(g['materials']) if m['name'] == 'armor_paint'][0]
    vals = []
    for node in g['nodes']:
        if 'mesh' not in node or not node['name'].endswith('LOD0'):
            continue
        for p in g['meshes'][node['mesh']]['primitives']:
            if p.get('material') != paint or 'COLOR_0' not in p['attributes']:
                continue
            a = g['accessors'][p['attributes']['COLOR_0']]; bv = g['bufferViews'][a['bufferView']]
            f, size, scale = FMT[a['componentType']]; n = 4 if a['type'] == 'VEC4' else 3
            stride = bv.get('byteStride', size * n); start = bv.get('byteOffset', 0) + a.get('byteOffset', 0)
            for i in range(a['count']):
                c = struct.unpack_from('<%d%s' % (n, f), b, start + i * stride)
                vals.append(c[3] / scale if n == 4 else None)
    vals = [v for v in vals if v is not None]
    vals.sort()
    q = lambda t: vals[int(t * (len(vals) - 1))]
    return len(vals), sum(vals) / len(vals), q(0.5), q(0.9), q(0.99), sum(v > 0.3 for v in vals) / len(vals)
for path in sys.argv[1:]:
    n, mean, p50, p90, p99, over = wear(path)
    print(f"{path}: {n} paint verts (tier 0), alpha mean {mean:.4f}, p50 {p50:.3f}, p90 {p90:.3f}, p99 {p99:.3f}, share > 0.3 {over:.4f}")
