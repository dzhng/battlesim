"""Spike 03: a modern MBT scripted in Blender with named, articulated nodes.

Engine basis throughout: Z up, +X forward, origin at the hull centre on the ground (= the simulation's unit position).
Node tree: tank > hull > {turret > {gun > muzzle, hmg > hmg_gun > hmg_muzzle}, wheel_L_*, wheel_R_*, track_L, track_R}
"""
import bpy, bmesh, sys, os, math, json
from mathutils import Vector, Matrix

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import *

ARGS = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
MODE = ARGS[0] if ARGS else "all"
GUN_REACH = float(ARGS[1]) if len(ARGS) > 1 else 5.9  # muzzle distance from the turret axis, metres
TAG = ARGS[2] if len(ARGS) > 2 else ""

scn = reset()
scn.render.fps = 30

# sim authority: physics.tank_half_extents_m [3.5, 1.8, 1.2], tank_muzzle_local_m [3, 0, 2]
TRUNNION = Vector((1.70, 0, 2.0))
TURRET_Z = 1.45
WHEEL_R = 0.33
WHEEL_Z = 0.38
TRACK_W = 0.62
TRACK_Y = 1.44
TRACK_T = 0.07
LINK_PITCH = 0.16

camo = camo_mat("tank_camo", [(0.13, 0.16, 0.085), (0.19, 0.145, 0.085), (0.035, 0.035, 0.03), (0.13, 0.16, 0.085)],
                scale=0.6, wear=0.4, dirt=0.6, rough=0.62, seed=2.0)
dark = mat("running_gear", (0.07, 0.075, 0.06), rough=0.7)
rubber = mat("rubber", (0.025, 0.025, 0.025), rough=0.9)
steel = mat("steel", (0.2, 0.2, 0.19), rough=0.45, metal=0.8)
glass = mat("glass", (0.02, 0.04, 0.05), rough=0.05)
light_m = mat("headlight", (0.6, 0.6, 0.55), rough=0.2)


def nodeobj(name, loc, parent):
    e = empty(name, loc, parent, 0.3)
    return e


tank = empty("tank", (0, 0, 0), size=0.5)
hull = nodeobj("hull", (0, 0, 0), tank)

# ------------------------------------------------------------------ hull
# upper hull / sponsons over the tracks, full width
prism("hull_upper", [(-3.45, 0.98), (3.25, 0.98), (3.50, 1.12), (2.35, 1.45), (-3.30, 1.47), (-3.48, 1.30)], 3.52,
      mat_=camo, parent=hull, bevel=0.035)
# lower hull between the tracks
prism("hull_lower", [(-3.25, 0.46), (2.95, 0.46), (3.42, 1.0), (-3.40, 1.0)], 2.16, mat_=camo, parent=hull, bevel=0.03)
# engine deck grille slats
for i in range(9):
    box(f"deck_slat_{i}", (0.06, 1.6, 0.025), (-3.1 + i * 0.16, 0, 1.475), dark, hull)
box("deck_access", (1.1, 2.2, 0.03), (-1.2, 0, 1.47), camo, hull, bevel=0.01)
# driver's hatch and periscopes
cyl("driver_hatch", 0.32, 0.05, (2.25, 0, 1.47), "Z", camo, hull, bevel=0.01)
for i, y in enumerate((-0.22, 0, 0.22)):
    box(f"driver_periscope_{i}", (0.1, 0.16, 0.07), (2.55, y, 1.43), glass, hull, rot=(0, math.radians(20), 0))
# headlights, tow hooks, rear plate details
for side in (-1, 1):
    box(f"headlight_guard_{side}", (0.14, 0.24, 0.16), (3.30, side * 1.35, 1.25), steel, hull, bevel=0.01)
    cyl(f"headlight_{side}", 0.05, 0.04, (3.36, side * 1.35, 1.25), "X", light_m, hull)
    box(f"tow_hook_{side}", (0.16, 0.08, 0.14), (3.45, side * 0.7, 0.70), steel, hull, bevel=0.01)
    box(f"rear_hook_{side}", (0.14, 0.08, 0.12), (-3.43, side * 0.75, 0.85), steel, hull, bevel=0.01)
    box(f"fender_{side}", (0.55, 0.64, 0.03), (3.28, side * TRACK_Y, 1.0), camo, hull, bevel=0.01, rot=(0, math.radians(-10), 0))
