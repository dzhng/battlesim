"""Tree and hedgerow appearance sources: one GLB per kind, four tiers each.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/trees.py [out_dir]

writes assets/source/trees/<kind>.glb (project-owned; record each file's
sha256 in the reuse manifest's third_party, then `asset bake`).

A crown is a lobed volume: a core sphere unioned with seeded lobe spheres,
taking along every direction from the crown's centre the farthest surface
(technique from ~/dev/game treeCrown.ts, rewritten). Every tier samples the
same shape on a finer or coarser icosphere, so the silhouettes agree when a
tree changes tier. The finest two tiers add leaf clumps (a Voronoi bulge)
and fine relief. Normals lean toward the crown's ellipsoid, so a crown shades
as one rounded mass rather than a faceted rock, and vertex colour carries the
crown's own occlusion (darker low and in the clumps' cavities). Species
colour variation and the season's tint come from the biome, not from here.

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

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
ARGS = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
OUT = os.path.abspath(ARGS[0]) if ARGS else os.path.join(ROOT, "assets", "source", "trees")

# Per tier: crown icosphere subdivisions, trunk sides (0: no trunk), how much
# of the leaf-clump relief it keeps, and whether it carries limbs.
TIERS = [
    dict(subdiv=5, sides=8, relief=1.0, limbs=True),
    dict(subdiv=4, sides=6, relief=0.7, limbs=True),
    dict(subdiv=3, sides=5, relief=0.0, limbs=False),
    dict(subdiv=2, sides=0, relief=0.0, limbs=False),
]

# Linear albedo (glTF base colour factors). The biome tints these per species.
LEAVES = (0.066, 0.112, 0.021, 1.0)
BARK = (0.070, 0.052, 0.036, 1.0)

KINDS = {
    # A broad, round field tree (oak, lime): the bulk of forests and hedgerows.
    "tree_broadleaf": dict(
        seed=11, height=11.0, crown_base=3.4, radius=4.3, half_height=3.7,
        lobes=9, trunk=0.3, clump_m=1.25,
    ),
    # A lower, wider-spreading tree with a lumpier outline.
    "tree_spreading": dict(
        seed=23, height=10.0, crown_base=2.8, radius=4.8, half_height=3.4,
        lobes=11, trunk=0.32, clump_m=1.4,
    ),
    # A tall, narrow crown (ash, poplar): breaks a tree line's skyline.
    "tree_tall": dict(
        seed=37, height=11.8, crown_base=2.6, radius=2.6, half_height=4.8,
        lobes=8, trunk=0.24, clump_m=1.0,
    ),
    # A hedgerow shrub: a 6 m run of hedge, 2.4 m wide and 2.8 m tall, set end
    # to end along field edges. No trunk; its ends round off so rows overlap.
    "hedge_shrub": dict(
        seed=5, height=2.8, crown_base=0.0, radius=1.2, half_height=1.4,
        length=6.0, lobes=10, trunk=0.0, clump_m=0.8,
    ),
}


def smoothstep(a, b, x):
    t = min(1.0, max(0.0, (x - a) / (b - a)))
    return t * t * (3.0 - 2.0 * t)


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


def crown_point(kind, lobes, d, relief):
    """A crown surface point (crown-centred metres) and its clump bulge 0..1."""
    hedge = "length" in kind
    axes = crown_axes(kind)
    r = lobed_radius(d, lobes, 0.62 if hedge else 0.58)
    if not hedge and d.z < 0.0:
        # Crowns hang flatter underneath.
        r *= 1.0 - 0.28 * d.z * d.z
    p = Vector((d.x * r * axes.x, d.y * r * axes.y, d.z * r * axes.z))
    n = Vector((p.x / axes.x**2, p.y / axes.y**2, p.z / axes.z**2)).normalized()
    cell = 1.0 / kind["clump_m"]
    q = p * cell + Vector((kind["seed"] * 1.37, 0.0, 0.0))
    distances, _ = noise.voronoi(q, distance_metric="DISTANCE", exponent=2.5)
    bulge = max(0.0, 1.0 - distances[0] * 1.35)
    fine = noise.noise(p * 2.6 + Vector((0.0, kind["seed"], 0.0)))
    p = p + n * relief * (kind["clump_m"] * 0.32 * (bulge - 0.35) + 0.07 * fine)
    return p, n, bulge


def build_crown(kind, lobes, tier, lift):
    """The crown mesh for one tier, with normals and colours. `lift` places the
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
        sun = h * h
        colours.append((ao * (0.92 + 0.12 * sun), ao * (0.98 + 0.06 * sun), ao * (0.92 - 0.1 * sun), 1.0))
    return finish(mesh, normals, colours)


