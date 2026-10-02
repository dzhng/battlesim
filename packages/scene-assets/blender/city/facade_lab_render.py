"""Photograph the facade lab's kit in Blender where the lab stands it: the picture its frames are compared with.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/facade_lab_render.py <meta.json> <out dir>

`meta.json` is the `facade` scene's evidence: where each module stands (`layout`), the sun
(`light`) and the cameras its station shots were taken from (`cameras`). The kit is read from
`assets/source/city/facade_lab/kit.glb`, as `assemble.py` reads a set's, and every placement is
instanced at tier 0. One PNG a camera is written to `<out dir>`, by Cycles: coverage is exact
there, so a cutout's edge is the recipe's, not a filter's.

The materials are `assemble.py`'s rebuild of the model shader's surface, with what the facade
materials add: a cutout or blended surface takes its coverage (the normal image's alpha times
the base colour's) as its alpha, a cutout cut at its cutoff. Light is one sun and a sky: a
stand-in for the battle's, not a copy of it.
"""
import json
import math
import os
import sys

import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import assemble  # noqa: E402

KIT = os.path.abspath(os.path.join(HERE, "../../../../assets/source/city/facade_lab/kit.glb"))
GROUND = (0.17, 0.18, 0.155)  # the lab's ground, linear
SAMPLES = 48


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


def load():
    """The kit's modules: {name: [(mesh object, its frame in the module)]} at tier 0."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    doc = assemble.glb_json(KIT)
    bpy.ops.import_scene.gltf(filepath=KIT)
    bpy.context.view_layer.update()
    for info in doc.get("materials", []):
        material = bpy.data.materials[info["name"]]
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
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.5, 0.64, 0.9, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = 0.9
    scene.world = world
    toward = Vector((math.cos(light["elevation"]) * math.cos(light["azimuth"]),
                     math.cos(light["elevation"]) * math.sin(light["azimuth"]), math.sin(light["elevation"])))
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.data.energy, sun.data.angle = 4.0, 0.03
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
        for source, local in modules[p["module"]]:
            o = bpy.data.objects.new("placed", source.data)
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
    modules = load()
    camera = stage(meta["light"])
    place(modules, meta["layout"])
    for name, pose in sorted(meta["cameras"].items()):
        shoot(camera, pose, os.path.join(out, f"{name}.png"))


if __name__ == "__main__":
    main()
