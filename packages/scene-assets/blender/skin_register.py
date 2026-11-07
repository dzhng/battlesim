"""Measure a generated skin against the model its guide views were drawn
from, for its `aircraft_skin.skin` corrections: the median offsets of its
belly and its top on the centreline along the length (away from the fins),
and its half span against the model's (fittings left out).

    blender -b --factory-startup --python skin_register.py -- <skin.glb> <length_m> <top_m> <model.glb> [nose]

A body standing high (belly and top both up) wants `lower=(dz, z_from)`
with `z_from` just above the canopy's top; a half span off by more than a
few centimetres wants `span=(y_from, half_span)`. `nose` is `skin`'s, for a
skin `load` reads the wrong way round.
"""
import os
import statistics
import sys

import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from aircraft_skin import REPO, load  # noqa: E402
from skin_views import SKIP  # noqa: E402

FITTINGS = SKIP + ("nav_", "antenna", "canopy")


def heights(tree, x):
    up = tree.ray_cast(Vector((x, 0, -5)), Vector((0, 0, 1)), 30)[0]
    down = tree.ray_cast(Vector((x, 0, 20)), Vector((0, 0, -1)), 30)[0]
    return (up.z, down.z) if up and down else None


def main():
    a = sys.argv[sys.argv.index("--") + 1:]
    skin_path, length, top, model = os.path.abspath(a[0]), float(a[1]), float(a[2]), a[3]
    nose = a[4] if len(a) > 4 else None
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bm = load(os.path.relpath(skin_path, REPO), length, top, nose=nose)
    skin_tree, skin_half = BVHTree.FromBMesh(bm), max(abs(v.co.y) for v in bm.verts)
    bpy.ops.import_scene.gltf(filepath=model)
    verts, polys = [], []
    for o in bpy.data.objects:
        if o.type != "MESH" or not o.name.endswith("LOD0"):
            continue
        chain, p = [o], o.parent
        while p is not None:
            chain.append(p)
            p = p.parent
        if any(c.name.startswith(FITTINGS) for c in chain):
            continue
        base = len(verts)
        verts += [o.matrix_world @ v.co for v in o.data.vertices]
        polys += [[base + i for i in poly.vertices] for poly in o.data.polygons]
    model_tree, model_half = BVHTree.FromPolygons(verts, polys), max(abs(v.y) for v in verts)
    belly, crown = [], []
    for k in range(5, 80):
        x = length / 2 - length * k / 85
        s, m = heights(skin_tree, x), heights(model_tree, x)
        if s and m and max(s[1], m[1]) < top - 0.8:
            belly.append(s[0] - m[0])
            crown.append(s[1] - m[1])
    print(f"belly {statistics.median(belly):+.3f} m, top {statistics.median(crown):+.3f} m over {len(belly)} stations")
    print(f"half span: skin {skin_half:.3f} m, model {model_half:.3f} m")


if __name__ == "__main__":
    main()
