"""Photograph the facade lab's kit in Blender where the lab stands it: the picture its frames are compared with.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/facade_lab_render.py <meta.json> <out dir>

`meta.json` is the `facade` scene's evidence: where each module stands (`layout`), the sun
(`light`) and the cameras its station shots were taken from (`cameras`). The kit is read from
`assets/source/city/facade_lab/kit.glb`, as `assemble.py` reads a set's, and every placement is
instanced at tier 0. One PNG a camera is written to `<out dir>`, by Cycles: coverage is exact
there, so a cutout's edge is the recipe's, not a filter's.

The materials are `assemble.py`'s rebuild of the model shader's surface, with what the facade
materials add: a cutout or blended surface takes its coverage (the normal image's alpha times
the base colour's) as its alpha, a cutout cut at its cutoff; a room shows its cell of the
interior atlas (`interiors.py`'s sheets) by the lookup the battle's shader does, the cell chosen
by the same hash of where its module stands, as an emission as bright as a matte surface in
sun shadow. Light is one sun and a sky: a stand-in for the battle's, not a copy of it.
"""
import json
import math
import os
import struct
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import assemble  # noqa: E402

KIT = os.path.abspath(os.path.join(HERE, "../../../../assets/source/city/facade_lab/kit.glb"))
INTERIORS = os.path.abspath(os.path.join(HERE, "../../../../assets/source/city/interiors"))
GROUND = (0.17, 0.18, 0.155)  # the lab's ground, linear
SAMPLES = 48
SUN_ENERGY, SKY, SKY_STRENGTH = 4.0, (0.5, 0.64, 0.9), 0.9
SHADOW_FLOOR = 0.4  # the share of the sun a shaded surface keeps in the battle's light
# The interior atlas's contract (city/README.md, "Interiors"; `INTERIOR_ATLAS` in scene-assets).
CELLS, SHEET_COLUMNS, SHEET_ROWS, PINHOLE_M, ROOM_DEPTH_M = 10, 2, 5, 16.0, 4.5
ROOM_CUTS = 8  # a room's faces are cut this fine, so its lookup is the shader's to a hair
ROOMS = {}  # material name -> sheet


def coverage(material, info, doc):
    """Give a cutout or blended material its alpha: the coverage value, cut at a cutout's cutoff."""
    mode = info.get("alphaMode", "OPAQUE")
    if mode == "OPAQUE":
        return
    tree = material.node_tree
    bsdf = next(n for n in tree.nodes if n.type == "BSDF_PRINCIPLED")
    alpha = info.get("pbrMetallicRoughness", {}).get("baseColorFactor", (1, 1, 1, 1))[3]
    if "normalTexture" in info:
        name = doc["images"][doc["textures"][info["normalTexture"]["index"]]["source"]]["name"]
        image = tree.nodes.new("ShaderNodeTexImage")
        image.image = bpy.data.images[name]
        image.image.colorspace_settings.name = "Non-Color"
        image.image.alpha_mode = "CHANNEL_PACKED"
        alpha = assemble._math(tree, "MULTIPLY", image.outputs["Alpha"], alpha)
    if mode == "MASK":
        alpha = assemble._math(tree, "GREATER_THAN", alpha, info.get("alphaCutoff", 0.5))
    assemble._set(tree, bsdf.inputs["Alpha"], alpha)


def room_pick(x, y, z):
    """The battle shader's `roomPick`: PCG's permutation chained over the position's float32 bits."""
    h = 0
    for v in (x, y, z):
        bits = struct.unpack("<I", struct.pack("<f", v))[0]
        state = ((h ^ bits) * 747796405 + 2891336453) & 0xFFFFFFFF
        word = (((state >> ((state >> 28) + 4)) ^ state) * 277803737) & 0xFFFFFFFF
        h = ((word >> 22) ^ word) & 0xFFFFFFFF
    return h


def room_surface(material, sheet, light):
    """A room: its sheet as an emission, as bright as a white matte surface in sun shadow on the stage."""
    tree = material.node_tree
    tree.nodes.clear()
    out = tree.nodes.new("ShaderNodeOutputMaterial")
    emit = tree.nodes.new("ShaderNodeEmission")
    image = tree.nodes.new("ShaderNodeTexImage")
    image.image = bpy.data.images.load(os.path.join(INTERIORS, f"{sheet}.png"), check_existing=True)
    image.extension = "EXTEND"
    emit.inputs["Strength"].default_value = (SHADOW_FLOOR * SUN_ENERGY * math.sin(light["elevation"]) / math.pi
                                             + SKY_STRENGTH * sum(SKY) / 3)
    tree.links.new(image.outputs["Color"], emit.inputs["Color"])
    tree.links.new(emit.outputs[0], out.inputs[0])


