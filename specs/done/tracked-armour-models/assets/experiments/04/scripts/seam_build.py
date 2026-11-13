"""Slice 04 probe: export one roster variant as it ships, or with shallow
seam grooves cut into chosen plates before the shared `finish` bakes paint,
wear and occlusion into vertex colour.

    bun run --cwd web asset -- blender <abs>/throwaway/seams04/seam_build.py -- \
        <family> <variant> <out.glb> [--width=0.01 --depth=0.006] [--no-seams]

Grooves are cut with an exact boolean (one swept box per polyline segment),
the cheapest general way to inset a strip into an arbitrary plate. Seam
polylines are given in each part's own frame and snapped to its surface.
Prints SEAMS <name> tris_before tris_after per part and TIERS counts."""
import json
import os
import runpy
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

BLENDER = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../packages/scene-assets/blender"))
sys.path.insert(0, BLENDER)
sys.path.insert(0, os.path.join(BLENDER, "roster"))

a = [v for v in sys.argv[sys.argv.index("--") + 1:] if v != "--"]
family, variant_id, out = a[0], a[1], os.path.abspath(a[2])
opts = dict(x[2:].split("=", 1) for x in a[3:] if "=" in x)
WIDTH, DEPTH = float(opts.get("width", 0.01)), float(opts.get("depth", 0.006))
NO_SEAMS = "--no-seams" in a
REPORT = {}


def tris(o):
    o.data.calc_loop_triangles()
    return len(o.data.loop_triangles)


def snap(o, p):
    ok, loc, nor, _ = o.closest_point_on_mesh(p)
    return loc, nor


def cut(o, polylines, width=None, depth=None):
    """Cut each local-frame polyline into tier-0 mesh `o` as a groove."""
    width, depth = width or WIDTH, depth or DEPTH
    before = tris(o)
    for line in polylines:
        pts = [snap(o, Vector(p)) for p in line]
        for (pa, na), (pb, nb) in zip(pts, pts[1:]):
            t = pb - pa
            length = t.length
            if length < 1e-4:
                continue
            t.normalize()
            n = (na + nb).normalized()
            n = (n - t * n.dot(t)).normalized()
            s = n.cross(t).normalized()
            # extend by half a width so polyline corners close
            a0 = pa - t * width / 2
            span = length + width
            top, bottom = 0.03, depth
            bm = bmesh.new()
            bmesh.ops.create_cube(bm, size=1.0)
            mid = a0 + t * span / 2 + n * (top - bottom) / 2
            m = Matrix((
                (t.x * span, s.x * width, n.x * (top + bottom), mid.x),
                (t.y * span, s.y * width, n.y * (top + bottom), mid.y),
                (t.z * span, s.z * width, n.z * (top + bottom), mid.z),
                (0, 0, 0, 1)))
            bmesh.ops.transform(bm, matrix=m, verts=bm.verts)
            me = bpy.data.meshes.new("seam_cutter")
            bm.to_mesh(me)
            bm.free()
            cutter = bpy.data.objects.new("seam_cutter", me)
            bpy.context.scene.collection.objects.link(cutter)
            cutter.matrix_world = o.matrix_world.copy()
            mod = o.modifiers.new("seam", "BOOLEAN")
            mod.operation, mod.solver, mod.object = "DIFFERENCE", "MANIFOLD", cutter
            bpy.context.view_layer.update()
            ev = o.evaluated_get(bpy.context.evaluated_depsgraph_get())
            new = bpy.data.meshes.new_from_object(ev)
            o.modifiers.remove(mod)
            if len(new.polygons) < len(o.data.polygons):
                raise SystemExit(f"{o.name}: segment {pa} -> {pb} n={n} the cut lost faces ({len(o.data.polygons)} -> {len(new.polygons)})")
            old = o.data
            o.data = new
            bpy.data.meshes.remove(old)
            bpy.data.objects.remove(cutter, do_unlink=True)
    REPORT[o.name] = (before, tris(o))


def local_bounds(o):
    vs = [v.co for v in o.data.vertices]
    return Vector([min(v[k] for v in vs) for k in range(3)]), Vector([max(v[k] for v in vs) for k in range(3)])


def lid(o, axis, sign, inset, top_of=None):
    """An inset rectangle on the face of `o` whose normal is local `axis`
    (0 x, 1 y, 2 z) times `sign`."""
    lo, hi = local_bounds(o)
    k = [i for i in range(3) if i != axis]
    face = hi[axis] if sign > 0 else lo[axis]
    u0, u1 = lo[k[0]] + inset, hi[k[0]] - inset
    v0, v1 = lo[k[1]] + inset, (top_of if top_of is not None else hi[k[1]]) - inset

    def p(u, v):
        q = [0, 0, 0]
        q[axis], q[k[0]], q[k[1]] = face, u, v
        return tuple(q)

    return [[p(u0, v0), p(u1, v0), p(u1, v1), p(u0, v1), p(u0, v0)]]


def obj(name):
    return bpy.data.objects[f"{name}_LOD0"]


