"""Tree and hedgerow appearance sources: one GLB per kind, four tiers each.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/trees.py [out_dir] [kind...]

writes assets/source/trees/<kind>.glb (then `asset bake`): every kind, or
those named.

A tree is a branch skeleton carrying solid leaf clumps, so a crown is
separate masses on its limbs with crevices between them, not one smooth
volume. The skeleton is grown to its clumps ([`tree_skeleton.py`](tree_skeleton.py)).

Leaves are closed solids, never alpha cards: they need no discarding prepass
and hold still under 4x MSAA. The three nearer tiers draw the same clumps,
each coarser than the last, because a tree's shadow is cast by the tier
below the one drawn: a caster of another shape shadows the crown it stands
for. Branches thin out with the tiers, down to the trunk alone.

A lobed volume is a core sphere unioned with lobe spheres, taking along every
direction from the crown's centre the farthest surface (technique from
~/dev/game treeCrown.ts, rewritten). A tree's size is that of a lobed volume
of seeded lobes, the crown trees had before they had clumps: the clumps are
placed on it and kept under its top and within its reach. A tree's far tier
is the lobed volume of its own clumps, each a lobe just inside its clump, so
it casts for the clumps without shadowing them and a tree keeps its outline
when it changes tier. A hedgerow shrub is a seeded lobed volume on every
tier, the finer two with a leaf relief (a Voronoi bulge).

Species differ in shape, never in size: every tree is one height and one
girth of bole (the validator's `fit.tree_size`), so the art cannot lie about
sight. A broadleaf's clumps stand on a seeded lobed crown. A conifer's are
boughs in whorls up one straight leader, under a spire, inside a tapering
profile (the profile of ~/dev/game treeCrown.ts's conifer, as technique);
its far tier is that outline turned round the leader. A pine is a broadleaf
crown of few flat pads high on a bare bole; a birch a narrow one of small
hanging clumps on a white bole (the proportions of ~/dev/game's aspen). A
snag is a tree's skeleton with no clumps, its branches ending in points.

Normals lean toward the crown's ellipsoid, so a crown shades as one rounded
mass rather than a heap of rocks, and vertex colour carries the crown's own
occlusion (darker low, deep inside and under each clump). Species colour
variation and the season's tint come from the biome, not from here.

Engine space: Z up, +X forward, metres, origin at the trunk's foot. The glTF
exporter writes Y up and the bake converts back.
"""

import math
import os
import random
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector, noise

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from tree_skeleton import UP, build_branches, droop, finish, grow, smoothstep, spiral

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
ARGS = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
OUT = os.path.abspath(ARGS[0]) if ARGS else os.path.join(ROOT, "assets", "source", "trees")

# Per tier. `subdiv` and `relief` are the lobed volume's: its icosphere
# subdivisions and how much leaf relief it keeps. A tree draws leaf clumps
# where `clump` > 0 (each an icosphere of that many subdivisions) and the
# lobed volume of its clumps where it is 0; `sides` is its trunk's (0: no
# trunk); with `twig` it draws the branches no thinner than that, metres,
# thinner ones on fewer sides. The per-tier triangle budget is the validator's
# (`SCENERY_KINDS.tree`): a new kind fits it by its count of clumps.
TIERS = [
    dict(subdiv=5, relief=1.0, clump=3, sides=16, twig=0.0),
    dict(subdiv=4, relief=0.7, clump=2, sides=8, twig=0.07),
    dict(subdiv=3, relief=0.0, clump=1, sides=5),
    dict(subdiv=2, relief=0.0, clump=0, sides=0),
]
# A bare tree's tiers (a snag): bark alone, so every tier draws the skeleton,
# the far one as the trunk and each limb from end to end.
BARE_TIERS = [
    dict(sides=16, twig=0.0),
    dict(sides=8, twig=0.07),
    dict(sides=5, twig=0.12),
    dict(sides=4, twig=0.12, ends=True),
]

