"""Build one infantry kind's body and kit on the rig, and export it as its
appearance source.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/infantry_kit.py rifle|recon|at|at_carried [a|b|c]

Writes assets/source/infantry/<kind>.glb (variant a) or <kind>_<variant>.glb:
the rig (scaled to the simulation's soldier height, not turned) and one skinned
mesh in four tiers (`soldier_LOD0..3`), plus the `eye` socket under `Head` and
the weapon's `muzzle` socket under `hand_r`.

Variants (`LOOKS`) are the same kind on the same rig and clips with another
head, kit and colouring: headgear, eyewear, vest colour, pack, pouches, skin
and hair. The battle picks one per soldier id (`AppearanceCatalog.resolve`), so
a squad never reads as copies of one man. The kind's own cue (the rifleman's
assault pack and whip, the recon ruck, the launcher) stays in every variant
that carries it.

Method (spike 03's, kept; the modelling is new):
- Clothing and soft kit are shells cut from the body, so they skin with the
  body for free: uniform, boots, gloves, plate carrier, belt, knee pads, cargo
  pockets, helmet, hair and eyewear. A shell smooths the anatomy away, pulls
  limbs toward round tubes (the UBC body is a superhero), then stands off the
  skin by a per-region offset, with a rim at its open edges.
- Hard kit (pouches, pack, radio, helmet fittings, the weapon) is modelled in
  place, flush on the shell it rides, and skinned 100% to one bone at join.
- The weapon sits where its low-ready hold puts it and rides `hand_r` in
  every clip of its hold family (`weapons.py`).
- Surface pattern is baked into vertex colour: camouflage, dirt that rises
  from the ground, wear, and ambient occlusion (Cycles, CPU, fixed seed).
- Materials that take the side colour carry a `tint` weight (`common.mat`).
"""

import math
import os
import random
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import weapons  # noqa: E402
from common import (REPO, box, cyl, empty, export_glb, fbm, mat, obj_from_bm, paint, script_args,  # noqa: E402
                    texture_mean, texture_uvs)
from infantry_rig import EYE_IN_HEAD, SCALE, Rig  # noqa: E402
from mesh_lods import canonical, make_tiers  # noqa: E402

# Linear albedo palettes: each part's hue, baked into vertex colour over its
# material's texture (the camouflage print itself is the texture's).
COYOTE = (0.13, 0.108, 0.075)
RANGER = (0.058, 0.064, 0.041)
DUST = (0.15, 0.13, 0.1)  # field dust, lighter than the cloth it films
BOOTS = (0.04, 0.031, 0.022)
LACES = (0.14, 0.11, 0.075)
PADS = (0.03, 0.03, 0.028)
GLOVES = (0.035, 0.034, 0.03)
SKIN = (0.4, 0.29, 0.23)
SCARF = (0.16, 0.14, 0.1)  # a sand shemagh

# Each variant's look. Per kind, `pack` and `head` may be overridden (`look_of`).
LOOKS = {
    "a": dict(head="nvg", eyewear=True, vest=RANGER, pouch=RANGER, pack="assault", radio=True, knee_pads=True,
              scarf=False, mags=3, skin=SKIN, hair=(0.04, 0.03, 0.022), stubble=((0.12, 0.08, 0.06), 0.55)),
    "b": dict(head="scrim", eyewear=False, vest=COYOTE, pouch=COYOTE, pack="hydration", radio=False, knee_pads=True,
              scarf=True, mags=2, skin=(0.2, 0.135, 0.1), hair=(0.012, 0.011, 0.01), stubble=((0.03, 0.022, 0.018), 0.4)),
    "c": dict(head="bare", eyewear=True, vest=RANGER, pouch=COYOTE, pack="none", radio=False, knee_pads=False,
              scarf=False, mags=3, skin=(0.5, 0.35, 0.28), hair=(0.13, 0.09, 0.05), stubble=((0.1, 0.065, 0.035), 0.85)),
}
KIND_LOOKS = {
    # the recon ruck is the kind's cue in every variant; the third wears a boonie hat
    "recon": {"a": dict(pack="ruck"), "b": dict(pack="ruck_roll"), "c": dict(pack="ruck", head="boonie")},
}


def look_of(kind, variant):
    return {**LOOKS[variant], **KIND_LOOKS.get(kind, {}).get(variant, {})}


