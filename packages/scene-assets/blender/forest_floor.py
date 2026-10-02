"""Forest floor appearance sources: the bodies that lie on it, and its dressing.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/forest_floor.py [out_dir] [kind...]

writes assets/source/forest/<kind>.glb (then `asset bake`): every kind, or
those named.

Two bodies the simulation places (`fixtures/props/forest/`), each authored to
its box (the catalog's `footprint_half_m`), origin at the box's centre on the
ground, +X along its first half extent:

  log      a fallen trunk, its ends broken off and a few stubs left on it;
           box [2.2, 0.35, 0.35]
  boulder  one rounded rock, mossed on top, sunk in the soil; box [1, 0.8, 0.75]

and the dressing the scenery layer scatters under the trees. A dressing kind
has no body, so none stands taller than a man's waist (0.9 m; the validator's
`fit.dressing`) and a rock stays under his knee:

  floor_fern     a rosette of arching fronds
  floor_bush     a low shrub of leaf clumps
  floor_sapling  a young tree: a thin stem and a few clumps
  floor_rock     a small rock and two stones beside it
  floor_litter   fallen branches

Bark is the tree generator's (`trees.tube`: a tube whose ring frame is carried
along its centreline, furrowed on the finest tier), and leaf clumps are its
clumps, smaller. A rock is the intersection of a ball with seeded planes, its
arrises rounded: along every direction the distance to the surface is the
power mean of the distances to each plane, so a face is flat and the edge
between two is a curve, never a crease or a pyramid's point.

Four tiers each, the same shape coarser. Colour is vertex colour over one
material a part (`bark`, `wood`, `leaves`, `rock`); the biome tints dressing
per kind. Origin on the ground; a body runs a little below it, so it does not
float where the ground slopes.
"""

import math
import os
import random
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector, noise

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from trees import BARK, LEAVES, PAD_LIGHT, ROOT, UP, bezier, clump_points, export, finish, icosphere
from trees import leaf_colour, material, smoothstep, tube

ARGS = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
OUT = os.path.abspath(ARGS[0]) if ARGS else os.path.join(ROOT, "assets", "source", "forest")

TIERS = 4
# Linear albedo. Dead bark is the trees' own, weathered grey; `MOSS` is what
# grows on a body, as a share of its material's colour.
DEAD_BARK = (0.17, 0.15, 0.115, 1.0)
WOOD = (0.36, 0.28, 0.17, 1.0)
ROCK = (0.125, 0.12, 0.11, 1.0)
MOSS = (0.42, 0.62, 0.2)
# How many planes cut a rock, and the power of the mean that rounds their
# arrises: higher is crisper.
FACETS = 12
ROUND = 9.0
# A rock's underside, as a share of its top's depth: it sits in the soil on
# its widest section, not on a point.
SIT = 0.3


class Solid:
    """One part under construction: vertices with a colour each (given, or a
    function of the finished vertex's position and normal), and the normals
    given outright."""

    def __init__(self):
        self.bm = bmesh.new()
        self.colours = []
        self.normals = {}

    def vert(self, co, colour, normal=None):
        if normal is not None:
            self.normals[len(self.colours)] = normal.normalized()
        self.colours.append(colour if callable(colour) else (*colour, 1.0))
        return self.bm.verts.new(co)

    def face(self, verts, toward=None):
        """A face of `verts`, turned to look along `toward` when given."""
        face = self.bm.faces.new(verts)
        if toward is not None:
            face.normal_update()
            if face.normal.dot(toward) < 0.0:
                face.normal_flip()

    def mesh(self, name):
        self.bm.normal_update()
        mesh = bpy.data.meshes.new(name)
        self.bm.to_mesh(mesh)
        self.bm.free()
        normals = [self.normals.get(i, v.normal.copy()) for i, v in enumerate(mesh.vertices)]
        colours = [
            (*c(v.co, n), 1.0) if callable(c) else c for c, v, n in zip(self.colours, mesh.vertices, normals)
        ]
        return finish(mesh, normals, colours)