# Linear albedo (glTF base colour factors). The biome tints these per species.
LEAVES = (0.066, 0.112, 0.021, 1.0)
BARK = (0.070, 0.052, 0.036, 1.0)
# A leaf clump's height above and below its middle, in its radii: a pad,
# flatter beneath.
PAD = (0.7, 0.5)

# How far past the seeded crown a clump is set, in clump radii. What passes
# the tree's size is brought back to it, so the widest and highest clumps
# reach it exactly.
PROUD = 0.4
# A clump's brightest leaves against white, and a far crown's: one tone, so a
# tree keeps it when it changes tier.
PAD_LIGHT = 0.85
FAR_LIGHT = 0.72
# A far crown's lobe, in its clump's radii: inside the clump, flattened as it
# is, so the far crown casts for the clumps without shadowing them.
FAR_LOBE = 0.7

KINDS = {
    # A broad, round field tree (oak, lime): the bulk of forests and hedgerows.
    "tree_broadleaf": dict(
        seed=11, height=11.0, crown_base=3.4, radius=4.3, half_height=3.7,
        lobes=9, trunk=0.3, clump_m=1.25, clumps=20, inner=4, limbs=6, droop=1.0,
    ),
    # A lower, wider-spreading tree with a lumpier outline.
    "tree_spreading": dict(
        seed=23, height=10.5, crown_base=2.8, radius=4.7, half_height=3.4,
        lobes=11, trunk=0.3, clump_m=1.4, clumps=20, inner=4, limbs=6, droop=1.6,
    ),
    # A tall, narrow crown (ash, poplar): breaks a tree line's skyline.
    "tree_tall": dict(
        seed=37, height=11.5, crown_base=2.6, radius=2.6, half_height=4.8,
        lobes=8, trunk=0.31, clump_m=1.0, clumps=18, inner=3, limbs=5, droop=0.5,
    ),
    # A spruce: whorls of boughs sloping down from one leader, under a spire.
    # `whorls` counts the boughs of each, lowest first.
    "tree_spruce": dict(
        seed=41, height=11.2, crown_base=3.6, radius=5.3, whorls=(6, 5, 5, 4, 3),
        trunk=0.4, droop=1.2, lean=0.3,
    ),
    # A pine: a few flat pads of needles high on a long bare bole, its limbs
    # leaving the bole level (`rise`) so none arches over the pads, its upper
    # bark orange.
    "tree_pine": dict(
        seed=53, height=11.0, crown_base=6.0, radius=4.9, half_height=2.5,
        lobes=7, trunk=0.32, clump_m=1.35, clumps=10, inner=0, limbs=5, droop=0.6, proud=0.25,
        pad=(0.5, 0.32), rise=0.1, bark=(0.16, 0.085, 0.045, 1.0), bark_low=(0.45, 0.62, 0.8),
    ),
    # A birch: a narrow crown of small hanging clumps on a white bole, dark
    # where the bark has split.
    "tree_birch": dict(
        seed=67, height=11.3, crown_base=3.4, radius=2.7, half_height=4.3,
        lobes=8, trunk=0.33, clump_m=0.85, clumps=20, inner=4, limbs=6, droop=3.0,
        bark=(0.36, 0.35, 0.32, 1.0), marks=0.75,
    ),
    # A standing snag: a dead tree's grey skeleton.
    "tree_snag": dict(
        seed=79, height=10.7, crown_base=3.2, radius=3.4, half_height=3.8,
        lobes=7, trunk=0.26, clump_m=1.1, clumps=8, inner=0, limbs=7, droop=0.3,
        bare=True, bark=(0.13, 0.115, 0.1, 1.0),
    ),
    # A hedgerow shrub: a 6 m run of hedge, 2.4 m wide and 2.8 m tall, set end
    # to end along field edges. No trunk; its ends round off so rows overlap.
    "hedge_shrub": dict(
        seed=5, height=2.8, crown_base=0.0, radius=1.2, half_height=1.4,
        length=6.0, lobes=10, trunk=0.0, clump_m=0.8,
    ),
}


# ------------------------------------------------------------ the lobed crown