box("rear_grille", (0.05, 1.8, 0.5), (-3.47, 0, 1.1), dark, hull, bevel=0.01)
box("rear_stowage", (0.25, 1.0, 0.25), (-3.45, 0, 1.55), camo, hull, bevel=0.03)

# side skirts: five armour panels a side with bolt rows
for side in (-1, 1):
    y = side * (TRACK_Y + TRACK_W / 2 + 0.035)
    for i in range(6):
        x0 = -3.05 + i * 1.06
        box(f"skirt_{'L' if side > 0 else 'R'}_{i}", (1.03, 0.06, 0.34), (x0 + 0.5, y, 0.88), camo, hull, bevel=0.012)
        for j in range(3):
            cyl(f"skirt_bolt_{side}_{i}_{j}", 0.018, 0.02, (x0 + 0.2 + j * 0.3, y + side * 0.035, 0.98), "Y", steel, hull, seg=8)
    box(f"skirt_front_{side}", (0.5, 0.06, 0.42), (3.35, y, 0.86), camo, hull, bevel=0.012, rot=(0, math.radians(-18), 0))

# ------------------------------------------------------------------ running gear
WHEELS = {}


def wheel(name, x, z, r, side, kind="road"):
    y = side * TRACK_Y
    node = nodeobj(name, (x, y, z), hull)
    w = 0.24 if kind == "road" else 0.30
    cyl(name + "_tyre", r, w, (0, 0, 0), "Y", rubber, node, seg=28)
    cyl(name + "_disc", r * 0.84, w + 0.02, (0, 0, 0), "Y", dark, node, seg=28, bevel=0.01)
    cyl(name + "_hub", r * 0.3, w + 0.08, (0, 0, 0), "Y", steel, node, seg=16)
    # six lightening holes / bolts: make spin readable
    for k in range(6):
        a = k * math.pi / 3
        cyl(f"{name}_hole_{k}", r * 0.12, w + 0.04, (math.cos(a) * r * 0.55, 0, math.sin(a) * r * 0.55), "Y", rubber, node, seg=10)
    if kind == "sprocket":
        for k in range(11):
            a = k * 2 * math.pi / 11
            box(f"{name}_tooth_{k}", (0.07, 0.12, 0.06), (math.cos(a) * (r + 0.02), 0, math.sin(a) * (r + 0.02)), steel, node,
                rot=(0, -a, 0))
    WHEELS[name] = dict(radius=r, x=x, z=z)
    return node


for side, s in ((1, "L"), (-1, "R")):
    for i in range(7):
        wheel(f"wheel_{s}_{i + 1}", -2.55 + i * 0.815, WHEEL_Z, WHEEL_R, side)
    wheel(f"wheel_{s}_sprocket", -3.12, 0.66, 0.33, side, "sprocket")
    wheel(f"wheel_{s}_idler", 3.05, 0.60, 0.30, side, "idler")
    for i in range(2):
        wheel(f"wheel_{s}_return_{i + 1}", -1.4 + i * 2.2, 0.83, 0.08, side, "return")


def hull2d(pts):
    pts = sorted(set(pts))
    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    lo, up = [], []
    for p in pts:
        while len(lo) >= 2 and cross(lo[-2], lo[-1], p) <= 0:
            lo.pop()
        lo.append(p)
    for p in reversed(pts):
        while len(up) >= 2 and cross(up[-2], up[-1], p) <= 0:
            up.pop()
        up.append(p)
    return lo[:-1] + up[:-1]


