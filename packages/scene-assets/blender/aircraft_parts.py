"""Reusable aircraft and rotorcraft parts: fuselages, flying surfaces, canopies,
intakes, nozzles, landing gear, pylons and stores, rotors, skids, and the
wreck every airframe comes down as.

The sibling of `vehicle_parts.py`, on the same contracts (read its doc): a
part is a pure Blender function on `parts.py` primitives, takes a unique
`name`, a placement in its parent's frame, sizes in metres, a material set
`mats` (role -> material) and a `parent`; each piece is made per tier and
fine detail drops out first; each material is its own object.

Engine basis: +X forward, +Y left, Z up, the origin on the ground under the
airframe's centre. An airframe stands on its gear (or skids) on the ground.

Nodes an airframe makes, for the mechanics that will move them (deferred):
`gear_*` (each landing gear leg, retractable) carrying its `wheel_*`
(spinning, as a vehicle's), `skid_L`/`skid_R` (what a skid airframe stands
on), `rotor_*` (a rotor head and its blades, turning about its local Z).
Static parts make no empty.

Roles the parts read, beyond `vehicle_parts`' ('paint', 'dark', 'steel',
'black', 'rubber', 'glass', 'lamp'), which `fit` adds to a vehicle's set:
'gear' (gear legs and bays, in the light enamel they are painted), 'nozzle'
(heat-stained engine metal), 'store' (missiles, bombs and pods in their own
greys), 'blade' (rotor blades, dark painted composite).
"""
import math
import os
import sys

import bmesh
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import SEG_SCALE, box, cyl, empty, mesh_part, textured  # noqa: E402
from vehicle_parts import ALL, FINE, MID, NEAR, _tube_part, _wheel_node  # noqa: E402

# Interpolated rings per station interval, per tier: a fuselage keeps its
# curves at the near tiers and its stations alone at the far ones.
BODY_STEPS = (3, 2, 1, 1)
# Chord samples of a flying surface's section, per tier (fractions of chord).
CHORD = ((0.0, 0.03, 0.1, 0.22, 0.38, 0.55, 0.72, 0.88, 1.0),
         (0.0, 0.08, 0.3, 0.6, 1.0),
         (0.0, 0.25, 1.0),
         (0.0, 0.3, 1.0))


def fit(v, gear=(0.42, 0.42, 0.41), store=(0.3, 0.31, 0.3), fittings=(0.05, 0.052, 0.055)):
    """Add the aircraft roles to `v.mats` and make its fittings a dark grey,
    not a ground vehicle's olive. `gear` is the gear and bay enamel, `store`
    the stores' grey."""
    v.mats.update(
        dark=textured("fittings_grey", "olive_paint", colour=fittings, chip=0.3, dirt=0.3, role="paint"),
        gear=textured("gear_enamel", "enamel", colour=gear, chip=0.4, dirt=0.5, role="paint"),
        nozzle=textured("nozzle_metal", "bare_steel", colour=(0.07, 0.065, 0.06), chip=0.2, dirt=0.1, role="steel"),
        store=textured("store_paint", "enamel", colour=store, chip=0.3, dirt=0.2, role="paint"),
        blade=textured("blade_paint", "olive_paint", colour=(0.03, 0.032, 0.033), chip=0.2, dirt=0.1, role="paint"),
    )
    return v.mats


def mirrored(make):
    """Call `make(side, suffix)` for the left (+Y, "L") and right ("R") side."""
    return [make(side, s) for side, s in ((1, "L"), (-1, "R"))]


# ---------------------------------------------------------------- lofted bodies
def _station(s):
    x, w, lo, hi = s[:4]
    mid = s[4] if len(s) > 4 and s[4] is not None else (lo + hi) / 2
    p = s[5] if len(s) > 5 else 2.0
    return [x, w, lo, hi, mid, p]