def lerp3(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def scale3(a, k):
    return tuple(x * k for x in a)


def clamp01(x):
    return 0.0 if x < 0 else 1.0 if x > 1 else x


def grime(p, c, knees=True):
    """Dust and mud rising from the ground, heavier on the knees: the cloth darkens
    and its wear (the texture breaks it into spatter, the material's wear colour)
    climbs. Returns (colour, wear) for a textured material's `paint`."""
    z = p[2]
    rise = clamp01((0.55 - z) / 0.5) * (0.35 + 0.65 * fbm(p, 9.0, 21.0))
    if knees and 0.44 < z < 0.62:
        rise = max(rise, 0.6 * fbm(p, 14.0, 22.0))
    c = scale3(lerp3(c, DUST, rise * 0.5), 0.95 + 0.1 * fbm(p, 40.0, 23.0))
    return c, clamp01(rise * 0.9)


class Kit:
    def __init__(self, rig, look):
        self.rig = rig
        self.look = look
        self.arm = rig.arm
        self.body = rig.body
        self.parts = []  # every mesh that ends up in the soldier
        self.painters = {}  # object name -> colour_at(p, n)
        m = self.mats = {
            "uniform": mat("uniform", (1, 1, 1), rough=1.0, tint=1.0, texture="multicam_ripstop"),
            "pads": mat("pads", (1, 1, 1), rough=0.9, texture="hard_plastic"),
            "gear": mat("gear", (1, 1, 1), rough=1.0, tint=0.6, texture="cordura"),
            "kit": mat("kit", (1, 1, 1), rough=1.0, tint=0.6, texture="nylon"),
            "boots": mat("boots", (1, 1, 1), rough=0.85, texture="leather"),
            "gloves": mat("gloves", (1, 1, 1), rough=0.85, texture="leather"),
            "skin": mat("skin", (1, 1, 1), rough=0.6, texture="skin"),
            "hair": mat("hair", (1, 1, 1), rough=0.9),
            "lens": mat("lens", (1, 1, 1), rough=0.35),
            "helmet": (mat("helmet", (1, 1, 1), rough=0.8, tint=1.0, texture="olive_paint") if look["head"] == "bare"
                       else mat("helmet", (1, 1, 1), rough=1.0, tint=1.0, texture="multicam_ripstop")),
            "scarf": mat("scarf", (1, 1, 1), rough=1.0, texture="canvas"),
            "gun_black": mat("gun_black", (1, 1, 1), rough=0.45, metal=0.3, texture="gunmetal"),
            "gun_fde": mat("gun_fde", (1, 1, 1), rough=0.6, texture="polymer"),
            "webbing": mat("webbing", (1, 1, 1), rough=1.0, tint=0.6, texture="cordura"),
            "launcher": mat("launcher", (1, 1, 1), rough=0.7, texture="polymer"),
            "marking": mat("marking", (1, 1, 1), rough=0.75),
        }
        self.flat = {"gear": RANGER, "kit": RANGER, "pads": PADS, "marking": (0.3, 0.29, 0.2), "gun_black": (0.03, 0.03, 0.03), "gun_fde": (0.25, 0.20, 0.13), "webbing": RANGER, "launcher": (0.075, 0.08, 0.052),
                     "lens": (0.02, 0.022, 0.02)}
        self.m = m
        # region vertex groups
        vg = self.body.vertex_groups
        self.g_hand = self.gidx([g.name for g in vg if g.name.startswith(("hand_", "index", "middle", "ring", "pinky", "thumb"))])
        self.g_head = self.gidx(["Head", "neck_01"])
        self.g_headonly = self.gidx(["Head"])
        self.g_foot = self.gidx(["foot_l", "foot_r", "ball_l", "ball_r", "ball_leaf_l", "ball_leaf_r"])
        self.g_pelvis = self.gidx(["pelvis", "spine_01"])
        self.slim_body()

    def slim_body(self):
        """The UBC body is a superhero. Pull each vertex toward its dominant bone's segment,
        by bone, before any shell is cut: narrower shoulders, thinner arms and neck, a
        shallower chest. Joints stay where the rig has them."""
        pull = {"clavicle": 0.25, "upperarm": 0.4, "lowerarm": 0.3, "spine_03": 0.22, "spine_02": 0.14,
                "spine_01": 0.06, "neck_01": 0.2, "thigh": 0.16, "calf": 0.1}
        bones = self.arm.data.bones
        names = {g.index: g.name for g in self.body.vertex_groups}
        me = self.body.data
        for v in me.vertices:
            if not v.groups:
                continue
            g = max(v.groups, key=lambda e: e.weight)
            name = names[g.group]
            f = next((k for key, k in pull.items() if name.startswith(key)), 0.0)
            if not f or name not in bones:
                continue
            b = bones[name]
            a, c = b.head_local, b.tail_local
            ab = c - a
            t = clamp01((v.co - a).dot(ab) / max(ab.length_squared, 1e-9))
            q = a + ab * t
            v.co = q + (v.co - q) * (1.0 - f)

    # ---------------------------------------------------------------- helpers
    def gidx(self, names):
        vg = self.body.vertex_groups
        return {vg[n].index for n in names if n in vg}

    @staticmethod
    def w(v, dl, groups):
        return sum(wt for gi, wt in v[dl].items() if gi in groups)

    def shell(self, name, keep, material, offset, smooth=8, tube=0.0, rim=0.008, post=None, painter=None, source=None,
              subdivide=False, relax=None, pin=True):
        """A skinned shell cut from the body (or from another shell, for patches such as
        pockets and pads). keep(co, normal, v, dl) picks vertices; offset(co, normal) is
        the standoff along the smoothed normal."""
        src = source or self.body
        o = src.copy()
        o.data = src.data.copy()
        o.name = o.data.name = name
        self.rig.scn.collection.objects.link(o)
        for mod in list(o.modifiers):
            o.modifiers.remove(mod)
        bm = bmesh.new()
        bm.from_mesh(o.data)
        dl = bm.verts.layers.deform.active
        bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
        bm.normal_update()
        bmesh.ops.delete(bm, geom=[v for v in bm.verts if not keep(v.co, v.normal, v, dl)], context="VERTS")
        if subdivide:
            bmesh.ops.subdivide_edges(bm, edges=bm.edges[:], cuts=1, use_grid_fill=True)
        # smooth the anatomy away, but pin the open edges so hems stay where they were cut
        inner = [v for v in bm.verts if not (pin and v.is_boundary)]
        for _ in range(smooth):
            bmesh.ops.smooth_vert(bm, verts=inner, factor=0.5, use_axis_x=True, use_axis_y=True, use_axis_z=True)
        if relax:  # (region(co), iterations): extra smoothing where the body shows through
            region, iterations = relax
            zone = [v for v in inner if region(v.co)]
            for _ in range(iterations):
                bmesh.ops.smooth_vert(bm, verts=zone, factor=0.5, use_axis_x=True, use_axis_y=True, use_axis_z=True)
        if tube:
            tubify(bm, tube)
        bm.normal_update()
        moves = [(v, v.normal * offset(v.co, v.normal)) for v in bm.verts]
        for v, d in moves:
            v.co += d
        if post:
            post(bm)
        bm.normal_update()
        if rim:
            add_rim(bm, rim)
        for f in bm.faces:
            f.smooth = True
        bm.to_mesh(o.data)
        bm.free()
        o.data.materials.clear()
        o.data.materials.append(material)
        o.parent = self.arm
        self.parts.append(o)
        if painter:
            self.painters[o.name] = painter
        return o

    def rigid(self, o, bone, painter=None):
        self.rig.bone_parent(o, bone)
        self.parts.append(o)
        if painter:
            self.painters[o.name] = painter
        return o

    # ---------------------------------------------------------------- clothing
    def uniform(self):
        w, m = self.w, self.m

        def keep(co, n, v, dl):
            return (w(v, dl, self.g_hand) < 0.5 and w(v, dl, self.g_foot) < 0.5 and 0.16 < co.z < 1.62
                    and w(v, dl, self.g_head) < 0.8)

        def offset(co, n):
            leg = co.z < 0.97 and abs(co.x) < 0.3
            arm = abs(co.x) > 0.2 and co.z > 1.2
            if leg:
                base = 0.022
                if co.z < 0.22:  # tucked into the boot shaft
                    base = 0.012
                elif co.z < 0.32:  # bloused over the boot top
                    base += 0.016 * clamp01((0.32 - co.z) / 0.1)
                bunch = 1.0 + 1.8 * math.exp(-(((co.z - 0.54) / 0.09) ** 2))  # behind and over the knee
                bunch += 1.2 * math.exp(-(((co.z - 0.9) / 0.07) ** 2))  # the seat and crotch
                base += 0.008 * bunch * (abs(math.sin(co.z * 40 + 6 * fbm(co, 8, 6))) - 0.64)  # sharp creases, soft crests
            elif arm:
                base = 0.02 + 0.008 * clamp01((abs(co.x) - 0.3) / 0.15)  # a sleeve, looser to the cuff
                bunch = 1.0 + 1.8 * math.exp(-(((abs(co.x) - 0.46) / 0.08) ** 2))  # at the elbow
                base += 0.009 * bunch * (abs(math.sin(abs(co.x) * 50 + 6 * fbm(co, 8, 5))) - 0.64)
            else:
                base = 0.018 if co.z > 1.5 and abs(co.x) < 0.11 else 0.014  # the collar stands
            # loose cloth: soft wrinkles everywhere, a little slack
            return base + 0.004 * (fbm(co, 18.0, 3.0) - 0.5)

        def post(bm):
            # a shirt hides the superhero's pectorals: the chest front falls toward one plane
            zone = [v for v in bm.verts if abs(v.co.x) < 0.19 and 1.15 < v.co.z < 1.47 and v.normal.y < -0.3]
            if zone:
                plane = sorted(v.co.y for v in zone)[len(zone) // 2]
                for v in zone:
                    if v.co.y < plane:
                        v.co.y += (plane - v.co.y) * 0.7

        seat = (lambda co: 0.7 < co.z < 1.02 and abs(co.x) < 0.2, 24)  # crotch and seat
        cloth = self.shell("uniform", keep, m["uniform"], offset, smooth=20, tube=1.0, rim=0.012, post=post, relax=seat,
                           painter=lambda p, n: grime(p, texture_mean(m["uniform"])))
        self.cloth_bvh = bvh_of(cloth)
        # knee pads and cargo pockets: patches standing off the trouser shell
        for side in (1, -1):
            kx = 0.114 * side

            def pad_keep(co, n, v, dl, kx=kx):
                return abs(co.x - kx) < 0.08 and abs(co.z - 0.53) < 0.07 and n.y < -0.25

            if self.look["knee_pads"]:
                self.shell(f"knee_pad_{'l' if side > 0 else 'r'}", pad_keep, m["pads"],
                           lambda co, n: 0.012, smooth=4, rim=0.014, source=cloth, pin=False,
                           painter=lambda p, n: grime(p, scale3(PADS, 0.85 + 0.3 * fbm(p, 30, 5))))

            def pocket_keep(co, n, v, dl, side=side):
                return 0.6 < co.z < 0.8 and n.x * side > 0.45 and abs(co.x) > 0.11 and abs(co.x) < 0.3

            self.shell(f"cargo_pocket_{'l' if side > 0 else 'r'}", pocket_keep, m["uniform"],
                       lambda co, n: 0.012 + (0.005 if co.z > 0.75 else 0.0), smooth=1, rim=0.018, source=cloth, pin=False,
                       painter=lambda p, n: grime(p, scale3(texture_mean(m["uniform"]), 0.92)))
        # gloves: a thin shell, lightened in the budget
        glove = self.shell("gloves", lambda co, n, v, dl: w(v, dl, self.g_hand) > 0.3, m["gloves"],
                           lambda co, n: 0.004, smooth=1, rim=0.0, pin=False,
                           painter=lambda p, n: scale3(GLOVES, 0.85 + 0.3 * fbm(p, 50, 7)))
        dec = glove.modifiers.new("lighten", "DECIMATE")
        dec.ratio = 0.6

    def boots(self):
        w, m = self.w, self.m

        def keep(co, n, v, dl):
            return co.z < 0.23 and w(v, dl, self.g_hand) < 0.1

        def offset(co, n):
            if co.z < 0.12:
                return 0.008 + (0.004 if co.y < -0.12 else 0.0)  # a toe cap
            return 0.014  # the shaft laced round the ankle

        def post(bm):
            for v in bm.verts:  # the upper sits on the sole
                if v.co.z < 0.024:
                    v.co.z = 0.024 + (v.co.z - 0.024) * 0.2

        def leather(p, n):
            c = scale3(BOOTS, 0.8 + 0.35 * fbm(p, 35, 11))
            z = p[2]
            if n.y < -0.3 and 0.07 < z < 0.27 and abs(abs(p[0]) - 0.114) < 0.028:  # laces up the tongue
                if (z * 70) % 1.0 < 0.4:
                    c = LACES
            if z > 0.205:  # the padded collar
                c = scale3(c, 0.6)
            return c, 0.75 * clamp01((0.1 - p[2]) / 0.1)

        boots = self.shell("boots", keep, m["boots"], offset, smooth=5, rim=0.008, post=post, painter=leather,
                           subdivide=True)
        self.soles(boots)

    def soles(self, boots):
        """A sole under each boot: the upper's footprint, a lip wider, 3 cm thick, skinned to foot and ball."""
        me = boots.data
        for side in (1, -1):
            pts = [v.co.copy() for v in me.vertices if v.co.z < 0.05 and v.co.x * side > 0]
            hull = convex_hull_2d([(p.x, p.y) for p in pts])
            cx = sum(p[0] for p in hull) / len(hull)
            cy = sum(p[1] for p in hull) / len(hull)
            ring = []
            for x, y in hull:
                d = Vector((x - cx, y - cy))
                d = d.normalized() * (d.length + 0.006)
                ring.append((cx + d.x, cy + d.y))
            bm = bmesh.new()
            low = [bm.verts.new((x, y, 0.0)) for x, y in ring]
            high = [bm.verts.new((x, y, 0.026 + 0.012 * clamp01((y + 0.02) / 0.12))) for x, y in ring]  # a heel
            bm.faces.new(low[::-1])
            bm.faces.new(high)
            for i in range(len(ring)):
                j = (i + 1) % len(ring)
                bm.faces.new((low[i], low[j], high[j], high[i]))
            bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
            o = obj_from_bm(f"sole_{'l' if side > 0 else 'r'}", bm, [self.m["boots"]])
            s = "l" if side > 0 else "r"
            ball_y = -0.055
            gf, gb = o.vertex_groups.new(name=f"foot_{s}"), o.vertex_groups.new(name=f"ball_{s}")
            for v in o.data.vertices:
                t = clamp01((ball_y + 0.02 - v.co.y) / 0.04)  # toward the toe: the ball bone
                if t < 1:
                    gf.add([v.index], 1 - t, "REPLACE")
                if t > 0:
                    gb.add([v.index], t, "REPLACE")
            o.parent = self.arm
            o.modifiers.new("bevel", "BEVEL").width = 0.005
            self.parts.append(o)
            self.painters[o.name] = lambda p, n: scale3((0.018, 0.017, 0.015), 0.9 + 0.2 * fbm(p, 60, 2))

    # ---------------------------------------------------------------- torso kit
    def plate_carrier(self):
        w, m = self.w, self.m
        # plate carrier: torso shell with deep, flat plate bags front and back, a
        # cummerbund at the sides, and shoulder straps over the trapezius
        def keep(co, n, v, dl):
            if not (1.0 < co.z < 1.52 and abs(co.x) < 0.24):
                return False
            if w(v, dl, self.g_head) > 0.25:
                return False
            if co.z > 1.43:  # plate tops at the sternum, straps over the trapezius
                return 0.055 < abs(co.x) < 0.15 or (co.z < 1.47 and co.y < -0.05 and abs(co.x) < 0.12)
            return True

        def offset(co, n):
            plate = abs(co.x) < 0.15 and 1.1 < co.z < 1.47 and abs(n.y) > 0.35
            return 0.075 if plate else 0.045

        def post(bm):
            for sign in (-1, 1):  # flatten each plate bag to a plane
                zone = [v for v in bm.verts if abs(v.co.x) < 0.14 and 1.1 < v.co.z < 1.47 and v.normal.y * sign > 0.35]
                if not zone:
                    continue
                plane = max(v.co.y * sign for v in zone)
                for v in zone:
                    v.co.y = sign * (v.co.y * sign + (plane - v.co.y * sign) * 0.95)

        vest_c = self.look["vest"]
        vest = self.shell("plate_carrier", keep, m["gear"], offset, smooth=8, rim=0.014, post=post, pin=False,
                          painter=lambda p, n: grime(p, scale3(vest_c, 0.85 + 0.25 * fbm(p, 25, 31)), knees=False))
        self.vest_bvh = bvh_of(vest)
        self.vest = vest

    def pack_straps(self):
        """Shoulder straps from the pack over the carrier's straps and down onto the chest."""
        def keep(co, n, v, dl):
            return 0.07 < abs(co.x) < 0.135 and (co.z > 1.43 or (n.y < -0.2 and co.z > 1.27) or (n.y > 0.2 and co.z > 1.3))

        self.shell("pack_straps", keep, self.m["webbing"], lambda co, n: 0.008, smooth=2, rim=0.016, source=self.vest, pin=False,
                   painter=lambda p, n: scale3(RANGER, 0.8 + 0.2 * fbm(p, 40, 33)))

    def belt(self, z=0.975, height=0.05, thick=0.012, steps=48):
        """A riggers belt: a band round the trousers at the waist, traced by rays from the
        body's axis so it sits flush all the way round; rigid on the pelvis."""
        centre = Vector((0, 0.03, z))
        ring = []
        for k in range(steps):
            b = 2 * math.pi * k / steps
            out = Vector((math.sin(b), -math.cos(b), 0))
            hit = self.cloth_bvh.ray_cast(centre + out * 0.6, -out)
            r = (hit[0] - centre).length if hit[0] is not None else 0.17
            ring.append((out, r + 0.002))
        bm = bmesh.new()
        rows = []
        for dz, dr in ((-height / 2, 0.0), (-height / 2, thick), (height / 2, thick), (height / 2, 0.0)):
            rows.append([bm.verts.new(centre + out * (r + dr) + Vector((0, 0, dz))) for out, r in ring])
        for i in range(4):
            a_, b_ = rows[i], rows[(i + 1) % 4]
            for k in range(steps):
                j = (k + 1) % steps
                bm.faces.new((a_[k], a_[j], b_[j], b_[k]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        belt = obj_from_bm("belt", bm, [self.m["webbing"]])
        self.rigid(belt, "pelvis", lambda p, n: scale3(RANGER, 0.85 + 0.3 * fbm(p, 30, 41)))
        buckle = box("belt_buckle", (0.06, 0.012, 0.045), centre + ring[0][0] * (ring[0][1] + thick + 0.004), self.m["gun_black"], bevel_=0.004)
        self.rigid(buckle, "pelvis")
        self.belt_bvh = bvh_of(belt)

    def on_vest(self, x, z, facing):
        """The vest surface point at (x, z), seen from the front (facing -1) or back (+1)."""
        hit = self.vest_bvh.ray_cast(Vector((x, facing * 0.6, z)), Vector((0, -facing, 0)))
        return hit[0] if hit[0] is not None else Vector((x, facing * 0.2, z))

    def on_belt(self, bearing_deg, z):
        """The belt's outer surface at a bearing about the waist (0 = front, 90 = his left)."""
        b = math.radians(bearing_deg)
        out = Vector((math.sin(b), -math.cos(b), 0))
        centre = Vector((0, 0.03, z))
        hit = self.belt_bvh.ray_cast(centre + out * 0.6, -out)
        return (hit[0] if hit[0] is not None else centre + out * 0.2), out

    def pouch(self, name, size, x, z, facing, bone="spine_03", mat_key="kit", flap=True, colour=RANGER):
        at = self.on_vest(x, z, facing)
        centre = at + Vector((0, facing * size[1] / 2 * 0.9, 0))
        body = box(name, size, centre, self.m[mat_key], bevel_=min(size) * 0.08, segments=2)
        paint_fn = lambda p, n, c=colour: grime(p, scale3(c, 0.85 + 0.25 * fbm(p, 30, 17)), knees=False)
        self.rigid(body, bone, paint_fn)
        if flap:
            f = box(name + "_flap", (size[0] * 1.04, size[1] * 1.08, size[2] * 0.28),
                    centre + Vector((0, facing * 0.004, size[2] * 0.38)), self.m[mat_key], bevel_=0.006)
            self.rigid(f, bone, lambda p, n, c=colour: scale3(c, 0.78 + 0.2 * fbm(p, 30, 19)))
        return body

    def belt_pouch(self, name, size, bearing, z, colour, flap=True):
        at, out = self.on_belt(bearing, z)
        centre = at + out * (size[1] / 2 * 0.9)
        rot = (0, 0, math.radians(bearing))
        body = box(name, size, centre, self.m["kit"], bevel_=min(size) * 0.2, rot=rot)
        self.rigid(body, "pelvis", lambda p, n, c=colour: grime(p, scale3(c, 0.85 + 0.25 * fbm(p, 30, 71))))
        if flap:
            f = box(name + "_flap", (size[0] * 1.04, size[1] * 1.08, size[2] * 0.3),
                    centre + out * 0.004 + Vector((0, 0, size[2] * 0.36)), self.m["kit"], bevel_=0.006, rot=rot)
            self.rigid(f, "pelvis", lambda p, n, c=colour: scale3(c, 0.78 + 0.2 * fbm(p, 30, 72)))

    def chest_rig(self):
        c = self.look["pouch"]
        xs = (-0.088, 0.0, 0.088) if self.look["mags"] == 3 else (-0.05, 0.04)
        for i, x in enumerate(xs):
            self.pouch(f"mag_pouch_{i}", (0.08, 0.06, 0.13), x, 1.17, -1, colour=c)
        if self.look["mags"] < 3:  # a frag pouch where the third magazine rode
            self.pouch("frag_pouch", (0.06, 0.06, 0.08), 0.12, 1.15, -1, colour=c)
        self.pouch("admin_pouch", (0.18, 0.04, 0.08), 0.0, 1.32, -1, flap=False, colour=c)
        self.pouch("tq_pouch", (0.045, 0.045, 0.1), 0.14, 1.31, -1, flap=False, colour=COYOTE)

    def molle(self):
        """PALS webbing sewn across the plate bags, front and back: rows of flat
        straps standing just proud of the carrier, the pouches hung on them."""
        for facing in (-1, 1):
            for k in range(6):
                z = 1.14 + k * 0.05
                at = self.on_vest(0.0, z, facing)
                strap = box(f"pals_{'front' if facing < 0 else 'back'}_{k}", (0.25, 0.006, 0.026),
                            at + Vector((0, facing * 0.003, 0)), self.m["webbing"])
                self.rigid(strap, "spine_03", lambda p, n: scale3(RANGER, 0.8 + 0.2 * fbm(p, 40, 35)))

    def markings(self):
        """Subdued patches on both upper sleeves (in the T-pose the sleeve's outside
        faces up): a dark badge on a lighter hook-and-loop field, stitched flat."""
        for side, bone in ((1, "upperarm_l"), (-1, "upperarm_r")):
            hit = self.cloth_bvh.ray_cast(Vector((side * 0.3, 0.06, 2.0)), Vector((0, 0, -1)))
            if hit[0] is None:
                continue
            at = hit[0]
            field = box(f"patch_field_{bone}", (0.075, 0.058, 0.004), at + Vector((0, 0, 0.001)), self.m["kit"])
            badge = box(f"patch_badge_{bone}", (0.06, 0.044, 0.004), at + Vector((0, 0, 0.003)), self.m["kit"])
            self.rigid(field, bone, lambda p, n: (0.07, 0.075, 0.05))
            self.rigid(badge, bone, lambda p, n: (0.025, 0.028, 0.02))

    def hip_kit(self):
        self.belt_pouch("dump_pouch", (0.1, 0.07, 0.14), 100, 0.95, RANGER)
        self.belt_pouch("utility_pouch", (0.1, 0.06, 0.1), -100, 0.96, COYOTE)
        self.belt_pouch("ifak", (0.15, 0.07, 0.1), 180, 0.97, RANGER, flap=False)
        self.belt_pouch("canteen", (0.08, 0.07, 0.13), 140, 0.95, COYOTE)

    def hydration(self):
        """A slim hydration carrier on the back plate."""
        m = self.m
        back = self.on_vest(0.0, 1.28, 1)
        colour = self.look["pouch"]
        centre = Vector((0.0, back.y + 0.028, 1.28))
        body = box("hydration", (0.2, 0.05, 0.34), centre, m["kit"], bevel_=0.018, segments=2)
        paint_fn = lambda p, n: grime(p, scale3(colour, 0.85 + 0.25 * fbm(p, 22, 57)), knees=False)
        self.rigid(body, "spine_03", paint_fn)

    def pack(self, kind):
        m = self.m
        style = self.look["pack"]
        if style == "none":
            return
        if style == "hydration":
            self.hydration()
            return
        back = self.on_vest(0.0, 1.26, 1)
        if style.startswith("ruck"):  # a long-range ruck: the recon team's silhouette cue
            size, z, colour = (0.3, 0.19, 0.46), 1.25, (0.09, 0.085, 0.06)
            if style == "ruck_roll":  # the same ruck in ranger green, a sleeping mat rolled on top
                colour = scale3(RANGER, 1.2)
        else:  # an assault pack
            size, z, colour = (0.28, 0.14, 0.38), 1.24, COYOTE
        centre = Vector((0.0, back.y + size[1] / 2 * 0.8, z))
        bm = bmesh.new()
        bmesh.ops.create_cube(bm, size=1.0)
        bmesh.ops.subdivide_edges(bm, edges=bm.edges[:], cuts=2, use_grid_fill=True)
        for v in bm.verts:  # a soft, slightly tapered bag
            x, y, zz = v.co
            taper = 1.0 - 0.12 * (zz + 0.5)
            v.co = Vector((x * size[0] * taper, y * size[1] * (0.85 + 0.15 * (0.5 - abs(zz))), zz * size[2]))
        pack = obj_from_bm("pack", bm, [m["kit"]])
        pack.location = centre
        sub = pack.modifiers.new("soft", "SUBSURF")
        sub.levels = 1
        paint_fn = lambda p, n, c=colour: grime(p, scale3(c, 0.85 + 0.25 * fbm(p, 22, 51)), knees=False)
        self.rigid(pack, "spine_03", paint_fn)
        lid = box("pack_lid", (size[0] * 0.92, size[1] * 0.8, 0.07), centre + Vector((0, 0.01, size[2] / 2 - 0.02)), m["kit"], bevel_=0.02)
        self.rigid(lid, "spine_03", lambda p, n, c=colour: scale3(c, 0.75 + 0.2 * fbm(p, 22, 53)))
        for sx in (-1, 1):  # compression straps
            strap = box(f"pack_strap_{sx}", (0.022, size[1] * 1.02, size[2] * 0.8), centre + Vector((sx * size[0] * 0.3, 0.004, -0.02)), m["webbing"])
            self.rigid(strap, "spine_03", lambda p, n: scale3(RANGER, 0.8 + 0.2 * fbm(p, 40, 55)))
        side = box("pack_bottle", (0.06, 0.07, 0.16), centre + Vector((size[0] / 2 + 0.028, 0, -size[2] * 0.2)), m["kit"], bevel_=0.02)
        self.rigid(side, "spine_03", paint_fn)
        if style == "ruck_roll":
            roll = cyl("pack_roll", 0.065, size[0] * 1.25, centre + Vector((0, 0.0, size[2] / 2 + 0.055)), "X", m["scarf"],
                       seg=12, bevel_=0.01)
            self.rigid(roll, "spine_03", lambda p, n: grime(p, scale3((0.06, 0.07, 0.045), 0.85 + 0.3 * fbm(p, 30, 59)),
                                                             knees=False))
        if style.startswith("ruck") or not self.look["radio"]:
            aerial_foot, tall, lean = None, 0, 0  # the ruck is the recon team's cue, not a whip
        else:
            radio = box("radio", (0.075, 0.06, 0.14), Vector((-0.15, back.y + 0.02, 1.2)), m["kit"], bevel_=0.012)
            self.rigid(radio, "spine_03", lambda p, n: scale3(RANGER, 0.8 + 0.2 * fbm(p, 30, 61)))
            aerial_foot, tall, lean = Vector((-0.17, back.y + 0.04, 1.27)), 0.2, -40
        if not tall:
            return
        # the whip antenna, swept back (it lies clear of the pack when he goes prone)
        ant = cyl("radio_antenna", 0.004, tall, (0, 0, 0), "Z", m["gun_black"], seg=6)
        ant.data.transform(Matrix.Rotation(math.radians(lean), 4, "X") @ Matrix.Translation((0, 0, tall / 2)))
        ant.location = aerial_foot
        self.rigid(ant, "spine_03")

    # ---------------------------------------------------------------- head
    def head_gear(self):
        w, m = self.w, self.m

        def rim_z(co):
            ang = math.atan2(co.x, -(co.y - 0.01))  # 0 front, +-pi back
            front = math.cos(ang)
            z = 1.672 + 0.034 * max(0.0, front) - 0.028 * max(0.0, -front)
            if abs(co.x) > 0.06 and -0.03 < co.y < 0.06:  # ear cut
                z += 0.022
            return z

        look = self.look
        head = look["head"]
        boonie = head == "boonie"

        def keep(co, n, v, dl):
            # a soft hat sits lower on the brow than a helmet's rim
            return w(v, dl, self.g_headonly) > 0.5 and co.z > rim_z(co) - (0.012 if boonie else 0.0)

        def helmet_paint(p, n):  # an olive cover, darker than the face below it
            c = lerp3(scale3(RANGER, 1.25), texture_mean(m["helmet"]), 0.3)
            return scale3(c, 0.9 + 0.2 * fbm(p, 40, 81))

        def bare_paint(p, n):  # a painted shell: its paint's own green, scuffed
            return scale3(lerp3(texture_mean(m["helmet"]), (0.05, 0.055, 0.04), 0.4), 0.9 + 0.2 * fbm(p, 40, 81)), 0.25

        def hat_paint(p, n):  # the uniform's print, sun-faded
            return scale3(texture_mean(m["helmet"]), 1.05 + 0.2 * fbm(p, 40, 81)), 0.2

        painter = bare_paint if head == "bare" else hat_paint if boonie else helmet_paint
        def standoff(co, n):  # a soft hat hugs the brow and stands its crown up, as tall as a helmet
            return 0.012 + 0.024 * clamp01((co.z - 1.72) / 0.07) if boonie else 0.03

        crown = self.shell("helmet", keep, m["helmet"], standoff, smooth=12, rim=0.013, painter=painter, pin=False)
        # short hair below the rim and over the ears
        hair_c = look["hair"]
        self.shell("hair", lambda co, n, v, dl: w(v, dl, self.g_headonly) > 0.5 and co.z > 1.6 and co.y > -0.02 and co.z > rim_z(co) - 0.07,
                   m["hair"], lambda co, n: 0.005, smooth=4, rim=0.0, pin=False,
                   painter=lambda p, n: scale3(hair_c, 0.8 + 0.4 * fbm(p, 60, 83)))
        if look["eyewear"]:  # ballistic eyewear across the eyes
            self.shell("eyewear", lambda co, n, v, dl: 1.655 < co.z < 1.705 and n.y < -0.35 and abs(co.x) < 0.075 and co.y < -0.05,
                       m["lens"], lambda co, n: 0.011, smooth=3, rim=0.004, painter=lambda p, n: (0.015, 0.017, 0.016), pin=False)
        black = m["gun_black"]
        if boonie:
            self.brim(crown)
            return
        rigid = [box("helmet_rail_l", (0.012, 0.10, 0.022), (0.121, 0.0, 1.705), black, bevel_=0.003),
                 box("helmet_rail_r", (0.012, 0.10, 0.022), (-0.121, 0.0, 1.705), black, bevel_=0.003)]
        if head == "nvg":
            rigid += [box("nvg_shroud", (0.05, 0.022, 0.04), (0, -0.150, 1.755), black, bevel_=0.006),
                      box("counterweight", (0.09, 0.03, 0.05), (0, 0.145, 1.705), m["kit"], bevel_=0.01)]
        for o in rigid:
            self.rigid(o, "Head")
        for side in (1, -1):  # comms headset cups
            cup = cyl(f"headset_{'l' if side > 0 else 'r'}", 0.031, 0.028, (side * 0.093, 0.012, 1.648), "X", m["webbing"], seg=16, bevel_=0.005)
            self.rigid(cup, "Head")
        if head in ("scrim", "bare"):
            self.helmet_band(crown, goggles=head == "scrim", scrim=head == "scrim")

    def ring_on(self, bvh, centre, z, pad, steps=40):
        """Points round `bvh`'s surface at height z, `pad` proud of it, traced by rays in
        from outside toward the vertical axis through `centre`, and their outward directions."""
        ring = []
        for k in range(steps):
            b = 2 * math.pi * k / steps
            out = Vector((math.sin(b), -math.cos(b), 0))
            hit = bvh.ray_cast(Vector((centre.x, centre.y, z)) + out * 0.5, -out)
            r = (hit[0] - Vector((centre.x, centre.y, z))).length if hit[0] is not None else 0.12
            ring.append((Vector((centre.x, centre.y, z)) + out * (r + pad), out))
        return ring

    def band(self, name, ring, height, thick, material, painter, bone="Head"):
        """A closed band through `ring` (points and outward directions), `height` tall."""
        bm = bmesh.new()
        rows = []
        for dz, dr in ((-height / 2, 0.0), (-height / 2, thick), (height / 2, thick), (height / 2, 0.0)):
            rows.append([bm.verts.new(p + out * dr + Vector((0, 0, dz))) for p, out in ring])
        n = len(ring)
        for i in range(4):
            a_, b_ = rows[i], rows[(i + 1) % 4]
            for k in range(n):
                j = (k + 1) % n
                bm.faces.new((a_[k], a_[j], b_[j], b_[k]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        return self.rigid(obj_from_bm(name, bm, [material]), bone, painter)

    def helmet_band(self, crown, goggles, scrim):
        """A rubber band round the helmet, and with it goggles pushed up on the front, or
        burlap scrim tucked under it."""
        m = self.m
        bvh = bvh_of(crown)
        centre = Vector((0.0, 0.01, 0.0))
        self.band("helmet_band", self.ring_on(bvh, centre, 1.735, 0.0), 0.018, 0.005, m["gun_black"],
                  lambda p, n: (0.02, 0.02, 0.018))
        if goggles:
            front = self.ring_on(bvh, centre, 1.738, 0.004, steps=40)
            for side in (1, -1):
                p, out = front[(40 - 3 * side) % 40]
                lens = box(f"goggle_{'l' if side > 0 else 'r'}", (0.052, 0.022, 0.034), p + out * 0.008, m["lens"],
                           bevel_=0.008, rot=(0, 0, math.atan2(out.x, -out.y)))
                self.rigid(lens, "Head", lambda p, n: (0.03, 0.04, 0.035))
                frame_ = box(f"goggle_frame_{'l' if side > 0 else 'r'}", (0.06, 0.016, 0.042), p + out * 0.002, m["gun_black"],
                             bevel_=0.008, rot=(0, 0, math.atan2(out.x, -out.y)))
                self.rigid(frame_, "Head", lambda p, n: (0.025, 0.025, 0.022))
        if scrim:  # strips of burlap under the band, hanging over the shell
            rng = random.Random(29)
            ring = self.ring_on(bvh, centre, 1.735, 0.006, steps=28)
            for k, (p, out) in enumerate(ring):
                if 30 < math.degrees(math.atan2(out.x, -out.y)) % 360 < 330 or k % 3 == 0:
                    tall = rng.uniform(0.03, 0.06)
                    strip = box(f"scrim_{k}", (rng.uniform(0.012, 0.022), 0.006, tall), p + Vector((0, 0, -tall * 0.2)),
                                m["scarf"], rot=(rng.uniform(-0.4, 0.4), rng.uniform(-0.3, 0.3), math.atan2(out.x, -out.y)))
                    tone = rng.uniform(0.7, 1.2)
                    self.rigid(strip, "Head", lambda p, n, t=tone: (scale3((0.07, 0.075, 0.045), t), 0.1))

    def brim(self, crown):
        """A boonie hat's soft brim: a drooping ring round the crown at the hat band."""
        m = self.m
        bvh = bvh_of(crown)
        ring = self.ring_on(bvh, Vector((0.0, 0.01, 0.0)), 1.69, -0.004, steps=40)
        bm = bmesh.new()
        rows = []
        for reach, drop, t in ((0.0, 0.0, 0.0), (0.035, 0.012, 0.0), (0.065, 0.03, 0.0), (0.065, 0.03, 0.006),
                               (0.0, 0.006, 0.006)):
            rows.append([bm.verts.new(p + out * reach + Vector((0, 0, -drop + t))) for p, out in ring])
        n = len(ring)
        for i in range(len(rows)):
            a_, b_ = rows[i], rows[(i + 1) % len(rows)]
            for k in range(n):
                j = (k + 1) % n
                bm.faces.new((a_[k], a_[j], b_[j], b_[k]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        self.rigid(obj_from_bm("hat_brim", bm, [m["helmet"]]), "Head",
                   lambda p, n: (scale3(texture_mean(m["helmet"]), 1.0 + 0.2 * fbm(p, 40, 85)), 0.3))
        self.band("hat_band", self.ring_on(bvh, Vector((0.0, 0.01, 0.0)), 1.71, 0.0), 0.02, 0.004, m["webbing"],
                  lambda p, n: scale3(RANGER, 0.9))

    def scarf(self):
        """A shemagh wound round the neck over the collar, bunched."""
        w = self.w

        def keep(co, n, v, dl):
            return 1.47 < co.z < 1.6 and w(v, dl, self.g_head) > 0.05 and w(v, dl, self.g_hand) < 0.1

        def offset(co, n):
            return 0.03 + 0.012 * fbm(co, 30.0, 3.0) + 0.012 * clamp01((1.53 - co.z) / 0.06)

        self.shell("scarf", keep, self.m["scarf"], offset, smooth=6, rim=0.01, pin=False,
                   painter=lambda p, n: (scale3(SCARF, 0.85 + 0.3 * fbm(p, 40, 87)), 0.15))

    def skin(self):
        skin, stubble = self.look["skin"], self.look["stubble"]

        def face(p, n):  # skin, a stubbled jaw, darker brows and lips
            c = scale3(skin, 0.88 + 0.2 * fbm(p, 30, 91))
            front = n.y < -0.2
            if front and 1.575 < p[2] < 1.625 and abs(p[0]) < 0.06 and p[1] < -0.07:
                c = lerp3(c, stubble[0], stubble[1])  # stubble or a beard
            if front and 1.605 < p[2] < 1.615 and abs(p[0]) < 0.022:
                c = (0.22, 0.1, 0.08)  # lips
            if front and 1.7 < p[2] < 1.712 and 0.012 < abs(p[0]) < 0.055:
                c = (0.05, 0.035, 0.025)  # brows
            return c

        """The body under the clothes: head and neck only, with a little overlap."""
        w = self.w
        bm = bmesh.new()
        bm.from_mesh(self.body.data)
        dl = bm.verts.layers.deform.active
        bmesh.ops.delete(bm, geom=[v for v in bm.verts if v.co.z < 1.40 or (v.co.z < 1.60 and w(v, dl, self.g_head) < 0.2)], context="VERTS")
        bm.to_mesh(self.body.data)
        bm.free()
        for mod in list(self.body.modifiers):
            self.body.modifiers.remove(mod)
        self.body.data.materials.clear()
        self.body.data.materials.append(self.m["skin"])
        self.parts.append(self.body)
        self.painters[self.body.name] = face

    # ---------------------------------------------------------------- weapon, sockets
    def carried_launcher(self):
        """Strap the same launcher across the back, with its CLU facing out.
        The tube and its retention straps ride the torso, never the rifle hand."""
        root = weapons.launcher(self.m, "carried_launcher")
        for c in list(root.children_recursive):
            if c.type == "EMPTY":
                bpy.data.objects.remove(c, do_unlink=True)
        # Clear the pack's outer face in every LOOK, with the tube's high end
        # beside the head and its low end outside the opposite hip.
        bpy.context.view_layer.update()
        back = max((o.matrix_world @ Vector(c)).y
                   for o in self.parts if bone_of(o) == "spine_03"
                   for c in o.bound_box)
        tube_y = back + 0.095
        angle = math.radians(30)
        forward = Vector((-math.sin(angle), 0, math.cos(angle)))
        side = Vector((0, 1, 0))
        up = side.cross(-forward)
        rotation = Matrix((side, -forward, up)).transposed().to_4x4()
        centre = Vector((0, tube_y, 1.15))
        root.matrix_world = Matrix.Translation(centre - up * 0.1) @ rotation
        self.rig.bone_parent(root, "spine_03")
        for c in root.children_recursive:
            if c.type == "MESH":
                self.parts.append(c)
        for k, (along, x, z) in enumerate(((-0.4, -0.10, 1.42), (0.4, 0.10, 1.02))):
            lug = root.matrix_world @ Vector((-0.075, along, 0.1))
            anchor = Vector((x, back, z))
            reach = lug - anchor
            strap = box(f"launcher_retention_{k}", (0.035, 0.008, reach.length),
                        (anchor + lug) * 0.5, self.m["webbing"],
                        rot=reach.to_track_quat("Z", "Y").to_euler())
            self.rigid(strap, "spine_03")

    def weapon(self, spec):
        """Place the weapon where its low-ready hold puts it and ride it on hand_r:
        in the clips, hand_r's armature-space matrix equals its IK target."""
        root = spec["build"](self.m)
        hold = spec["hold"]
        right, _ = self.rig.hold_targets(hold["low"], hold["grip"], hold["support"])
        s = hold["scale"]
        placed = hold["low"] @ Matrix.Diagonal((s, s, s, 1.0))
        hand_rest = self.arm.matrix_world @ self.arm.data.bones["hand_r"].matrix_local
        root.matrix_world = hand_rest @ right.inverted() @ placed
        self.rig.bone_parent(root, "hand_r")
        for c in root.children_recursive:
            if c.type == "MESH":
                self.parts.append(c)
        for c in list(root.children):
            if c.type == "EMPTY":  # sockets ride hand_r itself
                self.rig.bone_parent(c, "hand_r")
        return root

    def eye(self):
        at = self.arm.matrix_world @ self.arm.data.bones["Head"].matrix_local @ EYE_IN_HEAD
        self.rig.bone_parent(empty("eye", at, size=0.02), "Head")

    # ---------------------------------------------------------------- surface
    def paint_all(self):
        for o in self.parts:
            name = o.data.materials[0].name if o.data.materials else ""
            fn = self.painters.get(o.name)
            if fn is None:
                rgb = self.flat.get(name, (0.5, 0.5, 0.5))
                # handled kit scuffs: a little wear the texture breaks into chips and rubs
                fn = lambda p, n, rgb=rgb: (scale3(rgb, 0.9 + 0.2 * fbm(p, 40, 97)), 0.3 + 0.2 * fbm(p, 12, 98))
            paint(o, fn)

    # ---------------------------------------------------------------- one skinned mesh
    def join(self):
        """Apply every shape modifier, skin rigid kit 100% to its bone, fold leaf-joint
        weights into their parents, and join everything into one mesh."""
        bpy.context.view_layer.update()
        dg = bpy.context.evaluated_depsgraph_get()
        meshes = []
        for o in self.parts:
            bone = bone_of(o)
            mw = o.matrix_world.copy()
            me = bpy.data.meshes.new_from_object(o.evaluated_get(dg), preserve_all_data_layers=True, depsgraph=dg)
            flat = bpy.data.objects.new(o.name + "_baked", me)
            self.rig.scn.collection.objects.link(flat)
            flat.matrix_world = mw
            if bone:  # the mesh keeps its source's group names; a rigid part has one bone
                flat.vertex_groups.clear()
                g = flat.vertex_groups.new(name=bone)
                g.add([v.index for v in me.vertices], 1.0, "REPLACE")
            meshes.append(flat)
        for o in list(self.parts):
            bpy.data.objects.remove(o, do_unlink=True)
        bpy.ops.object.select_all(action="DESELECT")
        for o in meshes:
            o.select_set(True)
            o.data.transform(o.matrix_world)
            o.matrix_world = Matrix.Identity(4)
        bpy.context.view_layer.objects.active = meshes[0]
        bpy.ops.object.join()
        soldier = bpy.context.view_layer.objects.active
        soldier.name = soldier.data.name = "soldier"
        canonical(soldier)
        fold_leaf_weights(soldier, self.arm)
        return soldier

    def bake_ao(self, soldier, strength=0.55):
        """Ambient occlusion in the rest pose, multiplied into the vertex colour."""
        scn = self.rig.scn
        scn.render.engine = "CYCLES"
        scn.cycles.device = "CPU"
        scn.cycles.samples = 64
        scn.cycles.seed = 0
        scn.cycles.use_denoising = False
        world = bpy.data.worlds.new("ao_world")
        scn.world = world
        me = soldier.data
        col = me.color_attributes["Col"]
        ao = me.color_attributes.new("AO", "BYTE_COLOR", "CORNER")
        me.color_attributes.active_color = ao
        bpy.ops.object.select_all(action="DESELECT")
        soldier.select_set(True)
        bpy.context.view_layer.objects.active = soldier
        bpy.ops.object.bake(type="AO", target="VERTEX_COLORS")
        for i in range(len(col.data)):
            a = ao.data[i].color[0]
            k = 1.0 - strength * (1.0 - a)
            c = col.data[i].color
            col.data[i].color = (c[0] * k, c[1] * k, c[2] * k, c[3])
        me.color_attributes.remove(ao)
        me.color_attributes.active_color = me.color_attributes["Col"]
        me.color_attributes.render_color_index = 0


# ---------------------------------------------------------------- geometry helpers

def tubify(bm, k):
    """Pull limb vertices toward the mean radius of their slice about the limb's axis
    (T-pose arms along X, legs from hip to ankle), so muscles read as cloth over a tube."""
    arms = [v for v in bm.verts if abs(v.co.x) > 0.26 and v.co.z > 1.2]
    legs = [v for v in bm.verts if v.co.z < 0.93 and abs(v.co.x) > 0.015]
    groups = []
    for v in arms:
        axis = Vector((v.co.x, 0.07, 1.456))
        groups.append((v, axis, round(v.co.x / 0.025) * (1 if v.co.x > 0 else -1)))
    for v in legs:
        side = 1 if v.co.x > 0 else -1
        t = clamp01((0.971 - v.co.z) / (0.971 - 0.086))
        axis = Vector((side * 0.114, 0.036 + 0.052 * t * t, v.co.z))
        groups.append((v, axis, 1000 * side + round(v.co.z / 0.025)))
    radii = {}
    for v, axis, key in groups:
        radii.setdefault(key, []).append((v.co - axis).length)
    raw = {k: sum(r) / len(r) for k, r in radii.items()}
    # cloth hangs straight over a muscle: average each slice's radius with its neighbours
    # along the limb (the keys are consecutive 2.5 cm slices), so biceps and calves smooth away
    mean = {}
    for key in raw:
        near = [raw[j] for j in range(key - 3, key + 4) if j in raw]
        mean[key] = sum(near) / len(near)
    for v, axis, key in groups:
        d = v.co - axis
        r = d.length
        # The superhero's arms are a size too big: slim the sleeves away from the shoulder.
        slim = 1.0 - 0.18 * clamp01((abs(v.co.x) - 0.26) / 0.12) if abs(v.co.x) > 0.26 and v.co.z > 1.2 else 1.0
        if r > 1e-6:
            v.co = axis + d * ((r + (mean[key] * slim - r) * k) / r)


def add_rim(bm, depth):
    """Turn a shell's open edges inward by `depth`, so cut cloth shows a thickness."""
    edges = [e for e in bm.edges if e.is_boundary]
    if not edges:
        return
    ret = bmesh.ops.extrude_edge_only(bm, edges=edges)
    new = [e for e in ret["geom"] if isinstance(e, bmesh.types.BMVert)]
    for v in new:
        n = Vector((0, 0, 0))
        for f in v.link_faces:
            n += f.normal
        linked = [e.other_vert(v) for e in v.link_edges if e.other_vert(v) not in new]
        base = linked[0].normal if linked else n
        v.co -= base.normalized() * depth


def convex_hull_2d(points):
    pts = sorted(set((round(x, 5), round(y, 5)) for x, y in points))
    if len(pts) < 3:
        return pts

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return lower[:-1] + upper[:-1]


def bvh_of(o):
    bm = bmesh.new()
    bm.from_mesh(o.data)
    tree = BVHTree.FromBMesh(bm)
    bm.free()
    return tree


def bone_of(o):
    """The bone a rigid part rides: its own bone parent, or its nearest ancestor's."""
    while o is not None:
        if o.parent_type == "BONE" and o.parent_bone:
            return o.parent_bone
        o = o.parent
    return None


def fold_leaf_weights(o, arm):
    """Move the weight of every `*_leaf` joint group onto its parent bone's group, so the
    body keeps exactly the skeleton clips' joints (the leaves carry no animation)."""
    vg = o.vertex_groups
    for g in [g for g in vg if "_leaf" in g.name]:
        parent = arm.data.bones[g.name].parent.name
        target = vg.get(parent) or vg.new(name=parent)
        for v in o.data.vertices:
            for e in v.groups:
                if e.group == g.index and e.weight > 0:
                    target.add([v.index], e.weight, "ADD")
        vg.remove(g)


def build(kind, variant):
    rig = Rig()
    for n in ("Eyes", "Eyebrows"):
        if n in bpy.data.objects:
            bpy.data.objects.remove(bpy.data.objects[n], do_unlink=True)
    kit = Kit(rig, look_of(kind, variant))
    kit.uniform()
    kit.boots()
    kit.plate_carrier()
    kit.molle()
    if kit.look["pack"] != "none":
        kit.pack_straps()
    kit.belt()
    kit.chest_rig()
    kit.pack(kind)
    kit.hip_kit()
    kit.markings()
    kit.head_gear()
    if kit.look["scarf"]:
        kit.scarf()
    kit.skin()
    if kind == "at_carried":
        kit.carried_launcher()
    kit.weapon(weapons.KINDS[kind])
    kit.eye()
    kit.paint_all()
    soldier = kit.join()
    kit.bake_ao(soldier)
    return rig, soldier


def main():
    args = script_args()
    kind = args[0] if args else "rifle"
    variant = args[1] if len(args) > 1 else "a"
    rig, soldier = build(kind, variant)
    tiers = make_tiers(soldier, "soldier")
    for t in tiers:
        canonical(t)
        t.parent = rig.arm
        t.modifiers.new("armature", "ARMATURE").object = rig.arm
    rig.arm.scale = (SCALE, SCALE, SCALE)
    bpy.context.view_layer.update()
    texture_uvs(tiers)  # in the rest pose, at the soldier's scale
    for o in list(bpy.data.objects):
        if o.type == "EMPTY" and o.name not in ("eye", "muzzle"):
            bpy.data.objects.remove(o, do_unlink=True)
    sockets = [bpy.data.objects["eye"], bpy.data.objects["muzzle"]]
    rel = f"assets/source/infantry/{kind if variant == 'a' else f'{kind}_{variant}'}.glb"
    export_glb(os.path.join(REPO, rel), [rig.arm, *tiers, *sockets])
    print(f"wrote {rel}")


if __name__ == "__main__":
    main()