def track(name, side):
    pts = []
    for n, w in WHEELS.items():
        if n.startswith(f"wheel_{name[-1]}_"):
            r = w["radius"] + TRACK_T / 2
            for k in range(72):
                a = k * 2 * math.pi / 72
                pts.append((round(w["x"] + math.cos(a) * r, 5), round(w["z"] + math.sin(a) * r, 5)))
    loop = hull2d(pts)
    # resample by arc length
    segs = []
    L = 0
    for i in range(len(loop)):
        a, b = Vector(loop[i]), Vector(loop[(i + 1) % len(loop)])
        segs.append((a, b, L))
        L += (b - a).length
    n = int(L / 0.03)
    samples = []
    for k in range(n):
        d = k * L / n
        for a, b, s0 in segs:
            if s0 <= d <= s0 + (b - a).length + 1e-9:
                t = (d - s0) / max((b - a).length, 1e-9)
                samples.append((a.lerp(b, t), d))
                break
    bm = bmesh.new()
    uv = bm.loops.layers.uv.new("UVMap")
    y0, y1 = side * TRACK_Y - TRACK_W / 2, side * TRACK_Y + TRACK_W / 2
    ring_out, ring_in = [], []
    for i, (p, d) in enumerate(samples):
        q = samples[(i + 1) % n][0]
        pr = samples[i - 1][0]
        tng = (q - pr).normalized()
        nrm = Vector((tng.y, -tng.x))  # outward for a CCW loop
        po = p + nrm * TRACK_T / 2
        pi = p - nrm * TRACK_T / 2
        ring_out.append((bm.verts.new((po.x, y0, po.y)), bm.verts.new((po.x, y1, po.y)), d))
        ring_in.append((bm.verts.new((pi.x, y0, pi.y)), bm.verts.new((pi.x, y1, pi.y)), d))
    for i in range(n):
        j = (i + 1) % n
        u0, u1 = ring_out[i][2] / LINK_PITCH, (ring_out[i][2] + L / n) / LINK_PITCH
        for ring, flip in ((ring_out, False), (ring_in, True)):
            a0, a1, _ = ring[i]
            b0, b1, _ = ring[j]
            f = bm.faces.new((a0, b0, b1, a1) if not flip else (a1, b1, b0, a0))
            for lp, (uu, vv) in zip(f.loops, ((u0, 0), (u1, 0), (u1, 1), (u0, 1)) if not flip else ((u0, 1), (u1, 1), (u1, 0), (u0, 0))):
                lp[uv].uv = (uu, vv)
        for k in (0, 1):
            a, b = ring_out[i][k], ring_out[j][k]
            c, d_ = ring_in[j][k], ring_in[i][k]
            f = bm.faces.new((a, d_, c, b) if k == 0 else (a, b, c, d_))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    o = obj_from_bm(name, bm, [track_mat], hull)
    o["track_length_m"] = L
    o["link_pitch_m"] = LINK_PITCH
    return o, L


# track material: link pattern on U, scrolled by the "track_offset" custom property (links travelled)
track_mat = bpy.data.materials.new("track_links")
track_mat.use_nodes = True
nt = track_mat.node_tree
bsdf = nt.nodes["Principled BSDF"]
bsdf.inputs["Metallic"].default_value = 0.6
bsdf.inputs["Roughness"].default_value = 0.55
uvn = nt.nodes.new("ShaderNodeUVMap")
mp = nt.nodes.new("ShaderNodeMapping")
link(nt, uvn.outputs["UV"], mp.inputs["Vector"])
attr = nt.nodes.new("ShaderNodeAttribute")
attr.attribute_type = "OBJECT"
attr.attribute_name = "track_offset"
cmb = nt.nodes.new("ShaderNodeCombineXYZ")
link(nt, attr.outputs["Fac"], cmb.inputs["X"])
add = nt.nodes.new("ShaderNodeVectorMath")
add.operation = "ADD"
link(nt, mp.outputs["Vector"], add.inputs[0])
link(nt, cmb.outputs["Vector"], add.inputs[1])
sep = nt.nodes.new("ShaderNodeSeparateXYZ")
link(nt, add.outputs["Vector"], sep.inputs[0])
fr = nt.nodes.new("ShaderNodeMath")
fr.operation = "FRACT"
link(nt, sep.outputs["X"], fr.inputs[0])
grouser = nt.nodes.new("ShaderNodeMapRange")
grouser.inputs["From Min"].default_value = 0.0
grouser.inputs["From Max"].default_value = 0.25
link(nt, fr.outputs[0], grouser.inputs["Value"])
pad = nt.nodes.new("ShaderNodeMath")
pad.operation = "GREATER_THAN"
pad.inputs[1].default_value = 0.35
link(nt, fr.outputs[0], pad.inputs[0])
centre = nt.nodes.new("ShaderNodeMath")
centre.operation = "PINGPONG"
centre.inputs[1].default_value = 0.5
link(nt, sep.outputs["Y"], centre.inputs[0])
ramp = nt.nodes.new("ShaderNodeValToRGB")
ramp.color_ramp.elements[0].color = (0.035, 0.033, 0.03, 1)
ramp.color_ramp.elements[1].color = (0.22, 0.21, 0.19, 1)
link(nt, pad.outputs[0], ramp.inputs["Fac"])
link(nt, ramp.outputs["Color"], bsdf.inputs["Base Color"])
bump = nt.nodes.new("ShaderNodeBump")
bump.inputs["Strength"].default_value = 0.9
link(nt, pad.outputs[0], bump.inputs["Height"])
link(nt, bump.outputs["Normal"], bsdf.inputs["Normal"])