def _spline(stations, steps):
    """Stations with `steps` Catmull-Rom rings per interval (x stays linear)."""
    if steps <= 1:
        return stations
    out = []
    n = len(stations)
    for i in range(n - 1):
        p0, p1, p2, p3 = (stations[max(i - 1, 0)], stations[i], stations[i + 1], stations[min(i + 2, n - 1)])
        for k in range(steps):
            t = k / steps
            row = [p1[0] + (p2[0] - p1[0]) * t]
            for j in range(1, 6):
                a, b, c, d = p0[j], p1[j], p2[j], p3[j]
                val = 0.5 * ((2 * b) + (-a + c) * t + (2 * a - 5 * b + 4 * c - d) * t * t + (-a + 3 * b - 3 * c + d) * t ** 3)
                lo_, hi_ = min(b, c), max(b, c)
                row.append(min(max(val, lo_ - 0.25 * (hi_ - lo_)), hi_ + 0.25 * (hi_ - lo_)))
            row[1] = max(row[1], 0.0)
            out.append(row)
    out.append(stations[-1])
    return out


def _ring(st, n):
    x, w, lo, hi, mid, p = st
    pts = []
    for k in range(n):
        a = math.tau * k / n
        c, s = math.cos(a), math.sin(a)
        cy = math.copysign(abs(c) ** (2 / p), c)
        sz = math.copysign(abs(s) ** (2 / p), s)
        z = mid + (hi - mid) * sz if sz >= 0 else mid + (mid - lo) * sz
        pts.append((x, w * cy, z))
    return pts


def body(name, stations, mat, parent, seg=24, lods=ALL, loc=(0, 0, 0), rot=(0, 0, 0), steps=BODY_STEPS):
    """A smooth body lofted along X through `stations`, each
    `(x, half_width, bottom_z, top_z[, widest_z[, power]])`: a superellipse
    section (power 2 an ellipse, higher squarer, below 2 a chined diamond)
    widest at `widest_z`. A station of no width closes the body to a point;
    otherwise its ends are capped. Fuselages, nacelles, booms, stores."""
    st = [_station(s) for s in stations]

    def build(bm, lod):
        n = max(6, int(round(seg * SEG_SCALE[lod] / 2)) * 2)
        rows = _spline(st, steps[lod])
        rings = []
        for row in rows:
            if row[1] < 1e-3:
                rings.append([bm.verts.new((row[0], 0.0, row[4]))])
            else:
                rings.append([bm.verts.new(p) for p in _ring(row, n)])
        for a, b in zip(rings, rings[1:]):
            if len(a) == 1 and len(b) == 1:
                continue
            if len(a) == 1:
                for k in range(n):
                    bm.faces.new((a[0], b[(k + 1) % n], b[k]))
            elif len(b) == 1:
                for k in range(n):
                    bm.faces.new((a[k], a[(k + 1) % n], b[0]))
            else:
                for k in range(n):
                    bm.faces.new((a[k], a[(k + 1) % n], b[(k + 1) % n], b[k]))
        for f in bm.faces:
            f.smooth = True
        for ring in (rings[0], rings[-1]):
            if len(ring) > 1:
                cap = bm.faces.new(ring)
                cap.smooth = False
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    return mesh_part(name, build, mat, parent, lods=lods, loc=loc, rot=rot)


# ---------------------------------------------------------------- flying surfaces
def _thickness(c):
    """A symmetric section's thickness at chord fraction `c`, 1 at its thickest."""
    t = 5 * (0.2969 * math.sqrt(c) - 0.126 * c - 0.3516 * c * c + 0.2843 * c ** 3 - 0.1036 * c ** 4)
    return max(t / 0.5, 0.02)


