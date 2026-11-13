"""Slice 04 probe: render one exported vehicle the way the battle camera sees
it at its closest zoom, and at a low cinematic three-quarter, in EEVEE with
the vertex colour (paint, wear, occlusion) multiplying the tiled albedo, as
the game's model shader does.

    bun run --cwd web asset -- blender <abs>/render_seams.py -- <out_dir> <label> <low.glb> <view>
        [--bake <high.glb> --parts <prefix,...> --density <px per m> --ao-distance <m>]

<view> is t72 or leopard (which crops to cut). With --bake, the named tier-0
parts of <low.glb> get a unique UV and a normal and an AO map baked in
Cycles from the same parts of <high.glb> (the grooved export), which their
own material copy then wears: the per-model detail map candidate.
Camera: fixtures/presentation.json camera (fov_y 0.8, zoom_min 25 m at
pitch 0.22 rad), 1920x1080. Writes <label>-<shot>.png and crops
<label>-<shot>-<crop>.png (native) and -x4.png (nearest, x4)."""
import json
import math
import os
import sys

import bmesh  # noqa: F401
import bpy
import numpy as np
from bpy_extras.object_utils import world_to_camera_view
from mathutils import Vector

a = [v for v in sys.argv[sys.argv.index("--") + 1:] if v != "--"]
out_dir, label, low_glb, view = a[0], a[1], os.path.abspath(a[2]), a[3]
opts = {}
i = 4
while i < len(a):
    opts[a[i][2:]] = a[i + 1]
    i += 2
os.makedirs(out_dir, exist_ok=True)
FOV_Y, ZOOM_MIN, PITCH = 0.8, 25.0, 0.22
RES = (1920, 1080)
YAW = math.radians(35)  # front-left quarter: glacis, left side, left cheek
CROPS = {
    "t72": {"glacis": ["glacis_era_", "hull_upper_fender_"], "side": ["side_era_L_"]},
    "leopard": {"cheek": ["wedge_L"]},
}[view]

bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene


def tier0(o):
    r = o
    while r is not None:
        if r.name.endswith(("LOD1", "LOD2", "LOD3")):
            return False
        if r.name.endswith("LOD0"):
            return True
        r = r.parent
    return True


def imported(path, prefix=""):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    new = [o for o in bpy.data.objects if o not in before]
    for o in new:
        base = o.name.rsplit(".", 1)[0] if o.name[-4:-3] == "." and o.name[-3:].isdigit() else o.name
        o.name = prefix + base
        if o.type == "MESH" and not tier0(o):
            bpy.data.objects.remove(o, do_unlink=True)
    return [o for o in bpy.data.objects if o.name.startswith(prefix)] if prefix else \
        [o for o in bpy.data.objects if not o.name.startswith("HI_")]


def vertex_colour_materials(objs):
    """The glTF importer already draws albedo x vertex colour; scale the
    vertex colour by the material's `colour_scale` (glTF extras), as the
    game's model shader does (models_material.wgsl: color * factors.w)."""
    done = set()
    for o in objs:
        if o.type != "MESH" or not o.data.color_attributes:
            continue
        for mat in o.data.materials:
            if mat is None or mat.name in done or not mat.use_nodes:
                continue
            done.add(mat.name)
            s = float(mat.get("colour_scale", 1.0))
            nt = mat.node_tree
            for col in [n for n in nt.nodes if n.type == "VERTEX_COLOR"]:
                for link in list(col.outputs["Color"].links):
                    mul = nt.nodes.new("ShaderNodeMix")
                    mul.data_type, mul.blend_type = "RGBA", "MULTIPLY"
                    mul.inputs[0].default_value = 1.0
                    mul.inputs[7].default_value = (s, s, s, 1.0)
                    nt.links.new(col.outputs["Color"], mul.inputs[6])
                    nt.links.new(mul.outputs[2], link.to_socket)
            REPORT_SCALES[mat.name] = s


REPORT_SCALES = {}


if low_glb.endswith(".blend"):  # a scene this script baked and saved (--save)
    bpy.ops.wm.open_mainfile(filepath=low_glb)
    sc = bpy.context.scene
    low = list(bpy.data.objects)
else:
    low = imported(low_glb)
    vertex_colour_materials(low)
meshes = [o for o in low if o.type == "MESH"]
REPORT = {"label": label, "colour_scale": REPORT_SCALES}


def parts(prefixes, objs):
    return [o for o in objs if o.type == "MESH" and any(o.name.startswith(p) for p in prefixes)
            and o.name.endswith("LOD0") and "_bolt_" not in o.name]


