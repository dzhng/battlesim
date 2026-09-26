"""The main battle tank: an articulated appearance (and, with --wreck, its burnt wreck).

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/tank.py -- [out.glb] [--wreck]

Ported from spike 03's frozen `tank.py` (specs/battle-look/assets/spikes/03/scripts/):
the same node tree, pivots and proportions, with the muzzle at the rule's
`tank_muzzle_local_m` [5.9, 0, 2.0].

    tank ─ hull ─┬ turret (yaw about the hull origin) ─┬ gun (pitch at the trunnion) ─ muzzle
                 │                                     └ hmg (yaw on the cupola) ─ hmg_gun (pitch) ─ hmg_muzzle
                 ├ wheel_{L,R}_{1..7,sprocket,idler,return_1,return_2}   (radius_m; roll about +Y)
                 └ track_L, track_R   (track_length_m, link_pitch_m; U = arc length / link pitch)
"""
import bpy, bmesh, sys, os, math, json
from mathutils import Vector, Matrix

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import *

ARGS = script_args()
WRECK = "--wreck" in ARGS
POS = [a for a in ARGS if not a.startswith("--")]
OUT = POS[0] if POS else os.path.abspath("tank.glb")
GUN_REACH = 5.9  # physics.tank_muzzle_local_m[0]: muzzle distance from the turret axis
MUZZLE_Z = 2.0  # physics.tank_muzzle_local_m[2]

reset()

# sim authority: physics.tank_half_extents_m [3.5, 1.8, 1.2]
TRUNNION = Vector((1.70, 0, MUZZLE_Z))
TURRET_Z = 1.45
WHEEL_R = 0.33
TRACK_T = 0.07
TRACK_E = 0.025  # end connectors stand proud of the shoes at both edges
WHEEL_Z = WHEEL_R + TRACK_T + TRACK_E  # the track's end connectors rest on the ground
LIFT = WHEEL_Z - 0.38  # the spike sat its wheels at 0.38 and sank 2 cm; everything on the running gear rises with them
TRACK_W = 0.62
TRACK_Y = 1.44
LINK_PITCH = 0.16

if WRECK:
    camo = burnt("tank_camo")
    dark = burnt("running_gear", seed=7.0)
    rubber = burnt("rubber", seed=9.0)
    steel = burnt("steel", seed=11.0)
    glass = flat_paint("glass", (0.02, 0.02, 0.02), rough=0.9)
    light_m = flat_paint("headlight", (0.03, 0.03, 0.03), rough=0.9)
    canvas = burnt("canvas", seed=13.0)
    canvas2 = canvas
    ammo = burnt("ammo_can", seed=15.0)
    track_mat = burnt("track_links", seed=17.0)
else:
    camo = woodland("tank_camo")
    dark = flat_paint("running_gear", (0.07, 0.075, 0.06), rough=0.7, wear=0.3)
    rubber = flat_paint("rubber", (0.025, 0.025, 0.025), rough=0.9)
    steel = flat_paint("steel", (0.1, 0.1, 0.09), rough=0.5, metal=0.5, wear=0.8)
    glass = flat_paint("glass", (0.02, 0.04, 0.05), rough=0.05, grime=0.2)
    light_m = flat_paint("headlight", (0.6, 0.6, 0.55), rough=0.2)
    canvas = flat_paint("canvas", (0.22, 0.2, 0.13), rough=0.95)
    canvas2 = flat_paint("canvas2", (0.16, 0.17, 0.1), rough=0.95)
    ammo = flat_paint("ammo_can", (0.14, 0.16, 0.09), rough=0.6, wear=0.4)
    track_mat = flat_paint("track_links", (0.22, 0.21, 0.19), rough=0.55, metal=0.6, wear=0.6)

def glacis_z(x):
    """The hull roof's height at x: the upper hull profile, deck (from (-3.30, 1.47) to
    (2.35, 1.45)) then glacis (down to (3.50, 1.12))."""
    if x <= 2.35:
        return 1.45 + (2.35 - x) / 5.65 * 0.02
    return 1.45 - (x - 2.35) / 1.15 * 0.33


