"""A crown's foliage: leaf clumps, and the leaves on them.

A clump is a lumpy pad on the end of a branch (`clump_points`). Near, it is a
small dark core shingled with leaves: each leaf one triangle, narrow at a
stalk just inside the clump's surface and broad at its tip, its blade lying
along the surface, tilted out, so a crown's outline is the scalloped ends of
leaves and the view into it falls into shade between leaves rather than onto a
smooth skin. Far, it is the pad alone, drawn solid.

Leaves grow where they show: on the clump surface that the sky and the
camera reach past the other clumps, not on what the crown buries. A tier
names how many leaves its crown has, so its triangles are its budget; their
size follows from that count and the shown area, so a crown of small clumps
(a birch's) has small leaves.

Leaves are opaque triangles, never alpha cards: they need no discarding
prepass and hold still under 4x MSAA. A leaf is drawn from its face only: it
faces out of the crown and up, and from behind it lies on the far side of its
core.

Each leaf carries its own colour: darker at the stalk than the tip, and
lighter or yellower than its neighbours by a seeded step, so leaves read one
by one where the crown's light alone would read as one mass. Its normal leans
from the crown's rounded mass toward the leaf's own face.
"""

import math
import random

import bmesh
import bpy
from mathutils import Matrix, Vector, noise

from tree_skeleton import UP, finish, smoothstep, spiral

# A clump's brightest leaves against white.
PAD_LIGHT = 0.85
# How much of the crown's shown surface its leaves cover between them,
# counting the overlap: past 1 the core shows only in the gaps between leaves.
COVER = 1.4
# The ways out of a crown a spot is looked at from: straight out of its
# clump, and leant up toward the sky by these shares of the vertical.
LOOKS = (0.0, 1.0, 3.0)
# How much of a clump hides what lies behind it, in its radii: the leaves at
# its edge let some through.
OCCLUDE = 0.85
# How far a leaf stands out of the surface it lies along: the tangent of its
# angle to it.
TILT = 0.35
# Spots offered per leaf, so the shown share of the clumps still has more
# spots than leaves.
SPOTS = 5
# How deep a leaf's stalk sits under the surface it grows from, in its
# lengths: its tip stands proud of it, so the outermost leaves fill the crown.
STALK = 0.2
# The core under a clump's leaves, in the clump's radii, and how dark it is.
CORE = 0.85
CORE_SHADE = 0.45
# A leaf's shade at the stalk against its tip.
STALK_SHADE = 0.62
# Seeded step of each leaf's brightness, and of its yellow.
LEAF_JITTER = 0.22
LEAF_YELLOW = 0.12

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


def clump_offset(site, u, relief):
    """A leaf clump's surface along unit direction `u` from its centre: a
    lumpy pad, longer one way across, flatter beneath, tipped a little toward
    the crown's outside. Returns (offset, up 0..1)."""
    tip = UP.rotation_difference((UP * 0.65 + site["normal"] * 0.35).normalized())
    turn = Matrix.Rotation(site["turn"], 3, "Z")
    at = Vector((site["seed"], site["seed"] * 0.37, 0.0))
    lump = noise.noise(u * 1.3 + at)
    knob = noise.noise(u * 2.7 + at * 1.3)
    fine = noise.noise(u * 5.1 + at * 1.7)
    r = site["radius"] * (1.0 + 0.3 * lump + 0.16 * knob + 0.08 * relief * fine)
    flat = site["pad"][0] if u.z > 0.0 else site["pad"][1]
    pad = Vector((u.x * r * site["stretch"], u.y * r / site["stretch"], u.z * r * flat))
    return tip @ (turn @ pad), 0.5 + 0.5 * u.z


def clump_points(site, subdiv, relief):
    """A leaf clump's surface at an icosphere's directions: (offset, up 0..1)."""
    directions, _ = icosphere(subdiv)
    return [clump_offset(site, u, relief) for u in directions]