def surface(name, sections, mat, parent, lods=ALL, loc=(0, 0, 0), rot=(0, 0, 0)):
    """A flying surface (wing, tailplane, fin, canard, rotor blade) through
    `sections`, each `(x_le, y, z, chord, thickness)` in metres, root first:
    a symmetric aerofoil swept back from its leading edge, thick across the
    plane its chords and span lie in, so a fin, a canted fin or an anhedral
    tailplane is the same call."""
    secs = [(Vector((x, y, z)), c, t) for x, y, z, c, t in sections]

    def build(bm, lod):
        fr = CHORD[lod]
        rings = []
        for i, (le, chord, thick) in enumerate(secs):
            a = secs[max(i - 1, 0)][0]
            b = secs[min(i + 1, len(secs) - 1)][0]
            span = (b - a).normalized()
            normal = Vector((1, 0, 0)).cross(span).normalized()
            upper = [le + Vector((-chord * c, 0, 0)) + normal * (thick / 2 * _thickness(c) if 0 < c < 1 else 0)
                     for c in fr]
            lower = [le + Vector((-chord * c, 0, 0)) - normal * (thick / 2 * _thickness(c)) for c in fr[1:-1]]
            rings.append([bm.verts.new(p) for p in upper + lower[::-1]])
        n = len(rings[0])
        for ra, rb in zip(rings, rings[1:]):
            for k in range(n):
                bm.faces.new((ra[k], ra[(k + 1) % n], rb[(k + 1) % n], rb[k]))
        for f in bm.faces:
            f.smooth = True
        bm.faces.new(rings[0])
        bm.faces.new(rings[-1])
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    return mesh_part(name, build, mat, parent, lods=lods, loc=loc, rot=rot)


def paired_surface(name, sections, mat, parent, lods=ALL):
    """A surface and its mirror across the centreline (`<name>_L`, `<name>_R`);
    `sections` describe the left (+Y) one."""
    return mirrored(lambda side, s: surface(f"{name}_{s}", [(x, side * y, z, c, t) for x, y, z, c, t in sections],
                                            mat, parent, lods=lods))


def fin(name, root_x, root_z, height, root_chord, tip_chord, sweep_m, mats, parent, y=0.0, cant=0.0, thick=0.12,
        rudder=True):
    """A vertical fin from its root's leading edge (`root_x`, `y`, `root_z`):
    `sweep_m` back to the tip's leading edge, canted outboard by `cant`
    radians toward its side, with a rudder hinge line at the near tiers."""
    side = 1 if y >= 0 else -1
    tip = (root_x - sweep_m, y + side * height * math.sin(cant), root_z + height * math.cos(cant))
    made = surface(name, [(root_x, y, root_z, root_chord, thick * root_chord),
                          (tip[0], tip[1], tip[2], tip_chord, thick * tip_chord * 0.8)], mats["paint"], parent)
    if rudder:
        # The rudder's hinge: a dark seam a quarter of the way from the trailing edge.
        sx = root_x - root_chord * 0.72
        tx = tip[0] - tip_chord * 0.72
        length = math.hypot(tx - sx, tip[2] - root_z)
        ang = math.atan2(sx - tx, tip[2] - root_z)
        box(f"{name}_hinge", (0.03, thick * root_chord * 0.9, length * 0.86),
            ((sx + tx) / 2, (y + tip[1]) / 2, (root_z + tip[2]) / 2), mats["black"], parent,
            rot=(-side * cant, -ang, 0), lods=NEAR)
    return made


# ---------------------------------------------------------------- canopy and fittings
def canopy(name, x_front, x_back, sill, top, half_width, mats, parent, bows=(), peak=0.45, lods=ALL):
    """A bubble canopy on the sill line from `x_front` to `x_back`, highest
    (`top`) at `peak` of the way back, in dark glass, with frame bows (in the
    paint) at the X positions `bows` and a sill rail along each side."""
    length = x_front - x_back
    rise = top - sill
    prof = ((0.0, 0.05, 0.35), (0.12, 0.45, 0.7), (0.3, 0.85, 0.95), (peak, 1.0, 1.0), (0.75, 0.86, 0.94),
            (1.0, 0.45, 0.7))
    st = [(x_front - f * length, half_width * wf, sill - 0.04, sill + rise * hf, sill) for f, hf, wf in prof]
    made = body(f"{name}_glass", st, mats["glass"], parent, seg=20, lods=lods)
    for k, bx in enumerate(bows):
        f = (x_front - bx) / length
        # The canopy's height and width at the bow, from its profile.
        for (f0, h0, w0), (f1, h1, w1) in zip(prof, prof[1:]):
            if f0 <= f <= f1:
                t = (f - f0) / (f1 - f0)
                hf, wf = h0 + (h1 - h0) * t, w0 + (w1 - w0) * t
                break
        else:
            continue
        body(f"{name}_bow_{k}", [(bx + 0.04, half_width * wf * 1.05, sill - 0.02, sill + rise * hf * 1.04, sill),
                                 (bx - 0.04, half_width * wf * 1.05, sill - 0.02, sill + rise * hf * 1.04, sill)],
             mats["dark"], parent, seg=16, lods=MID, steps=(1, 1, 1, 1))
    mirrored(lambda side, s: box(f"{name}_sill_{s}", (length * 0.95, 0.05, 0.05),
                                 (x_back + length / 2, side * half_width * 0.95, sill + 0.01), mats["dark"], parent,
                                 lods=NEAR))
    return made