tank = empty("tank")
hull = empty("hull", parent=tank)

# ------------------------------------------------------------------ hull
prism("hull_upper", [(-3.45, 0.98), (3.25, 0.98), (3.50, 1.12), (2.35, 1.45), (-3.30, 1.47), (-3.48, 1.30)], 3.52,
      mat=camo, parent=hull, bevel=0.035)
prism("hull_lower", [(-3.25, 0.46), (2.95, 0.46), (3.42, 1.0), (-3.40, 1.0)], 2.16, mat=camo, parent=hull, bevel=0.03)
for i in range(9):
    box(f"deck_slat_{i}", (0.06, 1.6, 0.025), (-3.1 + i * 0.16, 0, 1.475), dark, hull, lods=(0, 1))
box("deck_access", (1.1, 2.2, 0.03), (-1.2, 0, 1.47), camo, hull, bevel=0.01, lods=(0, 1, 2))
cyl("driver_hatch", 0.32, 0.05, (2.25, 0, glacis_z(2.25) + 0.02), "Z", camo, hull, bevel=0.01, lods=(0, 1, 2))
for i, y in enumerate((-0.22, 0, 0.22)):
    box(f"driver_periscope_{i}", (0.1, 0.16, 0.07), (2.55, y, 1.43), glass, hull, rot=(0, math.radians(20), 0), lods=(0, 1))
def front_lower_x(z):
    """The lower glacis's face (the hull_lower profile's front edge) at height z."""
    return 2.95 + (z - 0.46) / 0.54 * 0.47


def rear_lower_x(z):
    return -3.25 - (z - 0.46) / 0.54 * 0.15


for side in (-1, 1):
    s = "L" if side > 0 else "R"
    box(f"headlight_guard_{s}", (0.14, 0.24, 0.16), (3.30, side * 1.35, 1.25), steel, hull, bevel=0.01, lods=(0, 1))
    cyl(f"headlight_{s}", 0.05, 0.04, (3.36, side * 1.35, 1.25), "X", light_m, hull, lods=(0, 1))
    box(f"taillight_{s}", (0.05, 0.16, 0.1), (-3.47, side * 1.5, 1.38), light_m, hull, lods=(0,))
    # tow hooks sit on the lower glacis and rear plate, each with its shackle
    zf = 0.66
    box(f"tow_hook_{s}", (0.16, 0.1, 0.16), (front_lower_x(zf) + 0.05, side * 0.7, zf), steel, hull, bevel=0.015,
        rot=(0, math.radians(-41), 0), lods=(0, 1))
    cyl(f"tow_shackle_{s}", 0.07, 0.035, (front_lower_x(zf) + 0.14, side * 0.7, zf - 0.03), "Y", steel, hull, seg=12,
        lods=(0,))
    zr = 0.85
    box(f"rear_hook_{s}", (0.14, 0.1, 0.14), (rear_lower_x(zr) - 0.04, side * 0.75, zr), steel, hull, bevel=0.015,
        lods=(0, 1))
    box(f"fender_{s}", (0.55, 0.64, 0.03), (3.2, side * TRACK_Y, 1.0), camo, hull, bevel=0.01, rot=(0, math.radians(-10), 0),
        lods=(0, 1, 2))
    box(f"mudflap_{s}", (0.02, 0.6, 0.28), (3.46, side * TRACK_Y, 0.9), rubber, hull, lods=(0, 1))
    # tow cable along the hull roof edge, eyes at each end
    cyl(f"tow_cable_{s}", 0.022, 4.6, (-0.9, side * 1.66, 1.5), "X", steel, hull, seg=8, lods=(0, 1))
    for k, x in enumerate((-3.2, 1.4)):
        box(f"tow_cable_eye_{s}_{k}", (0.12, 0.05, 0.06), (x, side * 1.66, 1.5), steel, hull, lods=(0,))
    # fuel caps and lifting eyes on the deck
    cyl(f"fuel_cap_{s}", 0.09, 0.04, (-2.3, side * 1.2, 1.49), "Z", steel, hull, seg=12, lods=(0,))
    for k, x in enumerate((2.2, -3.0)):
        box(f"lift_eye_{s}_{k}", (0.1, 0.03, 0.08), (x, side * 1.5, 1.49), steel, hull, lods=(0,))