def lobes_of(kind):
    """Seeded lobe spheres in the crown's unit space: (centre, radius)."""
    rng = random.Random(kind["seed"])
    hedge = "length" in kind
    out = []
    for i in range(kind["lobes"]):
        if hedge:
            x = -0.62 + 1.24 * (i + rng.uniform(0.2, 0.8)) / kind["lobes"]
            centre = Vector((x, rng.uniform(-0.25, 0.25), rng.uniform(-0.1, 0.35)))
            out.append((centre, rng.uniform(0.42, 0.55)))
            continue
        # Golden-angle spiral over the upper hemisphere and the waist.
        z = 0.75 - 1.1 * (i + 0.5) / kind["lobes"] + rng.uniform(-0.08, 0.08)
        ring = math.sqrt(max(0.0, 1.0 - z * z))
        a = i * 2.39996 + kind["seed"] * 0.7
        reach = rng.uniform(0.38, 0.52)
        centre = Vector((math.cos(a) * ring * reach, math.sin(a) * ring * reach, z * reach))
        out.append((centre, rng.uniform(0.44, 0.56)))
    return out


def lobed_radius(d, lobes, core):
    """Farthest surface along unit direction d of the core ∪ lobes union."""
    r = core
    for centre, radius in lobes:
        along = d.dot(centre)
        disc = along * along - (centre.length_squared - radius * radius)
        if disc > 0.0:
            r = max(r, along + math.sqrt(disc))
    return r


def crown_axes(kind):
    hedge = "length" in kind
    axes = Vector((kind["length"] / 2 if hedge else kind["radius"], kind["radius"], kind["half_height"]))
    # A hedge's extents are measured once and corrected to its nominal size.
    fit = kind.get("fit", Vector((1.0, 1.0, 1.0)))
    return Vector((axes.x * fit.x, axes.y * fit.y, axes.z * fit.z))


def ellipsoid_normal(p, axes):
    return Vector((p.x / axes.x**2, p.y / axes.y**2, p.z / axes.z**2)).normalized()


def crown_point(kind, lobes, d, relief):
    """A crown surface point (crown-centred metres) and its clump bulge 0..1."""
    hedge = "length" in kind
    axes = crown_axes(kind)
    r = lobed_radius(d, lobes, 0.62 if hedge else 0.58)
    if not hedge and d.z < 0.0:
        # Crowns hang flatter underneath.
        r *= 1.0 - 0.28 * d.z * d.z
    p = Vector((d.x * r * axes.x, d.y * r * axes.y, d.z * r * axes.z))
    n = ellipsoid_normal(p, axes)
    cell = 1.0 / kind["clump_m"]
    q = p * cell + Vector((kind["seed"] * 1.37, 0.0, 0.0))
    distances, _ = noise.voronoi(q, distance_metric="DISTANCE", exponent=2.5)
    bulge = max(0.0, 1.0 - distances[0] * 1.35)
    fine = noise.noise(p * 2.6 + Vector((0.0, kind["seed"], 0.0)))
    p = p + n * relief * (kind["clump_m"] * 0.32 * (bulge - 0.35) + 0.07 * fine)
    return p, n, bulge


def leaf_colour(ao, h):
    """Vertex colour of a leaf `h` of the way up its crown: sunnier on top."""
    sun = h * h
    return (ao * (0.92 + 0.12 * sun), ao * (0.98 + 0.06 * sun), ao * (0.92 - 0.1 * sun), 1.0)


