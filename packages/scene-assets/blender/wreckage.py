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
family's own damage, `scatter` throws debris clear of the hull from what the
vehicle carries, `burn` turns what is left to burnt steel and char before
`finish()`, and `export_wreck` writes the wreck and, where it has a turret to
throw, its `hull` and `turret` pieces, each where it lies in the whole, and
its thrown `debris` apart (`cut_to`).

Thrown debris is presentation, not cover: it lies past the box the simulation
keeps, so the battle draws it only where it watched the death and lets it sink
away (`effects/cookOff.ts`). Everything under a `debris_*` node is thrown
debris, exported only in the wreck's `debris` state and held there to its own
allowance (`SCENERY_KINDS.wreck.debris`); a family names nothing `debris_*`
itself. What a wreck keeps on or beside its hull (a hatch on the deck, a panel
in a wheel gap) is the wreck's own and stays inside its footprint.

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

from parts import _split_long_edges, empty, mesh_part, smoothstep, tier_of

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
    (`default`), its `hull` and `turret` pieces and its thrown `debris`."""
    stem = out[:-4] if out.endswith(".glb") else out
    return {"default": f"{stem}_wreck.glb", "hull": f"{stem}_wreck_hull.glb", "turret": f"{stem}_wreck_turret.glb",
            "debris": f"{stem}_wreck_debris.glb"}


def burn():
    """The damaged vehicle burnt out, before `finish()`: every
    surface burnt (smoke-blackened steel, char where rubber, glass and canvas
    were), its thrown debris with it, its dressing (antennas, masts) and crew gone, its barrels sagging on
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


def cut_to(state):
    """Cut the finished wreck in the scene down to one of its states, each
    where it lies in the whole: `default` (the wreck without its thrown
    debris), `hull` (that without its turret), `turret` (the turret alone)
    or `debris` (the `debris_*` trees alone). A kept part whose parent goes
    keeps its place in the world."""
    bpy.context.view_layer.update()
    turret = bpy.data.objects.get(TURRET)
    thrown = {turret, *turret.children_recursive} if turret is not None else set()
    debris = {o for o in bpy.data.objects if is_debris(o)}
    keep = {"default": lambda o: o not in debris,
            "hull": lambda o: o not in debris and o not in thrown,
            "turret": lambda o: o in thrown,
            "debris": lambda o: o in debris}[state]
    kept = [o for o in bpy.data.objects if keep(o)]
    for o in kept:
        if o.parent is not None and not keep(o.parent):
            lies = o.matrix_world.copy()
            o.parent = None
            o.matrix_world = lies
    for o in [o for o in bpy.data.objects if not keep(o)]:
        bpy.data.objects.remove(o, do_unlink=True)
    bpy.context.view_layer.update()


def export_wreck(out, **export_kw):
    """Export the burnt vehicle beside `out` (`wreck_paths`): whole, then, if
    it has a turret, its hull without it and its turret alone, and, if it
    threw any, its debris (`cut_to`). Returns the states written, by name."""
    import tempfile
    from parts import export
    paths = wreck_paths(out)
    states = ["default"]
    if bpy.data.objects.get(TURRET) is not None:
        states += ["hull", "turret"]
    if any(is_debris(o) for o in bpy.data.objects):
        states.append("debris")
    # Each state cuts the others from a copy of the whole (the exporter
    # follows parents, not collections).
    whole = os.path.join(tempfile.mkdtemp(), "wreck.blend")
    bpy.ops.wm.save_as_mainfile(filepath=whole, copy=True)
    for k, state in enumerate(states):
        if k:
            bpy.ops.wm.open_mainfile(filepath=whole)
        cut_to(state)
        export(paths[state], **export_kw)
    os.remove(whole)
    return {state: paths[state] for state in states}


# ------------------------------------------------------------- thrown debris
DEBRIS = "debris_"
# The gap from the hull's box to a thrown piece's nearest point, metres: at
# least the first, and up to the first and the second.
THROW_GAP_M = (0.5, 3.5)
# A piece lying flat rests this far above the ground, so the two don't fight
# for depth.
LIFT_M = 0.006
# How many pieces a hull throws: one per this many metres of its box's
# perimeter, within the bounds.
PIECE_EVERY_M = 3.5
PIECES = (3, 7)