def mix(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


# ----------------------------------------------------------------------- rocks


def rock_points(seed, subdiv, relief):
    """A rock's surface in its own unit space, one point per icosphere
    direction: a ball cut by seeded planes with rounded arrises, lumpy, the
    finer tiers pitted (`relief`)."""
    rng = random.Random(seed)
    planes = [
        (Vector((rng.gauss(0, 1), rng.gauss(0, 1), rng.gauss(0, 1))).normalized(), rng.uniform(0.45, 0.85))
        for _ in range(FACETS)
    ]
    at = Vector((seed * 1.37, seed * 0.61, 0.0))
    out = []
    for d in icosphere(subdiv)[0]:
        cut = 1.0 + sum((d.dot(n) / h) ** ROUND for n, h in planes if d.dot(n) > 0.0)
        r = cut ** (-1.0 / ROUND)
        r *= 1.0 + 0.08 * noise.noise(d * 1.6 + at) + 0.03 * relief * noise.noise(d * 5.3 + at)
        p = d * r
        out.append(Vector((p.x, p.y, p.z * SIT if p.z < 0.0 else p.z)))
    return out


def rock_colour(seed):
    """A rock's vertex colour: mottled, pale with lichen in patches, mossed
    where it faces up, stained by the soil at its foot."""
    at = Vector((seed * 0.71, 0.0, seed * 1.3))

    def colour(p, n):
        tone = 0.62 + 0.26 * noise.noise(p * 1.9 + at) + 0.1 * noise.noise(p * 8.3 + at)
        rgb = (tone, tone * 0.985, tone * 0.95)
        rgb = mix(rgb, (1.0, 0.99, 0.94), 0.35 * smoothstep(0.2, 0.5, noise.noise(p * 5.1 + at * 2.0)))
        moss = smoothstep(0.2, 0.75, n.z) * smoothstep(-0.3, 0.2, noise.noise(p * 1.7 + at * 3.0))
        rgb = mix(rgb, MOSS, 0.85 * moss)
        soil = 1.0 - smoothstep(0.0, 0.3, p.z)
        return mix(rgb, (0.5, 0.42, 0.33), 0.6 * soil)

    return colour


def rock(solid, seed, subdiv, relief, place):
    """One rock into `solid`: its unit-space surface through `place`."""
    colour = rock_colour(seed)
    verts = [solid.vert(place(p), colour) for p in rock_points(seed, subdiv, relief)]
    for a, b, c in icosphere(subdiv)[1]:
        solid.face((verts[a], verts[b], verts[c]))


def boulder(tier):
    half, sink = Vector((1.0, 0.8, 0.75)), 0.12
    seed = 31
    # The finest tier fills the box exactly, every tier through the same fit.
    fine = rock_points(seed, 4, 1.0)
    lo = Vector((min(p.x for p in fine), min(p.y for p in fine), min(p.z for p in fine)))
    hi = Vector((max(p.x for p in fine), max(p.y for p in fine), max(p.z for p in fine)))

    def place(p):
        return Vector(
            (
                (2.0 * (p.x - lo.x) / (hi.x - lo.x) - 1.0) * half.x,
                (2.0 * (p.y - lo.y) / (hi.y - lo.y) - 1.0) * half.y,
                -sink + (p.z - lo.z) / (hi.z - lo.z) * (2.0 * half.z + sink),
            )
        )

    solid = Solid()
    # Faces enough to keep its facets flat and their arrises round from as
    # far as it is seen as a body: the coarsest alone is 20.
    rock(solid, seed, (4, 3, 2, 1)[tier], (1.0, 0.6, 0.0, 0.0)[tier], place)
    return [("rock", solid, "rock")]


def floor_rock(tier):
    stones = [
        (43, Vector((0.0, 0.0, 0.1)), Vector((0.4, 0.3, 0.27)), 0.0, (2, 1, 1, 1)),
        (47, Vector((0.36, 0.2, 0.03)), Vector((0.17, 0.13, 0.11)), 1.1, (1, 1, None, None)),
        (53, Vector((-0.3, -0.27, 0.02)), Vector((0.13, 0.11, 0.08)), 2.3, (1, None, None, None)),
    ]
    solid = Solid()
    for seed, centre, axes, turn, subdivs in stones:
        if subdivs[tier] is None:
            continue
        spin = Matrix.Rotation(turn, 3, "Z")

        def place(p, centre=centre, axes=axes, spin=spin):
            q = centre + spin @ Vector((p.x * axes.x, p.y * axes.y, p.z * axes.z))
            return Vector((q.x, q.y, max(q.z, 0.0)))

        rock(solid, seed, subdivs[tier], 1.0 if tier == 0 else 0.0, place)
    return [("rock", solid, "rock")]


# ------------------------------------------------------------------------ wood


def dead_bark(seed, moss):
    """Bark's vertex colour on a fallen bole about the centreline point `p`:
    mottled, paler on top and bare to the grey wood in patches there, mossed,
    dark where it lies on the ground."""
    at = Vector((seed * 0.9, seed * 0.3, 0.0))

    def shade(p, v):
        out = v - p
        up = out.z / max(out.length, 1e-6)
        tone = (0.36 + 0.2 * smoothstep(-0.6, 0.8, up)) * (0.85 + 0.3 * noise.noise(v * 2.3 + at))
        rgb = (tone, tone * 0.86, tone * 0.8)
        bare = smoothstep(0.1, 0.7, up) * smoothstep(0.1, 0.35, noise.noise(v * 0.9 + at * 2.0))
        rgb = mix(rgb, (0.9, 0.86, 0.8), 0.8 * bare)
        grown = moss * smoothstep(0.2, 0.85, up) * smoothstep(-0.2, 0.3, noise.noise(v * 1.4 + at))
        return mix(rgb, MOSS, 0.9 * grown)

    return shade


def cap(wood, ring, centre, axis, annulus, rough=0.0, seed=0):
    """A broken end's pale wood inside `ring` (the bark's last ring), looking
    along `axis`: the bark's thickness, then the wood to its darker heart,
    torn back into the bole by up to `rough` metres."""
    rng = random.Random(seed)
    rim = [wood.vert(v.co.copy(), (0.4, 0.34, 0.28), axis) for v in ring]
    if annulus:
        inner = [
            wood.vert(centre + (v.co - centre) * 0.86 - axis * rng.uniform(0.0, rough), (0.9, 0.9, 0.9), axis)
            for v in ring
        ]
        for k in range(len(ring)):
            j = (k + 1) % len(ring)
            wood.face((rim[k], rim[j], inner[j], inner[k]), axis)
        rim = inner
    heart = wood.vert(centre - axis * rough * 0.5, (0.6, 0.55, 0.5), axis)
    for k in range(len(ring)):
        wood.face((heart, rim[k], rim[(k + 1) % len(ring)]), axis)


def log(tier):
    half = Vector((2.2, 0.35, 0.35))
    sides, count = ((16, 9), (12, 5), (8, 3), (5, 2))[tier]
    # A trunk, not a pole: thick at the butt where it flares, thin at the
    # top, a little out of true, lying on the ground along its whole length
    # (its axis a little under its own radius up).
    girth = lambda t: 0.33 - 0.12 * t + 0.03 * math.exp(-t / 0.06)
    axis = lambda t: Vector(((2.0 * t - 1.0) * half.x, 0.06 * math.sin(t * 3.3 + 0.4), girth(t) - 0.04))
    points = [axis(i / (count - 1)) for i in range(count)]
    radii = [girth(i / (count - 1)) for i in range(count)]
    bark, wood = Solid(), Solid()
    shade = dead_bark(17, 1.0)
    first, last = tube(bark.bm, bark.colours, points, radii, sides, sides >= 12, shade)
    cap(wood, first, points[0], Vector((-1.0, 0.0, 0.0)), tier < 2, 0.14, 3)
    cap(wood, last, points[-1], Vector((1.0, 0.0, 0.0)), tier < 2, 0.1, 5)
    if tier < 3:
        # Stubs of the limbs it had, toward its top: (along, turn from up, length).
        for t, turn, length in ((0.45, 0.55, 0.2), (0.63, -0.6, 0.26), (0.8, 0.2, 0.3)):
            out = Vector((0.3, math.sin(turn), math.cos(turn))).normalized()
            foot = axis(t) + out * girth(t) * 0.7
            tip = axis(t) + out * (girth(t) + length)
            _, end = tube(bark.bm, bark.colours, [foot, tip], [0.075, 0.05], (6, 5, 4)[tier], False, shade)
            cap(wood, end, tip, out, False)
    return [("bark", bark, "bark"), ("wood", wood, "wood")]


def floor_litter(tier):
    rng = random.Random(61)
    sides, count = ((5, 5), (4, 3), (3, 2), (3, 2))[tier]
    bark = Solid()
    shade = dead_bark(61, 0.0)
    for i in range(3 if tier < 3 else 2):
        a = rng.uniform(0.0, math.pi)
        along = Vector((math.cos(a), math.sin(a), 0.0))
        across = Vector((-along.y, along.x, 0.0))
        length = rng.uniform(0.7, 1.7)
        radius = rng.uniform(0.04, 0.06)
        mid = Vector((rng.uniform(-0.45, 0.45), rng.uniform(-0.45, 0.45), 0.0))
        bend = across * rng.uniform(-0.18, 0.18)
        # It lies on the ground, the thin end propped on what is under it.
        start = mid - along * length / 2 + UP * radius
        end = mid + along * length / 2 + UP * (radius + rng.uniform(0.0, 0.12))
        points = bezier(start, start + along * length * 0.3 + bend, end - along * length * 0.3 + bend, end, count)
        radii = [radius * (1.0 - 0.5 * k / (count - 1)) for k in range(count)]
        tube(bark.bm, bark.colours, points, radii, sides, False, shade)
        fork = rng.uniform(0.35, 0.6)
        side = (across * rng.choice((-1.0, 1.0)) + along * 0.8 + UP * 0.35).normalized()
        if tier == 0:
            # A side twig, forking toward the thin end.
            knot = points[2]
            tube(bark.bm, bark.colours, [knot, knot + side * fork * 0.5, knot + side * fork], [radius * 0.5, radius * 0.4, radius * 0.2], 3, False, shade)
    return [("bark", bark, "bark")]


# ---------------------------------------------------------------------- leaves


def clumps(solid, sites, subdiv, relief, top):
    """Leaf clumps into `solid`, each a `trees` pad about its centre: shaded
    as one rounded mass about their middle, darker low and inside."""
    heart = sum((s["centre"] for s in sites), Vector()) / len(sites)
    faces = icosphere(subdiv)[1]
    for site in sites:
        verts = []
        for offset, up in clump_points(site, subdiv, relief):
            p = site["centre"] + offset
            p.z = max(p.z, 0.0)
            h = min(1.0, p.z / top)
            ao = (0.5 + 0.5 * smoothstep(0.0, 0.8, h)) * (0.75 + 0.25 * up) * PAD_LIGHT
            normal = offset.normalized() * 0.5 + (p - heart).normalized() * 0.3 + UP * 0.2
            verts.append(solid.vert(p, leaf_colour(ao, h)[:3], normal))
        for a, b, c in faces:
            solid.face((verts[a], verts[b], verts[c]))


def site(centre, radius, rng, pad):
    return dict(
        centre=centre,
        normal=UP,
        radius=radius,
        seed=rng.uniform(0.0, 97.0),
        turn=rng.uniform(0.0, math.pi),
        stretch=rng.uniform(1.0, 1.25),
        pad=pad,
    )


def merged(sites, count, grow):
    """`sites` as `count` larger clumps, for a coarser tier: neighbours in
    order, each group about its middle."""
    out = []
    for g in range(count):
        group = sites[g * len(sites) // count : (g + 1) * len(sites) // count]
        centre = sum((s["centre"] for s in group), Vector()) / len(group)
        reach = max((s["centre"] - centre).length + s["radius"] for s in group)
        out.append(dict(group[0], centre=centre, radius=reach * grow))
    return out


def floor_bush(tier):
    rng = random.Random(71)
    top = 0.62
    sites = []
    for i in range(6):
        a = i * 2.39996 + 0.4
        reach = 0.12 + 0.3 * math.sqrt(i / 5)
        radius = rng.uniform(0.2, 0.28)
        centre = Vector((math.cos(a) * reach, math.sin(a) * reach, rng.uniform(0.2, 0.42) - 0.12 * i / 5))
        sites.append(site(centre, radius, rng, (0.8, 0.7)))
    # The highest pad's top is the bush's.
    high = max(s["centre"].z + s["radius"] * 0.8 * 1.45 for s in sites)
    for s in sites:
        s["centre"].z *= top / high
        s["radius"] *= top / high
    leaves = Solid()
    if tier == 2:
        sites = merged(sites, 3, 0.8)
    if tier == 3:
        sites = merged(sites, 1, 0.75)
    clumps(leaves, sites, 2 if tier == 0 else 1, (1.0, 0.5, 0.0, 0.0)[tier], top)
    return [("crown", leaves, "leaves")]


def floor_sapling(tier):
    rng = random.Random(83)
    top = 0.88
    lean = Vector((0.07, -0.04, 0.0))
    stem = bezier(Vector(), Vector((0.0, 0.0, 0.3)), lean * 0.6 + UP * 0.55, lean + UP * 0.74, 5)
    sites = []
    # A whip with small sprays of leaves up it, not a little tree.
    for i in range(8):
        a = i * 2.39996 + 1.3
        z = 0.28 + 0.5 * i / 7
        reach = 0.2 - 0.15 * i / 7
        at = lean * (z / 0.74) + Vector((math.cos(a) * reach, math.sin(a) * reach, z))
        sites.append(site(at, rng.uniform(0.08, 0.12), rng, (0.6, 0.45)))
    leaves, bark = Solid(), Solid()
    if tier == 2:
        sites = merged(sites, 3, 0.7)
    if tier == 3:
        sites = merged(sites, 1, 0.6)
    for s in sites:
        # No pad passes the sapling's top.
        s["centre"].z = min(s["centre"].z, top - s["radius"] * 0.6 * 1.45)
    clumps(leaves, sites, 1, (1.0, 0.5, 0.0, 0.0)[tier], top)
    parts = [("crown", leaves, "leaves")]
    if tier < 2:
        shade = lambda p, v: (0.8, 0.8, 0.8)
        points = stem if tier == 0 else [stem[0], stem[2], stem[4]]
        radii = [0.024 - 0.014 * k / (len(points) - 1) for k in range(len(points))]
        tube(bark.bm, bark.colours, points, radii, 5 if tier == 0 else 3, False, shade)
        if tier == 0:
            for s in sites:
                # A twig from the stem to each pad.
                knot = min(stem, key=lambda p: abs(p.z - s["centre"].z + 0.12))
                tube(bark.bm, bark.colours, [knot, s["centre"]], [0.008, 0.004], 3, False, shade)
        parts.append(("stem", bark, "bark"))
    return parts


def floor_fern(tier):
    rng = random.Random(97)
    fronds, count = ((14, 9), (10, 5), (7, 3), (5, 3))[tier]
    leaves = Solid()
    for i in range(14):
        a = i * 2.39996 + rng.uniform(-0.25, 0.25)
        length = rng.uniform(0.42, 0.66)
        rise = rng.uniform(0.3, 0.55)
        tip = rng.uniform(0.02, 0.14)
        width = rng.uniform(0.055, 0.075)
        if i >= fronds:
            continue
        along = Vector((math.cos(a), math.sin(a), 0.0))
        across = Vector((-along.y, along.x, 0.0))
        # The midrib arches up out of the crown and over to its tip.
        rib = bezier(
            Vector(),
            along * length * 0.12 + UP * rise * 0.9,
            along * length * 0.65 + UP * rise * 1.15,
            along * length + UP * tip,
            count,
        )
        ribs, lefts, rights = [], [], []
        for k, p in enumerate(rib):
            t = k / (count - 1)
            lit = 0.4 + 0.6 * smoothstep(0.0, 0.55, t)
            ribs.append(leaves.vert(p, (lit * 0.7, lit * 0.75, lit * 0.7), UP * 0.8 + along * 0.2))
            if k in (0, count - 1):
                lefts.append(None)
                rights.append(None)
                continue
            # The pinnae: widest a third of the way out, hanging a little, the
            # frond's edge notched between them where the tier has the points.
            w = width * (4.0 * t * (1.0 - t)) ** 0.6 * (1.25 - 0.5 * t)
            if tier < 2 and k % 2 == 0:
                w *= 0.35
            hang = UP * -0.4 * w
            edge = (lit * 0.9, lit, lit * (0.9 - 0.1 * t))
            lefts.append(leaves.vert(p + across * w + hang, edge, UP * 0.75 + across * 0.5))
            rights.append(leaves.vert(p - across * w + hang, edge, UP * 0.75 - across * 0.5))
        for k in range(count - 1):
            for edges, turn in ((lefts, 1.0), (rights, -1.0)):
                a0, a1 = edges[k], edges[k + 1]
                verts = [v for v in (ribs[k], ribs[k + 1], a1, a0) if v is not None]
                leaves.face(verts if turn > 0 else verts[::-1], UP)
    return [("fronds", leaves, "leaves")]


# ---------------------------------------------------------------------- export

MATERIALS = {
    "bark": DEAD_BARK,
    "wood": WOOD,
    "rock": ROCK,
    "leaves": LEAVES,
}
# A sapling's stem is live bark; what lies on the ground is dead.
LIVE = {"floor_sapling": {"bark": BARK}}

KINDS = {
    "log": log,
    "boulder": boulder,
    "floor_fern": floor_fern,
    "floor_bush": floor_bush,
    "floor_sapling": floor_sapling,
    "floor_rock": floor_rock,
    "floor_litter": floor_litter,
}


def build_kind(name, build):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    materials = {}
    root = bpy.data.objects.new(name, None)
    scene.collection.objects.link(root)
    for tier in range(TIERS):
        for part, solid, mat in build(tier):
            if mat not in materials:
                materials[mat] = material(mat, LIVE.get(name, {}).get(mat, MATERIALS[mat]))
            mesh = solid.mesh(part)
            mesh.materials.append(materials[mat])
            obj = bpy.data.objects.new(f"{part}_LOD{tier}", mesh)
            obj.parent = root
            scene.collection.objects.link(obj)
    export(os.path.join(OUT, f"{name}.glb"))


for kind_name, builder in KINDS.items():
    if not ARGS[1:] or kind_name in ARGS[1:]:
        build_kind(kind_name, builder)
