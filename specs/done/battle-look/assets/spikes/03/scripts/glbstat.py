import json, struct, sys

p = sys.argv[1]
b = open(p, "rb").read()
magic, ver, length = struct.unpack_from("<III", b, 0)
jl, jt = struct.unpack_from("<II", b, 12)
g = json.loads(b[20:20 + jl])
acc = g.get("accessors", [])
bvs = g.get("bufferViews", [])
anim_acc = set()
for a in g.get("animations", []):
    for s in a["samplers"]:
        anim_acc.add(s["input"])
        anim_acc.add(s["output"])
skin_acc = {s.get("inverseBindMatrices") for s in g.get("skins", [])}


def acc_bytes(i):
    a = acc[i]
    n = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}[a["type"]]
    c = {5126: 4, 5123: 2, 5125: 4, 5121: 1, 5122: 2, 5120: 1}[a["componentType"]]
    return a["count"] * n * c


anim_b = sum(acc_bytes(i) for i in anim_acc)
tot = sum(acc_bytes(i) for i in range(len(acc)))
tris = 0
for m in g.get("meshes", []):
    for pr in m["primitives"]:
        if "indices" in pr:
            tris += acc[pr["indices"]]["count"] // 3
print(json.dumps({
    "file_bytes": length,
    "accessor_bytes_total": tot,
    "animation_bytes": anim_b,
    "mesh_and_other_bytes": tot - anim_b,
    "animations": [(a.get("name"), len(a["channels"])) for a in g.get("animations", [])],
    "skins": [(len(s["joints"])) for s in g.get("skins", [])],
    "nodes": len(g.get("nodes", [])),
    "meshes": len(g.get("meshes", [])),
    "triangles": tris,
    "node_names": [n.get("name") for n in g.get("nodes", [])][:200] if "--names" in sys.argv else None,
}, indent=1))
