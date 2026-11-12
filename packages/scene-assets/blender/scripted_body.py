"""A vehicle body built by its own Blender script, worn by a roster family.

The script (`build.py`, kept with its source rounds under
`assets/source/skins/<family>/`) is a standalone Blender program: run alone
(`OUT_DIR=<dir> blender -b -P build.py`) it builds the vehicle, renders its
views and writes a GLB; imported, its `build()` builds the vehicle into the
caller's scene and returns. Gemini Argon writes it in a Bento session from
the references and our measurements (at most two rounds); after that it is
edited here like any source. It names each part by what the game moves, and
paints each face with one of the material slots `ROLES` names:

- `hull`: the hull and everything fixed to it;
- `turret`, `gun`, `hmg`, `hmg_gun`: what turns with the turret, what pitches
  with the cannon, the roof gun's turning base and its pitching gun;
- `wheel_*`: each wheel that turns on its own axle (road wheels, sprocket,
  idler, return rollers), its object's origin on the axle;
- `track_L`, `track_R`: the track runs; only their width and place are
  read (see below);
- `dressing_*`: fittings standing over the roof line (sights, sensors),
  hung on the part they ride.

A part is the nearest of these among an object and its parents (a wheel,
track or dressing the outermost of its own kind).

`wear` runs the script and fits what it built to the export contract that
the family's procedural builds keep (`vehicle_parts`):

- **The rig is the frame's.** The family has already made its mount nodes
  (`vehicle_export.rig`) at the battle frame's pivots and muzzles; each part
  moves under its node keeping its world place, so it turns and pitches
  about the frame's pivot wherever the script put its own.
- **One object per material,** painted in the family's scheme by role
  (`ROLES`): box UVs take the tile of an object's first material. Its paint
  chips at its edges as a procedural build's does (`EDGE_CHIP`).
- **Wheels spin on `wheel_*` nodes** at the script's axles, with the radius
  they measure.
- **Each track run is the shared belt** (`running_gear.track_loop`) wrapped
  round that side's wheels, at the script's run's width and place, as thick
  as the gap under the road wheels: the renderer scrolls a belt's links
  along its UVs, and the belt sags between the rollers there.
- **Tiers:** each object is tier 0 as built; tiers 1-3 are its reduced
  copies (`mesh_lods.reduced_copy`), its share of `TIERS`.

The family's receipt hashes the script with every other source the export
loaded (`vehicle_export._receipt`).
"""
import importlib.util
import math
import os
import re
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mesh_lods  # noqa: E402
from parts import empty, tier_of  # noqa: E402
from running_gear import track_loop  # noqa: E402
from vehicle_parts import wheel_node  # noqa: E402

REPO = Path(__file__).resolve().parents[3]
# A script's material slots, and the export's role painting each.
ROLES = {"armor_paint": "paint", "steel": "steel", "rubber": "rubber", "optics": "glass", "fittings": "dark",
         "lamps": "lamp", "recesses": "black"}
# The parts the rig moves, by the node `vehicle_export.rig` names for each.
RIG = ("hull", "turret", "gun", "hmg", "hmg_gun")
OWN = ("wheel_", "track_", "dressing_")
# Tiers 1-3 of the whole body (`mesh_lods.reduced_copy`): its triangles,
# before the paint's edge splits (`parts.finish`), and the size under which a
# loose piece drops out. A script builds its parts from hundreds of small
# closed pieces (bolts, slats, blocks), which the collapse leaves whole, so
# the far tiers shed the pieces too small to see from them. Tier 0 is the
# script's own.
TIERS = (None, (22000, 0.02, 8), (6500, 0.12, 4), (1500, 0.5, 4))
# A belt is never thinner than this, however low the road wheels sit.
MIN_BELT = 0.03
# Renamed on arrival, so the script's names never take the export's own.
PREFIX = "script:"
# A script's edges are sharp or single bevels, which `parts.finish` reads as
# whole edges (1.0) where a procedural part's two-segment bevel reads 2/3:
# its paint chips by the family's `chip` times that square, so a scripted
# body wears at its edges as a built one does.
EDGE_CHIP = (2 / 3) ** 2


def _base(name):
    """`name` without the prefix, Blender's `.001` and a tier suffix."""
    name = re.sub(r"\.\d{3}$", "", name.removeprefix(PREFIX))
    return re.sub(r"_LOD0$", "", name)