def build_crown(kind, lobes, tier, lift):
    """The seeded lobed crown for one tier, with normals and colours: a hedge
    as drawn, and the volume a tree's size is read from. `lift` places the
    crown's centre above the foot so its lowest point sits at crown_base."""
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=tier["subdiv"], radius=1.0)
    bulges = []
    ellipsoid = []
    for v in bm.verts:
        d = v.co.normalized()
        p, n, bulge = crown_point(kind, lobes, d, tier["relief"])
        v.co = p + Vector((0.0, 0.0, lift))
        bulges.append(bulge)
        ellipsoid.append(n)
    if "length" in kind:
        # A hedge stands on the ground: nothing dips below its foot.
        for v in bm.verts:
            v.co.z = max(v.co.z, 0.0)
    bm.normal_update()
    zs = [v.co.z for v in bm.verts]
    lo, hi = min(zs), max(zs)
    mesh = bpy.data.meshes.new("crown")
    bm.to_mesh(mesh)
    bm.free()
    normals = []
    colours = []
    for i, v in enumerate(mesh.vertices):
        n = (v.normal * 0.45 + ellipsoid[i] * 0.55).normalized()
        normals.append(n)
        h = (v.co.z - lo) / max(hi - lo, 1e-6)
        ao = (0.5 + 0.5 * smoothstep(0.0, 0.85, h)) * (0.72 + 0.28 * bulges[i] * min(1.0, tier["relief"] + 0.3))
        colours.append(leaf_colour(ao, h))
    return finish(mesh, normals, colours)


# ------------------------------------------------------------------- the clumps


def clump_sites(kind, lobes):
    """Where a crown's leaf clumps sit, crown-centred metres: a shell of them
    standing `PROUD` of the lobed crown, thinner underneath, and a few deeper
    ones that close the view through the crown."""
    if "whorls" in kind:
        return whorl_sites(kind)
    rng = random.Random(kind["seed"] * 13 + 1)
    out = []
    shell, inner = kind["clumps"], kind["inner"]
    for i in range(shell + inner):
        deep = i >= shell
        if deep:
            d = spiral(i - shell, inner, 0.9, -0.25, kind["seed"] * 1.9)
        else:
            d = spiral(i, shell, 1.0, -0.62, kind["seed"] * 0.37)
        d = (d + Vector((rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-1, 1))) * 0.12).normalized()
        surface, n, _ = crown_point(kind, lobes, d, 0.0)
        if deep:
            radius = kind["clump_m"] * rng.uniform(1.3, 1.6)
            centre = surface * rng.uniform(0.3, 0.5)
        else:
            radius = kind["clump_m"] * rng.uniform(1.0, 1.5)
            centre = surface - n * radius * (1.0 - kind.get("proud", PROUD))
        out.append(
            dict(
                centre=centre,
                normal=n,
                radius=radius,
                seed=rng.uniform(0.0, 97.0),
                # A pad is longer one way across than the other.
                turn=rng.uniform(0.0, math.pi),
                stretch=rng.uniform(1.0, 1.35),
                pad=kind.get("pad", PAD),
            )
        )
    return out


def profile(t):
    """A conifer crown's radius `t` of the way up it, as a share of its
    widest: a spire that closes in quickly underneath."""
    return (1.0 - t) ** 0.85 * min(1.0, t * 7.0)


# A bough's length along its branch against its width.
BOUGH = 1.25
# Where the lowest and highest whorls hang, as shares of the crown's height.
WHORLS = (0.15, 0.85)


def whorl_sites(kind):
    """A conifer's clumps, in metres about the crown's base on the trunk's
    axis: boughs in whorls up the leader, each a pad running out from it and
    sloping down, and a spire at the tip (the last site)."""
    rng = random.Random(kind["seed"] * 13 + 1)
    crown = kind["height"] - kind["crown_base"]
    whorls = kind["whorls"]
    out = []
    for k, count in enumerate(whorls):
        t = WHORLS[0] + (WHORLS[1] - WHORLS[0]) * k / (len(whorls) - 1)
        reach = kind["radius"] * profile(t)
        for i in range(count):
            a = kind["seed"] * 0.37 + k * 2.39996 + 2.0 * math.pi * (i + rng.uniform(-0.15, 0.15)) / count
            along = Vector((math.cos(a), math.sin(a), 0.0))
            # Wide enough to close its whorl and hide the one above.
            radius = max(0.7, 0.56 * reach) * rng.uniform(0.9, 1.1)
            height = crown * (t + rng.uniform(-0.12, 0.12) / len(whorls))
            out.append(
                dict(
                    centre=along * (reach - (1.0 - PROUD * 0.5) * radius * BOUGH) + UP * height,
                    normal=(along + UP * 0.6).normalized(),
                    radius=radius,
                    seed=rng.uniform(0.0, 97.0),
                    turn=a,
                    stretch=BOUGH,
                    pad=(0.62, 0.42),
                )
            )
    # The spire: a clump taller than it is wide, on the leader's tip.
    out.append(
        dict(
            centre=UP * (crown - 1.1),
            normal=UP,
            radius=0.6,
            seed=rng.uniform(0.0, 97.0),
            turn=0.0,
            stretch=1.0,
            pad=(2.0, 0.9),
        )
    )
    return out


