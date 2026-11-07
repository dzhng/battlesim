"""The markings painted onto an airframe's skin: national insignia and
lettering (tail codes, serials, modexes), each pressed onto the parts it
lies on (`marking_*`), in the marking paints a modern air arm uses. Painted
after the skin is built (`markings`), since they are laid onto it.

`aircraft_parts` imports the public names, so roster scripts reach them
through it as they reach every other airframe part.
"""
import math
import os
import sys

import bmesh
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import textured  # noqa: E402
from vehicle_parts import NEAR  # noqa: E402

# The marking paints, linear albedo: national colours and the low-visibility
# greys a modern air arm paints its insignia in.
MARK = {
    "white": (0.62, 0.62, 0.6), "black": (0.018, 0.018, 0.018), "red": (0.4, 0.025, 0.02),
    "blue": (0.018, 0.05, 0.24), "sky": (0.12, 0.3, 0.55), "yellow": (0.62, 0.42, 0.03),
    "green": (0.03, 0.17, 0.05), "brown": (0.12, 0.06, 0.025), "orange": (0.6, 0.18, 0.02),
    "lowvis_dark": (0.07, 0.075, 0.08), "lowvis_light": (0.3, 0.31, 0.32),
    "pale_red": (0.32, 0.09, 0.08), "pale_blue": (0.1, 0.14, 0.24),
}


def mark(m, colour):
    """The marking paint `colour` (a `MARK` key), made once into `m`."""
    key = f"mark_{colour}"
    if key not in m:
        m[key] = textured(f"marking_{colour}", "marking_paint", colour=MARK[colour], chip=0.5, dirt=0.25,
                          streak=0.0, role="marking")
    return m[key]


def _ngon(n, r=1.0, phase=0.0):
    return [(r * math.cos(phase + math.tau * k / n), r * math.sin(phase + math.tau * k / n)) for k in range(n)]


def _star(points=5, inner=0.382):
    """A star's outline, a point up (+V)."""
    return [((1.0 if k % 2 == 0 else inner) * math.cos(math.pi / 2 + math.pi * k / points),
             (1.0 if k % 2 == 0 else inner) * math.sin(math.pi / 2 + math.pi * k / points)) for k in range(points * 2)]


def _rect(u0, v0, u1, v1):
    return [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]


def _cross(arm, foot):
    """A Balkenkreuz's outline: arms `arm` long from the centre, `foot` the
    half-width of each arm."""
    a, f = arm, foot
    return [(f, f), (f, a), (-f, a), (-f, f), (-a, f), (-a, -f), (-f, -f), (-f, -a), (f, -a), (f, -f), (a, -f),
            (a, f)]


# A grinning mouth's outline: pointed ahead (+U), deepest a third of the way
# back, rounded off behind.
_MOUTH = [(1.0, 0.0), (0.5, 0.14), (0.0, 0.3), (-0.5, 0.38), (-0.85, 0.3), (-1.0, 0.1), (-1.0, -0.1),
          (-0.85, -0.3), (-0.5, -0.34), (0.0, -0.24), (0.5, -0.1)]
