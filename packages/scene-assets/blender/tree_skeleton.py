"""A tree's skeleton and bark: the trunk, a limb to each cluster of leaf
clumps and a branch to each clump, grown to the clumps it carries and drawn
as tubes. Three ideas shape it (technique from reading owenyuwono/dryad, MIT,
rewritten for Blender; nothing copied):

- pipe model: a branch is as thick as the clumps it carries, so the area of a
  parent is the sum of its children's (`PIPE`);
- gravity droop: every branch sags as a beam under its own weight, most at
  its tip, and carries what grows from it down with it;
- parallel transport: a branch is a tube whose ring frame is carried along
  the centreline without twisting, so bark furrows run true along a bend.

`trees.py` places the clumps and builds each tier; this grows and draws what
carries them.
"""

import math
import random

import bmesh
import bpy
from mathutils import Vector, noise

# A parent's radius^PIPE is the sum of its children's: 2 conserves area
# (Leonardo's rule); a little more keeps twigs visible at the game's camera.
PIPE = 2.4
# Centreline points of the bole (foot to the crown's base) and of the leader
# through the crown.
BOLE, LEADER = 5, 6
# How far a limb runs toward the middle of the clumps it carries.
LIMB = 0.6

UP = Vector((0.0, 0.0, 1.0))


def smoothstep(a, b, x):
    t = min(1.0, max(0.0, (x - a) / (b - a)))
    return t * t * (3.0 - 2.0 * t)


def spiral(i, n, top, bottom, turn):
    """The i-th of n golden-angle directions between heights top and bottom."""
    z = top - (top - bottom) * (i + 0.5) / n
    ring = math.sqrt(max(0.0, 1.0 - z * z))
    a = i * 2.39996 + turn
    return Vector((math.cos(a) * ring, math.sin(a) * ring, z))


def finish(mesh, normals, colours):
    mesh.shade_smooth()
    mesh.normals_split_custom_set_from_vertices(normals)
    attr = mesh.color_attributes.new("Col", "FLOAT_COLOR", "POINT")
    for i, c in enumerate(colours):
        attr.data[i].color = c
    mesh.color_attributes.active_color = attr
    return mesh




def bezier(a, b, c, d, n):
    """n points along the cubic from a to d."""
    out = []
    for i in range(n):
        t = i / (n - 1)
        s = 1.0 - t
        out.append(a * (s * s * s) + b * (3 * s * s * t) + c * (3 * s * t * t) + d * (t * t * t))
    return out


class Branch:
    """A centreline whose first point lies on its parent's, the branches that
    fork from it (each at one of its points) and the leaf clump at its end."""

    def __init__(self, points, clump=None):
        self.points = points
        self.clump = clump
        self.forks = []
        # Per point: the leaf clumps carried at or beyond it.
        self.load = []

    def fork(self, index, child):
        self.forks.append((index, child))
        return child

    def count(self):
        beyond = [1 if self.clump is not None else 0] * len(self.points)
        for index, child in self.forks:
            carried = child.count()
            for i in range(index + 1):
                beyond[i] += carried
        self.load = beyond
        return beyond[0]

    def length(self):
        return sum((b - a).length for a, b in zip(self.points, self.points[1:]))

    def walk(self):
        yield self
        for _, child in self.forks:
            yield from child.walk()


def limb_clusters(directions, limbs, turn):
    """Group clump directions round `limbs` seeded directions (k-means on the
    sphere); returns the non-empty groups' indices."""
    centres = [spiral(i, limbs, 0.95, -0.35, turn) for i in range(limbs)]
    groups = []
    for _ in range(4):
        groups = [[] for _ in centres]
        for i, d in enumerate(directions):
            groups[max(range(len(centres)), key=lambda k: d.dot(centres[k]))].append(i)
        centres = [
            sum((directions[i] for i in g), Vector()).normalized() if g else c
            for g, c in zip(groups, centres)
        ]
    return [g for g in groups if g]


