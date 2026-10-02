"""Reassemble a city set from its two files and photograph it at the game's camera.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/assemble.py <set dir> <out dir> [sheet ...]

A proof of the export, for eyes: nothing here is read from the script that made
the set. `kit.glb` and `templates.json` are loaded into an empty scene, every
row is instanced as the readme says (scale, yaw about +Z, move; drawn at the
tiers its mask names; tint on tint-masked surfaces), and the descriptor's part
boxes are drawn over them as wires.

Sheets (all of them unless named):
  each     one sheet a template: 30 m, 80 m and 250 m, with its part boxes
  street   every template in a row, seen from 80 m
  suburb   a made-up block of about twenty houses, seen from 250 m
  tiers    one template at each of its four detail tiers, from 80 m

The camera is the battle's: 0.8 rad of vertical view at 1920x1080, pitched 0.34 rad
down at 30 m and 0.85 rad from 65 m out. A building is drawn at the tier its
projected height asks for, by the model thresholds in `fixtures/game.json`. The
materials rebuild the model shader's surface (vertex colour times texture, wear
past the albedo's alpha, tint through the mask), lit by one sun and a sky: a
stand-in for the battle's light, not a copy of it.
"""
import json
import math
import os
import struct
import sys

import bpy
import numpy as np
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
import textures  # noqa: E402

WIDTH, HEIGHT, VFOV = 1920, 1080, 0.8
FOCAL_PX = HEIGHT / 2 / math.tan(VFOV / 2)
LOD_PX = json.load(open(os.path.join(HERE, "../../../../fixtures/game.json")))["presentation"]["models"]["lod_px"]
WEAR_EDGE = 0.04  # the model shader's
SUN_TOWARD = Vector((0.55, -0.6, 0.62)).normalized()  # from the south-east, mid-morning


def pitch_at(distance):
    return 0.34 + (0.85 - 0.34) * min(1.0, max(0.0, (distance - 30) / 35))


def tier_at(height_m, distance):
    px = height_m * FOCAL_PX / distance
    return 0 if px > LOD_PX[0] else 1 if px > LOD_PX[1] else 2 if px > LOD_PX[2] else 3


def srgb(c):
    c /= 255
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


# ---------------------------------------------------------------- the kit
def glb_json(path):
    data = open(path, "rb").read()
    return json.loads(data[20:20 + struct.unpack_from("<I", data, 12)[0]])


def _set(tree, socket, value):
    if isinstance(value, bpy.types.NodeSocket):
        tree.links.new(value, socket)
    else:
        socket.default_value = value


def _times(tree, a, b):
    n = tree.nodes.new("ShaderNodeVectorMath")
    n.operation = "MULTIPLY"
    _set(tree, n.inputs[0], a)
    _set(tree, n.inputs[1], b)
    return n.outputs[0]


def _mix(tree, factor, a, b):
    n = tree.nodes.new("ShaderNodeMix")
    n.data_type = "RGBA"
    _set(tree, n.inputs[0], factor)
    _set(tree, n.inputs[6], a)
    _set(tree, n.inputs[7], b)
    return n.outputs[2]


def _math(tree, operation, a, b, c=None, clamp=True):
    n = tree.nodes.new("ShaderNodeMath")
    n.operation = operation
    n.use_clamp = clamp
    for socket, value in zip(n.inputs, (a, b, c)):
        if value is not None:
            _set(tree, socket, value)
    return n.outputs[0]


