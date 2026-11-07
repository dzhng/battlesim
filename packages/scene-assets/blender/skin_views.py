"""The guide views a generated skin is made from (`aircraft_skin`): a roster
GLB's near tier without its gear, stores, pylons, pods and markings, shaded
on white from its right, left, top and bottom, each cropped to the airframe
with a small margin. Only views whose longest side is the airframe's length
are drawn: the generator scales each view to fill it, so a nose-on view
would read its span as its length.

    blender -b --factory-startup --python skin_views.py -- <model.glb> <out_dir> <length_m>

Writes `<out_dir>/{right,left,top,bottom}.png`.
"""
import os
import sys

import bpy
import numpy as np

SKIP = ("gear_", "store_", "pylon_", "cheek_", "marking_", "litter_")
VIEWS = (("right", (0, -30, 2.2), (1.5708, 0, 0)), ("left", (0, 30, 2.2), (1.5708, 0, 3.14159)),
         ("top", (0, 0, 30), (0, 0, 0)), ("bottom", (0, 0, -30), (3.14159, 0, 0)))


def hide_fittings():
    for o in bpy.data.objects:
        if o.type != "MESH":
            continue
        chain, p = [o], o.parent
        while p is not None:
            chain.append(p)
            p = p.parent
        if any(c.name.startswith(SKIP) for c in chain) or not o.name.endswith("LOD0"):
            o.hide_render = True


def crop(path):
    """Crop the image at `path` to its subject on the white ground, a 3% margin round it."""
    img = bpy.data.images.load(path)
    w, h = img.size
    px = np.array(img.pixels[:]).reshape(h, w, 4)
    bpy.data.images.remove(img)
    ys, xs = np.where(np.abs(px[..., :3] - 1.0).sum(-1) > 0.12)
    y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
    m = int(max(y1 - y0, x1 - x0) * 0.03)
    out = np.ones((y1 - y0 + 2 * m, x1 - x0 + 2 * m, 4))
    out[m:m + y1 - y0, m:m + x1 - x0] = px[y0:y1, x0:x1]
    o = bpy.data.images.new("crop", out.shape[1], out.shape[0])
    o.pixels = out.ravel()
    o.filepath_raw, o.file_format = path, "PNG"
    o.save()
    bpy.data.images.remove(o)


def main():
    src, out, length = sys.argv[sys.argv.index("--") + 1:][:3]
    os.makedirs(out, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=src)
    hide_fittings()
    sc = bpy.context.scene
    sc.render.engine = "BLENDER_WORKBENCH"
    sh = sc.display.shading
    sh.light, sh.studio_light = "STUDIO", "rim.sl"
    sh.color_type, sh.single_color = "SINGLE", (0.62, 0.64, 0.66)
    sh.show_cavity, sh.cavity_type, sh.show_shadows = True, "BOTH", False
    sc.world = bpy.data.worlds.new("white")
    sc.world.color = (1, 1, 1)
    sc.view_settings.view_transform = "Standard"
    sc.render.resolution_x = sc.render.resolution_y = 1024
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    sc.collection.objects.link(cam)
    sc.camera = cam
    cam.data.type, cam.data.ortho_scale = "ORTHO", float(length) * 1.12
    for name, loc, rot in VIEWS:
        cam.location, cam.rotation_euler = loc, rot
        sc.render.filepath = os.path.join(out, f"{name}.png")
        bpy.ops.render.render(write_still=True)
        crop(sc.render.filepath)


if __name__ == "__main__":
    main()