def bake(high_glb, prefixes, density, ao_distance):
    high = imported(high_glb, "HI_")
    targets = parts(prefixes, low)
    sc.render.engine = "CYCLES"
    sc.cycles.device = "CPU"
    sc.cycles.samples = 32
    sc.world = sc.world or bpy.data.worlds.new("w")
    sc.world.light_settings.distance = ao_distance
    bk = sc.render.bake
    bk.use_selected_to_active = True
    bk.cage_extrusion = 0.02
    bk.max_ray_distance = 0.06
    bk.margin = 4
    total_px = 0
    area = 0.0
    for o in targets:
        hi = bpy.data.objects.get("HI_" + o.name)
        if hi is None:
            raise SystemExit(f"no high part for {o.name}")
        me = o.data
        a_ = sum(p.area for p in me.polygons) * (o.matrix_world.to_scale()[0] ** 2)
        area += a_
        size = 1 << max(5, math.ceil(math.log2(math.sqrt(a_ / 0.6) * density)))
        total_px += size * size
        uv = me.uv_layers.new(name="bake")
        me.uv_layers.active = uv
        bpy.ops.object.select_all(action="DESELECT")
        o.select_set(True)
        bpy.context.view_layer.objects.active = o
        with bpy.context.temp_override(active_object=o, object=o, selected_objects=[o], selected_editable_objects=[o]):
            bpy.ops.object.mode_set(mode="EDIT")
            bpy.ops.mesh.select_all(action="SELECT")
            bpy.ops.uv.smart_project(angle_limit=math.radians(66), island_margin=0.01)
            bpy.ops.object.mode_set(mode="OBJECT")
        mat = o.data.materials[0].copy()
        o.data.materials[0] = mat
        nt = mat.node_tree
        imgs = {}
        for kind in ("NORMAL", "AO"):
            img = bpy.data.images.new(f"{o.name}_{kind}", size, size, float_buffer=False, is_data=kind == "NORMAL")
            if kind == "AO":
                img.colorspace_settings.name = "Non-Color"
            node = nt.nodes.new("ShaderNodeTexImage")
            node.image = img
            uvn = nt.nodes.new("ShaderNodeUVMap")
            uvn.uv_map = "bake"
            nt.links.new(uvn.outputs[0], node.inputs[0])
            imgs[kind] = node
        # the low part must not shade the high one's occlusion
        for attr in ("visible_diffuse", "visible_glossy", "visible_shadow", "visible_transmission"):
            setattr(o, attr, False)
        for kind in ("NORMAL", "AO"):
            for n in nt.nodes:
                n.select = False
            nt.nodes.active = imgs[kind]
            imgs[kind].select = True
            bpy.ops.object.select_all(action="DESELECT")
            hi.select_set(True)
            o.select_set(True)
            bpy.context.view_layer.objects.active = o
            bpy.ops.object.bake(type=kind, use_selected_to_active=True, cage_extrusion=0.02, max_ray_distance=0.06,
                                margin=4, normal_space="TANGENT", uv_layer="bake")
            path = os.path.abspath(os.path.join(out_dir, "bake", label, f"{o.name}_{kind}.png"))
            os.makedirs(os.path.dirname(path), exist_ok=True)
            imgs[kind].image.filepath_raw = path
            imgs[kind].image.file_format = "PNG"
            imgs[kind].image.save()
        for attr in ("visible_diffuse", "visible_glossy", "visible_shadow", "visible_transmission"):
            setattr(o, attr, True)
        bsdf = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
        nmap = nt.nodes.new("ShaderNodeNormalMap")
        nmap.uv_map = "bake"
        nt.links.new(imgs["NORMAL"].outputs["Color"], nmap.inputs["Color"])
        old = bsdf.inputs["Normal"].links[0].from_node if bsdf.inputs["Normal"].links else None
        nt.links.new(nmap.outputs["Normal"], bsdf.inputs["Normal"])
        if old is not None:
            REPORT.setdefault("replaced_tiled_normal", []).append(o.name)
        mix = nt.nodes.new("ShaderNodeMix")
        mix.data_type, mix.blend_type = "RGBA", "MULTIPLY"
        mix.inputs[0].default_value = 1.0
        sock = bsdf.inputs["Base Color"]
        nt.links.new(sock.links[0].from_socket, mix.inputs[6])
        nt.links.new(imgs["AO"].outputs["Color"], mix.inputs[7])
        nt.links.new(mix.outputs[2], sock)
    for o in [o for o in bpy.data.objects if o.name.startswith("HI_")]:
        bpy.data.objects.remove(o, do_unlink=True)
    pngs = sum(os.path.getsize(os.path.join(out_dir, "bake", label, f)) for f in os.listdir(os.path.join(out_dir, "bake", label))
               if f.startswith(tuple(t.name for t in targets)))
    REPORT["bake"] = {"parts": len(targets), "area_m2": round(area, 2), "texels": total_px, "png_bytes": pngs,
                      "density_px_per_m": density}


if "bake" in opts:
    bake(os.path.abspath(opts["bake"]), opts["parts"].split(","), float(opts.get("density", 400)),
         float(opts.get("ao-distance", 0.1)))
if "save" in opts:  # bake on the CPU now, render later under the GPU lock
    for img in bpy.data.images:
        if img.filepath_raw and img.is_dirty is False:
            img.filepath = img.filepath_raw
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(opts["save"]))
    print("REPORT", json.dumps(REPORT), flush=True)
    sys.exit(0)

# every paint surface at tier 0, for the cost of a whole-model map
REPORT["paint_area_m2"] = round(sum(sum(p.area for p in o.data.polygons) for o in meshes if o.data.materials and
                                    o.data.materials[0] and "paint" in o.data.materials[0].name), 1)