def surface(material, info, doc):
    """The model shader's surface, as nodes: see `modelSurface` in the battle renderer."""
    tree = material.node_tree
    tree.nodes.clear()
    out = tree.nodes.new("ShaderNodeOutputMaterial")
    bsdf = tree.nodes.new("ShaderNodeBsdfPrincipled")
    tree.links.new(bsdf.outputs[0], out.inputs[0])
    pbr, extras = info.get("pbrMetallicRoughness", {}), info.get("extras", {})
    scale = extras.get("colour_scale", 1.0)
    paint = tree.nodes.new("ShaderNodeVertexColor")
    colour = _times(tree, paint.outputs["Color"], tuple(c * scale for c in pbr.get("baseColorFactor", (1, 1, 1, 1))[:3]))
    tint = extras.get("tint", 0.0)

    def image(slot, data=True):
        name = doc["images"][doc["textures"][slot["index"]]["source"]]["name"]
        node = tree.nodes.new("ShaderNodeTexImage")
        node.image = bpy.data.images[name]
        node.image.colorspace_settings.name = "Non-Color" if data else "sRGB"
        return node

    if "baseColorTexture" in pbr:
        albedo = image(pbr["baseColorTexture"], data=False)
        colour = _times(tree, colour, albedo.outputs["Color"])
        if "wear" in extras:
            past = _math(tree, "SUBTRACT", paint.outputs["Alpha"], albedo.outputs["Alpha"], clamp=False)
            worn = _math(tree, "MULTIPLY_ADD", past, 0.5 / WEAR_EDGE, 0.5)  # the shader's smoothstep, as a ramp
            colour = _mix(tree, worn, colour, (*extras["wear"][:3], 1.0))
    bsdf.inputs["Roughness"].default_value = pbr.get("roughnessFactor", 1.0)
    bsdf.inputs["Metallic"].default_value = pbr.get("metallicFactor", 1.0)
    if "metallicRoughnessTexture" in pbr:
        orm = image(pbr["metallicRoughnessTexture"])
        split = tree.nodes.new("ShaderNodeSeparateColor")
        tree.links.new(orm.outputs["Color"], split.inputs[0])
        tree.links.new(split.outputs[1], bsdf.inputs["Roughness"])
        tree.links.new(split.outputs[2], bsdf.inputs["Metallic"])
        tint = _math(tree, "MULTIPLY", orm.outputs["Alpha"], tint)
    if "normalTexture" in info:
        bump = tree.nodes.new("ShaderNodeNormalMap")
        tree.links.new(image(info["normalTexture"]).outputs["Color"], bump.inputs["Color"])
        tree.links.new(bump.outputs[0], bsdf.inputs["Normal"])
    row = tree.nodes.new("ShaderNodeObjectInfo")
    colour = _mix(tree, tint, colour, _times(tree, colour, row.outputs["Color"]))
    tree.links.new(colour, bsdf.inputs["Base Color"])


def tier_of(name):
    i = name.rfind("_LOD")
    return int(name[i + 4:]) if i >= 0 and name[i + 4:].isdigit() else None


