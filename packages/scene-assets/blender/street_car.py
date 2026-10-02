"""The parked car: a generic five-door hatchback, no badge, plate or livery (and, with
--wreck, its burnt-out shell).

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/street_car.py -- <out.glb> [--wreck]

Authored to the street catalog's boxes (`fixtures/props/city/street.json`): the
`parked_car` at [2.2, 0.9, 0.72] and the `car_wreck` it is destroyed into, the same
plan 0.7 m high. Origin at the box's centre on the ground, +X the car's nose.

The body is one side profile extruded across the car, its wheel arches cut in the
profile, over a dark floor pan that closes the arches behind the wheels. The
greenhouse is a second, narrower profile leaning in toward the roof; the glass is
thin dark slabs lying on its faces.
"""
import bpy, bmesh, sys, os, math, json
from mathutils import Vector, Matrix

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import *

ARGS = script_args()
WRECK = "--wreck" in ARGS
OUT = [a for a in ARGS if not a.startswith("--")][0]
reset()

HALF = (2.2, 0.9, 0.72)
WRECK_TOP = 0.7  # `parked_car.destroyed.into.height_m`
BODY_Y = 0.86  # the body's half width; the mirrors stand out to the box
TYRE_R, TYRE_W = 0.31, 0.21
AXLES = (("F", 1.32), ("R", -1.32))
WHEEL_Y = 0.775
ARCH_R = 0.375
SILL_Z = 0.2
BELT_Z = 0.9  # where the glass starts
ROOF_Z = 2 * HALF[2]
CABIN_Y = 0.83  # the greenhouse's half width at the belt
LEAN = 0.17  # how much narrower it is at the roof

if WRECK:
    paint = textured("car_paint", "burnt_metal", chip=0.9, dirt=0.3, soot=0.3, streak=0.3, ash=0.45)
    trim = textured("trim", "burnt_metal", colour=(0.03, 0.028, 0.026), chip=0.3, dirt=0.2, ash=0.2, seed=7.0)
    under = textured("floor_pan", "burnt_metal", chip=0.5, dirt=0.3, soot=0.6, ash=0.3, seed=9.0)
    wheel_m = textured("wheel", "burnt_metal", chip=0.6, dirt=0.3, ash=0.2, seed=11.0)
    rubber = textured("tyre", "burnt_metal", colour=(0.07, 0.04, 0.022), chip=0.2, dirt=0.2, ash=0.05, seed=13.0)
    lamp = tail = glass = trim
else:
    # one colour for every car: a pale, dusty grey that is neither side's tint
    paint = textured("car_paint", "enamel", colour=(0.34, 0.35, 0.34), chip=0.35, dirt=0.8, rise=0.7, streak=0.15)
    trim = textured("trim", "rubber", chip=0.0, dirt=0.5, streak=0.0, rise=0.6)
    under = textured("floor_pan", "bare_steel", colour=(0.03, 0.03, 0.03), chip=0.2, dirt=1.0)
    wheel_m = textured("wheel", "galvanised", chip=0.2, dirt=0.15)
    rubber = textured("tyre", "rubber", dirt=0.55, chip=0.0, streak=0.0, rise=0.6)
    lamp = flat_paint("lamp", (0.6, 0.6, 0.55), rough=0.2, grime=0.3)
    tail = flat_paint("tail_lamp", (0.22, 0.02, 0.015), rough=0.25, grime=0.3)
    glass = flat_paint("glass", (0.03, 0.045, 0.055), rough=0.05, grime=0.15)

car = empty("car")


def arch(cx, seg):
    """A wheel arch in the body's side profile, rear to front: the arc of `ARCH_R`
    about the wheel's centre, from the sill up over the tyre and down to the sill."""
    dip = math.asin((TYRE_R - SILL_Z) / ARCH_R)
    return [(cx + ARCH_R * math.cos(a), TYRE_R + ARCH_R * math.sin(a))
            for a in (math.pi + dip - k * (math.pi + 2 * dip) / seg for k in range(seg + 1))]


def body_profile(seg):
    return ([(-2.2, 0.5), (-2.14, 0.3)] + arch(-1.32, seg) + arch(1.32, seg) +
            [(2.1, 0.26), (2.19, 0.4), (2.2, 0.6), (2.1, 0.72), (0.98, BELT_Z), (-1.9, 0.93), (-2.18, 0.88)])


