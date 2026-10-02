"""Reassemble a city source set in Blender and render it at the game's camera.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/assemble.py -- <set dir> <out dir> [--tiers] [template id...]

A review aid for a set the renderer cannot draw yet. It reads only the two
files a set is (`kit.glb`, `templates.json`): every row of a template's intact
state is placed as the format says (scale per axis, yaw, position, tint), with
the materials rebuilt the way the game shades them (albedo times vertex colour
times base colour, the row's tint on masked surfaces, roughness and metalness
from the ORM image). The descriptor's part boxes are drawn over it as a wire.

Each template is rendered at 30 m, 80 m and 250 m (the three columns) at detail
tiers 0, 1 and 2, with a fourth view of tier 3 at 250 m, into
`<out dir>/<template id>.png`. With `--tiers` the four views are the four tiers
side by side at 80 m, into `<template id>_tiers.png`.
"""
import json
import math
import os
import struct
import sys

import bpy
import numpy as np
from mathutils import Matrix, Vector

FOV_Y = 0.8  # fixtures/game.json presentation.camera
PITCH = ((25.0, 0.22), (65.0, 0.85), (2000.0, 0.85))
VIEWS = ((30.0, 0), (80.0, 1), (250.0, 2), (250.0, 3))  # distance in metres, detail tier
SIZE = (1280, 720)
# Seen from the street side (-Y), toward the east end.
AZIMUTH = math.radians(-55)


def pitch_for(distance):
    for (d0, p0), (d1, p1) in zip(PITCH, PITCH[1:]):
        if distance <= d1:
            t = math.log(max(distance, d0) / d0) / math.log(d1 / d0)
            return p0 + (p1 - p0) * t
    return PITCH[-1][1]


def stage():
    """A neutral ground, sun and sky, and the game's camera."""
    sc = bpy.context.scene
    ground = bpy.data.meshes.new("ground")
    ground.from_pydata([(-900, -900, -0.02), (900, -900, -0.02), (900, 900, -0.02), (-900, 900, -0.02)], [], [(0, 1, 2, 3)])
    mat = bpy.data.materials.new("ground")
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = (0.21, 0.22, 0.2, 1)
    bsdf.inputs["Roughness"].default_value = 0.95
    ground.materials.append(mat)
    sc.collection.objects.link(bpy.data.objects.new("ground", ground))
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.data.energy = 4.0
    sun.data.angle = math.radians(2)
    sun.rotation_euler = (math.radians(52), 0, math.radians(-25))
    sc.collection.objects.link(sun)
    world = bpy.data.worlds.new("sky")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.62, 0.72, 0.85, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = 0.9
    sc.world = world
    sc.render.engine = "BLENDER_EEVEE"
    sc.eevee.taa_render_samples = 16
    sc.eevee.use_raytracing = False
    sc.view_settings.view_transform = "Standard"
    sc.view_settings.look = "None"
    sc.view_settings.exposure = 0.0
    cam = bpy.data.objects.new("camera", bpy.data.cameras.new("camera"))
    cam.data.sensor_fit = "VERTICAL"
    cam.data.angle_y = FOV_Y
    cam.data.clip_end = 5000
    sc.collection.objects.link(cam)
    sc.camera = cam
    sc.render.resolution_x, sc.render.resolution_y = SIZE
    sc.render.resolution_percentage = 100
    sc.render.image_settings.file_format = "PNG"
    return cam


def aim(cam, distance, height):
    """The game camera `distance` from a building `height` tall standing at the origin.
    Close in, it looks at the facade rather than the pavement."""
    pitch = pitch_for(distance)
    target = Vector((0, 0, height * 0.35 if distance < 60 else 0))
    away = Vector((math.cos(pitch) * math.cos(AZIMUTH), math.cos(pitch) * math.sin(AZIMUTH), math.sin(pitch)))
    cam.location = target + away * distance
    cam.rotation_euler = (target - cam.location).to_track_quat("-Z", "Y").to_euler()


def render(path):
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)


def pixels(path):
    image = bpy.data.images.load(path)
    w, h = image.size
    raw = np.empty(w * h * 4, dtype=np.float32)
    image.pixels.foreach_get(raw)
    bpy.data.images.remove(image)
    return raw.reshape(h, w, 4)