def load(set_dir):
    """The set's modules ({name: [(tier or None, mesh object, its frame in the module)]}) and its templates."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    path = os.path.join(set_dir, "kit.glb")
    doc = glb_json(path)
    bpy.ops.import_scene.gltf(filepath=path)
    bpy.context.view_layer.update()
    for info in doc.get("materials", []):
        surface(bpy.data.materials[info["name"]], info, doc)
    modules = {}
    for o in list(bpy.data.objects):
        if o.parent is None and o.type == "EMPTY":
            inverse = o.matrix_world.inverted()
            modules[o.name] = [(tier_of(c.name), c, inverse @ c.matrix_world) for c in o.children_recursive if c.type == "MESH"]
    for o in bpy.data.objects:
        o.hide_render = True
    templates = json.load(open(os.path.join(set_dir, "templates.json")))
    missing = sorted(set(templates["modules"]) - set(modules))
    if missing:
        raise SystemExit(f"templates.json names modules kit.glb lacks: {missing}")
    return modules, templates


# ---------------------------------------------------------------- the scene
def stage():
    scene = bpy.context.scene
    engines = {e.identifier for e in bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items}
    scene.render.engine = "BLENDER_EEVEE" if "BLENDER_EEVEE" in engines else "BLENDER_EEVEE_NEXT"
    scene.eevee.taa_render_samples = 24
    scene.render.resolution_x, scene.render.resolution_y, scene.render.resolution_percentage = WIDTH, HEIGHT, 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = False
    world = bpy.data.worlds.new("sky")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.5, 0.64, 0.9, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = 0.9
    scene.world = world
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.data.energy, sun.data.angle = 4.0, 0.03
    sun.rotation_euler = (-SUN_TOWARD).to_track_quat("-Z", "Y").to_euler()
    scene.collection.objects.link(sun)
    camera = bpy.data.objects.new("camera", bpy.data.cameras.new("camera"))
    camera.data.sensor_fit, camera.data.angle, camera.data.clip_start, camera.data.clip_end = "VERTICAL", VFOV, 1.0, 4000.0
    scene.collection.objects.link(camera)
    scene.camera = camera
    slab("ground", (-2000, 2000, -2000, 2000), -0.02, (0.085, 0.11, 0.055))
    return camera


def flat(name, colour, emit=0.0):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    bsdf = m.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = (*colour, 1)
    bsdf.inputs["Roughness"].default_value = 1.0
    if emit:
        bsdf.inputs["Emission Color"].default_value = (*colour, 1)
        bsdf.inputs["Emission Strength"].default_value = emit
    return m


def slab(name, rect, z, colour):
    x0, x1, y0, y1 = rect
    me = bpy.data.meshes.new(name)
    me.from_pydata([(x0, y0, z), (x1, y0, z), (x1, y1, z), (x0, y1, z)], [], [(0, 1, 2, 3)])
    me.materials.append(flat(name, colour))
    o = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(o)
    return o


def build(modules, names, template, frame=Matrix.Identity(4), tier=0, wires=0.0, into=None):
    """Instance a template's rows at one tier under `frame`; `wires` metres thick part boxes over it."""
    made = []
    for index, x, y, z, yaw, sx, sy, sz, tiers, r, g, b in template["states"]["intact"]:
        if not tiers >> tier & 1:
            continue
        at = frame @ Matrix.Translation((x, y, z)) @ Matrix.Rotation(yaw, 4, "Z") @ Matrix.Diagonal((sx, sy, sz, 1))
        for mesh_tier, source, local in modules[names[index]]:
            if mesh_tier is None or mesh_tier == tier:
                o = bpy.data.objects.new("row", source.data)
                o.matrix_world = at @ local
                o.color = (srgb(r), srgb(g), srgb(b), 1.0)
                made.append(o)
    if wires:
        ink = bpy.data.materials.get("wire") or flat("wire", (1.0, 0.1, 0.8), emit=3.0)
        for part in template["descriptor"]["parts"]:
            (cx, cy), (hx, hy, hz), base = part["center"], part["half_extents"], part["base_z"]
            corners = [(cx + sx * hx, cy + sy * hy, base + sz * 2 * hz) for sz in (0, 1) for sy in (-1, 1) for sx in (-1, 1)]
            me = bpy.data.meshes.new("part")
            me.from_pydata(corners, [], [(0, 1, 3, 2), (4, 5, 7, 6), (0, 1, 5, 4), (2, 3, 7, 6), (0, 2, 6, 4), (1, 3, 7, 5)])
            me.materials.append(ink)
            o = bpy.data.objects.new("part", me)
            o.matrix_world = frame
            o.modifiers.new("wire", "WIREFRAME").thickness = wires
            o.visible_shadow = False
            made.append(o)
    for o in made:
        (into or bpy.context.scene.collection).objects.link(o)
    return made


def clear(objects):
    for o in objects:
        bpy.data.objects.remove(o, do_unlink=True)


def label(text, at, size=1.6):
    curve = bpy.data.curves.new("label", "FONT")
    curve.body, curve.size, curve.align_x = text, size, "CENTER"
    curve.materials.append(bpy.data.materials.get("label") or flat("label", (1, 1, 1), emit=1.5))
    o = bpy.data.objects.new("label", curve)
    o.location = (at[0], at[1], 0.05)
    bpy.context.scene.collection.objects.link(o)
    return o