def crown_frame(kind, lobes, q):
    """At `q` (metres about the crown's centre): the normal of the crown's
    one rounded mass there, and how near its surface `q` lies, 0 to 1."""
    if "whorls" in kind:
        across = math.hypot(q.x, q.y)
        t = min(1.0, max(WHORLS[0], q.z / (kind["height"] - kind["crown_base"])))
        edge = kind["radius"] * max(profile(t), 0.12)
        # A cone's side faces out and up; its axis faces up.
        n = Vector((q.x, q.y, 0.0)).normalized() * 0.75 + UP * 0.65 if across > 1e-6 else UP
        return n.normalized(), min(1.0, across / edge)
    surface, _, _ = crown_point(kind, lobes, q.normalized(), 0.0)
    return ellipsoid_normal(q, crown_axes(kind)), q.length / max(surface.length, 1e-6)


_ICOSPHERES = {}


def icosphere(subdiv):
    """A unit icosphere's directions and faces, built once per subdivision."""
    if subdiv not in _ICOSPHERES:
        bm = bmesh.new()
        bmesh.ops.create_icosphere(bm, subdivisions=subdiv, radius=1.0)
        bm.verts.index_update()
        _ICOSPHERES[subdiv] = (
            [v.co.normalized() for v in bm.verts],
            [tuple(v.index for v in f.verts) for f in bm.faces],
        )
        bm.free()
    return _ICOSPHERES[subdiv]


def clump_points(site, subdiv, relief):
    """A leaf clump's surface about its centre: a lumpy pad, longer one way
    across, flatter beneath, tipped a little toward the crown's outside.
    Returns (offset, up 0..1)."""
    directions, _ = icosphere(subdiv)
    tip = UP.rotation_difference((UP * 0.65 + site["normal"] * 0.35).normalized())
    turn = Matrix.Rotation(site["turn"], 3, "Z")
    at = Vector((site["seed"], site["seed"] * 0.37, 0.0))
    out = []
    for u in directions:
        lump = noise.noise(u * 1.3 + at)
        knob = noise.noise(u * 2.7 + at * 1.3)
        fine = noise.noise(u * 5.1 + at * 1.7)
        r = site["radius"] * (1.0 + 0.3 * lump + 0.16 * knob + 0.08 * relief * fine)
        flat = site["pad"][0] if u.z > 0.0 else site["pad"][1]
        pad = Vector((u.x * r * site["stretch"], u.y * r / site["stretch"], u.z * r * flat))
        out.append((tip @ (turn @ pad), 0.5 + 0.5 * u.z))
    return out