def is_debris(o):
    """Whether `o` is, or hangs under, a `debris_*` node: thrown debris."""
    return any(a.name.startswith(DEBRIS) for a in (o, *_ancestors(o)))


def _carried():
    """What the built vehicle carries for the blast to throw: its tracks'
    width (None without tracks), its road wheels' radius (None without
    `wheel_*` nodes, or with tracks), and whether it carries jerrycans."""
    bpy.context.view_layer.update()
    track = None
    for side in ("track_L", "track_R"):
        node = bpy.data.objects.get(side)
        if node is None:
            continue
        ys = [(o.matrix_world @ v.co).y for o in node.children_recursive
              if o.type == "MESH" and tier_of(o) in (0, None) for v in o.data.vertices]
        if ys:
            track = max(track or 0.0, max(ys) - min(ys))
    wheel = None
    if track is None:
        radii = [o["radius_m"] for o in sorted(bpy.data.objects, key=lambda o: o.name)
                 if o.type == "EMPTY" and o.name.startswith("wheel_") and "radius_m" in o]
        wheel = max(radii) if radii else None
    cans = any("jerrycan" in o.name for o in bpy.data.objects)
    return track, wheel, cans


def _place_on(rng, half, t):
    """A point `t` metres across the ground from the box of half extents
    `half` (on the box grown by `t`, a rounded rectangle), chosen evenly
    along its length by `rng`."""
    hx, hy = half
    sides = (2 * hy, 2 * hx, 2 * hy, 2 * hx)
    arc = math.pi / 2 * t
    s = rng.random() * (sum(sides) + 4 * arc)
    # Walk the straight sides and corner arcs counter-clockwise from the
    # front right corner: +x side, +y side, -x side, -y side.
    corners = ((hx, -hy), (hx, hy), (-hx, hy), (-hx, -hy))
    for k in range(4):
        cx, cy = corners[k]
        nx, ny = corners[(k + 1) % 4]
        out = (math.pi / 2) * k  # the outward normal's angle along this side
        if s < sides[k]:
            u = s / sides[k]
            return (cx + (nx - cx) * u + t * math.cos(out), cy + (ny - cy) * u + t * math.sin(out))
        s -= sides[k]
        if s < arc:
            a = out + s / t if t > 0 else out
            return (nx + t * math.cos(a), ny + t * math.sin(a))
        s -= arc
    return (hx + t, 0.0)


def _meshes(e):
    return [o for o in e.children_recursive if o.type == "MESH"]


def _piece(kind, k, mats, rng, track, wheel):
    """Build one thrown piece of `kind` lying at the origin under its own
    `debris_<kind>_<k>` node; returns the node, how far the piece reaches from
    it across the ground, and whether its plates warp in the fire."""
    from parts import box, mesh_part
    from vehicle_parts import ALL, bolted_panel, hatch, jerrycan, stowage_box, tyre_wheel
    e = empty(f"{DEBRIS}{kind}_{k}")
    name = f"thrown_{kind}_{k}"
    if kind == "track":
        length, width = rng.uniform(1.8, 2.6), track

        def run(bm, lod):
            cube = bmesh.ops.create_cube(bm, size=1.0)
            bmesh.ops.transform(bm, matrix=Matrix.Diagonal((length, width, 0.05, 1.0)), verts=cube["verts"])
            if lod < 2:  # the links' grousers across the run
                for j in range(int(length / 0.17)):
                    x = -length / 2 + 0.085 + j * 0.17
                    g = bmesh.ops.create_cube(bm, size=1.0)
                    bmesh.ops.transform(bm, matrix=Matrix.Translation((x, 0, 0.04))
                                        @ Matrix.Diagonal((0.05, width, 0.035, 1.0)), verts=g["verts"])

        mesh_part(name, run, mats["track"], e, lods=ALL)
        return e, math.hypot(length, width) / 2, True
    if kind == "wheel":
        width = wheel * 0.7
        node = tyre_wheel(f"wheel_{name}", (0, 0, width / 2), wheel, width, 1, mats, e)
        node.rotation_euler = (math.pi / 2, 0, 0)
        return e, wheel, False
    if kind == "hatch":
        hatch(name, (0, 0, 0), mats, e, radius=0.34, rot=(math.pi, 0, 0))
        box(f"{name}_slab", (0.5, 0.5, 0.02), (0, 0, 0.01), mats["paint"], e, lods=(3,))
        return e, 0.42, False
    if kind in ("pack", "door"):
        size = ((rng.uniform(0.7, 1.05), rng.uniform(0.45, 0.65), 0.09) if kind == "pack"
                else (rng.uniform(0.9, 1.15), rng.uniform(0.75, 0.95), 0.05))
        bolted_panel(name, (0, 0, 0), size, mats, e, bolts=(3, 2) if kind == "pack" else (2, 2), lods=ALL)
        return e, math.hypot(size[0], size[1]) / 2, True
    if kind == "crate":
        size = (rng.uniform(0.7, 0.95), 0.45, 0.42)
        # Tipped onto its back, centred on the node.
        stowage_box(name, (0, size[2] / 2, size[1] / 2), size, mats, e, rot=(math.pi / 2, 0, 0))
        box(f"{name}_far", (size[0], size[2], size[1]), (0, 0, size[1] / 2), mats["paint"], e, lods=(3,))
        return e, math.hypot(size[0], size[2]) / 2, False
    if kind == "can":
        # Lying on its side, centred on the node.
        jerrycan(name, (-0.235, 0, 0.0825), mats, e, rot=(0, math.pi / 2, 0))
        return e, 0.3, False
    plate(name, ragged_outline(rng.uniform(0.3, 0.5), 10, 0.25, rng.randrange(1 << 16)), 0.03, (0, 0, 0),
          (0, 0, 0), mats["paint"], e, lods=ALL, curl=0.12, seed=rng.randrange(1 << 16))
    return e, 0.6, True