def intake(name, loc, width, height, depth, mats, parent, rot=(0, 0, 0), lods=MID):
    """An air intake's mouth facing +X: a lip in the paint and the dark duct
    behind it (`width`, `height` the opening, `depth` the lip's length)."""
    x, y, z = loc
    box(f"{name}_lip", (depth, width + 0.1, height + 0.1), loc, mats["paint"], parent, bevel=0.03, rot=rot)
    box(f"{name}_duct", (0.04, width, height), (x + depth / 2 - 0.01, y, z), mats["black"], parent, rot=rot,
        lods=lods)


def round_intake(name, loc, radius, depth, mats, parent, lods=MID):
    """A round intake facing +X: a lip ring in the paint and the dark duct."""
    cyl(f"{name}_lip", radius + 0.05, depth, loc, "X", mats["paint"], parent, seg=20, bevel=0.02)
    cyl(f"{name}_duct", radius, 0.04, (loc[0] + depth / 2 - 0.01, loc[1], loc[2]), "X", mats["black"], parent,
        seg=20, lods=lods)


def nozzle(name, loc, r_front, r_exit, length, mats, parent, petals=0):
    """A jet pipe from `loc` aft along -X, `r_front` to `r_exit`: heat-stained
    metal, the dark pipe inside and, given `petals`, the exhaust petals'
    seams round the exit."""
    x, y, z = loc
    cyl(f"{name}_pipe", r_exit, length, (x - length / 2, y, z), "X", mats["nozzle"], parent, seg=24, r2=r_front,
        caps=False, min_seg=8)
    cyl(f"{name}_bore", r_exit * 0.88, 0.05, (x - length + 0.04, y, z), "X", mats["black"], parent, seg=20, lods=MID)
    for k in range(petals):
        a = math.tau * k / petals
        box(f"{name}_petal_{k}", (length * 0.45, 0.025, 0.02),
            (x - length * 0.75, y + math.cos(a) * r_exit * 0.98, z + math.sin(a) * r_exit * 0.98), mats["dark"],
            parent, rot=(a, 0, 0), lods=FINE)


def blade_antenna(name, loc, height, mats, parent, chord=0.18, down=False):
    """A swept blade antenna standing `height` off the skin at `loc`, or
    hanging under it (`down`)."""
    h = -height if down else height
    surface(name, [(0.0, 0.0, 0.0, chord, 0.025), (-chord * 0.6, 0.0, h, chord * 0.5, 0.015)], mats["dark"], parent,
            lods=NEAR, loc=loc)


def pitot(name, loc, length, mats, parent):
    """A probe forward from the nose."""
    cyl(name, 0.025, length, (loc[0] + length / 2, loc[1], loc[2]), "X", mats["steel"], parent, seg=8, r2=0.012,
        lods=MID)


def sensor_ball(name, loc, radius, mats, parent, window=0.55):
    """A turreted sensor ball (targeting, night vision): a painted sphere
    with its dark window facing forward."""
    body(f"{name}_ball", [(loc[0] + radius, 0.0, loc[2], loc[2]), (loc[0] + radius * 0.7, radius * 0.72,
                                                                  loc[2] - radius * 0.72, loc[2] + radius * 0.72),
                          (loc[0], radius, loc[2] - radius, loc[2] + radius),
                          (loc[0] - radius * 0.7, radius * 0.72, loc[2] - radius * 0.72, loc[2] + radius * 0.72),
                          (loc[0] - radius, 0.0, loc[2], loc[2])], mats["dark"], parent, seg=16,
         loc=(0, loc[1], 0))
    box(f"{name}_window", (0.03, radius * window * 1.4, radius * window), (loc[0] + radius * 0.96, loc[1], loc[2]),
        mats["glass"], parent, lods=MID)