def room_mesh(mesh, pick):
    """A copy of a room's mesh whose UVs are its cell of the sheet: the unfolded box each loop
    carries, through the atlas's pinhole, into the cell and mirroring `pick` chooses."""
    cell, mirrored = pick % CELLS, (pick >> 16) & 1
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bmesh.ops.subdivide_edges(bm, edges=bm.edges[:], cuts=ROOM_CUTS, use_grid_fill=True)
    uv = bm.loops.layers.uv.verify()
    for face in bm.faces:
        for loop in face.loops:
            u, v = loop[uv].uv
            depth = 1.0 - max(-u, u - 1.0, -v, v - 1.0, 0.0)
            k = PINHOLE_M / (PINHOLE_M + ROOM_DEPTH_M * depth)
            cu = 0.5 + (min(max(u, 0.0), 1.0) - 0.5) * k
            cv = 0.5 + (min(max(v, 0.0), 1.0) - 0.5) * k
            if mirrored:
                cu = 1.0 - cu
            row = cell // SHEET_COLUMNS
            loop[uv].uv = ((cell % SHEET_COLUMNS + cu) / SHEET_COLUMNS, 1.0 - (row + 1.0 - cv) / SHEET_ROWS)
    out = bpy.data.meshes.new(mesh.name + "_room")
    bm.to_mesh(out)
    bm.free()
    for material in mesh.materials:
        out.materials.append(material)
    return out


def load(light):
    """The kit's modules: {name: [(mesh object, its frame in the module)]} at tier 0."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    doc = assemble.glb_json(KIT)
    bpy.ops.import_scene.gltf(filepath=KIT)
    bpy.context.view_layer.update()
    for info in doc.get("materials", []):
        material = bpy.data.materials[info["name"]]
        sheet = info.get("extras", {}).get("interior")
        if sheet:
            ROOMS[material.name] = sheet
            room_surface(material, sheet, light)
            continue
        assemble.surface(material, info, doc)
        coverage(material, info, doc)
    modules = {}
    for o in list(bpy.data.objects):
        if o.parent is None and o.type == "EMPTY":
            inverse = o.matrix_world.inverted()
            modules[o.name] = [(c, inverse @ c.matrix_world) for c in o.children_recursive
                               if c.type == "MESH" and assemble.tier_of(c.name) in (None, 0)]
    for o in bpy.data.objects:
        o.hide_render = True
    return modules


def stage(light):
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = SAMPLES
    scene.cycles.use_denoising = False
    scene.cycles.device = "GPU"
    scene.render.resolution_x, scene.render.resolution_y, scene.render.resolution_percentage = assemble.WIDTH, assemble.HEIGHT, 100
    scene.render.image_settings.file_format = "PNG"
    world = bpy.data.worlds.new("sky")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (*SKY, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = SKY_STRENGTH
    scene.world = world
    toward = Vector((math.cos(light["elevation"]) * math.cos(light["azimuth"]),
                     math.cos(light["elevation"]) * math.sin(light["azimuth"]), math.sin(light["elevation"])))
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.data.energy, sun.data.angle = SUN_ENERGY, 0.03
    sun.rotation_euler = (-toward).to_track_quat("-Z", "Y").to_euler()
    scene.collection.objects.link(sun)
    camera = bpy.data.objects.new("camera", bpy.data.cameras.new("camera"))
    camera.data.sensor_fit, camera.data.clip_start, camera.data.clip_end = "VERTICAL", 0.5, 4000.0
    scene.collection.objects.link(camera)
    scene.camera = camera
    assemble.slab("ground", (-500, 500, -500, 500), 0.0, GROUND)
    return camera


def place(modules, layout):
    for p in layout:
        at = Matrix.Translation((p["x"], p["y"], p["z"])) @ Matrix.Rotation(p["yaw"], 4, "Z")
        pick = room_pick(p["x"], p["y"], p["z"])
        for source, local in modules[p["module"]]:
            room = any(m is not None and m.name in ROOMS for m in source.data.materials)
            o = bpy.data.objects.new("placed", room_mesh(source.data, pick) if room else source.data)
            o.matrix_world = at @ local
            o.color = (*p.get("tint", (1.0, 1.0, 1.0)), 1.0)
            bpy.context.scene.collection.objects.link(o)


def shoot(camera, pose, path):
    target = Vector(pose["target"])
    pitch, yaw = pose["pitch"], pose["yaw"]
    camera.data.angle = pose["fovY"]
    camera.location = target + pose["distance"] * Vector((math.cos(pitch) * math.cos(yaw), math.cos(pitch) * math.sin(yaw),
                                                          math.sin(pitch)))
    camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    print("RENDER", path)


def main():
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    if len(args) != 2:
        raise SystemExit("facade_lab_render.py <meta.json> <out dir>")
    meta, out = json.load(open(args[0])), os.path.abspath(args[1])
    os.makedirs(out, exist_ok=True)
    modules = load(meta["light"])
    camera = stage(meta["light"])
    place(modules, meta["layout"])
    for name, pose in sorted(meta["cameras"].items()):
        shoot(camera, pose, os.path.join(out, f"{name}.png"))


if __name__ == "__main__":
    main()
