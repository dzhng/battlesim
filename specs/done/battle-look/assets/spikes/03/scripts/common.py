"""Shared Blender helpers for spike 03 (throwaway)."""
import bpy, bmesh, math, os, subprocess
from mathutils import Vector, Matrix, Euler

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "out")
os.makedirs(OUT, exist_ok=True)


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scn = bpy.context.scene
    scn.unit_settings.system = "METRIC"
    return scn


def mat(name, base=(0.5, 0.5, 0.5), rough=0.6, metal=0.0):
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (*base, 1)
    b.inputs["Roughness"].default_value = rough
    b.inputs["Metallic"].default_value = metal
    return m


def link(nt, a, b):
    nt.links.new(a, b)


def camo_mat(name, colors, scale=0.55, wear=0.0, dirt=0.0, rough=0.7, seed=0.0, coord="Object"):
    """Three/four-tone disruptive camo from noise thresholds, optional edge wear (Cycles bevel) and
    ground dirt by world height."""
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    N = nt.nodes
    bsdf = N["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = rough
    tc = N.new("ShaderNodeTexCoord")
    mp = N.new("ShaderNodeMapping")
    mp.inputs["Location"].default_value = (seed, seed * 0.7, seed * 1.3)
    link(nt, tc.outputs[coord], mp.inputs["Vector"])
    noise = N.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = scale
    noise.inputs["Detail"].default_value = 3.0
    noise.inputs["Roughness"].default_value = 0.55
    link(nt, mp.outputs["Vector"], noise.inputs["Vector"])
    noise2 = N.new("ShaderNodeTexNoise")
    noise2.inputs["Scale"].default_value = scale * 1.7
    noise2.inputs["Detail"].default_value = 2.0
    mp2 = N.new("ShaderNodeMapping")
    mp2.inputs["Location"].default_value = (7.3 + seed, 1.1, 3.7)
    link(nt, tc.outputs[coord], mp2.inputs["Vector"])
    link(nt, mp2.outputs["Vector"], noise2.inputs["Vector"])
    ramp = N.new("ShaderNodeValToRGB")
    ramp.color_ramp.interpolation = "CONSTANT"
    els = ramp.color_ramp.elements
    els[0].position = 0.0
    els[0].color = (*colors[0], 1)
    els[1].position = 0.5
    els[1].color = (*colors[1], 1)
    if len(colors) > 2:
        e = els.new(0.62)
        e.color = (*colors[2], 1)
    link(nt, noise.outputs["Fac"], ramp.inputs["Fac"])
    col = ramp.outputs["Color"]
    if len(colors) > 3:
        r2 = N.new("ShaderNodeValToRGB")
        r2.color_ramp.interpolation = "CONSTANT"
        r2.color_ramp.elements[0].position = 0
        r2.color_ramp.elements[0].color = (0, 0, 0, 1)
        r2.color_ramp.elements[1].position = 0.66
        r2.color_ramp.elements[1].color = (1, 1, 1, 1)
        link(nt, noise2.outputs["Fac"], r2.inputs["Fac"])
        mix = N.new("ShaderNodeMix")
        mix.data_type = "RGBA"
        link(nt, r2.outputs["Color"], mix.inputs["Factor"])
        link(nt, col, mix.inputs[6])
        mix.inputs[7].default_value = (*colors[3], 1)
        col = mix.outputs[2]
    if dirt > 0:
        # dirt rises from the ground: world z gradient + noise
        sep = N.new("ShaderNodeSeparateXYZ")
        link(nt, tc.outputs["Object"] if coord == "Object" else tc.outputs[coord], sep.inputs[0])
        geo = N.new("ShaderNodeNewGeometry")
        sepw = N.new("ShaderNodeSeparateXYZ")
        link(nt, geo.outputs["Position"], sepw.inputs[0])
        mr = N.new("ShaderNodeMapRange")
        mr.inputs["From Min"].default_value = 0.2
        mr.inputs["From Max"].default_value = 1.3
        mr.inputs["To Min"].default_value = dirt
        mr.inputs["To Max"].default_value = 0.0
        link(nt, sepw.outputs["Z"], mr.inputs["Value"])
        dn = N.new("ShaderNodeTexNoise")
        dn.inputs["Scale"].default_value = 6.0
        link(nt, tc.outputs["Object"], dn.inputs["Vector"])
        mul = N.new("ShaderNodeMath")
        mul.operation = "MULTIPLY"
        link(nt, mr.outputs["Result"], mul.inputs[0])
        link(nt, dn.outputs["Fac"], mul.inputs[1])
        mul2 = N.new("ShaderNodeMath")
        mul2.operation = "MULTIPLY"
        mul2.inputs[1].default_value = 1.8
        mul2.use_clamp = True
        link(nt, mul.outputs[0], mul2.inputs[0])
        mixd = N.new("ShaderNodeMix")
        mixd.data_type = "RGBA"
        link(nt, mul2.outputs[0], mixd.inputs["Factor"])
        link(nt, col, mixd.inputs[6])
        mixd.inputs[7].default_value = (0.23, 0.19, 0.14, 1)
        col = mixd.outputs[2]
    if wear > 0:
        bev = N.new("ShaderNodeBevel")
        bev.inputs["Radius"].default_value = 0.025
        geo2 = N.new("ShaderNodeNewGeometry")
        dp = N.new("ShaderNodeVectorMath")
        dp.operation = "DOT_PRODUCT"
        link(nt, bev.outputs["Normal"], dp.inputs[0])
        link(nt, geo2.outputs["Normal"], dp.inputs[1])
        inv = N.new("ShaderNodeMapRange")
        inv.inputs["From Min"].default_value = 0.985
        inv.inputs["From Max"].default_value = 0.9
        link(nt, dp.outputs["Value"], inv.inputs["Value"])
        wn = N.new("ShaderNodeTexNoise")
        wn.inputs["Scale"].default_value = 18
        link(nt, tc.outputs["Object"], wn.inputs["Vector"])
        mm = N.new("ShaderNodeMath")
        mm.operation = "MULTIPLY"
        link(nt, inv.outputs["Result"], mm.inputs[0])
        link(nt, wn.outputs["Fac"], mm.inputs[1])
        mm2 = N.new("ShaderNodeMath")
        mm2.operation = "MULTIPLY"
        mm2.use_clamp = True
        mm2.inputs[1].default_value = wear * 2.5
        link(nt, mm.outputs[0], mm2.inputs[0])
        mixw = N.new("ShaderNodeMix")
        mixw.data_type = "RGBA"
        link(nt, mm2.outputs[0], mixw.inputs["Factor"])
        link(nt, col, mixw.inputs[6])
        mixw.inputs[7].default_value = (0.42, 0.40, 0.36, 1)
        col = mixw.outputs[2]
    link(nt, col, bsdf.inputs["Base Color"])
    return m


def obj_from_bm(name, bm, mats=None, parent=None):
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    o = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(o)
    if mats:
        for m in mats:
            me.materials.append(m)
    if parent:
        o.parent = parent
    return o


def box(name, size, loc=(0, 0, 0), mat_=None, parent=None, bevel=0.0, rot=(0, 0, 0)):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * size[0], v.co.y * size[1], v.co.z * size[2]))
    o = obj_from_bm(name, bm, [mat_] if mat_ else None, parent)
    o.location = loc
    o.rotation_euler = rot
    if bevel > 0:
        bv = o.modifiers.new("bevel", "BEVEL")
        bv.width = bevel
        bv.segments = 2
        bv.limit_method = "ANGLE"
    return o


