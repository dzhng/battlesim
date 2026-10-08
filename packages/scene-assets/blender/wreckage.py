"""Destruction modelling for wrecks: the vehicle's own parts deformed, torn and
cut, and debris thrown round it, never only a burnt paint.

A wreck script builds the live vehicle's parts (`tank.py --wreck`,
`supply_truck.py --wreck`), so its hull is the live hull, then works them over
with these helpers before `finish()` bakes paint and occlusion onto the result:

- `densify` splits long edges, per tier, so a plate has vertices to bend;
- `warp` moves every vertex of some parts by a world-space field: `heat`
  (plates warped by the fire), `dent` (a hit pushed in, or an explosion
  pushing out), `sag` (a run that droops between two points);
- `bend` folds a part about a line, as a torn skirt or a crushed fender does;
- `ragged` eats an edge of a part away, so a plate ends in a torn line;
- `hollow` makes a part a shell, and `cut` punches a jagged hole through it (exact
  booleans), so every opening shows an interior with depth, not a painted patch;
- `plate` makes a torn sheet (a jagged outline with thickness) for debris.

Every roster wreck (`--wreck` on its exporter) ends the same way: after the
family's own damage, `burn` turns what is left to burnt steel and char before
`finish()`, and `export_wreck` writes the wreck and, where it has a turret to
throw, its `hull` and `turret` pieces, each where it lies in the whole, as
`tank.py --wreck --piece=` does.

Every helper applies to all four tiers alike, with the same field, so the
coarser tiers are the same wreck. Deterministic: fixed seeds, no randomness
from the clock.
"""
import math
import os
import random

import bmesh
import bpy
from mathutils import Matrix, Vector, noise

from parts import _split_long_edges, mesh_part, smoothstep, tier_of

# Longest edge a deformed part keeps, per tier (finest first): 12 cm is enough
# for a warped plate to read at the close view.
EDGE_M = (0.12, 0.3, 0.8, None)


def parts(*prefixes):
    """Every tier's mesh object of the parts named by `prefixes`."""
    return [o for o in bpy.data.objects if o.type == "MESH" and o.name.startswith(prefixes)]


def densify(objs, scale=1.0):
    for o in objs:
        t = tier_of(o)
        limit = EDGE_M[t if t is not None else 0]
        if limit:
            _split_long_edges(o.data, limit * scale)


def warp(objs, *fields):
    """Move each vertex of `objs` by the sum of `fields` (world point -> world offset)."""
    bpy.context.view_layer.update()
    for o in objs:
        mw = o.matrix_world
        inv = mw.inverted_safe()
        for v in o.data.vertices:
            w = mw @ v.co
            d = Vector((0, 0, 0))
            for f in fields:
                d += f(w)
            v.co = inv @ (w + d)
        o.data.update()


def heat(amp, scale, seed=0.0):
    """Plates warped by the fire: a smooth vector field about `scale` metres across."""
    off = Vector((seed, seed * 1.7, seed * 2.3))
    # One scalar noise per axis, at fixed offsets: `noise.noise_vector` is not
    # repeatable between Blender processes (its offsets are drawn per run, and
    # `noise.seed_set` does not fix them), so a wreck would differ every export.
    axes = (Vector((0.0, 0.0, 0.0)), Vector((31.4, 0.0, 0.0)), Vector((0.0, 47.1, 0.0)))

    def f(p):
        q = p / scale + off
        return Vector([noise.noise(q + a, noise_basis="PERLIN_ORIGINAL") for a in axes]) * amp

    return f


def dent(centre, radius, depth, direction=None, seed=0.0):
    """A dent `depth` metres deep toward `direction` (default: toward `centre`'s
    inside, i.e. down) fading to nothing at `radius`; ragged at its rim. A
    negative depth bulges out."""
    c = Vector(centre)
    axis = Vector(direction).normalized() if direction is not None else Vector((0, 0, -1))

    def f(p):
        d = (p - c).length
        if d >= radius:
            return Vector((0, 0, 0))
        k = 1.0 - smoothstep(0.0, radius, d)
        k *= 0.75 + 0.25 * noise.noise(p * 9.0 + Vector((seed, 0, 0)), noise_basis="PERLIN_ORIGINAL")
        return axis * (depth * k * k)

    return f