box("rear_grille", (0.05, 1.8, 0.5), (-3.47, 0, 1.1), dark, hull, bevel=0.01, lods=(0, 1, 2))
for k in range(7):
    box(f"exhaust_louvre_{k}", (0.07, 1.7, 0.03), (-3.51, 0, 0.9 + k * 0.065), steel, hull, rot=(0, math.radians(35), 0),
        lods=(0,))
box("rear_stowage", (0.25, 1.0, 0.25), (-3.40, 0, 1.55), camo, hull, bevel=0.03, lods=(0, 1, 2))
for k, x in enumerate((-2.2, -0.9)):
    box(f"deck_panel_{k}", (1.0, 1.2, 0.02), (x + 0.3, 0, 1.485), camo, hull, bevel=0.008, lods=(0,))
    for j in range(4):
        cyl(f"deck_bolt_{k}_{j}", 0.02, 0.02, (x + 0.3 + (j % 2 - 0.5) * 0.9, (j // 2 - 0.5) * 1.1, 1.5), "Z", steel, hull,
            seg=6, lods=(0,))
# spare track links on the glacis
for k in range(4):
    x = 2.72 + 0.17 * k * 0.95
    box(f"spare_link_{k}", (0.16, 0.62, 0.05), (x, 0, glacis_z(x) + 0.02), dark, hull,
        rot=(0, math.radians(17), 0), lods=(0, 1))

for side in (-1, 1):
    s = "L" if side > 0 else "R"
    y = side * (TRACK_Y + TRACK_W / 2 + 0.035)
    for i in range(6):
        x0 = -3.05 + i * 1.06
        box(f"skirt_{s}_{i}", (1.03, 0.06, 0.34), (x0 + 0.5, y, 0.88), camo, hull, bevel=0.012)
        for j in range(3):
            cyl(f"skirt_bolt_{s}_{i}_{j}", 0.018, 0.02, (x0 + 0.2 + j * 0.3, y + side * 0.035, 0.98), "Y", steel, hull, seg=8,
                lods=(0,))
        box(f"skirt_hanger_{s}_{i}", (0.06, 0.12, 0.08), (x0 + 0.5, y - side * 0.07, 1.02), steel, hull, lods=(0,))
    box(f"skirt_rail_{s}", (6.4, 0.08, 0.05), (0.15, y - side * 0.02, 1.06), steel, hull, lods=(0, 1))
    box(f"skirt_front_{s}", (0.5, 0.06, 0.42), (3.22, y, 0.86), camo, hull, bevel=0.012, rot=(0, math.radians(-18), 0))

# ------------------------------------------------------------------ running gear
WHEELS = {}


def wheel(name, x, z, r, side, kind="road"):
    y = side * TRACK_Y
    node = empty(name, (x, y, z), hull, props={"radius_m": r})
    w = 0.24 if kind == "road" else 0.30
    lods = (0, 1, 2) if kind == "road" else (0, 1) if kind == "return" else TIERS
    cyl(name + "_tyre", r, w, (0, 0, 0), "Y", rubber, node, seg=28, lods=lods)
    cyl(name + "_disc", r * 0.84, w + 0.02, (0, 0, 0), "Y", dark, node, seg=28, bevel=0.01, lods=lods)
    cyl(name + "_hub", r * 0.3, w + 0.08, (0, 0, 0), "Y", steel, node, seg=16, lods=tuple(l for l in lods if l < 2))
    for k in range(6):
        a = k * math.pi / 3
        cyl(f"{name}_hole_{k}", r * 0.12, w + 0.04, (math.cos(a) * r * 0.55, 0, math.sin(a) * r * 0.55), "Y", rubber, node,
            seg=10, lods=(0,))
    if kind == "sprocket":
        for k in range(11):
            a = k * 2 * math.pi / 11
            box(f"{name}_tooth_{k}", (0.07, 0.12, 0.06), (math.cos(a) * (r + 0.02), 0, math.sin(a) * (r + 0.02)), steel, node,
                rot=(0, -a, 0), lods=(0, 1))
    if kind in ("idler", "sprocket"):
        # spokes and a cap, so the idler reads as a wheel under the track
        for k in range(5):
            a = k * 2 * math.pi / 5 + 0.3
            box(f"{name}_spoke_{k}", (r * 0.7, 0.05, 0.06), (math.cos(a) * r * 0.42, side * (w / 2 + 0.02), math.sin(a) * r * 0.42),
                dark, node, rot=(0, -a, 0), lods=(0,))
        cyl(name + "_cap", r * 0.2, w + 0.14, (0, 0, 0), "Y", steel, node, seg=10, lods=(0, 1))
    WHEELS[name] = dict(radius=r, x=x, z=z)
    # the suspension arm (road wheels) or bracket that holds the hub to the hull: static, on the hull
    hy = side * (TRACK_Y - (w / 2 + 0.1))
    if kind == "road":
        box(f"{name}_arm", (0.5, 0.1, 0.12), (x + 0.22, hy, z + 0.12), dark, hull, rot=(0, math.radians(-25), 0), lods=(0, 1))
    elif kind in ("idler", "sprocket"):
        box(f"{name}_bracket", (0.3, 0.12, 0.3), (x - side * 0 + (0.1 if kind == "idler" else -0.05), hy, z + 0.1), dark, hull,
            bevel=0.02, lods=(0, 1))
    return node


for side, s in ((1, "L"), (-1, "R")):
    for i in range(7):
        wheel(f"wheel_{s}_{i + 1}", -2.55 + i * 0.815, WHEEL_Z, WHEEL_R, side)
    wheel(f"wheel_{s}_sprocket", -3.12, 0.66 + LIFT, 0.33, side, "sprocket")
    wheel(f"wheel_{s}_idler", 3.05, 0.60 + LIFT, 0.30, side, "idler")
    for i in range(2):
        wheel(f"wheel_{s}_return_{i + 1}", -1.4 + i * 2.2, 0.83 + LIFT, 0.08, side, "return")


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


def track_loop(side_name):
    """The spike's band path: the convex hull of every wheel, grown by half the track's thickness."""
    pts = []
    for n, w in WHEELS.items():
        if n.startswith(f"wheel_{side_name}_"):
            r = w["radius"] + TRACK_T / 2
            for k in range(72):
                a = k * 2 * math.pi / 72
                pts.append((round(w["x"] + math.cos(a) * r, 5), round(w["z"] + math.sin(a) * r, 5)))
    loop = hull2d(pts)
    segs, L = [], 0.0
    for i in range(len(loop)):
        a, b = Vector(loop[i]), Vector(loop[(i + 1) % len(loop)])
        segs.append((a, b, L))
        L += (b - a).length
    return segs, L


def track(name, side):
    segs, L = track_loop(name[-1])
    node = empty(name, (0, 0, 0), hull, props={"track_length_m": L, "link_pitch_m": LINK_PITCH})
    spacing = (0.03, 0.08, 0.2, 0.45)

    def build(bm, lod):
        n = int(L / spacing[lod])
        samples = []
        for k in range(n):
            d = k * L / n
            for a, b, s0 in segs:
                if s0 <= d <= s0 + (b - a).length + 1e-9:
                    t = (d - s0) / max((b - a).length, 1e-9)
                    samples.append((a.lerp(b, t), d))
                    break
        n = len(samples)
        uv = bm.loops.layers.uv.new("UVMap")
        y0 = side * TRACK_Y - TRACK_W / 2
        h, e = TRACK_T / 2, TRACK_E
        # the link's cross-section across the width (w 0..1) and out from the band's
        # centre line (r): shoes, with end connectors standing proud at both edges;
        # the finest tiers carry the connectors
        profile = ([(0, -h), (0, h + e), (0.1, h + e), (0.12, h), (0.88, h), (0.9, h + e), (1, h + e), (1, -h)]
                   if lod < 2 else [(0, -h), (0, h), (1, h), (1, -h)])
        rings = []
        for i, (p, d) in enumerate(samples):
            q = samples[(i + 1) % n][0]
            pr = samples[i - 1][0]
            tng = (q - pr).normalized()
            nrm = Vector((tng.y, -tng.x))
            ring = []
            for w, r in profile:
                pp = p + nrm * r
                ring.append(bm.verts.new((pp.x, y0 + w * TRACK_W, pp.y)))
            rings.append((ring, d))
        m = len(profile)
        for i in range(n):
            j = (i + 1) % n
            u0 = rings[i][1] / LINK_PITCH
            u1 = u0 + (L / n) / LINK_PITCH
            for k in range(m):
                k2 = (k + 1) % m
                a0, a1 = rings[i][0][k], rings[i][0][k2]
                b0, b1 = rings[j][0][k], rings[j][0][k2]
                f = bm.faces.new((a0, a1, b1, b0))
                v0, v1 = profile[k][0], profile[k2][0]
                for lp, (uu, vv) in zip(f.loops, ((u0, v0), (u0, v1), (u1, v1), (u1, v0))):
                    lp[uv].uv = (uu, vv)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    mesh_part(f"{name}_band", build, track_mat, node)
    return node, L


track_L, LEN_L = track("track_L", 1)
track_R, LEN_R = track("track_R", -1)

# ------------------------------------------------------------------ turret
turret = empty("turret", (0, 0, TURRET_Z), hull)
# plan (x, half width) of the shell's foot, front to rear: a broad flat face that holds the
# mantlet, angled cheeks, straight sides and a long bustle (the spike's pointed nose read as a toy)
TURRET_PLAN = [(1.85, 0.95), (1.55, 1.36), (0.9, 1.47), (-1.5, 1.47), (-2.3, 1.28), (-2.42, 0.9), (-2.42, 0.0)]


def turret_shell(bm, lod):
    plan = TURRET_PLAN
    outline = [(x, y) for x, y in plan] + [(x, -y) for x, y in reversed(plan) if y > 0]
    bot = [bm.verts.new((x, y, 0.0)) for x, y in outline]
    top = [bm.verts.new((x * 0.95 - 0.10, y * 0.84, 0.70)) for x, y in outline]
    n = len(outline)
    bm.faces.new(bot[::-1])
    bm.faces.new(top)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((bot[i], bot[j], top[j], top[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    w, s = (0.04, 2) if lod == 0 else (0.04, 1) if lod == 1 else (0, 0)
    if w:
        edges = [e for e in bm.edges if e.calc_face_angle(0) > math.radians(30)]
        bmesh.ops.bevel(bm, geom=edges, offset=w, offset_type="OFFSET", segments=s, profile=0.5, affect="EDGES",
                        clamp_overlap=True)


mesh_part("turret_shell", turret_shell, camo, turret)
cyl("turret_ring", 1.05, 0.12, (0, 0, 0.0), "Z", dark, turret, seg=40, lods=(0, 1, 2))
for i, x in enumerate((-2.38, -2.75)):
    for j, y in enumerate((-1.25, 1.25)):
        cyl(f"rack_post_{i}_{j}", 0.02, 0.35, (x, y * 0.88, 0.5), "Z", steel, turret, seg=6, lods=(0, 1))
box("rack_rail_back", (0.04, 2.3, 0.04), (-2.75, 0, 0.66), steel, turret, lods=(0, 1))
box("rack_floor", (0.40, 2.2, 0.03), (-2.57, 0, 0.36), steel, turret, lods=(0, 1, 2))
box("rack_bag_0", (0.36, 0.8, 0.26), (-2.57, 0.5, 0.51), canvas, turret, bevel=0.06, lods=(0, 1, 2))
box("rack_bag_1", (0.34, 0.6, 0.22), (-2.57, -0.45, 0.49), canvas2, turret, bevel=0.06, lods=(0, 1, 2))
def turret_half_width(x, z):
    """Half width of the turret shell's side at (x, z), from its plan and 0.84 top taper."""
    plan = TURRET_PLAN
    for (xa, ya), (xb, yb) in zip(plan, plan[1:]):
        if xb <= x <= xa:
            y = ya + (yb - ya) * (x - xa) / (xb - xa)
            break
    else:
        y = 1.4
    return y * (1 - 0.16 * z / 0.7)


for side in (-1, 1):
    s = "L" if side > 0 else "R"
    # stowage bins hung on the turret sides, touching the armour
    yb = turret_half_width(-0.9, 0.38) + 0.09
    box(f"turret_box_{s}", (1.1, 0.2, 0.3), (-0.9, side * yb, 0.38), camo, turret, bevel=0.02, lods=(0, 1, 2))
    for k in range(3):
        box(f"turret_box_strap_{s}_{k}", (0.04, 0.21, 0.31), (-1.3 + k * 0.4, side * yb, 0.38), dark, turret, lods=(0,))
    # smoke grenade launchers: two banks of four on brackets bolted to the front cheeks,
    # firing forward, up and out
    yl = turret_half_width(1.0, 0.5)
    box(f"smoke_bracket_{s}", (0.34, 0.1, 0.26), (1.0, side * (yl + 0.04), 0.5), dark, turret, bevel=0.01, lods=(0, 1, 2))
    for k in range(8):
        col, row = k % 4, k // 4
        cyl(f"smoke_{s}_{k}", 0.042, 0.22,
            (0.93 + col * 0.075 + 0.09, side * (yl + 0.13 + 0.02 * col), 0.45 + row * 0.1),
            "X", dark, turret, seg=10, rot=(0, math.radians(-25), side * math.radians(20)), lods=(0, 1))
    cyl(f"antenna_base_{s}", 0.05, 0.1, (-1.7, side * 1.0, 0.75), "Z", dark, turret, seg=10, lods=(0, 1))
    cyl(f"antenna_{s}", 0.007, 1.2, (-1.7, side * 1.0, 1.4), "Z", dark, turret, seg=6, lods=(0, 1))
    box(f"lift_eye_turret_{s}", (0.1, 0.03, 0.08), (0.4, side * turret_half_width(0.4, 0.7), 0.72), steel, turret, lods=(0,))
# blow-out panels on the bustle roof
for k, y in enumerate((-0.45, 0.45)):
    box(f"blowout_panel_{k}", (0.9, 0.7, 0.02), (-1.65, y, 0.71), camo, turret, bevel=0.006, lods=(0, 1))
# the commander's independent sight on its pedestal
cyl("citv_pedestal", 0.12, 0.2, (0.2, -0.05, 0.8), "Z", dark, turret, seg=12, lods=(0, 1))
box("citv_head", (0.34, 0.3, 0.26), (0.22, -0.05, 1.0), camo, turret, bevel=0.03, lods=(0, 1, 2))
box("citv_glass", (0.02, 0.2, 0.12), (0.4, -0.05, 1.0), glass, turret, lods=(0,))
# the bustle rack's load: a bedroll and jerrycans
cyl("rack_bedroll", 0.14, 1.0, (-2.6, 0.0, 0.52), "Y", canvas, turret, seg=12, lods=(0, 1))
for k in range(2):
    box(f"rack_jerrycan_{k}", (0.17, 0.35, 0.45), (-2.62, -1.0 + k * 0.2, 0.6), camo, turret, bevel=0.02, lods=(0, 1))
box("gps_sight", (0.5, 0.4, 0.28), (0.95, 0.62, 0.82), camo, turret, bevel=0.03, lods=(0, 1, 2))
box("gps_glass", (0.02, 0.3, 0.16), (1.2, 0.62, 0.84), glass, turret, lods=(0, 1))
cyl("loader_hatch", 0.3, 0.06, (-0.2, 0.55, 0.71), "Z", camo, turret, bevel=0.01, lods=(0, 1, 2))
box("wind_sensor", (0.04, 0.04, 0.3), (-0.8, 0.2, 0.9), dark, turret, lods=(0,))
cup_xy = (-0.25, -0.58)
cyl("cupola", 0.38, 0.2, (cup_xy[0], cup_xy[1], 0.8), "Z", camo, turret, seg=24, bevel=0.02, lods=(0, 1, 2))
for k in range(8):
    a = k * math.pi / 4
    box(f"vision_block_{k}", (0.1, 0.12, 0.08), (cup_xy[0] + math.cos(a) * 0.40, cup_xy[1] + math.sin(a) * 0.40, 0.83), glass,
        turret, rot=(0, 0, a), lods=(0,))

# ------------------------------------------------------------------ gun
gun = empty("gun", TRUNNION - Vector((0, 0, TURRET_Z)), turret)
L = GUN_REACH - TRUNNION.x
prism("mantlet", [(-0.08, -0.24), (0.40, -0.17), (0.45, 0.15), (0.36, 0.22), (-0.08, 0.25)], 0.84, mat=camo, parent=gun, bevel=0.03)
box("mantlet_boot", (0.3, 0.62, 0.36), (-0.02, 0, -0.02), canvas, gun, bevel=0.06, lods=(0, 1, 2))
for k, y in enumerate((-0.3, 0.3)):
    box(f"mantlet_bolt_row_{k}", (0.03, 0.04, 0.3), (0.43, y, 0), steel, gun, lods=(0,))
cyl("coax", 0.02, 0.4, (0.5, -0.32, -0.02), "X", dark, gun, seg=8, lods=(0, 1))
bl = L - 0.3
cyl("barrel", 0.085, bl, (0.3 + bl / 2, 0, 0), "X", camo, gun, seg=24, r2=0.075)
cyl("thermal_sleeve_0", 0.10, 0.06, (0.3 + bl * 0.30, 0, 0), "X", camo, gun, seg=24, lods=(0, 1))
cyl("thermal_sleeve_1", 0.10, 0.06, (0.3 + bl * 0.75, 0, 0), "X", camo, gun, seg=24, lods=(0, 1))
cyl("fume_extractor", 0.125, min(0.55, bl * 0.2), (0.3 + bl * 0.52, 0, 0), "X", camo, gun, seg=24, bevel=0.02, lods=(0, 1, 2))
cyl("muzzle_ring", 0.09, 0.08, (0.3 + bl - 0.04, 0, 0), "X", dark, gun, seg=24, lods=(0, 1))
cyl("muzzle_ref_sensor", 0.025, 0.1, (0.3 + bl - 0.1, 0, 0.12), "Z", dark, gun, seg=8, lods=(0,))
muzzle = empty("muzzle", (0.3 + bl, 0, 0), gun)

# ------------------------------------------------------------------ HMG
hmg = empty("hmg", (cup_xy[0], cup_xy[1], 0.9), turret)
box("hmg_cradle_post", (0.08, 0.08, 0.3), (0.1, 0, 0.12), dark, hmg, lods=(0, 1, 2))
box("hmg_shield", (0.04, 0.45, 0.28), (0.35, 0, 0.3), camo, hmg, bevel=0.01, lods=(0, 1, 2))
hmg_gun = empty("hmg_gun", (0.15, 0, 0.32), hmg)
box("hmg_receiver", (0.28, 0.13, 0.15), (0.0, 0, 0), dark, hmg_gun, bevel=0.01, lods=(0, 1, 2))
cyl("hmg_jacket", 0.035, 0.45, (0.36, 0, 0.0), "X", dark, hmg_gun, seg=12, lods=(0, 1, 2))
cyl("hmg_barrel", 0.018, 0.75, (0.9, 0, 0.0), "X", dark, hmg_gun, seg=10, lods=(0, 1, 2))
box("hmg_grips", (0.08, 0.12, 0.08), (-0.2, 0, 0.0), dark, hmg_gun, lods=(0, 1))
box("hmg_ammo", (0.26, 0.12, 0.18), (0.02, -0.14, -0.08), ammo, hmg_gun, bevel=0.01, lods=(0, 1))
hmg_muzzle = empty("hmg_muzzle", (1.28, 0, 0), hmg_gun)

# ------------------------------------------------------------------ wreck pose, finish, export
if WRECK:
    # the ammunition cook-off lifted the turret off its ring and dropped it askew
    turret.rotation_euler = (math.radians(4.5), math.radians(-3.0), math.radians(33))
    turret.location = (-0.45, 0.3, TURRET_Z + 0.04)
    gun.rotation_euler = (0, math.radians(9), 0)  # the barrel droops
    hmg.rotation_euler = (0, 0, math.radians(-110))
    hmg_gun.rotation_euler = (0, math.radians(22), 0)
    # burnt away or blown off: antennas, the bustle's load, the canvas boot, skirt panels,
    # mudflaps, lamps, the hatch lids and a blow-out panel
    gone = ("antenna_", "rack_bag", "rack_bedroll", "rack_jerrycan", "mantlet_boot", "skirt_L_2", "skirt_L_3",
            "skirt_R_4", "mudflap_", "skirt_bolt_L_2", "skirt_bolt_L_3", "skirt_bolt_R_4", "headlight_", "blowout_panel_0",
            "loader_hatch", "driver_hatch", "turret_box_R", "citv_", "fender_L", "tow_cable_L")
    for o in list(bpy.data.objects):
        if o.type == "MESH" and o.name.startswith(gone):
            bpy.data.objects.remove(o, do_unlink=True)
    # open hatches: dark holes, the lids thrown back on their hinges
    cyl("driver_hole", 0.28, 0.02, (2.25, 0, glacis_z(2.25) + 0.03), "Z", burnt("hole", seed=21.0), hull, lods=(0, 1, 2))
    cyl("driver_lid_open", 0.32, 0.05, (1.9, 0.0, glacis_z(2.25) + 0.25), "Z", camo, hull, rot=(0, math.radians(-70), 0),
        lods=(0, 1, 2))
    cyl("loader_hole", 0.26, 0.02, (-0.2, 0.55, 0.72), "Z", material("hole"), turret, lods=(0, 1, 2))
    cyl("loader_lid_open", 0.3, 0.06, (-0.52, 0.55, 0.98), "Z", camo, turret, rot=(0, math.radians(-80), 0), lods=(0, 1, 2))
    # the left track thrown: its band lies on the ground beside the wheels, bunched and twisted
    track_L.location = (-0.35, 0.55, 0.0)
    track_L.rotation_euler = (math.radians(-6), math.radians(1.5), math.radians(2.5))
    # the blown-out bustle panel lies askew on the deck
    box("blowout_panel_loose", (0.9, 0.7, 0.02), (-2.6, 0.9, 1.55), camo, hull, rot=(0.25, -0.2, 0.7), lods=(0, 1))

bpy.context.view_layer.update()
rest_on_ground()
finish(ao_distance=1.2)
bpy.context.view_layer.update()
info = dict(
    tris=triangles_by_tier(),
    muzzle=[round(v, 4) for v in muzzle.matrix_world.translation],
    track_length_m=[LEN_L, LEN_R],
    nodes=sorted(o.name for o in bpy.data.objects if o.type == "EMPTY"),
)
print("TANK", json.dumps(info))
export(OUT)
