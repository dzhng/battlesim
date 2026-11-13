"""Render where a GLB's paint is worn: tier 0, the armor_paint vertex
alpha (wear) as grey, every other material near black, flat-lit, from
metre-space shots. usage: wear_mask.py -- <out_dir> <glb>... --shot name:ex,ey,ez:tx,ty,tz:lens --res WxH"""
import os, sys
import bpy
from mathutils import Vector
a = [v for v in sys.argv[sys.argv.index("--") + 1:] if v != "--"]
out_dir, models, shots, res = a[0], [], [], (1920, 1080)
i = 1
while i < len(a):
    if a[i] == "--shot": shots.append(a[i + 1]); i += 2
    elif a[i] == "--res": res = tuple(int(v) for v in a[i + 1].split("x")); i += 2
    else: models.append(a[i]); i += 1
os.makedirs(out_dir, exist_ok=True)
for glb in models:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    sc = bpy.context.scene
    bpy.ops.import_scene.gltf(filepath=glb)
    for o in bpy.data.objects:
        if o.type != "MESH":
            continue
        root = o
        while root.parent and not root.name.endswith(("LOD0", "LOD1", "LOD2", "LOD3")):
            root = root.parent
        o.hide_render = not root.name.endswith("LOD0")
        me = o.data
        paint = any(m and m.name.startswith("armor_paint") for m in me.materials)
        attr = me.color_attributes.get("Color") or (me.color_attributes[0] if me.color_attributes else None)
        if attr is None:
            attr = me.color_attributes.new("Color", "FLOAT_COLOR", "POINT")
        for d in attr.data:
            w = d.color[3] if paint else 0.0
            d.color = (w, w, w, 1.0) if paint else (0.03, 0.03, 0.03, 1.0)
        me.color_attributes.active_color = attr
    sc.render.engine = "BLENDER_WORKBENCH"
    sc.view_settings.view_transform = "Standard"
    sc.render.resolution_x, sc.render.resolution_y = res
    sc.world = bpy.data.worlds.new("w"); sc.world.color = (0.2, 0.25, 0.3)
    sh = sc.display.shading
    sh.light, sh.color_type, sh.show_cavity = "FLAT", "VERTEX", False
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam")); sc.collection.objects.link(cam); sc.camera = cam
    cam.data.clip_start, cam.data.clip_end = 0.05, 200
    name = os.path.splitext(os.path.basename(glb))[0]
    for s in shots:
        n, e, t, lens = s.split(":")
        e = Vector([float(v.rstrip("m")) for v in e.split(",")]); t = Vector([float(v.rstrip("m")) for v in t.split(",")])
        cam.location = e; cam.rotation_euler = (t - e).to_track_quat("-Z", "Y").to_euler(); cam.data.lens = float(lens)
        sc.render.filepath = os.path.join(os.path.abspath(out_dir), f"{name}-{n}-wear.png")
        bpy.ops.render.render(write_still=True)