def sag(a, b, depth, width):
    """A span from `a` to `b` (world, horizontal) that droops `depth` in the middle,
    for points within `width` of the line."""
    a, b = Vector(a), Vector(b)
    ab = b - a
    length = ab.length
    ab.normalize()

    def f(p):
        t = (p - a).dot(ab)
        if t <= 0 or t >= length:
            return Vector((0, 0, 0))
        off = (p - a) - ab * t
        if Vector((off.x, off.y, 0)).length > width:
            return Vector((0, 0, 0))
        return Vector((0, 0, -depth * math.sin(math.pi * t / length)))

    return f


def bend(objs, origin, axis, normal, angle):
    """Fold `objs` about the line through `origin` along `axis`: every point on the
    `normal` side of the line turns by `angle` about it (world space)."""
    o_, ax, nm = Vector(origin), Vector(axis).normalized(), Vector(normal).normalized()
    turn = Matrix.Rotation(angle, 4, ax)
    bpy.context.view_layer.update()
    for o in objs:
        mw = o.matrix_world
        inv = mw.inverted_safe()
        for v in o.data.vertices:
            w = mw @ v.co
            r = w - o_
            if r.dot(nm) > 0:
                v.co = inv @ (o_ + turn @ r)
        o.data.update()


def ragged(objs, edge_axis, keep_above, depth, seed=0):
    """Tear an edge away: vertices of `objs` within `depth` of the plane
    (`edge_axis` · p = `keep_above`, world) are pulled up past it by a jagged,
    per-position amount, so the edge ends in a torn line."""
    ax = Vector(edge_axis).normalized()
    bpy.context.view_layer.update()
    for o in objs:
        mw = o.matrix_world
        inv = mw.inverted_safe()
        for v in o.data.vertices:
            w = mw @ v.co
            h = w.dot(ax) - keep_above
            if h < depth:
                jag = depth * (0.5 + 0.5 * noise.noise(w * 7.0 + Vector((seed, 0, 0)), noise_basis="PERLIN_ORIGINAL"))
                jag += depth * 0.35 * abs(math.sin(w.x * 23.0 + w.y * 17.0 + seed))
                if h < jag:
                    w = w + ax * (jag - h)
                    v.co = inv @ w
        o.data.update()


def ragged_outline(radius, n, rough, seed, squash=1.0):
    """A closed polygon about the origin: a circle of `radius` whose rim wanders
    (two slow lobes) and tears (a small jitter per point), never a star."""
    rng = random.Random(seed)
    p1, p2 = rng.uniform(0, 2 * math.pi), rng.uniform(0, 2 * math.pi)
    pts = []
    for k in range(n):
        a = 2 * math.pi * k / n
        lobe = 0.6 * math.sin(2 * a + p1) + 0.4 * math.sin(3 * a + p2)
        r = radius * (1.0 + rough * lobe + 0.35 * rough * (rng.random() * 2 - 1))
        pts.append((math.cos(a) * r, math.sin(a) * r * squash))
    return pts


def _prism_bm(bm, outline, depth):
    lo = [bm.verts.new((x, y, -depth / 2)) for x, y in outline]
    hi = [bm.verts.new((x, y, depth / 2)) for x, y in outline]
    bm.faces.new(lo[::-1])
    bm.faces.new(hi)
    n = len(outline)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)


def frame(centre, normal, up=(0, 0, 1)):
    """A world frame at `centre` whose Z is `normal`, X horizontal across it and Y
    as near `up` as it can be: an outline's (x, y) lie across a wall's face, x along
    the ground and y up it (for a horizontal `normal`, y is world Y instead)."""
    z = Vector(normal).normalized()
    u = Vector(up)
    if abs(z.dot(u.normalized())) > 0.99:
        u = Vector((0, 1, 0))
    x = u.cross(z).normalized()
    y = z.cross(x)
    m = Matrix((x, y, z)).transposed().to_4x4()
    m.translation = Vector(centre)
    return m