def sheet(grid, path):
    """`grid` is rows of image paths; one PNG of them all."""
    tiles = [[pixels(p) for p in row] for row in grid]
    h, w = tiles[0][0].shape[:2]
    out = np.zeros((h * len(tiles), w * len(tiles[0]), 4), dtype=np.float32)
    for r, row in enumerate(tiles):
        for c, tile in enumerate(row):
            y = (len(tiles) - 1 - r) * h
            out[y:y + h, c * w:(c + 1) * w] = tile
    image = bpy.data.images.new("sheet", out.shape[1], out.shape[0], alpha=False)
    image.pixels.foreach_set(out.ravel())
    image.filepath_raw = path
    image.file_format = "PNG"
    image.save()
    bpy.data.images.remove(image)


# ---------------------------------------------------------------- the kit
def glb_json(path):
    data = open(path, "rb").read()
    return json.loads(data[20:20 + struct.unpack_from("<I", data, 12)[0]])


def game_materials(doc):
    """Rebuild every imported material as the game shades it."""
    images = [im["name"] for im in doc.get("images", [])]
    image_of = lambda ref: images[doc["textures"][ref["index"]]["source"]] if ref else None
    for spec in doc["materials"]:
        mat = bpy.data.materials[spec["name"]]
        pbr = spec.get("pbrMetallicRoughness", {})
        tree = mat.node_tree
        tree.nodes.clear()
        new = tree.nodes.new
        out, bsdf = new("ShaderNodeOutputMaterial"), new("ShaderNodeBsdfPrincipled")
        tree.links.new(bsdf.outputs[0], out.inputs["Surface"])

        def texture(name, colour):
            node = new("ShaderNodeTexImage")
            node.image = bpy.data.images[name]
            node.image.colorspace_settings.name = "sRGB" if colour else "Non-Color"
            return node

        def times(a, b):
            node = new("ShaderNodeMix")
            node.data_type, node.blend_type = "RGBA", "MULTIPLY"
            node.inputs["Factor"].default_value = 1.0
            for socket, value in ((node.inputs[6], a), (node.inputs[7], b)):
                if isinstance(value, bpy.types.NodeSocket):
                    tree.links.new(value, socket)
                else:
                    socket.default_value = value
            return node.outputs[2]

        vertex = new("ShaderNodeVertexColor")
        albedo = times(vertex.outputs["Color"], tuple(pbr.get("baseColorFactor", (1, 1, 1, 1))))
        if image_of(pbr.get("baseColorTexture")):
            albedo = times(albedo, texture(image_of(pbr["baseColorTexture"]), True).outputs["Color"])
        if spec.get("extras", {}).get("tint"):
            albedo = times(albedo, new("ShaderNodeObjectInfo").outputs["Color"])
        tree.links.new(albedo, bsdf.inputs["Base Color"])
        rough, metal = pbr.get("roughnessFactor", 1.0), pbr.get("metallicFactor", 1.0)
        if image_of(pbr.get("metallicRoughnessTexture")):
            split = new("ShaderNodeSeparateColor")
            tree.links.new(texture(image_of(pbr["metallicRoughnessTexture"]), False).outputs["Color"], split.inputs[0])
            for socket, channel, factor in (("Roughness", "Green", rough), ("Metallic", "Blue", metal)):
                scale = new("ShaderNodeMath")
                scale.operation = "MULTIPLY"
                scale.inputs[1].default_value = factor
                tree.links.new(split.outputs[channel], scale.inputs[0])
                tree.links.new(scale.outputs[0], bsdf.inputs[socket])
        else:
            bsdf.inputs["Roughness"].default_value = rough
            bsdf.inputs["Metallic"].default_value = metal
        if image_of(spec.get("normalTexture")):
            bump = new("ShaderNodeNormalMap")
            tree.links.new(texture(image_of(spec["normalTexture"]), False).outputs["Color"], bump.inputs["Color"])
            tree.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])


def load_kit(path):
    """{module id: [mesh per tier]}, with the imported objects themselves hidden."""
    bpy.ops.import_scene.gltf(filepath=path)
    game_materials(glb_json(path))
    kit = {}
    for o in bpy.data.objects:
        if o.type == "MESH" and o.parent is not None and "_LOD" in o.name:
            module, tier = o.name.rsplit("_LOD", 1)
            kit.setdefault(module, {})[int(tier)] = o.data
    for o in list(bpy.data.objects):
        if o.type in ("MESH", "EMPTY"):
            bpy.data.objects.remove(o)
    return kit