# An insignia is layers, back first: (outline, scale, centre offset (u, v) in
# its size, colour). An outline is star-shaped about its own centre.
DISC = _ngon(32)
INSIGNIA = {
    # The US star-and-bar, low-visibility: one dark grey outline on the grey.
    "us_lowvis": [(_rect(-1.0, -0.32, 1.0, 0.32), 1.0, (0, 0), "lowvis_dark"),
                  (DISC, 0.5, (0, 0), "lowvis_dark"), (_star(), 0.46, (0, 0), "lowvis_light")],
    # The full-colour star-and-bar (Army and Marine helicopters in black).
    "us_black": [(_rect(-1.0, -0.32, 1.0, 0.32), 1.0, (0, 0), "black"), (DISC, 0.5, (0, 0), "black"),
                 (_star(), 0.46, (0, 0), "lowvis_light")],
    # Russia's VKS: a red star bordered white, then blue.
    "ru_star": [(_star(), 1.0, (0, 0), "blue"), (_star(), 0.88, (0, 0), "white"), (_star(), 0.74, (0, 0), "red")],
    # China's PLA: a red star bordered yellow with red bars.
    "cn_star": [(_rect(-1.0, -0.24, 1.0, 0.24), 1.0, (0, 0), "yellow"), (_rect(-0.97, -0.2, 0.97, 0.2), 1.0,
                                                                         (0, 0), "red"),
                (_star(), 0.58, (0, 0), "yellow"), (_star(), 0.52, (0, 0), "red")],
    # The PLA's star alone, as on a fin.
    "cn_fin": [(_star(), 1.0, (0, 0), "yellow"), (_star(), 0.88, (0, 0), "red")],
    # The Flying Tigers' shark mouth, its point toward +U (the nose, given
    # `ahead`): red lips, the dark maw, white teeth top and bottom.
    "shark_mouth": [(_MOUTH, 1.0, (0, 0), "red"), (_MOUTH, 0.86, (0.03, 0), "black"),
                    *[(_ngon(3, 0.075, -math.pi / 2), 1.0, (u, 0.2 - 0.12 * max(0.0, u)), "white")
                      for u in (-0.62, -0.42, -0.22, -0.02, 0.18, 0.38)],
                    *[(_ngon(3, 0.075, math.pi / 2), 1.0, (u + 0.1, -0.16 + 0.08 * max(0.0, u)), "white")
                      for u in (-0.62, -0.42, -0.22, -0.02, 0.18)]],
    # The roundels, outer ring first.
    "uk_lowvis": [(DISC, 1.0, (0, 0), "pale_blue"), (DISC, 0.45, (0, 0), "pale_red")],
    # The fin flash, red aft of blue.
    "uk_flash": [(_rect(-1.0, -0.6, 1.0, 0.6), 1.0, (0, 0), "pale_blue"),
                 (_rect(-0.5, -0.6, 0.5, 0.6), 1.0, (0.5, 0), "pale_red")],
    "uk": [(DISC, 1.0, (0, 0), "blue"), (DISC, 0.66, (0, 0), "white"), (DISC, 0.33, (0, 0), "red")],
    "fr": [(DISC, 1.0, (0, 0), "red"), (DISC, 0.66, (0, 0), "white"), (DISC, 0.33, (0, 0), "blue")],
    "it": [(DISC, 1.0, (0, 0), "red"), (DISC, 0.66, (0, 0), "white"), (DISC, 0.33, (0, 0), "green")],
    # Sweden's three crowns, as three gold spots on the blue.
    "se": [(DISC, 1.0, (0, 0), "blue"), *[(_ngon(12), 0.2, at, "yellow") for at in ((0, 0.3), (-0.27, -0.17),
                                                                                    (0.27, -0.17))]],
    # Germany's flag, black over red over gold, as on a fin.
    "de_flag": [(_rect(-1.0, -0.6, 1.0, 0.6), 1.0, (0, 0), "black"), (_rect(-1.0, -0.4, 1.0, 0.4), 1.0, (0, -0.2), "red"),
                (_rect(-1.0, -0.2, 1.0, 0.2), 1.0, (0, -0.4), "yellow")],
    "de": [(_cross(1.0, 0.45), 1.0, (0, 0), "black"), (_cross(0.82, 0.27), 1.0, (0, 0), "white"),
           (_cross(0.64, 0.1), 1.0, (0, 0), "black")],
}


def near_shell(prefixes):
    """A BVH of the near tier of the parts `prefixes` names, in world space."""
    import bpy
    from mathutils.bvhtree import BVHTree
    bpy.context.view_layer.update()
    verts, polys = [], []
    for o in bpy.data.objects:
        if o.type != "MESH" or not o.name.endswith("_LOD0") or not o.name.startswith(tuple(prefixes)):
            continue
        base = len(verts)
        verts += [o.matrix_world @ v.co for v in o.data.vertices]
        polys += [[base + i for i in p.vertices] for p in o.data.polygons]
    if not polys:
        raise ValueError(f"no near-tier parts named {prefixes} to paint on")
    return BVHTree.FromPolygons(verts, polys)


def _lay(points, centre, normal, up, size, tree, lift):
    """Each (u, v) of `points`, `size` metres to the unit, on the plane at
    `centre` facing `normal` with +V toward `up`, then pressed onto the skin
    `tree` along -`normal` and stood `lift` off it."""
    n = Vector(normal).normalized()
    ev = (Vector(up) - n * Vector(up).dot(n)).normalized()
    eu = ev.cross(n)
    out = []
    for u, w in points:
        p = Vector(centre) + eu * (u * size) + ev * (w * size)
        hit = tree.ray_cast(p + n * 1.5, -n, 8.0)[0] if tree else None
        out.append((hit if hit is not None else p) + n * lift)
    return out