def _subtract(objs, bm):
    """Subtract the closed mesh in `bm` (world space) from each of `objs` (exact
    boolean), then order the result canonically: the solver's threads must not
    reach the exported bytes."""
    me = bpy.data.meshes.new("cutter")
    bm.to_mesh(me)
    bm.free()
    cutter = bpy.data.objects.new("cutter", me)
    bpy.context.scene.collection.objects.link(cutter)
    bpy.context.view_layer.update()
    for o in objs:
        mod = o.modifiers.new("cut", "BOOLEAN")
        mod.operation = "DIFFERENCE"
        mod.solver = "EXACT"
        mod.object = cutter
        dg = bpy.context.evaluated_depsgraph_get()
        me2 = bpy.data.meshes.new_from_object(o.evaluated_get(dg), preserve_all_data_layers=True, depsgraph=dg)
        o.modifiers.remove(mod)
        mats = list(o.data.materials)
        o.data = me2
        me2.materials.clear()
        for m in mats:
            me2.materials.append(m)
        for p in me2.polygons:
            p.material_index = 0
        canonical(o)
    bpy.data.objects.remove(cutter, do_unlink=True)


def cut(objs, centre, normal, outline, depth, up=(0, 0, 1)):
    """Punch a hole through `objs`: a prism of `outline` (x, y in `frame(centre,
    normal, up)`) `depth` long, centred on `centre` along `normal` (world)."""
    bm = bmesh.new()
    _prism_bm(bm, outline, depth)
    bmesh.ops.transform(bm, matrix=frame(centre, normal, up), verts=bm.verts)
    _subtract(objs, bm)


def hollow(objs, at, size):
    """Make `objs` a shell: subtract the box of full `size` at the world matrix
    `at` (its centre and axes), which must lie wholly inside them, so a hole cut
    through the skin opens onto a lit, occluded interior rather than a solid."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.transform(bm, matrix=at @ Matrix.Diagonal((*size, 1.0)), verts=bm.verts)
    _subtract(objs, bm)


def canonical(o):
    """Hash-stable geometry: positions snapped to 0.1 mm, then vertices ordered by
    position and neighbours and faces by their vertices, whatever order the
    boolean's threads produced them in."""
    for v in o.data.vertices:
        v.co = Vector(round(c, 4) for c in v.co)
    bm = bmesh.new()
    bm.from_mesh(o.data)

    def vkey(v):
        ring = Vector((0, 0, 0))
        for e in v.link_edges:
            ring += e.other_vert(v).co
        return (tuple(v.co), tuple(round(c, 5) for c in ring), len(v.link_edges))

    rank = {v: i for i, v in enumerate(sorted(bm.verts, key=vkey))}
    bm.verts.sort(key=lambda v: rank[v])
    bm.verts.index_update()
    order = sorted(bm.faces, key=lambda f: (sorted(v.index for v in f.verts), [v.index for v in f.verts]))
    rank = {f: i for i, f in enumerate(order)}
    bm.faces.sort(key=lambda f: rank[f])
    bm.to_mesh(o.data)
    bm.free()


def plate(name, outline, thick, loc, rot, mat, parent=None, lods=(0, 1, 2), curl=0.0, seed=0):
    """A torn sheet of metal: `outline` (x, y) with `thick`ness, curled by `curl`
    (metres of rise across its width), lying at `loc` turned by `rot`."""

    def build(bm, lod):
        _prism_bm(bm, outline, max(thick, 0.04))
        if lod < 2:
            bmesh.ops.subdivide_edges(bm, edges=[e for e in bm.edges if e.calc_length() > 0.15], cuts=2,
                                      use_grid_fill=True)
        for v in bm.verts:
            v.co.z += curl * (v.co.x ** 2) + 0.05 * noise.noise(v.co * 4.0 + Vector((seed, 0, 0)))

    return mesh_part(name, build, mat, parent, lods=lods, loc=loc, rot=rot)


def remove(*prefixes):
    for o in parts(*prefixes):
        bpy.data.objects.remove(o, do_unlink=True)


# ------------------------------------------------------------- the burn and the pieces
WRECK_ARG = "--wreck"
# The articulated rig a wreck keeps from its live vehicle: the turret it throws
# (`MOUNT_NODES.gun.yaw`), and the pitch nodes whose barrels sag.
TURRET = "turret"
BARRELS = ("gun", "hmg_gun")
SAG_DEG = 7.0
# The cook-off heaved the turret off its ring: it sits askew across the deck,
# where its thrown piece lands (`tank.py`'s wreck sits the same way).
HEAVE_DEG = (-5.0, 3.0, 24.0)  # roll, pitch, yaw
HEAVE_M = (-0.25, 0.3, 0.08)
# What was rubber, glass, canvas or a black opening burns to char; the rest is
# steel, its paint burnt off.
CHARRED = ("rubber", "glass", "fabric")