def linear(byte):
    c = byte / 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def place(template, modules, kit, tier):
    """One object per row that draws at `tier`; returns the collection."""
    group = bpy.data.collections.new(f"tier{tier}")
    bpy.context.scene.collection.children.link(group)
    for module, x, y, z, yaw, sx, sy, sz, tiers, r, g, b in template["states"]["intact"]:
        if not tiers >> tier & 1:
            continue
        o = bpy.data.objects.new(modules[module], kit[modules[module]][tier])
        o.matrix_world = Matrix.Translation((x, y, z)) @ Matrix.Rotation(yaw, 4, "Z") @ Matrix.Diagonal((sx, sy, sz, 1.0))
        o.color = (linear(r), linear(g), linear(b), 1.0)
        group.objects.link(o)
    return group


def wire(descriptor, fit):
    """The descriptor's part boxes as a wire, and the same boxes grown by the fit."""
    group = bpy.data.collections.new("parts")
    bpy.context.scene.collection.children.link(group)
    for grow, colour, width in (((0.0, 0.0), (1.0, 0.1, 0.6, 1), 0.06), ((fit["side_m"], fit["top_m"]), (0.1, 0.9, 1.0, 1), 0.03)):
        mat = bpy.data.materials.new("wire")
        mat.use_nodes = True
        tree = mat.node_tree
        tree.nodes.clear()
        out, glow = tree.nodes.new("ShaderNodeOutputMaterial"), tree.nodes.new("ShaderNodeEmission")
        glow.inputs["Color"].default_value = colour
        tree.links.new(glow.outputs[0], out.inputs["Surface"])
        for part in descriptor["parts"]:
            hx, hy, hz = part["half_extents"]
            hx, hy, top = hx + grow[0], hy + grow[0], 2 * hz + grow[1]
            corners = [(sx * hx, sy * hy, z) for z in (0.0, top) for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            edges = [(i, (i + 1) % 4) for i in range(4)] + [(4 + i, 4 + (i + 1) % 4) for i in range(4)] + [(i, i + 4) for i in range(4)]
            me = bpy.data.meshes.new("wire")
            me.from_pydata(corners, edges, [])
            me.materials.append(mat)
            o = bpy.data.objects.new("wire", me)
            o.matrix_world = Matrix.Translation((*part["center"], part["base_z"])) @ Matrix.Rotation(part["yaw"], 4, "Z")
            group.objects.link(o)
            skin = o.modifiers.new("skin", "SKIN")
            for v in me.skin_vertices[0].data:
                v.radius = (width, width)
            skin.use_smooth_shade = False
    return group


def main():
    args = [a for a in sys.argv[sys.argv.index("--") + 1:] if a != "--"]
    tiers = "--tiers" in args
    args = [a for a in args if a != "--tiers"]
    source, out, wanted = args[0], args[1], args[2:]
    views = tuple((80.0, tier) for tier in range(4)) if tiers else VIEWS
    os.makedirs(out, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    doc = json.load(open(os.path.join(source, "templates.json")))
    kit = load_kit(os.path.join(source, "kit.glb"))
    cam = stage()
    for template in doc["templates"]:
        desc = template["descriptor"]
        if wanted and desc["id"] not in wanted:
            continue
        height = max(p["base_z"] + 2 * p["half_extents"][2] for p in desc["parts"])
        boxes = wire(desc, doc["fit"])
        shots = []
        for distance, tier in views:
            group = place(template, doc["modules"], kit, tier)
            aim(cam, distance, height)
            shots.append(os.path.join(out, f"{desc['id']}_{int(distance):03d}m_tier{tier}.png"))
            render(shots[-1])
            for o in list(group.objects):
                bpy.data.objects.remove(o)
            bpy.data.collections.remove(group)
        for o in list(boxes.objects):
            bpy.data.objects.remove(o)
        bpy.data.collections.remove(boxes)
        name = os.path.join(out, desc["id"] + ("_tiers.png" if tiers else ".png"))
        sheet([shots[:2], shots[2:]], name)
        print("SHEET", name)


if __name__ == "__main__":
    main()
