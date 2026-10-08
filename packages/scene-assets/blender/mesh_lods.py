"""Four mesh tiers from one skinned mesh, finest first, named `<name>_LOD<n>`.

Technique from ~/dev/game packages/soldier-assets/bake/blender-mesh-lods.py:
split the mesh into its loose islands, drop
islands smaller than the tier's extent cutoff, share the triangle budget over
the rest with a per-island floor, decimate each in bind space (before the
armature modifier), and join. Blender interpolates UVs during the
collapse; materials travel with each surface. Deform weights and corner
colours are not taken from the collapse, whose interpolation differs in the
last float bit between runs (a colour byte then rounds either way): each kept
vertex and corner takes those of the island's nearest original one.
"""

import bmesh
import bpy
from mathutils import Vector
from mathutils.kdtree import KDTree

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
        source = _attributes_by_position(obj)
        mod = obj.modifiers.new("lod", "DECIMATE")
        mod.ratio = min(1.0, (floors[obj] + (count - floors[obj]) * ratio) / count)
        mod.use_collapse_triangulate = True
        _select_only([obj])
        while obj.modifiers.find("lod") > 0:
            bpy.ops.object.modifier_move_up(modifier="lod")
        bpy.ops.object.modifier_apply(modifier="lod")
        _take_attributes(obj, source)
    for obj in pieces:
        canonical(obj)
    _select_only(pieces)
    bpy.ops.object.join()
    result = bpy.context.view_layer.objects.active
    result.name = name
    return result


def _corner_probes(me):
    """A point per face corner, a quarter of the way from its vertex to the face's centre."""
    for poly in me.polygons:
        for li in poly.loop_indices:
            co = me.vertices[me.loops[li].vertex_index].co
            yield li, co + (poly.center - co) * 0.25


def _attributes_by_position(obj):
    """What a decimate would interpolate, kept to copy back exactly: the island's
    vertices with their deform weights, and its face corners with their colours."""
    me = obj.data
    verts = KDTree(len(me.vertices))
    for v in me.vertices:
        verts.insert(v.co, v.index)
    verts.balance()
    corners = KDTree(len(me.loops))
    for li, p in _corner_probes(me):
        corners.insert(p, li)
    corners.balance()
    colours = {a.name: [tuple(d.color) for d in a.data] for a in me.color_attributes if a.domain == "CORNER"}
    return verts, [[(g.group, g.weight) for g in v.groups] for v in me.vertices], corners, colours


def _take_attributes(obj, source):
    """Give each vertex the weights of the nearest original vertex, and each
    face corner the colours of the nearest original corner (`_attributes_by_position`)."""
    verts, weights, corners, colours = source
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    deform = bm.verts.layers.deform.verify()
    for v in bm.verts:
        dv = v[deform]
        dv.clear()
        for g, w in weights[verts.find(v.co)[1]]:
            dv[g] = w
    bm.to_mesh(me)
    bm.free()
    for li, p in _corner_probes(me):
        nearest = corners.find(p)[1]
        for name, values in colours.items():
            me.color_attributes[name].data[li].color = values[nearest]


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
    order vertices by position and weights, then faces by material and vertices, and edges
    by vertices, so the order Blender joined or collapsed pieces in never reaches the bytes."""
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
    # Edges too: a decimate collapses equal-cost edges in edge order.
    rank = {e: i for i, e in enumerate(sorted(bm.edges, key=lambda e: sorted(v.index for v in e.verts)))}
    bm.edges.sort(key=lambda e: rank[e])
    bm.to_mesh(o.data)
    bm.free()