def wreck_paths(out):
    """A wreck's files beside the live vehicle's `out`: its whole
    (`default`) and its `hull` and `turret` pieces."""
    stem = out[:-4] if out.endswith(".glb") else out
    return {"default": f"{stem}_wreck.glb", "hull": f"{stem}_wreck_hull.glb", "turret": f"{stem}_wreck_turret.glb"}


def burn():
    """The damaged vehicle burnt out, before `finish()`: every
    surface burnt (smoke-blackened steel, char where rubber, glass and canvas
    were), its dressing (antennas, masts) and crew gone, its barrels sagging on
    broken trunnions, its turret heaved askew off the ring, and the fire vented
    through the ring."""
    from parts import SCORCH, textured
    shell = textured("wreck_steel", "burnt_metal", chip=0.6, dirt=0.3, soot=0.6, ash=0.3, seed=7.0)
    char = textured("wreck_char", "burnt_metal", colour=(0.02, 0.019, 0.018), chip=0.2, dirt=0.2, seed=9.0)
    # Decide everything first: removing a dressing node would orphan its parts.
    gone = [o for o in bpy.data.objects
            if any(a.name.startswith("dressing_") for a in (o, *_ancestors(o)))
            or (o.type == "MESH" and any(m and m.name.startswith("crew_") for m in o.data.materials))]
    for o in gone:
        bpy.data.objects.remove(o, do_unlink=True)
    for o in bpy.data.objects:
        if o.type != "MESH":
            continue
        was = o.data.materials[0] if o.data.materials else None
        burnt = char if was is not None and (was.get("role") in CHARRED or was.name.startswith("black")) else shell
        for k in range(len(o.data.materials)):
            o.data.materials[k] = burnt
        if not o.data.materials:
            o.data.materials.append(burnt)
        # UVs made for another texture (a track's links) would sample the
        # burnt one at their scale: let the export box-project it in metres.
        while o.data.uv_layers:
            o.data.uv_layers.remove(o.data.uv_layers[0])
    for name in BARRELS:
        pitch = bpy.data.objects.get(name)
        if pitch is not None:
            pitch.rotation_euler.y += math.radians(SAG_DEG)
    turret = bpy.data.objects.get(TURRET)
    if turret is not None:
        SCORCH.append((turret.matrix_world.translation.copy(), 2.4))
        for k in range(3):
            turret.rotation_euler[k] += math.radians(HEAVE_DEG[k])
            turret.location[k] += HEAVE_M[k]
    bpy.context.view_layer.update()


def _ancestors(o):
    while o.parent is not None:
        o = o.parent
        yield o


def export_wreck(out, **export_kw):
    """Export the burnt vehicle beside `out` (`wreck_paths`): whole, then, if
    it has a turret, its hull without it and its turret alone, unparented
    where it lies in the whole. Returns the states written, by name."""
    import tempfile
    from parts import export
    paths = wreck_paths(out)
    export(paths["default"], **export_kw)
    if bpy.data.objects.get(TURRET) is None:
        return {"default": paths["default"]}
    # Each piece deletes the other: cut the hull from the scene, then the
    # turret from a copy of it (the exporter follows parents, not collections).
    whole = os.path.join(tempfile.mkdtemp(), "wreck.blend")
    bpy.ops.wm.save_as_mainfile(filepath=whole, copy=True)
    for keep_turret in (False, True):
        if keep_turret:
            bpy.ops.wm.open_mainfile(filepath=whole)
        turret = bpy.data.objects[TURRET]
        thrown = {turret, *turret.children_recursive}
        if keep_turret:
            lies = turret.matrix_world.copy()
            turret.parent = None
            turret.matrix_world = lies
        for o in list(bpy.data.objects):
            if (o in thrown) != keep_turret:
                bpy.data.objects.remove(o, do_unlink=True)
        bpy.context.view_layer.update()
        export(paths["turret" if keep_turret else "hull"], **export_kw)
    os.remove(whole)
    return paths


__all__ = ["parts", "densify", "warp", "heat", "dent", "sag", "bend", "ragged", "ragged_outline", "frame", "cut", "hollow",
           "plate", "remove", "WRECK_ARG", "burn", "export_wreck", "wreck_paths"]