def leaf_colour(ao, h):
    """Vertex colour of a leaf `h` of the way up its crown: sunnier on top."""
    sun = h * h
    return (ao * (0.92 + 0.12 * sun), ao * (0.98 + 0.06 * sun), ao * (0.92 - 0.1 * sun), 1.0)


def blocked(origin, direction, solids):
    """Whether a ray from `origin` along `direction` meets any of `solids`,
    each `(centre, across, up)`: an upright ellipsoid's centre and its
    half-widths across and up."""
    for centre, across, up in solids:
        scale = Vector((1.0 / across, 1.0 / across, 1.0 / up))
        o = (origin - centre) * scale
        d = direction * scale
        a = d.dot(d)
        b = o.dot(d)
        disc = b * b - a * (o.dot(o) - 1.0)
        if disc > 0.0 and (-b + math.sqrt(disc)) / a > 0.0:
            return True
    return False


def clump_spots(tier, sites, centres):
    """Where a tier's leaves may grow on a crown's clumps: spots spread over
    the outer clumps by their area (`grow_leaves`), each showing as much as
    the light and the camera reach it past the other clumps (the share of
    `LOOKS` that leave the crown unblocked), and more on top, where the
    camera looks down on it."""
    solids = [
        (centre, site["radius"] * site["stretch"] * OCCLUDE, site["radius"] * 0.5 * sum(site["pad"]) * OCCLUDE)
        for site, centre in zip(sites, centres)
    ]
    outer = [i for i, s in enumerate(sites) if not s.get("deep")]
    total = sum(sites[i]["radius"] ** 2 * sites[i]["stretch"] for i in outer)
    out = []
    for i in outer:
        site = sites[i]
        others = solids[:i] + solids[i + 1 :]
        area = 4.0 * math.pi * site["radius"] ** 2 * site["stretch"]
        n = max(4, round(SPOTS * tier["leaves"] * area / (4.0 * math.pi * total)))
        for k in range(n):
            u = spiral(k, n, 1.0, -1.0, site["seed"] * 0.61)
            offset, up = clump_offset(site, u, tier["relief"])
            normal = offset.normalized()
            surface = centres[i] + offset
            looks = [(normal + UP * lift).normalized() for lift in LOOKS]
            shown = sum(not blocked(surface, look, others) for look in looks) / len(looks)
            out.append((centres[i] + offset, normal, up, area / n, shown * (0.5 + 0.5 * up)))
    return out


def leaf_frame(rng, out, shape):
    """A leaf's direction (stalk to tip) and the way its face looks, for a
    leaf on a surface facing `out`: it lies along the surface, tilted `TILT`
    out of it and bent toward the sky or the ground by the kind's habit, its
    face turned out and up toward the light."""
    jitter = Vector((rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-1, 1)))
    along = jitter - out * jitter.dot(out)
    along = along.normalized() if along.length > 1e-3 else out.orthogonal().normalized()
    reach = (along + out * TILT + UP * shape["rise"] + jitter * shape["scatter"]).normalized()
    face = (out + UP * 0.5 + jitter * 0.3).normalized()
    face = face - reach * face.dot(reach)
    if face.length < 1e-3:
        face = reach.orthogonal()
    return reach, face.normalized()