# ---------------------------------------------------------------- gear
def wheel(name, loc, radius, width, mats, parent, lods=ALL):
    """A gear wheel, the spinning node `name` (`wheel_*`, as `vehicle_parts`'
    wheels): a black tyre on a hub in the gear's enamel."""
    node = _wheel_node(name, loc, radius, parent)
    cyl(f"{name}_tyre", radius, width, (0, 0, 0), "Y", mats["rubber"], node, seg=20, bevel=0.02, lods=lods)
    cyl(f"{name}_hub", radius * 0.55, width + 0.02, (0, 0, 0), "Y", mats["gear"], node, seg=14,
        lods=tuple(t for t in lods if t < 3))
    return node


def gear(name, x, y, top, radius, width, mats, parent, wheels=1, spread=None, tandem=0.0, door=None, rake=0.0):
    """A landing gear leg: the node `gear_<name>` at its top (`x`, `y`,
    `top`), the strut down to wheels resting on the ground. `wheels` side by
    side `spread` apart (2 for a twin nose wheel), or with `tandem` two axles
    that far apart; `door` (length, height) a bay door beside the leg."""
    node = empty(f"gear_{name}", loc=(x, y, top), parent=parent)
    drop = top - radius
    cyl(f"gear_{name}_strut", 0.06 + radius * 0.12, drop, (0, 0, -drop / 2), "Z", mats["gear"], node, seg=10,
        rot=(0, rake, 0), lods=MID)
    cyl(f"gear_{name}_oleo", 0.04 + radius * 0.08, drop * 0.4, (0, 0, -drop * 0.8), "Z", mats["steel"], node, seg=10,
        lods=NEAR)
    spread = spread if spread is not None else width + 0.06
    axles = [0.0] if not tandem else [tandem / 2, -tandem / 2]
    for i, ax in enumerate(axles):
        for k in range(wheels):
            off = (k - (wheels - 1) / 2) * spread
            wheel(f"wheel_{name}_{i}{k}", (ax, off, -drop), radius, width, mats, node)
        if wheels > 1 or tandem:
            cyl(f"gear_{name}_axle_{i}", 0.035, spread * (wheels - 1) + width, (ax, 0, -drop), "Y", mats["steel"], node,
                seg=8, lods=NEAR)
    if tandem:
        box(f"gear_{name}_bogie", (tandem + 0.1, 0.1, 0.1), (0, 0, -drop), mats["gear"], node, lods=MID)
    if door:
        side = 1 if y >= 0 else -1
        box(f"gear_{name}_door", (door[0], 0.025, door[1]), (0, side * (width / 2 + 0.12), -door[1] / 2),
            mats["paint"], node, bevel=0.01, lods=MID)
    return node


def pylon(name, loc, length, depth, mats, parent):
    """A store pylon hanging `depth` below `loc`, `length` long."""
    x, y, z = loc
    box(name, (length, 0.1, depth), (x, y, z - depth / 2), mats["paint"], parent, bevel=0.02, taper=(1.15, 1.0),
        lods=MID)