def grow(kind, tips, lift):
    """The skeleton reaching `tips` (one per leaf clump, foot-centred metres)."""
    rng = random.Random(kind["seed"] * 7)
    lean = Vector((rng.uniform(-0.25, 0.25), rng.uniform(-0.25, 0.25), 0.0)) * kind.get("lean", 1.0)
    base = Vector((0.0, 0.0, kind["crown_base"])) + lean * 0.5
    whorled = "whorls" in kind
    # A conifer's leader runs to its spire; a broadleaf's ends inside its crown.
    top = Vector((0.0, 0.0, tips[-1].z if whorled else lift + 0.3 * kind["half_height"])) + lean
    # The bole, straight to the crown's base, then the leader through the crown.
    points = bezier(Vector(), Vector((0.0, 0.0, base.z * 0.4)), base - UP * base.z * 0.3, base, BOLE)
    points += bezier(base, base + UP * (top.z - base.z) * 0.4, top - UP * (top.z - base.z) * 0.3, top, LEADER)[1:]
    trunk = Branch(points)
    if whorled:
        # The leader carries the spire, and each bough leaves it from just
        # above where its pad hangs.
        trunk.clump = len(tips) - 1
        for i, tip in enumerate(tips[:-1]):
            index = min(range(BOLE - 1, len(points)), key=lambda k: abs(points[k].z - tip.z - 0.3))
            knot = points[index]
            to = tip - knot
            trunk.fork(index, Branch(bezier(knot, knot + to * 0.35 + UP * 0.1, tip - to * 0.3 + UP * 0.25, tip, 4), i))
        trunk.count()
        return trunk

    heart = Vector((lean.x, lean.y, lift))
    clusters = limb_clusters([(p - heart).normalized() for p in tips], kind["limbs"], kind["seed"] * 0.61)
    centroid = lambda group: sum((tips[i] for i in group), Vector()) / len(group)
    # Lower clusters fork lower on the leader.
    clusters.sort(key=lambda g: centroid(g).z)
    for rank, cluster in enumerate(clusters):
        index = BOLE - 1 + round((LEADER - 1) * (0.1 + 0.9 * rank / max(1, len(clusters) - 1)))
        start = trunk.points[index]
        reach = centroid(cluster) - start
        # The limb runs most of the way to the middle of its clumps.
        end = start + reach * LIMB
        span = reach.length * LIMB
        out = reach.normalized()
        limb = trunk.fork(
            index,
            Branch(bezier(start, start + (out * 0.5 + UP * kind.get("rise", 0.8)).normalized() * span * 0.4, end - out * span * 0.3, end, 6)),
        )
        for i in cluster:
            # A clump's branch forks where the limb passes it.
            along = (tips[i] - start).dot(reach) / reach.length_squared
            at = max(2, min(5, round(along / LIMB * 5)))
            knot = limb.points[at]
            to = tips[i] - knot
            # It leaves along the limb and turns up into the leaves.
            limb.fork(at, Branch(bezier(knot, knot + to * 0.4 - UP * 0.1, tips[i] - to * 0.25 - UP * 0.15, tips[i], 4), i))
    trunk.count()
    return trunk


def pipe_radius(kind, trunk, load):
    """Pipe model: the trunk's radius at its foot, shared out by leaf clumps carried."""
    return kind["trunk"] * (load / trunk.load[0]) ** (1.0 / PIPE)


def droop(kind, trunk, branch, carried=Vector()):
    """Sag `branch` and everything on it under gravity. A branch bends as a
    beam under its own weight: its tip drops by length^4 / radius^2, more the
    nearer it lies to level, and each point along it by the beam's curve. What
    grows from a point drops with it."""
    points = branch.points
    span = branch.length()
    level = 0.0
    if span > 1e-6:
        reach = points[-1] - points[0]
        level = math.hypot(reach.x, reach.y) / span
    radius = pipe_radius(kind, trunk, branch.load[0])
    sag = 0.0 if branch is trunk else min(0.16 * span, kind["droop"] * 3e-4 * level * span**4 / radius**2)
    drops = []
    run = 0.0
    for i, p in enumerate(points):
        if i:
            run += (p - points[i - 1]).length
        t = run / span if span > 1e-6 else 0.0
        drops.append(carried - UP * sag * (t * t * (6.0 - 4.0 * t + t * t) / 3.0))
    branch.points = [p + drop for p, drop in zip(points, drops)]
    for index, child in branch.forks:
        droop(kind, trunk, child, drops[index])


