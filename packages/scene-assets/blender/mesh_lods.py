"""Four mesh tiers from one skinned mesh, finest first, named `<name>_LOD<n>`.

Technique from ~/dev/game packages/soldier-assets/bake/blender-mesh-lods.py:
split the mesh into its loose islands, drop
islands smaller than the tier's extent cutoff, share the triangle budget over
the rest with a per-island floor, decimate each in bind space (before the
armature modifier), and join. Blender interpolates UVs, colours and deform
weights during the collapse; materials travel with each surface.
"""

import bmesh
import bpy
from mathutils import Vector

# (triangle target or None for the full mesh, omitted extent m, island floor)
TIERS = (
    (None, 0.0, 12),
    (7000, 0.0, 12),
    (2400, 0.035, 8),
    (700, 0.10, 4),
)


def triangle_count(obj):
    obj.data.calc_loop_triangles()
    return len(obj.data.loop_triangles)


def _select_only(objs):
    bpy.ops.object.select_all(action="DESELECT")
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]


def reduced_copy(source, name, target, min_extent, floor):
    copy = source.copy()
    copy.data = source.data.copy()
    copy.name = name
    bpy.context.scene.collection.objects.link(copy)
    if target is None:
        return copy
    _select_only([copy])
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.separate(type="LOOSE")
    bpy.ops.object.mode_set(mode="OBJECT")
    # Blender's separate and join orders vary between runs: put every island in a
    # canonical vertex order and work through them in a canonical order.
    pieces = list(bpy.context.selected_objects)
    for obj in pieces:
        canonical(obj)
    pieces.sort(key=lambda o: (len(o.data.vertices), tuple(o.data.vertices[0].co)))
    for obj in list(pieces):
        pts = [obj.matrix_world @ v.co for v in obj.data.vertices]
        extent = max(max(p[i] for p in pts) - min(p[i] for p in pts) for i in range(3))
        if extent < min_extent and len(pieces) > 1:
            pieces.remove(obj)
            bpy.data.objects.remove(obj, do_unlink=True)
    counts = {o: triangle_count(o) for o in pieces}
    floors = {o: n if n <= 32 else floor for o, n in counts.items()}
    available = max(0, target - sum(floors.values()))
    reducible = sum(counts[o] - floors[o] for o in pieces)
    ratio = min(1.0, available / max(1, reducible))
    for obj, count in counts.items():
        if count <= 32 or ratio >= 1.0:
            continue
        mod = obj.modifiers.new("lod", "DECIMATE")
        mod.ratio = min(1.0, (floors[obj] + (count - floors[obj]) * ratio) / count)
        mod.use_collapse_triangulate = True
        _select_only([obj])
        while obj.modifiers.find("lod") > 0:
            bpy.ops.object.modifier_move_up(modifier="lod")
        bpy.ops.object.modifier_apply(modifier="lod")
    for obj in pieces:
        canonical(obj)
    _select_only(pieces)
    bpy.ops.object.join()
    result = bpy.context.view_layer.objects.active
    result.name = name
    return result


def make_tiers(source, base):
    """Replace `source` by `<base>_LOD0..3`; returns the four objects."""
    out = []
    for t, (target, extent, floor) in enumerate(TIERS):
        out.append(reduced_copy(source, f"{base}_LOD{t}", target, extent, floor))
    bpy.data.objects.remove(source, do_unlink=True)
    for o in out:
        print("LOD", o.name, triangle_count(o))
    return out


def canonical(o):
    """Make the export hash-stable: snap positions to 0.1 mm (threaded modifier evaluation
    differs in the last float bit between runs), triangulate with a fixed diagonal, and
    order vertices by position and weights, then faces by material and vertices, so the
    order Blender joined or collapsed pieces in never reaches the bytes."""
    for v in o.data.vertices:
        v.co = Vector(round(c, 4) for c in v.co)
    # custom split normals (from bevels) carry the same float noise; the export derives
    # normals from the geometry and its smooth flags instead
    if o.data.has_custom_normals:
        with bpy.context.temp_override(object=o, active_object=o):
            bpy.ops.mesh.customdata_custom_splitnormals_clear()
    bm = bmesh.new()
    bm.from_mesh(o.data)
    bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="FIXED", ngon_method="EAR_CLIP")
    deform = bm.verts.layers.deform.active

    def vkey(v):
        weights = tuple(sorted((g, round(w, 4)) for g, w in v[deform].items())) if deform else ()
        ring = Vector((0, 0, 0))
        for e in v.link_edges:  # coincident vertices differ by their neighbours
            ring += e.other_vert(v).co
        return (tuple(v.co), weights, tuple(round(c, 3) for c in v.normal), tuple(round(c, 5) for c in ring))

    rank = {v: i for i, v in enumerate(sorted(bm.verts, key=vkey))}
    bm.verts.sort(key=lambda v: rank[v])
    bm.verts.index_update()
    order = sorted(bm.faces, key=lambda f: (f.material_index, sorted(v.index for v in f.verts), [v.index for v in f.verts]))
    rank = {f: i for i, f in enumerate(order)}
    bm.faces.sort(key=lambda f: rank[f])
    bm.to_mesh(o.data)
    bm.free()