def shoot(camera, path, target, distance, azimuth, crop=None):
    """Render the game camera `distance` from `target`; `crop` (w, h) keeps the middle of the frame at its true scale."""
    pitch = pitch_at(distance)
    target = Vector(target)
    camera.location = target + distance * Vector((math.cos(pitch) * math.cos(azimuth), math.cos(pitch) * math.sin(azimuth),
                                                  math.sin(pitch)))
    camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler()
    render = bpy.context.scene.render
    render.use_border = render.use_crop_to_border = crop is not None
    if crop:
        w, h = crop[0] / WIDTH / 2, crop[1] / HEIGHT / 2
        dy = (crop[2] if len(crop) > 2 else 0) / HEIGHT
        render.border_min_x, render.border_max_x = 0.5 - w, 0.5 + w
        render.border_min_y, render.border_max_y = 0.5 + dy - h, 0.5 + dy + h
    render.filepath = path
    bpy.ops.render.render(write_still=True)
    return pixels(path)


def pixels(path):
    image = bpy.data.images.load(path)
    w, h = image.size
    data = np.empty(w * h * 4, np.float32)
    image.pixels.foreach_get(data)
    bpy.data.images.remove(image)
    return np.clip(np.round(data.reshape(h, w, 4)[::-1] * 255), 0, 255).astype(np.uint8)


def save(path, image):
    image = image.copy()
    image[..., 3] = 255
    open(path, "wb").write(textures.png(image))
    print("SHEET", path, image.shape[1], "x", image.shape[0])


def extent(template):
    """A template's box: (x0, x1, y0, y1, height)."""
    parts = template["descriptor"]["parts"]
    lo = [min(p["center"][i] - p["half_extents"][i] for p in parts) for i in (0, 1)]
    hi = [max(p["center"][i] + p["half_extents"][i] for p in parts) for i in (0, 1)]
    return lo[0], hi[0], lo[1], hi[1], max(p["base_z"] + 2 * p["half_extents"][2] for p in parts)


# ---------------------------------------------------------------- the sheets
def sheet_each(camera, modules, templates, out, scratch):
    names = templates["modules"]
    for template in templates["templates"]:
        x0, x1, y0, y1, height = extent(template)
        target = ((x0 + x1) / 2, (y0 + y1) / 2, height * 0.4)
        panels = []
        for distance, crop in ((30, None), (80, (960, 540)), (250, (960, 540))):
            made = build(modules, names, template, tier=tier_at(height, distance), wires=0.0012 * distance)
            panels.append(shoot(camera, os.path.join(scratch, "panel.png"), target, distance, math.radians(-118), crop))
            clear(made)
        save(os.path.join(out, f"{template['descriptor']['id']}.png"),
             np.concatenate([panels[0], np.concatenate(panels[1:], 0)], 1))


def in_order(templates):
    return sorted(templates["templates"], key=lambda t: (t["descriptor"]["category"] != "detached_home", t["descriptor"]["id"]))


def sheet_street(camera, modules, templates, out, scratch):
    """Every template side by side, street side to the camera, in runs short enough to see at 80 m."""
    names, gap, runs, run, x = templates["modules"], 6.0, [], [], 0.0
    for template in in_order(templates):
        x0, x1, *_ = extent(template)
        if run and x + (x1 - x0) > 84:
            runs.append((run, x - gap))
            run, x = [], 0.0
        run.append((template, x - x0))
        x += x1 - x0 + gap
    runs.append((run, x - gap))
    strips = []
    for run, length in runs:
        made = [slab("street", (-20, length + 20, -17, -9), 0.0, (0.05, 0.05, 0.052))]
        for template, at in run:
            x0, x1, y0, y1, height = extent(template)
            frame = Matrix.Translation((at, -y0, 0))  # every street front on one line
            made += build(modules, names, template, frame, tier=tier_at(height, 80))
            made.append(label(template["descriptor"]["id"].replace("china-", ""), (at + (x0 + x1) / 2, -6.0)))
        strips.append(shoot(camera, os.path.join(scratch, "panel.png"), (length / 2, 5.0, 3.0), 80, math.radians(-90),
                            (WIDTH, 600, 20)))
        clear(made)
    save(os.path.join(out, "street-80m.png"), np.concatenate(strips, 0))