def build_skeleton(kind, lobes, lift, top, reach):
    """The tree's skeleton with its clumps hung on it: (trunk, sites, clump
    centres, foot-centred metres). Drooped clumps hang short of the tree's
    `top`, so the crown is stretched up from its base until they pass it."""
    sites = clump_sites(kind, lobes)
    trunk = grow(kind, [s["centre"] + UP * lift for s in sites], lift)
    droop(kind, trunk, trunk)
    base = kind["crown_base"]
    for _ in range(4):
        centres = [None] * len(sites)
        for branch in trunk.walk():
            if branch.clump is not None:
                centres[branch.clump] = branch.points[-1]
        high = max(
            (centre + offset).z
            for site, centre in zip(sites, centres)
            for offset, _ in clump_points(site, TIERS[0]["clump"], TIERS[0]["relief"])
        )
        if high > top:
            break
        rise = 1.02 * (top - base) / (high - base)
        for branch in trunk.walk():
            branch.points = [Vector((p.x, p.y, base + max(0.0, p.z - base) * rise)) if p.z > base else p for p in branch.points]
    for _ in range(4 if "whorls" in kind else 0):
        # A conifer's boughs are spread until the widest passes the tree's
        # reach, as its crown was stretched to its top.
        wide = max(
            math.hypot((centre + offset).x, (centre + offset).y)
            for site, centre in zip(sites, centres)
            for offset, _ in clump_points(site, TIERS[0]["clump"], TIERS[0]["relief"])
        )
        if wide > reach:
            break
        spread = 1.02 * reach / wide
        for branch in trunk.walk():
            if branch is not trunk:
                foot = branch.points[0]
                branch.points = [Vector((foot.x + (p.x - foot.x) * spread, foot.y + (p.y - foot.y) * spread, p.z)) for p in branch.points]
                centres[branch.clump] = branch.points[-1]
    return trunk, sites, centres


def within(p, top, reach):
    """`p` brought under `top` and within `reach` of the trunk's axis: the
    tree's size, which no tier's crown passes."""
    across = math.hypot(p.x, p.y)
    scale = min(1.0, reach / across) if across > 0.0 else 1.0
    return Vector((p.x * scale, p.y * scale, min(p.z, top)))


def build_clumps(kind, lobes, tier, lift, sites, centres, top, reach):
    """One tier's leaf clumps as a mesh, with normals and colours."""
    _, faces = icosphere(tier["clump"])
    bm = bmesh.new()
    shade = []
    lows = [c.z - s["radius"] for s, c in zip(sites, centres)]
    highs = [c.z + s["radius"] for s, c in zip(sites, centres)]
    lo, hi = min(lows), max(highs)
    for site, centre in zip(sites, centres):
        verts = []
        for offset, up in clump_points(site, tier["clump"], tier["relief"]):
            p = within(centre + offset, top, reach)
            verts.append(bm.verts.new(p))
            # About the crown's centre: the crown's own normal there, and
            # how near its surface.
            normal, near = crown_frame(kind, lobes, p - UP * lift)
            exposed = smoothstep(0.55, 1.0, near)
            # Leaves mottle a pad, where the tier has the vertices to carry it.
            mottle = 0.85 + 0.3 * tier["relief"] * noise.noise(p * 1.9 + Vector((site["seed"], 0.0, 0.0)))
            shade.append((normal, exposed * mottle, up, (p.z - lo) / (hi - lo)))
        for a, b, c in faces:
            bm.faces.new((verts[a], verts[b], verts[c]))
    bm.normal_update()
    mesh = bpy.data.meshes.new("crown")
    bm.to_mesh(mesh)
    bm.free()
    normals = []
    colours = []
    for v, (ellipsoid, exposed, up, h) in zip(mesh.vertices, shade):
        normals.append((v.normal * 0.3 + ellipsoid * 0.7).normalized())
        ao = (0.5 + 0.5 * smoothstep(0.0, 0.85, h)) * (0.5 + 0.5 * exposed) * (0.8 + 0.2 * up) * PAD_LIGHT
        colours.append(leaf_colour(ao, h))
    return finish(mesh, normals, colours)