def cyl(name, r, depth, loc=(0, 0, 0), axis="Z", mat_=None, parent=None, seg=24, bevel=0.0, r2=None):
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r, radius2=r if r2 is None else r2, depth=depth)
    rot = {"Z": Matrix.Identity(4), "X": Matrix.Rotation(math.pi / 2, 4, "Y"), "Y": Matrix.Rotation(math.pi / 2, 4, "X")}[axis]
    bmesh.ops.transform(bm, matrix=rot, verts=bm.verts)
    for f in bm.faces:
        f.smooth = True
    o = obj_from_bm(name, bm, [mat_] if mat_ else None, parent)
    o.location = loc
    o.data.shade_smooth()
    if bevel > 0:
        bv = o.modifiers.new("bevel", "BEVEL")
        bv.width = bevel
        bv.segments = 2
        bv.limit_method = "ANGLE"
        bv.angle_limit = math.radians(50)
    return o


def prism(name, profile_xz, width, loc=(0, 0, 0), mat_=None, parent=None, bevel=0.0, taper=None):
    """Extrude an XZ profile polygon across Y (centered). taper: (top_width) scales verts with z above profile min."""
    bm = bmesh.new()
    hw = width / 2
    a = [bm.verts.new((x, -hw, z)) for x, z in profile_xz]
    b = [bm.verts.new((x, hw, z)) for x, z in profile_xz]
    n = len(profile_xz)
    bm.faces.new(a[::-1])
    bm.faces.new(b)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((a[i], a[j], b[j], b[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    o = obj_from_bm(name, bm, [mat_] if mat_ else None, parent)
    o.location = loc
    if bevel > 0:
        bv = o.modifiers.new("bevel", "BEVEL")
        bv.width = bevel
        bv.segments = 2
        bv.limit_method = "ANGLE"
    return o


def empty(name, loc=(0, 0, 0), parent=None, size=0.1):
    e = bpy.data.objects.new(name, None)
    e.empty_display_size = size
    bpy.context.scene.collection.objects.link(e)
    if parent:
        e.parent = parent
    e.location = loc
    return e


def setup_render(res=(640, 640), engine="CYCLES", samples=48, film_transparent=False):
    scn = bpy.context.scene
    scn.render.engine = engine
    scn.render.resolution_x, scn.render.resolution_y = res
    scn.render.resolution_percentage = 100
    scn.render.film_transparent = film_transparent
    if engine == "CYCLES":
        prefs = bpy.context.preferences.addons["cycles"].preferences
        try:
            prefs.compute_device_type = "METAL"
            prefs.get_devices()
            for d in prefs.devices:
                d.use = True
            scn.cycles.device = "GPU"
        except Exception as e:
            print("cycles gpu fail", e)
        scn.cycles.samples = samples
        scn.cycles.use_denoising = True
    scn.view_settings.view_transform = "AgX"
    scn.view_settings.look = "AgX - Medium High Contrast"
    scn.view_settings.exposure = -0.4
    return scn


def light_rig(sun_dir=(-0.35, 0.62), strength=4.0, sky=0.6, neutral=False):
    """Warm afternoon sun + sky; sun_dir = (azimuth rad, elevation rad)."""
    scn = bpy.context.scene
    w = bpy.data.worlds.new("world")
    scn.world = w
    w.use_nodes = True
    nt = w.node_tree
    bg = nt.nodes["Background"]
    sky_n = nt.nodes.new("ShaderNodeTexSky")
    try:
        sky_n.sky_type = "NISHITA"
    except Exception:
        pass
    try:
        sky_n.sun_elevation = sun_dir[1]
        sky_n.sun_rotation = sun_dir[0]
        sky_n.sun_disc = False
    except Exception:
        pass
    if neutral:
        bg.inputs[0].default_value = (0.55, 0.60, 0.68, 1)
    else:
        nt.links.new(sky_n.outputs[0], bg.inputs[0])
    bg.inputs[1].default_value = sky
    sd = bpy.data.lights.new("sun", "SUN")
    sd.energy = strength
    sd.color = (1.0, 0.93, 0.82)
    sd.angle = math.radians(1.5)
    so = bpy.data.objects.new("sun", sd)
    scn.collection.objects.link(so)
    az, el = sun_dir
    d = Vector((math.cos(el) * math.cos(az), math.cos(el) * math.sin(az), math.sin(el)))
    so.rotation_euler = d.to_track_quat("Z", "Y").to_euler()
    return so


def ground(size=60, color=(0.33, 0.36, 0.22)):
    bm = bmesh.new()
    bmesh.ops.create_grid(bm, x_segments=1, y_segments=1, size=size / 2)
    m = bpy.data.materials.new("ground")
    m.use_nodes = True
    nt = m.node_tree
    b = nt.nodes["Principled BSDF"]
    b.inputs["Roughness"].default_value = 0.95
    tc = nt.nodes.new("ShaderNodeTexCoord")
    n = nt.nodes.new("ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = 1.5
    link(nt, tc.outputs["Object"], n.inputs["Vector"])
    r = nt.nodes.new("ShaderNodeValToRGB")
    r.color_ramp.elements[0].color = (color[0] * 0.8, color[1] * 0.8, color[2] * 0.8, 1)
    r.color_ramp.elements[1].color = (color[0] * 1.15, color[1] * 1.1, color[2] * 0.9, 1)
    link(nt, n.outputs["Fac"], r.inputs["Fac"])
    link(nt, r.outputs["Color"], b.inputs["Base Color"])
    return obj_from_bm("ground", bm, [m])


def camera(name="cam", lens=50):
    cd = bpy.data.cameras.new(name)
    cd.lens = lens
    cd.clip_start = 0.05
    cd.clip_end = 500
    co = bpy.data.objects.new(name, cd)
    bpy.context.scene.collection.objects.link(co)
    bpy.context.scene.camera = co
    return co


def aim(cam, target, yaw_deg, pitch_deg, dist):
    """Place camera around target. yaw 0 = looking from +X (front of an +X-forward model)."""
    y, p = math.radians(yaw_deg), math.radians(pitch_deg)
    t = Vector(target)
    pos = t + Vector((math.cos(p) * math.cos(y), math.cos(p) * math.sin(y), math.sin(p))) * dist
    cam.location = pos
    cam.rotation_euler = (t - pos).to_track_quat("-Z", "Y").to_euler()


VIEWS8 = [
    ("front", 0, 8),
    ("q-front", 35, 20),
    ("left", 90, 8),
    ("q-rear", 145, 20),
    ("rear", 180, 8),
    ("right", -90, 8),
    ("top", 0, 89),
    ("battle", 40, 49),
]


def render_to(path):
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)


def tile(pattern_glob_list, out, cols, labels=None):
    """ffmpeg tile a list of equally sized PNGs."""
    lst = os.path.join(OUT, "_tile_list.txt")
    args = ["ffmpeg", "-y", "-loglevel", "error"]
    for p in pattern_glob_list:
        args += ["-i", p]
    n = len(pattern_glob_list)
    rows = (n + cols - 1) // cols
    inputs = "".join(f"[{i}:v]" for i in range(n))
    args += ["-filter_complex", f"{inputs}xstack=inputs={n}:layout=" + "|".join(
        f"{(i % cols)}_{(i // cols)}".replace("_", "_") for i in range(n)) + ":fill=black", out]
    # xstack layout needs pixel offsets; compute from first image size
    w, h = bpy.context.scene.render.resolution_x, bpy.context.scene.render.resolution_y
    layout = "|".join(f"{(i % cols) * w}_{(i // cols) * h}" for i in range(n))
    args[-2] = f"{inputs}xstack=inputs={n}:layout={layout}:fill=black"
    subprocess.run(args, check=True)