def sheet_suburb(camera, modules, templates, out, scratch):
    """A made-up block: two streets, houses facing each across them, a terrace and the shops on the corner."""
    names = templates["modules"]
    by = {t["descriptor"]["id"].replace("china-", ""): t for t in templates["templates"]}
    rows = [  # (the street's y, which side of it, the houses along it from the west)
        (0.0, 1, ["home-10x8-1f", "home-9x9-2f", "home-ell-2f", "home-8x11-1f", "home-12x9-2f", "home-10x8-1f", "home-9x9-2f"]),
        (0.0, -1, ["terrace-3x2f", "home-8x11-1f", "home-12x9-2f", "townhouse-2f", "home-ell-2f", "home-10x8-1f"]),
        (-64.0, 1, ["terrace-5x3f", "home-9x9-2f", "townhouse-2f", "home-12x9-2f", "home-8x11-1f"]),
        (-64.0, -1, ["corner-shop-3f", "shops-4x3f", "townhouse-2f", "terrace-3x2f", "home-ell-2f"]),
    ]
    target = Vector((62.0, -32.0, 0.0))
    eye = target + 250 * Vector((math.cos(0.85) * math.cos(math.radians(-112)), math.cos(0.85) * math.sin(math.radians(-112)),
                                 math.sin(0.85)))
    made = [slab(f"street_{y}", (-30, 160, y - 4, y + 4), 0.0, (0.05, 0.05, 0.052)) for y in (0.0, -64.0)]
    made.append(slab("cross_street", (-14, -6, -100, 30), 0.0, (0.05, 0.05, 0.052)))
    count = 0
    for street, side, houses in rows:
        x = 0.0
        for id_ in houses:
            x0, x1, y0, y1, height = extent(by[id_])
            # the template's street side is -Y: north of a street it faces south as authored, south of it it is turned round
            turn = 0.0 if side > 0 else math.pi
            centre = Vector((x + (x1 - x0) / 2, street + side * 9.0, 0.0))
            frame = Matrix.Translation(centre) @ Matrix.Rotation(turn, 4, "Z") @ Matrix.Translation((-(x0 + x1) / 2, -y0, 0))
            made += build(modules, names, by[id_], frame, tier=tier_at(height, (centre - eye).length))
            x += x1 - x0 + 8.0
            count += 1
    image = shoot(camera, os.path.join(scratch, "panel.png"), target, 250, math.radians(-112))
    clear(made)
    save(os.path.join(out, "suburb-250m.png"), image)
    print(f"suburb: {count} buildings")


def sheet_tiers(camera, modules, templates, out, scratch):
    names = templates["modules"]
    template = next(t for t in templates["templates"] if t["descriptor"]["id"].endswith("home-12x9-2f"))
    made = []
    for tier in range(4):
        made += build(modules, names, template, Matrix.Translation((tier * 18.0, 0, 0)), tier=tier)
        made.append(label(f"tier {tier}", (tier * 18.0, -8.0)))
    image = shoot(camera, os.path.join(scratch, "panel.png"), (27.0, 0.0, 3.0), 80, math.radians(-90), (WIDTH, 600, 20))
    clear(made)
    save(os.path.join(out, "tiers-80m.png"), image)


def main():
    args = [a for a in sys.argv[sys.argv.index("--") + 1:] if a != "--"] if "--" in sys.argv else []
    if len(args) < 2:
        raise SystemExit("assemble.py <set dir> <out dir> [each|street|suburb|tiers ...]")
    set_dir, out = os.path.abspath(args[0]), os.path.abspath(args[1])
    sheets = {"each": sheet_each, "street": sheet_street, "suburb": sheet_suburb, "tiers": sheet_tiers}
    scratch = os.path.join(out, "scratch")
    os.makedirs(scratch, exist_ok=True)
    modules, templates = load(set_dir)
    camera = stage()
    for name in args[2:] or list(sheets):
        sheets[name](camera, modules, templates, out, scratch)


if __name__ == "__main__":
    main()
