"""THROWAWAY (lane J, slice 06): smooth a guide's airframe in code.

The near tier's painted bodies (fuselage, belly, nacelles, intakes, booms:
everything painted but the flying surfaces) are joined, voxel-remeshed into
one closed shell so their junctions blend, smoothed, and decimated back to
the triangle count they had. The flying surfaces stay lofted aerofoils (a
voxel grid would eat their thin trailing edges). Markings are laid onto the
new shell afterwards. SMOOTH_VOXEL, SMOOTH_ITER and SMOOTH_FACTOR tune it.
"""
import os

import bmesh
import bpy

from parts import tier_of

VOXEL = float(os.environ.get("SMOOTH_VOXEL", "0.025"))
ITER = int(os.environ.get("SMOOTH_ITER", "12"))
FACTOR = float(os.environ.get("SMOOTH_FACTOR", "0.6"))
SURFACES = ("wing", "tail_stab", "tail_fin", "ventral")
# The lofted bodies that merge; fittings (gear doors, pylons, lips, sills) stay apart.
BODIES = ("fuselage", "nacelle", "tail_boom", "gun_bulge")
# The skin's near-tier budget (`aircraft_skin.TRIS[0]`, 26000) less the lofted surfaces'.
TRIS = int(os.environ.get("SMOOTH_TRIS", "20000"))


def _tris(me):
    return sum(len(p.vertices) - 2 for p in me.polygons)


def smoothed(build, A):
    def wrapped(variant, v):
        rows = []
        markings = A.markings
        A.markings = lambda _v, r: rows.extend(r)
        try:
            build(variant, v)
        finally:
            A.markings = markings
        paint = v.mats["paint"]
        hull = v.hull
        bpy.context.view_layer.update()
        shells = [o for o in bpy.data.objects if o.type == "MESH" and tier_of(o) == 0 and o.data.materials
                  and o.data.materials[0] == paint and o.name.startswith(BODIES)]
        bm = bmesh.new()
        to_hull = hull.matrix_world.inverted()
        budget = 0
        for o in shells:
            me = o.data.copy()
            me.transform(to_hull @ o.matrix_world)
            bm.from_mesh(me)
            budget += _tris(o.data)
            bpy.data.meshes.remove(me)
        print("SMOOTH joining", len(shells), "bodies,", budget, "triangles:", sorted(o.name for o in shells))
        for o in shells:
            bpy.data.objects.remove(o)
        me = bpy.data.meshes.new("airframe_LOD0")
        bm.to_mesh(me)
        bm.free()
        ob = bpy.data.objects.new("airframe_LOD0", me)
        bpy.context.scene.collection.objects.link(ob)
        ob.parent = hull
        me.materials.append(paint)
        r = ob.modifiers.new("remesh", "REMESH")
        r.mode, r.voxel_size, r.adaptivity = "VOXEL", VOXEL, 0.0
        s = ob.modifiers.new("smooth", "SMOOTH")
        s.factor, s.iterations = FACTOR, ITER
        dg = bpy.context.evaluated_depsgraph_get()
        dense = bpy.data.meshes.new_from_object(ob.evaluated_get(dg))
        ob.modifiers.clear()
        ob.data = dense
        d = ob.modifiers.new("decimate", "DECIMATE")
        d.ratio = min(1.0, TRIS / max(1, _tris(dense)))
        dg = bpy.context.evaluated_depsgraph_get()
        final = bpy.data.meshes.new_from_object(ob.evaluated_get(dg))
        ob.modifiers.clear()
        ob.data = final
        ob.data.name = "airframe_LOD0"
        for p in ob.data.polygons:
            p.use_smooth = True
        if not ob.data.materials:
            ob.data.materials.append(paint)
        print("SMOOTH airframe", _tris(dense), "->", _tris(final), "triangles at voxel", VOXEL)
        A.markings(v, [(what, dict(kw, onto=("airframe",) + tuple(p for p in kw["onto"]
                                                                 if p.startswith(SURFACES))))
                       for what, kw in rows])
    return wrapped