def build_far_crown(kind, tier, sites, centres, top, reach):
    """A tree's far crown: the lobed volume of its own clumps, each a sphere
    just inside its clump and a core closing the gaps between them, kept
    under `top` and within `reach` of the trunk's axis (the tree's size)."""
    heart = sum(centres, Vector()) / len(centres)
    lobes = [(centre - heart, FAR_LOBE * site["radius"]) for site, centre in zip(sites, centres)]
    spread = sorted(centre.length for centre, _ in lobes)
    core = 0.6 * spread[len(spread) // 2]
    axes = crown_axes(kind)
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=tier["subdiv"], radius=1.0)
    ellipsoid = []
    for v in bm.verts:
        d = v.co.normalized()
        p = within(heart + d * lobed_radius(d, lobes, core), top, reach)
        v.co = p
        ellipsoid.append(ellipsoid_normal(p - heart, axes))
    bm.normal_update()
    zs = [v.co.z for v in bm.verts]
    lo, hi = min(zs), max(zs)
    mesh = bpy.data.meshes.new("crown")
    bm.to_mesh(mesh)
    bm.free()
    normals = []
    colours = []
    for v, n in zip(mesh.vertices, ellipsoid):
        normals.append((v.normal * 0.45 + n * 0.55).normalized())
        h = (v.co.z - lo) / max(hi - lo, 1e-6)
        colours.append(leaf_colour((0.5 + 0.5 * smoothstep(0.0, 0.85, h)) * FAR_LIGHT, h))
    return finish(mesh, normals, colours)


# A conifer's far crown: rings up the leader, of this many sides.
SPIRE_RINGS, SPIRE_SIDES = 5, 8


def build_far_spire(kind, tier, lift, sites, centres, top, reach):
    """A conifer's far crown: the outline of its own boughs turned round the
    leader, each ring just inside the boughs at its height, closing to a
    point at the tree's top. It stands in for the lobed volume, which cannot
    follow a spire from one centre."""
    cloud = [
        within(centre + offset * FAR_LOBE, top, reach)
        for site, centre in zip(sites, centres)
        for offset, _ in clump_points(site, tier["subdiv"], 0.0)
    ]
    lo = min(p.z for p in cloud)
    step = (top - lo) / SPIRE_RINGS
    bm = bmesh.new()
    rings = []
    for j in range(SPIRE_RINGS):
        z = lo + step * j
        wide = max(math.hypot(p.x, p.y) for p in cloud if -0.2 * step <= p.z - z <= 0.6 * step)
        rings.append(
            [
                bm.verts.new((wide * math.cos(a), wide * math.sin(a), z))
                for a in (2.0 * math.pi * (k + 0.5 * j) / SPIRE_SIDES for k in range(SPIRE_SIDES))
            ]
        )
    tip = bm.verts.new((0.0, 0.0, top))
    bm.faces.new(reversed(rings[0]))
    for lower, upper in zip(rings, rings[1:]):
        for k in range(SPIRE_SIDES):
            j = (k + 1) % SPIRE_SIDES
            bm.faces.new((lower[k], lower[j], upper[k]))
            bm.faces.new((lower[j], upper[j], upper[k]))
    for k in range(SPIRE_SIDES):
        bm.faces.new((rings[-1][k], rings[-1][(k + 1) % SPIRE_SIDES], tip))
    bm.normal_update()
    mesh = bpy.data.meshes.new("crown")
    bm.to_mesh(mesh)
    bm.free()
    normals = []
    colours = []
    for v in mesh.vertices:
        n, _ = crown_frame(kind, [], v.co - UP * lift)
        normals.append((v.normal * 0.45 + n * 0.55).normalized())
        h = (v.co.z - lo) / (top - lo)
        colours.append(leaf_colour((0.5 + 0.5 * smoothstep(0.0, 0.85, h)) * FAR_LIGHT, h))
    return finish(mesh, normals, colours)


# ------------------------------------------------------------------- the export


def material(name, colour):
    m = bpy.data.materials.new(name)
    m.diffuse_color = colour
    m.use_nodes = True
    bsdf = m.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = colour
    bsdf.inputs["Roughness"].default_value = 0.9
    bsdf.inputs["Metallic"].default_value = 0.0
    return m


