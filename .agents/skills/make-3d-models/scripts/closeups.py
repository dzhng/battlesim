"""Close-ups of one or more GLBs in ONE Blender session: every shot shaded
(textures, cavity) and as a matcap (shape only), one PNG each.

    blender -b --factory-startup --python closeups.py -- <out_dir> <model.glb>... [--shot name:ex,ey,ez:tx,ty,tz[:lens]]...
        [--matcap-only] [--res 1000x640]

Without --shot it frames six views from the visible model's bounds: both
three-quarters from above, a low side, the belly, a close top of the front
third, and the tail from behind. Eye and target in a --shot are fractions of
the bounds (0 = min, 1 = max per axis), so one shot list fits every model.
Only tier 0 (objects named *LOD0, or a file without tiers) is drawn.
Writes <out_dir>/<model>-<shot>-<shaded|matcap>.png."""
import math
import os
import sys

import bpy
from mathutils import Vector

a = [v for v in sys.argv[sys.argv.index("--") + 1:] if v != "--"]  # the asset CLI adds its own separator
out_dir, models, shots, matcap_only, res = a[0], [], [], False, (1000, 640)
i = 1
while i < len(a):
    if a[i] == "--shot":
        shots.append(a[i + 1]); i += 2
    elif a[i] == "--matcap-only":
        matcap_only = True; i += 1
    elif a[i] == "--res":
        res = tuple(int(v) for v in a[i + 1].split("x")); i += 2
    else:
        models.append(os.path.abspath(a[i])); i += 1
os.makedirs(out_dir, exist_ok=True)

AUTO = {
    "front-quarter": ("1.15,1.25,1.4", "0.65,0.5,0.45", 35),
    "rear-quarter": ("-0.15,-0.25,1.4", "0.35,0.5,0.45", 35),
    "side-low": ("0.5,2.2,0.3", "0.5,0.5,0.45", 35),
    "belly": ("0.75,0.9,-0.9", "0.5,0.5,0.4", 35),
    "top-front": ("0.85,0.5,1.9", "0.75,0.5,0.6", 50),
    "tail": ("-1.1,0.5,0.6", "0.1,0.5,0.5", 50),
}
plan = []
for s in shots:
    p = s.split(":")
    plan.append((p[0], p[1], p[2], float(p[3]) if len(p) > 3 else 50))
if not plan:
    plan = [(k, *v) for k, v in AUTO.items()]


def frac(spec, lo, hi):
    f = [float(v) for v in spec.split(",")]
    return Vector([lo[k] + (hi[k] - lo[k]) * f[k] for k in range(3)])


sc = bpy.context.scene
for glb in models:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    sc = bpy.context.scene
    bpy.ops.import_scene.gltf(filepath=glb)
    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    tiered = any(o.name.endswith("LOD0") for o in meshes)
    shown = []
    for o in meshes:
        root = o
        while root.parent and not root.name.endswith(("LOD0", "LOD1", "LOD2", "LOD3")):
            root = root.parent
        keep = not tiered or root.name.endswith("LOD0")
        o.hide_render = not keep
        if keep:
            shown.append(o)
    vs = [o.matrix_world @ v.co for o in shown for v in o.data.vertices]
    lo = Vector([min(v[k] for v in vs) for k in range(3)])
    hi = Vector([max(v[k] for v in vs) for k in range(3)])
    # Keep the bounds' cube so a flat model's shots don't collapse onto it.
    size = max(hi - lo)
    mid = (lo + hi) / 2
    lo2 = Vector([min(lo[k], mid[k] - size / 2) for k in range(3)])
    hi2 = Vector([max(hi[k], mid[k] + size / 2) for k in range(3)])
    sc.render.engine = "BLENDER_WORKBENCH"
    sc.view_settings.view_transform = "Standard"
    sc.render.resolution_x, sc.render.resolution_y = res
    sc.world = bpy.data.worlds.new("w")
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    sc.collection.objects.link(cam)
    sc.camera = cam
    cam.data.clip_start, cam.data.clip_end = size * 0.005, size * 20
    name = os.path.splitext(os.path.basename(glb))[0]
    for shot, eye, tgt, lens in plan:
        e, t = frac(eye, lo2, hi2), frac(tgt, lo, hi)
        cam.location = e
        cam.rotation_euler = (t - e).to_track_quat("-Z", "Y").to_euler()
        cam.data.lens = lens
        sh = sc.display.shading
        for mode in (("matcap",) if matcap_only else ("shaded", "matcap")):
            if mode == "shaded":
                sh.light, sh.color_type, sh.show_cavity = "STUDIO", "TEXTURE", True
                sc.world.color = (0.75, 0.75, 0.72)
            else:
                sh.light, sh.studio_light, sh.color_type, sh.show_cavity = "MATCAP", "metal_carpaint.exr", "SINGLE", False
                sh.single_color = (0.8, 0.8, 0.8)
                sc.world.color = (0.05, 0.05, 0.05)
            sc.render.filepath = os.path.join(os.path.abspath(out_dir), f"{name}-{shot}-{mode}.png")
            bpy.ops.render.render(write_still=True)
            print("CLOSEUP", sc.render.filepath)