# the roofline, front to back: windscreen foot, its head, the roof's crown, the tailgate's head and foot
SCREEN_FOOT, SCREEN_HEAD = (1.0, BELT_Z), (0.32, 1.4)
GATE_HEAD, GATE_FOOT = (-1.5, 1.42), (-2.1, 0.95)
CABIN = [(1.0, 0.86), SCREEN_FOOT, SCREEN_HEAD, (-0.4, ROOF_Z), GATE_HEAD, GATE_FOOT, (-2.1, 0.86)]


def lean(z):
    """The greenhouse's half width at `z`, as a fraction of its belt width."""
    return 1.0 - LEAN * max(0.0, min(1.0, (z - BELT_Z) / (ROOF_Z - BELT_Z)))


def slab(name, pts, normal, mat, lods=TIERS, thick=0.03):
    """A thin sheet through `pts` (a face's outline, in order), half of `thick` proud
    of the surface it lies on along `normal`."""
    n = Vector(normal).normalized() * (thick / 2)

    def build(bm, lod):
        lo = [bm.verts.new(Vector(p) - n) for p in pts]
        hi = [bm.verts.new(Vector(p) + n) for p in pts]
        bm.faces.new(lo[::-1])
        bm.faces.new(hi)
        for i in range(len(pts)):
            j = (i + 1) % len(pts)
            bm.faces.new((lo[i], lo[j], hi[j], hi[i]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    return mesh_part(name, build, mat, car, lods=lods)


def along(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


# ------------------------------------------------------------------ body and greenhouse
for lod, seg in zip(TIERS, (12, 8, 5, 3)):
    prism("body", body_profile(seg), 2 * BODY_Y, (0, 0, 0), paint, car, bevel=0.04, lods=(lod,))
box("floor_pan", (4.1, 1.36, 0.5), (0, 0, 0.5), under, car)
prism("cabin", CABIN, 2 * CABIN_Y, (0, 0, 0), paint, car, bevel=0.03, taper_y=lean)

# glass: the windscreen and the tailgate's window across the car, three windows a side
for name, foot, head in (("windscreen", SCREEN_FOOT, SCREEN_HEAD), ("rear_window", GATE_FOOT, GATE_HEAD)):
    (x0, z0), (x1, z1) = along(foot, head, 0.1), along(foot, head, 0.9)
    y0, y1 = CABIN_Y * lean(z0) - 0.07, CABIN_Y * lean(z1) - 0.07
    up = Vector((x1 - x0, 0, z1 - z0))
    out = Vector((0, 1, 0)).cross(up) if name == "windscreen" else up.cross(Vector((0, 1, 0)))
    slab(name, [(x0, -y0, z0), (x0, y0, z0), (x1, y1, z1), (x1, -y1, z1)], out, glass)
SIDE_WINDOWS = (
    ("front", [(0.8, 0.95), (0.25, 1.36), (-0.28, 1.385), (-0.28, 0.95)]),
    ("rear", [(-0.36, 0.95), (-0.36, 1.385), (-1.2, 1.375), (-1.2, 0.95)]),
    ("quarter", [(-1.28, 0.95), (-1.28, 1.37), (-1.5, 1.36), (-1.95, 0.97)]),
)
for s, sn in ((1, "L"), (-1, "R")):
    side_normal = (0, s * (ROOF_Z - BELT_Z), CABIN_Y * LEAN)
    for name, outline in SIDE_WINDOWS:
        slab(f"window_{name}_{sn}", [(x, s * CABIN_Y * lean(z), z) for x, z in outline], side_normal, glass)

# ------------------------------------------------------------------ trim, lamps, mirrors
for name, x in (("front", 2.145), ("rear", -2.145)):
    box(f"bumper_{name}", (0.11, 2 * BODY_Y - 0.02, 0.17), (x, 0, 0.38), trim, car, bevel=0.03)
# a dark band across the nose carries the grille and sets the lamps off from the paint
box("grille", (0.04, 2 * BODY_Y - 0.1, 0.15), (2.165, 0, 0.63), trim, car, lods=(0, 1, 2))
for s, sn in ((1, "L"), (-1, "R")):
    box(f"headlamp_{sn}", (0.06, 0.34, 0.11), (2.175, s * 0.6, 0.63), lamp, car, bevel=0.015, lods=(0, 1, 2))
    box(f"tail_lamp_{sn}", (0.06, 0.2, 0.3), (-2.17, s * 0.7, 0.74), tail, car, bevel=0.015, lods=(0, 1, 2))
    box(f"mirror_{sn}", (0.1, 0.16, 0.09), (0.86, s * 0.88, 0.98), paint, car, bevel=0.02, lods=(0, 1))
    box(f"rub_strip_{sn}", (2.2, 0.02, 0.05), (0, s * (BODY_Y + 0.002), 0.56), trim, car, lods=(0, 1))
    for k, x in enumerate((0.05, -0.95)):
        box(f"door_handle_{sn}_{k}", (0.13, 0.025, 0.03), (x, s * (BODY_Y + 0.004), 0.82), trim, car, lods=(0,))

# ------------------------------------------------------------------ wheels
for row, x in AXLES:
    for s, sn in ((1, "L"), (-1, "R")):
        n = f"wheel_{row}{sn}"
        at = (x, s * WHEEL_Y, TYRE_R)
        cyl(n + "_tyre", TYRE_R, TYRE_W, at, "Y", rubber, car, seg=24, bevel=0.03)
        cyl(n + "_rim", 0.2, TYRE_W + 0.006, at, "Y", wheel_m, car, seg=16, lods=(0, 1, 2))
        cyl(n + "_hub", 0.07, TYRE_W + 0.02, at, "Y", trim, car, seg=8, lods=(0,))

# ------------------------------------------------------------------ the wreck
if WRECK:
    from wreckage import dent, densify, heat, hollow, parts, plate, remove, warp

    # burnt out: the glass, the lamps and the tyres are gone and the roof came down into
    # the cabin, so the shell stands no higher than its belt, settled on its rims
    remove("cabin", "windscreen", "rear_window", "window_", "headlamp_", "tail_lamp_", "mirror_", "rub_strip_",
           "door_handle_", "grille", "bumper_")
    for o in list(bpy.data.objects):
        if o.type == "MESH" and "_tyre_LOD" in o.name:
            bpy.data.objects.remove(o, do_unlink=True)
    # the cabin is an open tub down to the floor pan; the roof lies in it, caved in at the
    # middle, on the stubs of its pillars
    hollow(parts("body"), Matrix.Translation((-0.52, 0, 1.0)), (2.75, 2 * BODY_Y - 0.14, 0.6))
    plate("roof_fallen", [(-1.3, -0.64), (1.2, -0.7), (1.28, 0.66), (0.2, 0.72), (-1.25, 0.62)], 0.03, (-0.55, 0.0, 0.83),
          (0.03, -0.02, 0.05), paint, car, curl=0.015, seed=31)
    densify(parts("roof_fallen"), 2.0)
    warp(parts("roof_fallen"), dent((-0.6, 0.1, 0.83), 0.9, 0.08, (0, 0, -1), 7.0))
    for k, (x, s, lean_back) in enumerate(((0.86, 1, -0.5), (0.86, -1, -0.5), (-1.86, 1, 0.4), (-1.86, -1, 0.4))):
        box(f"pillar_stub_{k}", (0.05, 0.05, 0.2), (x, s * 0.74, 0.88), paint, car, rot=(s * 0.8, lean_back, 0),
            lods=(0, 1, 2))
    for row, x in AXLES:
        for s, sn in ((1, "L"), (-1, "R")):
            cyl(f"wheel_{row}{sn}_carcass", 0.27, TYRE_W - 0.04, (x, s * WHEEL_Y, TYRE_R), "Y", rubber, car, seg=20,
                caps=False, lods=(0, 1, 2))
    panels = parts("body")
    densify(panels, 2.2)
    warp(panels, heat(0.025, 0.6, 5.0), dent((1.55, 0.25, 0.85), 0.7, 0.09, (0, 0, -1), 3.0),
         dent((-2.2, -0.5, 0.7), 0.6, 0.07, (1, 0, 0), 5.0))
    # crushed and settled: the belt comes down to the wreck's height, the rims onto the ground
    squash = 0.93
    warp(parts(""), lambda p: Vector((0, 0, (squash - 1) * p.z)))
    car.location = (0, 0, WRECK_TOP - 0.93 * squash)
    car.rotation_euler = (math.radians(1.5), math.radians(-1.0), 0)
    bpy.context.view_layer.update()
    for vent, reach in (((-0.5, 0.0, 0.8), 1.5), ((1.5, 0.0, 0.8), 1.2)):
        SCORCH.append((car.matrix_world @ Vector(vent), reach))

rest_on_ground(0.006 if WRECK else 0.0)
bpy.context.view_layer.update()
finish(ao_distance=1.0)
half = [HALF[0], HALF[1], WRECK_TOP / 2 if WRECK else HALF[2]]
print("PROP", json.dumps(dict(kind="car_wreck" if WRECK else "parked_car", box=half, tris=triangles_by_tier())))
export(OUT)