def scatter(half, mats, seed):
    """Throw debris clear of a wreck whose hull box has half extents `half`
    (x, y, about the origin), from what the built vehicle carries: a run of
    track off a tracked hull, a wheel off a wheeled one, a blown hatch or a
    door, armour packs, a stowage box, a jerrycan if it carries them, torn
    plate; only torn plate off anything else (an airframe). More off a
    larger hull. Each piece lies flat on the ground under its own `debris_*`
    node at the scene's root, so the wreck's own tilt doesn't lift it,
    between `THROW_GAP_M` of the box, and never on another. `mats` are the
    vehicle's materials by role (`vehicle_export.materials`); `seed` picks
    every throw, so the same seed throws the same debris.

    Call it once the wreck lies as it will (after the family's damage and
    its settling) and before `burn`, which burns the debris with the wreck.
    `export_wreck` writes it as the wreck's `debris` state."""
    rng = random.Random(seed)
    track, wheel, cans = _carried()
    if track:
        kinds = ["track", "hatch", "pack", "crate", "pack"] + (["can"] if cans else []) + ["plate", "plate"]
    elif wheel:
        kinds = ["wheel", "door", "pack", "crate"] + (["can"] if cans else []) + ["wheel", "plate"]
    else:
        kinds = ["plate"] * PIECES[1]
    count = min(max(round(4 * (half[0] + half[1]) / PIECE_EVERY_M), PIECES[0]), PIECES[1])
    lying = []
    for k, kind in enumerate(kinds[:count]):
        e, reach, warps = _piece(kind, k, mats, rng, track, wheel)
        # Clear of the hull and of every piece already down.
        for _ in range(40):
            gap = THROW_GAP_M[0] + rng.random() * THROW_GAP_M[1]
            x, y = _place_on(rng, half, gap + reach)
            if all(math.hypot(x - px, y - py) > reach + pr + 0.2 for px, py, pr in lying):
                break
        lying.append((x, y, reach))
        e.location = (x, y, 0)
        e.rotation_euler = (rng.uniform(-0.05, 0.05), rng.uniform(-0.05, 0.05), rng.uniform(0, math.tau))
        bpy.context.view_layer.update()
        meshes = _meshes(e)
        if warps:
            densify(meshes)
            warp(meshes, heat(0.03, 0.5, seed=float(rng.randrange(1000))))
        bpy.context.view_layer.update()
        low = min((o.matrix_world @ v.co).z for o in meshes for v in o.data.vertices)
        e.location.z += LIFT_M - low
    bpy.context.view_layer.update()


__all__ = ["parts", "densify", "warp", "heat", "dent", "sag", "bend", "ragged", "ragged_outline", "frame", "cut", "hollow",
           "plate", "remove", "WRECK_ARG", "burn", "export_wreck", "wreck_paths", "cut_to", "DEBRIS", "is_debris", "scatter"]