def insignia(name, kind, centre, normal, up, size, mats, parent, onto, lods=NEAR, ahead=None):
    """A national insignia `kind` (`INSIGNIA`), `size` metres from its centre
    to its outer edge, painted on the skin of the parts named by `onto` at
    `centre`, facing `normal`, its top toward `up` (a star's point). A
    shape that isn't symmetric across its top (nose art) turns its +U
    toward `ahead`, so each side's copy faces the same way."""
    import bpy
    from parts import _obj
    tree = near_shell(onto)
    flip = 1
    if ahead is not None:
        n = Vector(normal).normalized()
        ev = (Vector(up) - n * Vector(up).dot(n)).normalized()
        flip = 1 if ev.cross(n).dot(Vector(ahead)) >= 0 else -1
    inv = parent.matrix_world.inverted_safe()
    for k, (outline, scale, (du, dv), colour) in enumerate(INSIGNIA[kind]):
        def make(lod, n, outline=outline, scale=scale, du=du, dv=dv, k=k):
            rings = 4 if lod == 0 else 2
            # The outline split so no edge is long, then scaled ring by ring
            # to its centre: every vertex has the skin under it.
            dense = []
            for a, b in zip(outline, outline[1:] + outline[:1]):
                steps = max(1, int(math.dist(a, b) * scale * size / 0.12))
                dense += [(a[0] + (b[0] - a[0]) * j / steps, a[1] + (b[1] - a[1]) * j / steps) for j in range(steps)]
            pts = [(flip * du, dv)] + [(flip * (du + x * scale * t), dv + y * scale * t) for t in
                                        [(r + 1) / rings for r in range(rings)] for x, y in dense]
            world = _lay(pts, centre, normal, up, size, tree, 0.006 + 0.004 * k)
            bm = bmesh.new()
            vs = [bm.verts.new(inv @ p) for p in world]
            m_ = len(dense)
            for j in range(m_):
                bm.faces.new((vs[0], vs[1 + j], vs[1 + (j + 1) % m_]))
            for r in range(rings - 1):
                a0, b0 = 1 + r * m_, 1 + (r + 1) * m_
                for j in range(m_):
                    j1 = (j + 1) % m_
                    bm.faces.new((vs[a0 + j], vs[b0 + j], vs[b0 + j1], vs[a0 + j1]))
            bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
            # Face outward, along `normal`.
            nn = inv.to_3x3() @ Vector(normal)
            if sum(f.normal.dot(nn) for f in bm.faces) < 0:
                bmesh.ops.reverse_faces(bm, faces=bm.faces)
            return _obj(n, bm, mark(mats, colour), parent, (0, 0, 0), (0, 0, 0))
        from parts import _each
        _each(f"marking_{name}_{k}", lods, make)
    bpy.context.view_layer.update()


def lettering(name, text, height, centre, normal, up, mats, parent, onto, colour="lowvis_dark", lods=NEAR):
    """Painted letters (a tail code, a serial, a modex): `text` `height`
    metres tall on the skin of the parts named by `onto`, as `insignia`."""
    import bpy
    from parts import _each, _obj
    tree = near_shell(onto)
    inv = parent.matrix_world.inverted_safe()

    def make(lod, n):
        cu = bpy.data.curves.new(n + "_text", "FONT")
        cu.body = text
        cu.size = height
        cu.align_x = "CENTER"
        cu.align_y = "CENTER"
        cu.resolution_u = 2
        tmp = bpy.data.objects.new(n + "_curve", cu)
        bpy.context.scene.collection.objects.link(tmp)
        bpy.context.view_layer.update()
        me = bpy.data.meshes.new_from_object(tmp.evaluated_get(bpy.context.evaluated_depsgraph_get()))
        bpy.data.objects.remove(tmp, do_unlink=True)
        bm = bmesh.new()
        bm.from_mesh(me)
        bpy.data.meshes.remove(me)
        world = _lay([(v.co.x, v.co.y) for v in bm.verts], centre, normal, up, 1.0, tree, 0.006)
        for v, p in zip(bm.verts, world):
            v.co = inv @ p
        nn = inv.to_3x3() @ Vector(normal)
        bm.normal_update()
        if sum(f.normal.dot(nn) for f in bm.faces) < 0:
            bmesh.ops.reverse_faces(bm, faces=bm.faces)
        return _obj(n, bm, mark(mats, colour), parent, (0, 0, 0), (0, 0, 0))

    _each(f"marking_{name}", lods, make)


def markings(v, rows):
    """Paint `rows` on an airframe once it is built: each
    `("insignia" | "text", keywords)` for `insignia` or `lettering`, and,
    when its `centre` is off the centreline, its mirror on the other side
    (the letters read the right way round from each side)."""
    for k, (what, kw) in enumerate(rows):
        kw = dict(kw)
        mirror = kw.pop("mirror", abs(kw["centre"][1]) > 1e-6)
        sides = [(kw["centre"], kw["normal"], "L" if kw["centre"][1] >= 0 else "R")]
        if mirror:
            c, n = kw["centre"], kw["normal"]
            sides.append(((c[0], -c[1], c[2]), (n[0], -n[1], n[2]), "R" if c[1] >= 0 else "L"))
        for centre, normal, s in sides:
            args = dict(kw, centre=centre, normal=normal, mats=v.mats, parent=v.hull)
            name = f"{what}_{k}_{s}"
            if what == "insignia":
                insignia(name, **args)
            else:
                lettering(name, **args)