def store(name, kind, loc, length, radius, mats, parent, fins=True):
    """A store hung with its nose at +X, centred on `loc`: `missile` (a slim
    body, a pointed seeker and cruciform fins), `bomb` (a fat body and its
    tail fins), `tank` (a drop tank) or `pod` (a sensor pod with its window)."""
    x, y, z = loc
    h = length / 2
    nose = {"missile": 0.12, "bomb": 0.25, "tank": 0.3, "pod": 0.15}[kind]
    tail = {"missile": 0.98, "bomb": 0.8, "tank": 0.75, "pod": 0.9}[kind]
    st = [(x + h, 0.0, z, z), (x + h - length * nose, radius, z - radius, z + radius),
          (x - h + length * (1 - tail), radius, z - radius, z + radius),
          (x - h, radius * (0.4 if kind != "missile" else 0.95), z - radius * 0.5, z + radius * 0.5)]
    body(f"{name}_body", st, mats["store"], parent, seg=12, lods=MID if kind == "missile" else ALL,
         loc=(0, y, 0), steps=(2, 1, 1, 1))
    if kind == "missile":
        cyl(f"{name}_seeker", radius * 0.7, 0.02, (x + h - 0.03, y, z), "X", mats["glass"], parent, seg=8, lods=FINE)
    if kind == "pod":
        box(f"{name}_window", (0.03, radius * 1.1, radius * 1.1), (x + h - length * nose * 0.4, y, z),
            mats["glass"], parent, lods=MID)
    if fins and kind in ("missile", "bomb"):
        span = radius * (2.4 if kind == "missile" else 1.8)
        for k in range(4):
            a = math.pi / 4 + k * math.pi / 2
            box(f"{name}_fin_{k}", (length * 0.12, 0.012, span),
                (x - h + length * 0.08, y + math.cos(a) * span / 2, z + math.sin(a) * span / 2), mats["store"], parent,
                rot=(a - math.pi / 2, 0, 0), lods=FINE)


# ---------------------------------------------------------------- rotors
def rotor(name, loc, radius, blades, chord, mats, parent, hub=0.3, mast=0.4, droop=0.03, phase=0.0, rot=(0, 0, 0),
          thick=0.1, tip_chord=None):
    """A rotor: the node `rotor_<name>` at the hub's centre, turning about its
    local Z; the mast below it, the hub and its blade grips, and `blades`
    blades of `chord` out to `radius`, drooping `droop` radians (a rotor at
    rest sags), each `blade_<name>_<k>` (a frame measure leaves blades out:
    a rotor's disc is not its airframe's size). A tail rotor is the same
    rotor turned on its side (`rot`)."""
    node = empty(f"rotor_{name}", loc=loc, parent=parent, rot=rot)
    if mast:
        cyl(f"rotor_{name}_mast", hub * 0.42, mast, (0, 0, -mast / 2), "Z", mats["dark"], node, seg=12, lods=MID)
    cyl(f"rotor_{name}_hub", hub, hub * 0.55, (0, 0, 0), "Z", mats["dark"], node, seg=14, bevel=0.02)
    cyl(f"rotor_{name}_cap", hub * 0.55, hub * 0.4, (0, 0, hub * 0.45), "Z", mats["steel"], node, seg=12, lods=MID)
    tip = tip_chord if tip_chord is not None else chord
    for k in range(blades):
        a = phase + math.tau * k / blades
        ca, sa = math.cos(a), math.sin(a)
        # The blade along its own +X from the hub, its leading edge forward of the axis.
        root, out = hub * 0.9, radius
        dz = -(out - root) * math.sin(droop)
        sec = [(chord / 2, root, 0.0, chord, chord * thick), (chord / 2, (root + out) / 2, dz / 2 - 0.0, chord,
                                                              chord * thick * 0.9),
               (tip / 2, out, dz, tip, tip * thick * 0.8)]
        # surface() sweeps chords along -X: lay the blade on Y, then turn it to its bearing.
        surface(f"blade_{name}_{k}", [(x, y, z, c, t) for x, y, z, c, t in sec], mats["blade"], node,
                rot=(0, 0, a - math.pi / 2))
        box(f"rotor_{name}_grip_{k}", (hub * 1.1, chord * 0.7, hub * 0.4),
            (ca * hub * 1.2, sa * hub * 1.2, 0), mats["dark"], node, rot=(0, 0, a), bevel=0.01, lods=MID)
    return node