track_L, LEN_L = track("track_L", 1)
track_R, LEN_R = track("track_R", -1)
for t in (track_L, track_R):
    t["track_offset"] = 0.0

# ------------------------------------------------------------------ turret (yaw about the hull origin: the sim's mount bearing)
turret = nodeobj("turret", (0, 0, TURRET_Z), hull)


def turret_shell():
    # plan outline (x, y half-width) bottom and top; angular composite-armour front, bustle at the rear
    plan = [(1.95, 0.40), (1.40, 1.42), (-1.20, 1.46), (-2.25, 1.25), (-2.35, 0.0)]
    outline = [(x, y) for x, y in plan] + [(x, -y) for x, y in reversed(plan) if y > 0]
    bm = bmesh.new()
    bot = [bm.verts.new((x, y, 0.0)) for x, y in outline]
    top = [bm.verts.new((x * 0.95 - 0.10, y * 0.84, 0.70)) for x, y in outline]
    n = len(outline)
    bm.faces.new(bot[::-1])
    bm.faces.new(top)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((bot[i], bot[j], top[j], top[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    o = obj_from_bm("turret_shell", bm, [camo], turret)
    bv = o.modifiers.new("bevel", "BEVEL")
    bv.width = 0.04
    bv.segments = 2
    bv.limit_method = "ANGLE"
    return o


turret_shell()
# turret ring skirt
cyl("turret_ring", 1.05, 0.12, (0, 0, 0.0), "Z", dark, turret, seg=40)
# bustle rack
for i, x in enumerate((-2.38, -2.75)):
    for y in (-1.25, 1.25):
        cyl(f"rack_post_{i}_{y}", 0.02, 0.35, (x, y * 0.88, 0.5), "Z", steel, turret, seg=6)
box("rack_rail_back", (0.04, 2.3, 0.04), (-2.75, 0, 0.66), steel, turret)
box("rack_floor", (0.40, 2.2, 0.03), (-2.57, 0, 0.36), steel, turret)
box("rack_bag_0", (0.36, 0.8, 0.26), (-2.57, 0.5, 0.51), mat("canvas", (0.22, 0.2, 0.13), rough=0.95), turret, bevel=0.06)
box("rack_bag_1", (0.34, 0.6, 0.22), (-2.57, -0.45, 0.49), mat("canvas2", (0.16, 0.17, 0.1), rough=0.95), turret, bevel=0.06)
# side stowage boxes and smoke launchers
for side in (-1, 1):
    box(f"turret_box_{side}", (1.1, 0.2, 0.3), (-0.9, side * 1.52, 0.38), camo, turret, bevel=0.02)
    for k in range(6):
        cyl(f"smoke_{side}_{k}", 0.045, 0.24, (1.25 - (k % 3) * 0.1, side * (1.30 + 0.05 * (k // 3)), 0.62 + 0.09 * (k // 3)), "X",
            dark, turret, seg=10)
    cyl(f"antenna_base_{side}", 0.05, 0.1, (-1.7, side * 1.0, 0.75), "Z", dark, turret, seg=10)
    cyl(f"antenna_{side}", 0.007, 1.2, (-1.7, side * 1.0, 1.4), "Z", dark, turret, seg=6)
# gunner's primary sight (left-front of roof), loader's hatch
box("gps_sight", (0.5, 0.4, 0.28), (0.95, 0.62, 0.82), camo, turret, bevel=0.03)
box("gps_glass", (0.02, 0.3, 0.16), (1.2, 0.62, 0.84), glass, turret)
cyl("loader_hatch", 0.3, 0.06, (-0.2, 0.55, 0.71), "Z", camo, turret, bevel=0.01)
box("wind_sensor", (0.04, 0.04, 0.3), (-0.8, 0.2, 0.9), dark, turret)
# commander's cupola (the HMG mount sits on it)
cup_xy = (-0.25, -0.58)
cyl("cupola", 0.38, 0.2, (cup_xy[0], cup_xy[1], 0.8), "Z", camo, turret, seg=24, bevel=0.02)
for k in range(8):
    a = k * math.pi / 4
    box(f"vision_block_{k}", (0.1, 0.12, 0.08), (cup_xy[0] + math.cos(a) * 0.40, cup_xy[1] + math.sin(a) * 0.40, 0.83), glass, turret,
        rot=(0, 0, a))

# ------------------------------------------------------------------ gun (pitch about the trunnion)
gun = nodeobj("gun", TRUNNION - Vector((0, 0, TURRET_Z)), turret)
L = GUN_REACH - TRUNNION.x  # barrel reach from the trunnion
box("mantlet", (0.50, 0.80, 0.44), (0.18, 0, 0.0), camo, gun, bevel=0.05)
cyl("coax", 0.02, 0.4, (0.5, -0.32, -0.02), "X", dark, gun, seg=8)
bl = L - 0.3
cyl("barrel", 0.085, bl, (0.3 + bl / 2, 0, 0), "X", camo, gun, seg=24, r2=0.075)
cyl("thermal_sleeve_0", 0.10, 0.06, (0.3 + bl * 0.30, 0, 0), "X", camo, gun, seg=24)
cyl("thermal_sleeve_1", 0.10, 0.06, (0.3 + bl * 0.75, 0, 0), "X", camo, gun, seg=24)
cyl("fume_extractor", 0.125, min(0.55, bl * 0.2), (0.3 + bl * 0.52, 0, 0), "X", camo, gun, seg=24, bevel=0.02)
cyl("muzzle_ring", 0.09, 0.08, (0.3 + bl - 0.04, 0, 0), "X", dark, gun, seg=24)
cyl("muzzle_ref_sensor", 0.025, 0.1, (0.3 + bl - 0.1, 0, 0.12), "Z", dark, gun, seg=8)
muzzle = empty("muzzle", (0.3 + bl, 0, 0), gun, 0.1)

# ------------------------------------------------------------------ HMG mount (own yaw on the cupola, own pitch)
hmg = nodeobj("hmg", (cup_xy[0], cup_xy[1], 0.9), turret)
box("hmg_cradle_post", (0.08, 0.08, 0.3), (0.1, 0, 0.12), dark, hmg)
box("hmg_shield", (0.04, 0.45, 0.28), (0.35, 0, 0.3), camo, hmg, bevel=0.01)
hmg_gun = nodeobj("hmg_gun", (0.15, 0, 0.32), hmg)
box("hmg_receiver", (0.28, 0.13, 0.15), (0.0, 0, 0), dark, hmg_gun, bevel=0.01)
cyl("hmg_jacket", 0.035, 0.45, (0.36, 0, 0.0), "X", dark, hmg_gun, seg=12)
cyl("hmg_barrel", 0.018, 0.75, (0.9, 0, 0.0), "X", dark, hmg_gun, seg=10)
box("hmg_grips", (0.08, 0.12, 0.08), (-0.2, 0, 0.0), dark, hmg_gun)
box("hmg_ammo", (0.26, 0.12, 0.18), (0.02, -0.14, -0.08), mat("ammo_can", (0.14, 0.16, 0.09), rough=0.6), hmg_gun, bevel=0.01)
hmg_muzzle = empty("hmg_muzzle", (1.28, 0, 0), hmg_gun, 0.05)


# ------------------------------------------------------------------ articulation API (what the renderer's pose driver sets)
def set_pose(turret_yaw_deg=0.0, gun_pitch_deg=0.0, hmg_yaw_deg=0.0, hmg_pitch_deg=0.0, distance_m=0.0):
    turret.rotation_euler = (0, 0, math.radians(turret_yaw_deg))
    gun.rotation_euler = (0, -math.radians(gun_pitch_deg), 0)  # +pitch raises the muzzle
    hmg.rotation_euler = (0, 0, math.radians(hmg_yaw_deg))
    hmg_gun.rotation_euler = (0, -math.radians(hmg_pitch_deg), 0)
    for n, w in WHEELS.items():
        o = bpy.data.objects[n]
        o.rotation_euler = (0, distance_m / w["radius"], 0)  # rolling forward (+X) spins about +Y
    for t in (track_L, track_R):
        t["track_offset"] = -distance_m / LINK_PITCH
        t.update_tag()
    bpy.context.view_layer.update()


# ------------------------------------------------------------------ checks
def muzzle_arc_check():
    """Turret yaw alone must move the muzzle on the sim's arc: position + rotate([r,0], bearing), z fixed."""
    worst = 0.0
    rows = []
    for yaw in range(0, 360, 30):
        set_pose(turret_yaw_deg=yaw)
        p = muzzle.matrix_world.translation
        r = GUN_REACH
        exp = Vector((r * math.cos(math.radians(yaw)), r * math.sin(math.radians(yaw)), TRUNNION.z))
        err = (p - exp).length
        worst = max(worst, err)
        rows.append((yaw, [round(v, 3) for v in p]))
    set_pose()
    return worst, rows


worst, rows = muzzle_arc_check()
print("ARC worst_err_m", worst)

# stats
dg = bpy.context.evaluated_depsgraph_get()
tris = {}
for o in bpy.data.objects:
    if o.type == "MESH":
        me = o.evaluated_get(dg).to_mesh()
        me.calc_loop_triangles()
        root = o.parent.name if o.parent else "-"
        tris[root] = tris.get(root, 0) + len(me.loop_triangles)
        o.evaluated_get(dg).to_mesh_clear()
bb = [Vector((1e9,) * 3), Vector((-1e9,) * 3)]
for o in bpy.data.objects:
    if o.type == "MESH" and not o.name.startswith("antenna"):
        for c in o.bound_box:
            w = o.matrix_world @ Vector(c)
            bb[0] = Vector(map(min, bb[0], w))
            bb[1] = Vector(map(max, bb[1], w))
info = dict(gun_reach_m=GUN_REACH, muzzle_arc_worst_err_m=worst, muzzle_world_at_yaw0=rows[0][1],
            tris_total=sum(tris.values()), tris_by_node=tris, bounds_min=list(bb[0]), bounds_max=list(bb[1]),
            track_length_m=[LEN_L, LEN_R], link_pitch_m=LINK_PITCH,
            nodes=sorted(o.name for o in bpy.data.objects if o.type == "EMPTY"))
print("TANK", json.dumps(info))
json.dump(info, open(os.path.join(OUT, f"tank{TAG}_info.json"), "w"), indent=1)

# ------------------------------------------------------------------ renders
setup_render((640, 480), samples=48)
light_rig(strength=3.2, sky=0.5, neutral=True)
ground(color=(0.36, 0.34, 0.29))
cam = camera(lens=50)
C = (0.4, 0, 1.1)
if MODE in ("all", "sheet"):
    set_pose(turret_yaw_deg=0, gun_pitch_deg=2)
    shots = []
    for name, yaw, pitch in VIEWS8:
        aim(cam, C, yaw, pitch, 17 if name not in ("battle", "top") else 22)
        p = os.path.join(OUT, f"tank{TAG}_view_{name}.png")
        render_to(p)
        shots.append(p)
    tile(shots, os.path.join(OUT, f"tank{TAG}_sheet.png"), 4)
if MODE in ("all", "strips"):
    # articulation: 8 phases of turret yaw, gun pitch, HMG yaw/pitch
    shots = []
    for i in range(8):
        set_pose(turret_yaw_deg=i * 45, gun_pitch_deg=[-8, 0, 8, 15, 4, -5, 12, 0][i], hmg_yaw_deg=-i * 60,
                 hmg_pitch_deg=[0, 20, 40, 10, -5, 30, 0, 15][i])
        aim(cam, C, 30, 40, 17)
        p = os.path.join(OUT, f"tank{TAG}_artic_{i}.png")
        render_to(p)
        shots.append(p)
    tile(shots, os.path.join(OUT, f"tank{TAG}_strip_articulation.png"), 8)
    # running gear: 8 phases of 0.1 m travel, close side view
    shots = []
    for i in range(8):
        set_pose(distance_m=i * 0.1)
        aim(cam, (-2.2, -1.6, 0.55), -75, 8, 3.4)
        p = os.path.join(OUT, f"tank{TAG}_roll_{i}.png")
        render_to(p)
        shots.append(p)
    tile(shots, os.path.join(OUT, f"tank{TAG}_strip_running_gear.png"), 8)
if MODE == "one":
    set_pose(turret_yaw_deg=0, gun_pitch_deg=2)
    aim(cam, C, 35, 20, 17)
    render_to(os.path.join(OUT, f"tank{TAG}_q-front.png"))
if MODE in ("all", "export"):
    set_pose()
    for o in list(bpy.data.objects):
        if o.type in ("CAMERA", "LIGHT") or o.name == "ground":
            bpy.data.objects.remove(o, do_unlink=True)
    path = os.path.join(OUT, f"tank{TAG}.glb")
    bpy.ops.export_scene.gltf(filepath=path, export_apply=True, export_yup=True)
    print("GLB", path, os.path.getsize(path))