def grow_leaves(bm, shade, spots, count, shape, clamp, light):
    """About `count` leaves into `bm`, their entries into `shade`
    (`finish_foliage`), over `spots`: each `(at, out, up, area, weight)`,
    where on the surface a leaf grows, the way out of the mass it grows from,
    how high on that mass it stands (0 to 1), the share of the surface it
    stands for and how much a leaf there shows (0 to 1). Leaves fall on
    spots as they show, and are as long as covers the shown surface `COVER`
    times. `light(p, up)` is the crown's normal, occlusion and height at
    `p`."""
    shown = sum(weight for *_, weight in spots)
    # A leaf's area is half its length by its width.
    length = math.sqrt(COVER * sum(a * w for *_, a, w in spots) / (count * 0.5 * shape["aspect"]))
    rng = random.Random(len(spots) * 7 + count)
    for at, out, up, _, weight in spots:
        if rng.random() >= weight * count / shown:
            continue
        reach, face = leaf_frame(rng, out, shape)
        size = length * rng.uniform(0.75, 1.25)
        stalk = at - out * (size * STALK)
        width = size * shape["aspect"]
        side = reach.cross(face) * (width * 0.5)
        tip = stalk + reach * size
        corners = [clamp(p) for p in (stalk, tip + side, tip - side)]
        # Its face looks along `face`: stalk, then its tip's left and right.
        bm.faces.new([bm.verts.new(p) for p in corners])
        tone = 1.0 + LEAF_JITTER * rng.uniform(-1.0, 1.0)
        yellow = LEAF_YELLOW * rng.uniform(-0.3, 1.0)
        for k, p in enumerate(corners):
            normal, ao, h = light(p, up)
            along = STALK_SHADE if k == 0 else 1.0
            r, g, b, a = leaf_colour(ao * tone * along, h)
            # A leaf's normal: the crown's mass, turned toward its own face.
            shade.append(((normal * 0.6 + face * 0.4).normalized(), 0.0, (r * (1.0 + yellow), g * (1.0 + 0.5 * yellow), b, a)))


def finish_foliage(bm, shade):
    """`bm` as a mesh, its vertices' `shade` entries `(normal, own, colour)`
    in order: each normal leant `own` of the way to the surface's own."""
    bm.normal_update()
    mesh = bpy.data.meshes.new("crown")
    bm.to_mesh(mesh)
    bm.free()
    normals = [(v.normal * own + normal * (1.0 - own)).normalized() for v, (normal, own, _) in zip(mesh.vertices, shade)]
    return finish(mesh, normals, [colour for *_, colour in shade])


def build_foliage(tier, sites, centres, shape, clamp, frame):
    """One tier's clumps as a mesh, with normals and colours: each clump's
    core, at `tier["core"]` of its radius, and with `tier["leaves"]` the
    crown's leaves on the outer clumps. `clamp` keeps a point within the
    tree's size; `frame` gives the crown's normal at a point and how near its
    surface the point lies, 0 to 1."""
    count = tier.get("leaves", 0)
    lows = [c.z - s["radius"] for s, c in zip(sites, centres)]
    highs = [c.z + s["radius"] for s, c in zip(sites, centres)]
    lo, hi = min(lows), max(highs)
    _, faces = icosphere(tier["clump"])
    bm = bmesh.new()
    shade = []

    def light(p, up, mottle=1.0):
        """The crown's normal at `p`, its occlusion and how high it stands."""
        normal, near = frame(p)
        h = (p.z - lo) / (hi - lo)
        exposed = smoothstep(0.55, 1.0, near) * mottle
        ao = (0.5 + 0.5 * smoothstep(0.0, 0.85, h)) * (0.5 + 0.5 * exposed) * (0.8 + 0.2 * up) * PAD_LIGHT
        return normal, ao, h

    for site, centre in zip(sites, centres):
        verts = []
        for offset, up in clump_points(site, tier["clump"], tier["relief"]):
            p = clamp(centre + offset * tier["core"])
            verts.append(bm.verts.new(p))
            # Leaves mottle a pad, where the tier has the vertices to carry it.
            mottle = 0.85 + 0.3 * tier["relief"] * noise.noise(p * 1.9 + Vector((site["seed"], 0.0, 0.0)))
            normal, ao, h = light(p, up, mottle)
            shade.append((normal, 0.3, leaf_colour(ao * (CORE_SHADE if count else 1.0), h)))
        for a, b, c in faces:
            bm.faces.new((verts[a], verts[b], verts[c]))
    if count:
        grow_leaves(bm, shade, clump_spots(tier, sites, centres), count, shape, clamp, light)
    return finish_foliage(bm, shade)