def _chain(o):
    while o is not None:
        yield o
        o = o.parent


def part_of(o):
    """The part `o` belongs to: (its own wheel, track or dressing name, or
    None; the rig part it rides)."""
    own = None
    for a in _chain(o):
        name = _base(a.name)
        if name in RIG:
            return own, name
        if name.startswith(OWN):
            own = name
    raise SystemExit(f"{_base(o.name)}: under no part the game moves ({', '.join(RIG)})")


def run_script(path):
    """Import the script at `path` (repository-relative) and call its
    `build()` in this scene; returns the objects it made, renamed `PREFIX`."""
    before = set(bpy.data.objects)
    spec = importlib.util.spec_from_file_location(f"scripted_body_{Path(path).parent.parent.name}", REPO / path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module  # the receipt hashes what the export loaded
    spec.loader.exec_module(module)
    module.build()
    made = sorted((o for o in bpy.data.objects if o not in before), key=lambda o: o.name)
    for o in made:
        o.name = PREFIX + o.name
    bpy.context.view_layer.update()
    return made


def _world(meshes):
    return [o.matrix_world @ v.co for o in meshes for v in o.data.vertices]


def _split(o, node, mats):
    """`o`'s faces as one object per material under `node`, at the same world
    place, each in its role's material; returns them."""
    to_node = node.matrix_world.inverted_safe() @ o.matrix_world
    made = []
    for index, slot in enumerate(o.data.materials):
        name = _base(slot.name)
        if name not in ROLES:
            raise SystemExit(f"{_base(o.name)}: material {slot.name} is not one of {', '.join(ROLES)}")
        bm = bmesh.new()
        bm.from_mesh(o.data)
        bmesh.ops.delete(bm, geom=[f for f in bm.faces if f.material_index != index], context="FACES")
        bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
        if not bm.faces:
            bm.free()
            continue
        for f in bm.faces:
            f.material_index = 0
        bm.transform(to_node)
        me = bpy.data.meshes.new(f"{_base(o.name)}_{name}_LOD0")
        bm.to_mesh(me)
        bm.free()
        while me.uv_layers:
            me.uv_layers.remove(me.uv_layers[0])
        while me.color_attributes:
            me.color_attributes.remove(me.color_attributes[0])
        me.materials.append(mats[ROLES[name]])
        part = bpy.data.objects.new(me.name, me)
        bpy.context.scene.collection.objects.link(part)
        part.parent = node
        made.append(part)
    return made


def _tiers(objects):
    """Tiers 1-3 of each tier-0 object, its share of `TIERS`."""
    counts = {o: mesh_lods.triangle_count(o) for o in objects}
    total = sum(counts.values())
    for o in sorted(objects, key=lambda o: o.name):
        base = o.name.removesuffix("_LOD0")
        for t in (1, 2, 3):
            target, extent, floor = TIERS[t]
            mesh_lods.reduced_copy(o, f"{base}_LOD{t}", max(1, round(target * counts[o] / total)), extent, floor)


def wear(v, path, link_pitch):
    """Build the script at `path` into `v` (see the module doc), its tracks'
    links `link_pitch` long. The family has made its rig already."""
    from vehicle_export import CHIP, materials
    v.mats["paint"] = materials(v.scheme, **{**v.looks, "chip": v.looks.get("chip", CHIP) * EDGE_CHIP})["paint"]
    made = run_script(path)
    meshes = [o for o in made if o.type == "MESH"]
    by_part = {}
    for o in meshes:
        by_part.setdefault(part_of(o), []).append(o)
    nodes = {name: bpy.data.objects[name] for name in RIG if name in bpy.data.objects}
    nodes["hull"] = v.hull
    for (own, rig), _ in by_part.items():
        if rig not in nodes:
            raise SystemExit(f"{own or rig}: rides {rig}, which the frame gives no node")
    # Where each wheel turns and what it measures, and where each run lies,
    # before the script's own objects go.
    axles = {own: next(a for a in _chain(o) if _base(a.name) == own).matrix_world.translation.copy()
             for (own, _), objs in by_part.items() if own and own.startswith("wheel_") for o in objs[:1]}
    runs = {}
    for (own, _), objs in by_part.items():
        if own and own.startswith("track_"):
            ys = [p.y for p in _world(objs)]
            runs[own] = ((min(ys) + max(ys)) / 2, max(ys) - min(ys))
    built = []
    wheels = []
    for (own, rig), objs in sorted(by_part.items(), key=lambda kv: (kv[0][1], kv[0][0] or "")):
        node = nodes[rig]
        if own and own.startswith("track_"):
            continue
        if own and own.startswith("wheel_"):
            axle = axles[own]
            radius = max(math.hypot(p.x - axle.x, p.z - axle.z) for p in _world(objs))
            node = wheel_node(own, tuple(node.matrix_world.inverted_safe() @ axle), round(radius, 4), node)
            wheels.append((axle, radius))
            bpy.context.view_layer.update()
        elif own:
            node = empty(own, parent=node)
            bpy.context.view_layer.update()
        for o in objs:
            built += _split(o, node, v.mats)
    for o in made:
        bpy.data.objects.remove(o, do_unlink=True)
    ours = set(v.mats.values())
    for m in [m for m in bpy.data.materials if m not in ours and m.users == 0]:
        bpy.data.materials.remove(m)
    _tiers(built)
    if set(runs) != {"track_L", "track_R"}:
        raise SystemExit(f"a tracked body names track_L and track_R, got {sorted(runs)}")
    # The belt fills the gap under the road wheels: its underside on the ground.
    thick = max(MIN_BELT, min(a.z - r for a, r in wheels))
    for name, (y, width) in sorted(runs.items()):
        circles = [(a.x, a.z, r) for a, r in wheels if (a.y > 0) == (y > 0)]
        track_loop(name, circles, y, width, link_pitch, v.mats, v.hull, thickness=thick)
    return built


def _body(v):
    """The worn body's tier objects: what rides the hull or the turret, not
    its running gear or dressing."""
    out = []
    for o in bpy.data.objects:
        if o.type != "MESH" or tier_of(o) is None:
            continue
        chain = [a.name for a in _chain(o.parent)]
        if not any(n.startswith(OWN) for n in chain) and ("hull" in chain or "turret" in chain):
            out.append(o)
    return out


def _top(objs, x, y):
    """The highest surface of `objs` over (x, y)."""
    hits = []
    for o in objs:
        inv = o.matrix_world.inverted_safe()
        ok, at, _, _ = o.ray_cast(inv @ Vector((x, y, 10.0)), inv.to_3x3() @ Vector((0, 0, -1)))
        if ok:
            hits.append((o.matrix_world @ at).z)
    return max(hits)


def wreck(v, side, wheels, seed=0):
    """A worn body after its cook-off: the `side` ("L" or "R") belt run off
    and lying flat beside the hull, its road wheels `wheels` (numbers) torn
    off, the body's plates warped by the fire and its glacis dented, torn
    plate on the ground. `wreckage.burn` then heaves the turret."""
    from parts import box, rest_on_ground
    from wreckage import densify, dent, heat, plate, remove, warp
    m = v.mats
    s = 1 if side == "L" else -1
    bpy.context.view_layer.update()
    band = bpy.data.objects[f"track_{side}_band_LOD0"]
    ys = [(band.matrix_world @ p.co).y for p in band.data.vertices]
    remove(f"track_{side}_band", *(f"wheel_{side}_{k}_" for k in wheels))
    thrown = box("thrown_track", (3.9, max(ys) - min(ys), 0.05), (-0.4, s * (v.width / 2 + 0.40), 0.03), m["track"],
                 v.hull, rot=(0, 0, -0.07 * s), lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=1.0 + seed))
    body = _body(v)
    nose = v.length / 2 - 0.55
    glacis = _top([o for o in body if tier_of(o) == 0], nose, 0.5)
    densify([o for o in body if o.data.materials[0] is m["paint"]], scale=2.0)
    warp(body, heat(0.022, 0.9, seed=11.0 + seed), dent((nose, 0.5, glacis), 0.5, 0.10, (-0.6, 0, -1)))
    for k, (loc, rot, size) in enumerate((((2.2, -s * 2.1, 0.03), (0.04, 0.02, 0.4), 0.40),
                                          ((-2.7, -s * 2.0, 0.03), (-0.03, 0.05, 1.8), 0.32),
                                          ((v.length / 2 + 0.5, 0.7, 0.03), (0.0, 0.06, 2.6), 0.26))):
        plate(f"litter_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=71 + k + seed)
    rest_on_ground(0.004)