def skids(x_front, x_back, y, cross, top, mats, parent, radius=0.045):
    """Landing skids, the nodes `skid_L` and `skid_R` (what a skid airframe
    stands on, as a wheel is): a tube each side at `y` from `x_back` to
    `x_front` (its toe turned up), on the ground, and the cross tubes at the
    X positions `cross` arching up to the belly at `top`."""
    def path(lod, side):
        steps = (5, 3, 2, 1)[lod]
        toe = [(x_front + 0.25 * math.sin(math.pi / 2 * j / steps), side * y,
                radius + 0.25 * (1 - math.cos(math.pi / 2 * j / steps))) for j in range(1, steps + 1)]
        return [(x_back, side * y, radius), (x_front, side * y, radius)] + toe

    for side, s in ((1, "L"), (-1, "R")):
        node = empty(f"skid_{s}", parent=parent)
        _tube_part(f"skid_{s}_tube", lambda lod, side=side: path(lod, side), radius, mats["dark"], node, ALL)
    for k, x in enumerate(cross):
        def arch(lod, x=x):
            steps = (6, 4, 2, 2)[lod]
            return [(x, y * math.cos(math.pi * j / steps), radius + (top - radius) * math.sin(math.pi * j / steps) ** 0.6)
                    for j in range(steps + 1)]
        _tube_part(f"skid_cross_{k}", arch, radius * 1.1, mats["dark"], parent, MID)


# ---------------------------------------------------------------- wreck
def crash(v, tail_x, wing_y=None, wing_side=-1, tail_yaw=0.35, tail_drop=0.18, blades_broken=(), seed=0):
    """The airframe after it came down and burnt, before `wreckage.burn`: gear
    torn away and the belly on the ground, the fuselage warped and dented,
    the tail behind `tail_x` broken and slewed by `tail_yaw`, the wing on
    `wing_side` folded down outboard of `wing_y`, the canopy glass gone, the
    rotor blades drooping to the ground and those named by `blades_broken`
    (`(rotor, blade index)`) snapped off and lying beside it."""
    import bpy
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, warp
    m = v.mats
    gone = [o for o in bpy.data.objects if o.name.startswith(("gear_", "canopy_glass"))]
    gone += [c for o in gone for c in o.children_recursive]
    for rotor_name, k in blades_broken:
        gone += [o for o in bpy.data.objects if o.name.startswith(f"blade_{rotor_name}_{k}_")]
    # In order, once each: a set's order follows object addresses, which vary run to run.
    for o in dict.fromkeys(gone):
        bpy.data.objects.remove(o, do_unlink=True)
    bpy.context.view_layer.update()
    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    low = min((o.matrix_world @ vert.co).z for o in meshes for vert in o.data.vertices)
    v.root.location.z -= low
    bpy.context.view_layer.update()
    shell = parts("fuselage", "tail_", "wing_", "nacelle", "boom", "nose")
    densify(shell, scale=1.6)
    warp(shell, heat(0.05, 1.4, seed=seed + 3.0), dent((v.length * 0.3, 0, v.height * 0.25), 1.2, -0.25))
    everything = [o for o in bpy.data.objects if o.type == "MESH"]
    mid_z = v.height * 0.3
    # The tail broke behind `tail_x`: slewed aside and down onto the ground.
    bend(everything, (tail_x, 0, mid_z), (0, 0, 1), (-1, 0, 0), tail_yaw)
    bend(everything, (tail_x, 0, mid_z), (0, 1, 0), (-1, 0, 0), -tail_drop)
    if wing_y is not None:
        s = "L" if wing_side > 0 else "R"
        bend(parts(f"wing_{s}", f"wing_outer_{s}"), (0, wing_side * wing_y, mid_z), (1, 0, 0), (0, wing_side, 0),
             -wing_side * 0.5)
    # Rotor blades sag to the ground.
    for o in everything:
        if o.name.startswith(("blade_main_", "blade_upper_", "blade_lower_", "blade_front_", "blade_rear_")):
            hub = o.parent.matrix_world.translation
            out = (o.matrix_world @ Vector((0, 1, 0)) - o.matrix_world.translation).normalized()
            axis = out.cross(Vector((0, 0, 1)))
            k = int(o.name.split("_")[2])
            bend([o], hub + out * 0.6, axis, out, -(0.22 + 0.05 * (k % 3)))
    for k, (dx, dy, size) in enumerate(((0.2, 1.1, 0.6), (-0.35, -1.3, 0.5), (-0.1, 0.8, 0.45))):
        plate(f"debris_{k}", [(-size, -size * 0.5), (size, -size * 0.6), (size * 0.8, size * 0.5),
                              (-size * 0.6, size * 0.7)], 0.03,
              (v.length * dx, v.width * 0.5 * dy, 0.03), (0.04 * k, -0.03, 0.7 + k * 1.3), m["paint"], v.root,
              curl=0.1, seed=seed + 71 + k)
    for rotor_name, k in blades_broken:
        plate(f"blade_debris_{rotor_name}_{k}", [(-2.4, -0.12), (2.6, -0.1), (2.5, 0.14), (-2.3, 0.12)], 0.04,
              (-v.length * 0.1 + k * 0.6, (1 if k % 2 else -1) * (v.width * 0.5 + 1.5 + k * 0.4), 0.04),
              (0, 0.02, 0.4 + k * 1.1), m["blade"], v.root, seed=seed + 91 + k)
    v.root.rotation_euler = (0.06 * (1 if seed % 2 else -1), 0.03, 0)
    rest_on_ground(0.004)