def cylinder(bm, a, b, ra, rb, sides, phase):
    """An open tapered cylinder from a to b; returns nothing, adds to bm."""
    axis = (b - a).normalized()
    side = axis.orthogonal().normalized()
    other = axis.cross(side)
    rings = []
    for centre, radius in ((a, ra), (b, rb)):
        ring = []
        for k in range(sides):
            t = phase + 2.0 * math.pi * k / sides
            ring.append(bm.verts.new(centre + (side * math.cos(t) + other * math.sin(t)) * radius))
        rings.append(ring)
    for k in range(sides):
        j = (k + 1) % sides
        bm.faces.new((rings[0][k], rings[0][j], rings[1][j], rings[1][k]))


def build_trunk(kind, tier, crown_centre):
    bm = bmesh.new()
    r = kind["trunk"]
    rng = random.Random(kind["seed"] * 7)
    lean = Vector((rng.uniform(-0.25, 0.25), rng.uniform(-0.25, 0.25), 0.0))
    mid = Vector((0.0, 0.0, kind["crown_base"])) + lean * 0.5
    top = Vector((0.0, 0.0, crown_centre)) + lean
    cylinder(bm, Vector((0.0, 0.0, 0.0)), mid, r * 1.15, r * 0.85, tier["sides"], 0.3)
    cylinder(bm, mid, top, r * 0.85, r * 0.45, tier["sides"], 0.3)
    # The leaning foot ring stands on the ground, never below it.
    for v in bm.verts:
        v.co.z = max(v.co.z, 0.0)
    if tier["limbs"]:
        for k in range(3):
            a = k * 2.0 * math.pi / 3.0 + kind["seed"]
            out = Vector((math.cos(a), math.sin(a), 0.0))
            start = mid + Vector((0.0, 0.0, 0.3))
            end = start + out * kind["radius"] * 0.62 + Vector((0.0, 0.0, kind["half_height"] * 0.9))
            cylinder(bm, start, end, r * 0.42, r * 0.16, max(4, tier["sides"] - 2), 0.0)
    bm.normal_update()
    mesh = bpy.data.meshes.new("trunk")
    bm.to_mesh(mesh)
    bm.free()
    normals = [v.normal.copy() for v in mesh.vertices]
    colours = [(1.0, 1.0, 1.0, 1.0)] * len(mesh.vertices)
    return finish(mesh, normals, colours)


def finish(mesh, normals, colours):
    mesh.shade_smooth()
    mesh.normals_split_custom_set_from_vertices(normals)
    attr = mesh.color_attributes.new("Col", "FLOAT_COLOR", "POINT")
    for i, c in enumerate(colours):
        attr.data[i].color = c
    mesh.color_attributes.active_color = attr
    return mesh


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
    bark = material("bark", BARK)
    lobes = lobes_of(kind)

    def probe_extents():
        probe = build_crown(kind, lobes, TIERS[0], 0.0)
        co = [v.co for v in probe.vertices]
        out = (max(abs(c.x) for c in co), max(abs(c.y) for c in co), min(c.z for c in co), max(c.z for c in co))
        bpy.data.meshes.remove(probe)
        return out

    x, y, low, high = probe_extents()
    if "length" in kind:
        # A hedge is sized per axis: its run, its width and its height.
        kind = dict(kind, fit=Vector((kind["length"] / 2 / x, kind["radius"] / y, kind["height"] / (high - low))))
        x, y, low, high = probe_extents()
        lift, scale = -low, 1.0
    else:
        # A tree scales whole (trunk too) so its top is exactly `height`, with
        # the crown's lowest point (finest tier) at crown_base before scaling.
        lift = kind["crown_base"] - low
        scale = kind["height"] / (high + lift)
    root = bpy.data.objects.new(name, None)
    scene.collection.objects.link(root)
    for t, tier in enumerate(TIERS):
        if "length" in kind and t == len(TIERS) - 1:
            # A far hedge is a low mound in a long row: 20 triangles do.
            tier = dict(tier, subdiv=1)
        parts = [("crown", build_crown(kind, lobes, tier, lift), leaves)]
        if kind["trunk"] > 0.0 and tier["sides"] > 0:
            parts.append(("trunk", build_trunk(kind, tier, lift), bark))
        for part, mesh, mat in parts:
            mesh.transform(Matrix.Scale(scale, 4))
            mesh.materials.append(mat)
            obj = bpy.data.objects.new(f"{part}_LOD{t}", mesh)
            obj.parent = root
            scene.collection.objects.link(obj)
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, f"{name}.glb")
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


for kind_name, spec in KINDS.items():
    build_kind(kind_name, spec)