# ---- scene
vs = [o.matrix_world @ v.co for o in meshes for v in o.data.vertices]
lo = Vector([min(v[k] for v in vs) for k in range(3)])
hi = Vector([max(v[k] for v in vs) for k in range(3)])
ground = bpy.data.meshes.new("ground")
ground.from_pydata([(-80, -80, 0), (80, -80, 0), (80, 80, 0), (-80, 80, 0)], [], [(0, 1, 2, 3)])
g = bpy.data.objects.new("ground", ground)
sc.collection.objects.link(g)
gm = bpy.data.materials.new("ground")
gm.use_nodes = True
gm.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.16, 0.17, 0.12, 1)
gm.node_tree.nodes["Principled BSDF"].inputs["Roughness"].default_value = 0.95
ground.materials.append(gm)
sc.world = sc.world or bpy.data.worlds.new("w")
sc.world.use_nodes = True
bg = sc.world.node_tree.nodes["Background"]
bg.inputs["Color"].default_value = (0.55, 0.62, 0.75, 1)
bg.inputs["Strength"].default_value = 0.9
sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
sun.data.energy = 4.0
sun.data.angle = math.radians(1.0)
sc.collection.objects.link(sun)
# sun at the fixture's 0.7 rad elevation, 70 degrees left of the camera's bearing
az = YAW + math.radians(70)
d = Vector((math.cos(0.7) * math.cos(az), math.cos(0.7) * math.sin(az), math.sin(0.7)))
sun.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
sc.render.engine = "BLENDER_EEVEE"
sc.eevee.taa_render_samples = 64
sc.view_settings.view_transform = "Standard"
sc.render.resolution_x, sc.render.resolution_y = RES
sc.render.resolution_percentage = 100
cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
sc.collection.objects.link(cam)
sc.camera = cam
cam.data.sensor_fit = "VERTICAL"
cam.data.clip_start, cam.data.clip_end = 0.5, 400

target = Vector(((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, 1.0))


def look(eye, tgt, fov=None, lens=None):
    cam.location = eye
    cam.rotation_euler = (tgt - eye).to_track_quat("-Z", "Y").to_euler()
    if fov:
        cam.data.lens_unit = "FOV"
        cam.data.angle = fov
    else:
        cam.data.lens_unit = "MILLIMETERS"
        cam.data.lens = lens


SHOTS = {
    "battle_zoom": (target + ZOOM_MIN * Vector((math.cos(PITCH) * math.cos(YAW), math.cos(PITCH) * math.sin(YAW),
                                                 math.sin(PITCH))), target, FOV_Y, None),
    # a low three-quarter front at 1.5 m eye height, 10 m out, a long lens
    "cinematic": (Vector((target.x + 10 * math.cos(YAW), target.y + 10 * math.sin(YAW), 1.5)),
                  Vector((target.x + 1.0, target.y, 1.1)) if view == "t72" else Vector((target.x + 0.6, target.y + 1.0, 2.0)),
                  None, 60),
}


def crop_box(prefixes):
    objs = parts(prefixes, low)
    pts = [o.matrix_world @ Vector(c) for o in objs for c in o.bound_box]
    xs, ys = [], []
    for p in pts:
        c = world_to_camera_view(sc, cam, p)
        xs.append(c.x * RES[0])
        ys.append((1 - c.y) * RES[1])
    pad = 0.12 * max(max(xs) - min(xs), max(ys) - min(ys))
    x0, x1 = max(0, int(min(xs) - pad)), min(RES[0], int(max(xs) + pad))
    y0, y1 = max(0, int(min(ys) - pad)), min(RES[1], int(max(ys) + pad))
    return x0, y0, x1, y1


def save(arr, path):
    h, w = arr.shape[:2]
    img = bpy.data.images.new(os.path.basename(path), w, h, alpha=True)
    img.pixels.foreach_set(np.flipud(arr).astype(np.float32).ravel())
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    bpy.data.images.remove(img)


boxes = {}
for shot, (eye, tgt, fov, lens) in SHOTS.items():
    look(eye, tgt, fov, lens)
    path = os.path.abspath(os.path.join(out_dir, f"{label}-{shot}.png"))
    sc.render.filepath = path
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(path)
    arr = np.flipud(np.array(img.pixels[:], dtype=np.float32).reshape(RES[1], RES[0], 4))
    bpy.data.images.remove(img)
    for name, prefixes in CROPS.items():
        x0, y0, x1, y1 = boxes.setdefault((shot, name), crop_box(prefixes))
        c = arr[y0:y1, x0:x1]
        save(c, os.path.join(os.path.abspath(out_dir), f"{label}-{shot}-{name}.png"))
        if shot == "battle_zoom":
            save(np.repeat(np.repeat(c, 4, axis=0), 4, axis=1),
                 os.path.join(os.path.abspath(out_dir), f"{label}-{shot}-{name}-x4.png"))
REPORT["crops"] = {f"{s}/{n}": b for (s, n), b in boxes.items()}
print("REPORT", json.dumps(REPORT), flush=True)