# ---------------------------------------------------------------- a whole airframe
def _both(rows, at):
    """Rows whose location (item `at`) is off the centreline, and their mirrors."""
    out = []
    for row in rows:
        out.append(row)
        loc = row[at]
        if abs(loc[1]) > 1e-6:
            out.append(tuple((loc[0], -loc[1], loc[2]) if i == at else r for i, r in enumerate(row)))
    return out

def jet(v, spec):
    """A fixed-wing airframe from its family's `spec` (all optional but the
    fuselage), every part named so `crash` finds it: `fuselage` (stations,
    `seg`), `bodies` (more bodies: `(name, stations[, y[, seg]])`, off the
    centreline by `y`), `canopy` (the
    `canopy()` arguments), `wing`, `strake`, `canard`, `stab` (left-side
    sections, mirrored), `fins` (`fin()` keyword sets), `intakes`
    (`(kind, loc, size..)`), `nozzles` (`nozzle()` keyword sets), `gear`
    (`gear()` keyword sets), `pylons` (`(loc, length, depth)`), `stores`
    (`(kind, loc, length, radius)`; an off-centre pylon or store is
    mirrored), `nose_probe` (`(loc, length)`)."""
    m, hull = fit(v), v.hull
    body("fuselage", spec["fuselage"], m["paint"], hull, seg=spec.get("seg", 28))
    for name, stations, *at in spec.get("bodies", ()):
        y, seg = (list(at) + [0.0, 18][len(at):])[:2]
        body(name, stations, m["paint"], hull, seg=seg, loc=(0, y, 0))
    if "canopy" in spec:
        canopy("canopy", mats=m, parent=hull, **spec["canopy"])
    # Wings and their strakes and canards are `wing*` (the crash folds them),
    # the tailplane is `tail_stab` (it breaks off with the tail).
    for key, name in (("wing", "wing"), ("strake", "wing_strake"), ("canard", "wing_canard"), ("stab", "tail_stab")):
        if key in spec:
            paired_surface(name, spec[key], m["paint"], hull)
    for k, f in enumerate(spec.get("fins", ())):
        fin(f"tail_fin_{k}", mats=m, parent=hull, **f)
    for k, (kind, *args) in enumerate(spec.get("intakes", ())):
        if kind == "round":
            round_intake(f"intake_{k}", *args, mats=m, parent=hull)
        else:
            loc, w, h, d, *rot = args
            intake(f"intake_{k}", loc, w, h, d, m, hull, rot=rot[0] if rot else (0, 0, 0))
    for k, n in enumerate(spec.get("nozzles", ())):
        nozzle(f"nozzle_{k}", mats=m, parent=hull, **n)
    for g in spec.get("gear", ()):
        gear(mats=m, parent=hull, **g)
    for k, (loc, length, depth) in enumerate(_both(spec.get("pylons", ()), 0)):
        pylon(f"pylon_{k}", loc, length, depth, m, hull)
    for k, (kind, loc, length, radius) in enumerate(_both(spec.get("stores", ()), 1)):
        store(f"store_{k}", kind, loc, length, radius, m, hull)
    if "nose_probe" in spec:
        pitot("nose_probe", *spec["nose_probe"], m, hull)
    return m