def build_kind(name, kind):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    leaves = material("leaves", LEAVES)
    bark = material("bark", kind.get("bark", BARK))
    hedge = "length" in kind
    whorled = "whorls" in kind
    bare = kind.get("bare", False)
    lobes = [] if whorled else lobes_of(kind)

    def probe_extents():
        """The finest lobed crown about its centre: its half-extents along x
        and y, its lowest and highest points, and its reach from the axis."""
        probe = build_crown(kind, lobes, TIERS[0], 0.0)
        co = [v.co for v in probe.vertices]
        out = (
            max(abs(c.x) for c in co),
            max(abs(c.y) for c in co),
            min(c.z for c in co),
            max(c.z for c in co),
            max(math.hypot(c.x, c.y) for c in co),
        )
        bpy.data.meshes.remove(probe)
        return out

    if hedge:
        # A hedge is sized per axis: its run, its width and its height.
        x, y, low, high, reach = probe_extents()
        kind = dict(kind, fit=Vector((kind["length"] / 2 / x, kind["radius"] / y, kind["height"] / (high - low))))
        x, y, low, high, reach = probe_extents()
        lift, scale = -low, 1.0
    else:
        if whorled:
            # A conifer's crown is measured from its base on the trunk's
            # axis, and its size is as given.
            lift, scale = kind["crown_base"], 1.0
            top, reach = kind["height"], kind["radius"]
        else:
            # A tree scales whole (trunk too) so its top is exactly `height`,
            # with the seeded crown's lowest point (finest tier) at crown_base
            # before scaling. That crown's top and reach are the tree's size.
            x, y, low, high, reach = probe_extents()
            lift = kind["crown_base"] - low
            scale = kind["height"] / (high + lift)
            top = high + lift
        trunk, sites, centres = build_skeleton(kind, lobes, lift, top, reach)
        if bare:
            # A bare tree's size is its skeleton's: its highest branch end.
            scale = kind["height"] / max(p.z for branch in trunk.walk() for p in branch.points)
    root = bpy.data.objects.new(name, None)
    scene.collection.objects.link(root)
    for t, tier in enumerate(BARE_TIERS if bare else TIERS):
        if hedge and t == len(TIERS) - 1:
            # A far hedge is a low mound in a long row: 20 triangles do.
            tier = dict(tier, subdiv=1)
        if hedge:
            parts = [("crown", build_crown(kind, lobes, tier, lift), leaves)]
        elif bare:
            parts = []
        elif tier["clump"] > 0:
            crown = build_clumps(kind, lobes, tier, lift, sites, centres, top, reach)
            if t == 0:
                # Placement sizes a tree by its finest tier: the clumps must
                # fill the size, not merely stay inside it.
                co = [v.co for v in crown.vertices]
                size = (max(c.z for c in co), max(math.hypot(c.x, c.y) for c in co))
                if top - size[0] > 1e-4 or reach - size[1] > 1e-4:
                    raise RuntimeError(f"{name}: clumps reach {size}, short of the tree's size {(top, reach)}")
            parts = [("crown", crown, leaves)]
        elif whorled:
            parts = [("crown", build_far_spire(kind, tier, lift, sites, centres, top, reach), leaves)]
        else:
            parts = [("crown", build_far_crown(kind, tier, sites, centres, top, reach), leaves)]
        if not hedge and tier["sides"] > 0:
            parts.append(("trunk", build_branches(kind, tier, lift, trunk), bark))
        for part, mesh, mat in parts:
            mesh.transform(Matrix.Scale(scale, 4))
            mesh.materials.append(mat)
            obj = bpy.data.objects.new(f"{part}_LOD{t}", mesh)
            obj.parent = root
            scene.collection.objects.link(obj)
    export(os.path.join(OUT, f"{name}.glb"))


def export(path):
    """The scene as a GLB at `path`: vertex colours and materials, Y up."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=path,
        export_format="GLB",
        export_yup=True,
        export_apply=True,
        export_normals=True,
        export_texcoords=False,
        export_vertex_color="ACTIVE",
        export_materials="EXPORT",
        export_animations=False,
        export_skins=False,
        export_morph=False,
        export_extras=False,
        export_cameras=False,
        export_lights=False,
    )
    print(f"wrote {os.path.relpath(path, ROOT)}")


if __name__ == "__main__":
    for kind_name, spec in KINDS.items():
        if not ARGS[1:] or kind_name in ARGS[1:]:
            build_kind(kind_name, spec)
