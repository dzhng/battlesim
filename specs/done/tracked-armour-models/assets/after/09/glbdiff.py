"""Compare two GLBs node by node: names, transforms and POSITION data.
usage: python3 glbdiff.py a.glb b.glb [lod_filter]"""
import json, struct, sys, math

def load(path):
    data = open(path, "rb").read()
    jlen, = struct.unpack_from("<I", data, 12)
    gltf = json.loads(data[20:20 + jlen])
    off = 20 + jlen
    blen, = struct.unpack_from("<I", data, off)
    binc = data[off + 8: off + 8 + blen]
    out = {}
    for node in gltf["nodes"]:
        if "mesh" not in node:
            continue
        pos = []
        for prim in gltf["meshes"][node["mesh"]]["primitives"]:
            acc = gltf["accessors"][prim["attributes"]["POSITION"]]
            bv = gltf["bufferViews"][acc["bufferView"]]
            start = bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
            stride = bv.get("byteStride", 12)
            for i in range(acc["count"]):
                pos.append(struct.unpack_from("<3f", binc, start + i * stride))
        trs = (tuple(node.get("translation", (0, 0, 0))), tuple(node.get("rotation", (0, 0, 0, 1))), tuple(node.get("scale", (1, 1, 1))))
        out[node["name"]] = (trs, pos)
    return out

a, b = load(sys.argv[1]), load(sys.argv[2])
filt = sys.argv[3] if len(sys.argv) > 3 else ""
names = sorted(set(a) | set(b))
same = 0
for n in names:
    if filt and filt not in n:
        continue
    if n not in a: print("ADDED  ", n, len(b[n][1]), "verts"); continue
    if n not in b: print("REMOVED", n, len(a[n][1]), "verts"); continue
    (ta, pa), (tb, pb) = a[n], b[n]
    if ta == tb and pa == pb:
        same += 1; continue
    dt = max(abs(x - y) for u, v in zip(ta, tb) for x, y in zip(u, v))
    if len(pa) != len(pb):
        print("CHANGED", n, f"verts {len(pa)} -> {len(pb)}", f"trs d={dt:.4g}")
        continue
    d = max(math.dist(p, q) for p, q in zip(pa, pb))
    print("CHANGED", n, f"same count {len(pa)}, max move {d:.4g} m", f"trs d={dt:.4g}")
print("identical nodes:", same, "of", len(names))