def to_local(o, parent, pts):
    """Points in `parent`'s frame (the hull or turret node) into `o`'s."""
    bpy.context.view_layer.update()
    m = o.matrix_world.inverted() @ parent.matrix_world
    return [tuple(m @ Vector(p)) for p in pts]


def t72_seams(g, v):
    bpy.context.view_layer.update()
    # Glacis: a cover plate's lid line inset on each reactive armour tile.
    for i in range(2):
        for j in range(5):
            cut(obj(f"glacis_era_{i}_{j}"), lid(obj(f"glacis_era_{i}_{j}"), 2, 1, 0.045))
    # The fender strips beside the glacis: a plate joint across each.
    for side, s in ((1, "L"), (-1, "R")):
        o = obj(f"hull_upper_fender_{s}")
        lines = []
        for x in (2.75, 1.20, -0.60):
            z = g["glacis_z"](x)
            lines.append(to_local(o, v.hull, [(x, side * 1.14, z), (x, side * 1.62, z)]))
        cut(o, lines)
    # Hull side: a lid line on each Relikt panel (both rows), on its outer face.
    for side, s in ((1, "L"), (-1, "R")):
        for r in range(2):
            for k in range(6):
                o = obj(f"side_era_{s}_{r}_{k}_plate")
                lo, hi = local_bounds(o)
                if r == 0:
                    tops = [vv.co.z for vv in o.data.vertices if vv.co.z > (lo.z + hi.z) / 2]
                    top = min(tops)
                else:
                    top = hi.z
                cut(o, lid(o, 1, side, 0.06, top_of=top))


def leopard_seams(g, v):
    bpy.context.view_layer.update()
    turret = bpy.data.objects["turret"]
    foot, roof = g["FOOT"], g["ROOF"]
    back = [(1.30, 0.42), (1.78, 0.46), (1.02, 1.47), (0.28, 1.47)]
    ridge = [(1.30, 0.42), (2.36, 0.50), (1.22, 1.53), (0.28, 1.53)]
    top = [(1.30, 0.42), (1.70, 0.46), (0.98, 1.44), (0.28, 1.44)]
    for side, s in ((1, "L"), (-1, "R")):
        o = obj(f"wedge_{s}")

        def mix(r0, z0, r1, z1, f, idx):
            return [(r0[i][0] + (r1[i][0] - r0[i][0]) * f, side * (r0[i][1] + (r1[i][1] - r0[i][1]) * f),
                     z0 + (z1 - z0) * f) for i in idx]

        lines = [
            # a plate joint along the upper and the lower face, parallel to the ridge
            to_local(o, turret, mix(ridge, 0.34, top, roof - 0.03, 0.5, (1, 2, 3))),
            to_local(o, turret, mix(back, foot + 0.04, ridge, 0.34, 0.5, (1, 2, 3))),
        ]
        # a vertical joint down the side face, and one across the angled face
        for f in (0.45,):
            def on(r, z):
                return (r[3][0] + (r[2][0] - r[3][0]) * f, side * (r[3][1] + (r[2][1] - r[3][1]) * f), z)
            lines.append(to_local(o, turret, [on(top, roof - 0.03 - 0.06), on(ridge, 0.34), on(back, foot + 0.04 + 0.06)]))
        for f in (0.5,):
            def on2(r, z):
                return (r[1][0] + (r[2][0] - r[1][0]) * f, side * (r[1][1] + (r[2][1] - r[1][1]) * f), z)
            lines.append(to_local(o, turret, [on2(top, roof - 0.03 - 0.06), on2(ridge, 0.34), on2(back, foot + 0.04 + 0.06)]))
        cut(o, lines)


SEAMS = {"t72": t72_seams, "leopard": leopard_seams}

import vehicle_export as VE  # noqa: E402
import parts as P  # noqa: E402

real_run = VE.run


def run(fam, scheme, build, wreck, **kw):
    def build_with_seams(variant, v):
        build(variant, v)
        v.crew = []  # slice 12 drops crew; the seam crops never show them
        if not NO_SEAMS:
            SEAMS[family](build.__globals__, v)
    return real_run(fam, scheme, build_with_seams, wreck, **kw)


VE.run = run
real_export = P.export


def export(path, **kw):
    counts = P.triangles_by_tier()
    real_export(path, **kw)
    for name, (b, c) in sorted(REPORT.items()):
        print("SEAMS", name, b, c, flush=True)
    added = sum(c - b for b, c in REPORT.values())
    print("TIERS", json.dumps({"tiers": counts, "seam_tris_added_pre_finish": added, "parts": len(REPORT)}), flush=True)
    bpy.ops.wm.save_as_mainfile(filepath=os.path.splitext(path)[0] + ".blend")


P.export = export
sys.argv = [sys.argv[0], "--", f"--variant={variant_id}", f"--out={out}", "--no-lint"]
runpy.run_path(os.path.join(BLENDER, "roster", f"{family}.py"), run_name="__main__")