def tube(bm, colours, points, radii, sides, furrowed, shade):
    """A branch as a tube: a ring of `sides` at every point, its frame carried
    along the centreline by parallel transport (each ring turned only as far
    as the centreline bends), so the rings never twist against each other.
    A furrowed tube's alternate ribs stand proud as bark ridges, the grooves
    between them darker. Returns its first and last rings, for a cap."""
    tangents = []
    for i in range(len(points)):
        a, b = points[max(i - 1, 0)], points[min(i + 1, len(points) - 1)]
        tangents.append((b - a).normalized())
    side = tangents[0].orthogonal().normalized()
    rings = []
    run = 0.0
    for i, (p, r, t) in enumerate(zip(points, radii, tangents)):
        if i:
            run += (p - points[i - 1]).length
            side = tangents[i - 1].rotation_difference(t) @ side
            side = (side - t * side.dot(t)).normalized()
        other = t.cross(side)
        ring = []
        for k in range(sides):
            a = 0.3 + 2.0 * math.pi * k / sides
            rib = 0.0
            if furrowed:
                # Ridges wander in depth along the bole, never in place.
                strength = 0.55 + 0.45 * noise.noise(Vector((k * 1.7, run * 0.9, 0.0)))
                rib = (1.0 if k % 2 == 0 else -1.0) * strength
            ring.append(bm.verts.new(p + (side * math.cos(a) + other * math.sin(a)) * r * (1.0 + 0.1 * rib)))
            groove = 1.0 - 0.2 * max(0.0, -rib) + 0.08 * max(0.0, rib)
            colours.append((*(c * groove for c in shade(p, ring[-1].co)), 1.0))
        rings.append(ring)
    for lower, upper in zip(rings, rings[1:]):
        for k in range(sides):
            j = (k + 1) % sides
            bm.faces.new((lower[k], lower[j], upper[j], upper[k]))
    return rings[0], rings[-1]


def bark_shade(kind, tier, lift):
    """Bark's vertex colour at a vertex `v` of the ring about `p`, over the
    kind's bark colour: darker as it climbs into the crown's shade; the old
    dark bark low on a bole whose upper bark is brighter (`bark_low`, a
    pine's); dark splits in a marked bole (a birch's), where the tier has the
    rings to carry them."""
    low = kind.get("bark_low")
    marks = kind.get("marks", 0.0) if "twig" in tier else 0.0
    at = Vector((kind["seed"], 0.0, 0.0))

    def shade(p, v):
        dark = 1.0 if kind.get("bare") else 1.0 - 0.45 * smoothstep(kind["crown_base"] * 0.8, lift, p.z)
        rgb = (dark, dark, dark)
        if low:
            old = 1.0 - smoothstep(0.3, 0.7, p.z / kind["crown_base"])
            rgb = tuple(dark * (1.0 + old * (c - 1.0)) for c in low)
        if marks:
            split = smoothstep(0.15, 0.4, noise.noise(Vector((v.x * 4.0, v.y * 4.0, v.z * 1.3)) + at))
            rgb = tuple(c * (1.0 - marks * split) for c in rgb)
        return rgb

    return shade


def build_branches(kind, tier, lift, trunk):
    """One tier's bark as a mesh: the trunk, and where the tier draws
    branches every one no thinner than its `twig`. A bare tree's branches
    end in points; its far tier draws each from end to end."""
    bm = bmesh.new()
    colours = []
    shade = bark_shade(kind, tier, lift)
    skeleton = "twig" in tier
    bare = kind.get("bare", False)
    # The finest tier's bole is furrowed.
    furrowed = tier["sides"] >= 12
    for branch in trunk.walk() if skeleton else [trunk]:
        radii = [pipe_radius(kind, trunk, load) for load in branch.load]
        if bare:
            radii[-1] *= 0.15
        if branch is trunk:
            # The bole flares into its roots, which stand on the ground.
            radii = [r * (1.0 + 0.3 * math.exp(-p.z / 0.45)) for r, p in zip(radii, branch.points)]
            sides, points = tier["sides"], branch.points
            if not skeleton or tier.get("ends"):
                # A far trunk is its foot, the crown's base and its top.
                keep = [0, BOLE - 1, len(points) - 1]
                points, radii = [points[i] for i in keep], [radii[i] for i in keep]
        else:
            if radii[0] < tier["twig"]:
                continue
            points = branch.points
            sides = max(3, round(tier["sides"] * (0.15 + 0.75 * radii[0] / kind["trunk"])))
            if tier.get("ends"):
                points, radii = [points[0], points[-1]], [radii[0], radii[-1]]
        if skeleton and not furrowed and not tier.get("ends"):
            # A coarser tier's tubes take every other ring.
            keep = sorted({*range(0, len(points), 2), len(points) - 1})
            points, radii = [points[i] for i in keep], [radii[i] for i in keep]
        tube(bm, colours, points, radii, sides, furrowed and branch is trunk, shade)
    # The leaning foot ring stands on the ground, never below it.
    for v in bm.verts:
        v.co.z = max(v.co.z, 0.0)
    bm.normal_update()
    mesh = bpy.data.meshes.new("trunk")
    bm.to_mesh(mesh)
    bm.free()
    return finish(mesh, [v.normal.copy() for v in mesh.vertices], colours)
