"""A generated soldier body, worn on the infantry rig in place of the cloth shells.

An image-to-3D generator's T-pose soldier (`skins/infantry.json`, one pin per
army, look and kind) becomes the clothed body of a roster kit. The kit
(`roster/infantry_equipment.py`) still adds the weapon, its hold, the sockets,
ambient occlusion, the tiers and the export. This module only makes the
body:

- Fit. The generated mesh is turned to face -Y (the pin says where its front
  faces), decimated to `BODY_TRIS`, and scaled to the rig's rest pose: its
  height (helmet on) to `HEIGHT_M`, its fingertip span to the body's, its
  feet to the ground, its torso centred on the body's. Then each arm and leg
  is seated on its bones, so every joint bends inside its limb.
- Weights. Each vertex takes its weights from the procedural (slimmed) body,
  by nearest face. An arm vertex takes them only from the body's arm, so a
  sleeve standing off a thinner arm never picks up the torso.
- Hands. The generator's hands are fused mittens that can't close on a grip.
  They are cut off at the wrist and replaced by the kit's own glove shell.
- Surface. The generator's texture is kept, its cloth regraded so its mean
  matches the army's cloth mean (`roster/armies.json`) with white flecks
  clamped (`FLECK`); skin keeps its own colour. It is embedded at `ATLAS_PX`. Skin faces get
  their own material with no side tint. Every other face takes the full tint.
  Both materials share one atlas (`textures.ATLASES`, which keeps their UVs
  and embeds it). The vertex colour carries ambient occlusion at
  `textures.COLOUR_SCALE`, as a recipe material's does.

While building, space is the rig's rest space (`infantry_rig.py`), in the UBC
body's units: Z up, facing -Y.
"""
import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector

import skin_pins
import textures
from common import fbm, mat
from infantry_rig import SCALE

# The atlas, square px. A soldier is at most ~150 px tall on screen at tier 0,
# and each generated look's atlas counts against the catalog's load budget
# (scene-assets unitArt), so 512 is the size that lets every look ship.
ATLAS_PX = 512
# The generated body's triangles before the kit joins it: with the gloves and
# the weapon, tier 0 lands where the procedural soldiers' did (~28k).
BODY_TRIS = 22000
# The soldier's height with his helmet on, metres: the validator holds the
# stand-aim pose to the simulation's 1.70 m (fixtures/rules.json) within 5 cm.
HEIGHT_M = 1.72
# How wide a slice of the head band must be to be the head, of the soldier's
# height: a helmet is ~15% of it, a whip antenna ~1%.
HEAD_WIDE = 0.03
# Cloth texels brighter than this many times the cloth mean are generator
# flecks (white OCP specks): they are pulled down to it, toward the cloth's hue.
FLECK = 2.0
# How far past the shoulder joint an arm's seating move takes full effect (body units).
ARM_BLEND = 0.08
# How far from the hip and the ankle a leg's seating move takes full effect (body units).
LEG_BLEND = 0.12
# A pin's `front`: where its generator's front faces (Tripo +X, Meshy -Y), as
# the turn that brings it to the rig's -Y.
FRONT_TURN = {"+x": -np.pi / 2, "-y": 0.0, "-x": np.pi / 2, "+y": np.pi}
ARM_PREFIXES = ("upperarm", "lowerarm", "hand_", "index", "middle", "ring", "pinky", "thumb")
FINGERS = ("index", "middle", "ring", "pinky", "thumb")
SKIN_BONES = ("Head", "neck_01")


def _import(path, front):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path, merge_vertices=True)
    new = list(set(bpy.data.objects) - before)
    meshes = [o for o in new if o.type == "MESH"]
    bpy.ops.object.select_all(action="DESELECT")
    for o in meshes:
        o.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]
    if len(meshes) > 1:
        bpy.ops.object.join()
    gen = bpy.context.view_layer.objects.active
    mw = gen.matrix_world.copy()
    gen.parent = None
    gen.matrix_world = mw
    for o in new:
        if o is not gen and o.name in bpy.data.objects:
            bpy.data.objects.remove(o, do_unlink=True)
    gen.data.transform(Matrix.Rotation(FRONT_TURN[front], 4, "Z") @ gen.matrix_world)
    gen.matrix_world = Matrix.Identity(4)
    gen.name = gen.data.name = "generated_body"
    return gen


def _decimate(gen):
    """Collapse the generator's mesh to `BODY_TRIS`; UV seams are kept."""
    tris = sum(len(p.vertices) - 2 for p in gen.data.polygons)
    if tris <= BODY_TRIS:
        return
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = gen
    gen.select_set(True)
    dec = gen.modifiers.new("budget", "DECIMATE")
    dec.ratio = BODY_TRIS / tris
    bpy.ops.object.modifier_apply(modifier=dec.name)


def _atlas(gen):
    """The generator's albedo as a linear float (H, W, 3) array, rows bottom
    first (Blender's order, so a Blender UV indexes it directly)."""
    m = gen.data.materials[0]
    img = next(n.image for n in m.node_tree.nodes if n.type == "TEX_IMAGE" and n.image)
    w, h = img.size
    px = np.empty(w * h * 4, dtype=np.float32)
    img.pixels.foreach_get(px)
    srgb = px.reshape(h, w, 4)[..., :3].astype(float)
    return np.where(srgb <= 0.04045, srgb / 12.92, ((srgb + 0.055) / 1.055) ** 2.4)


def _sample(atlas, uv):
    h, w, _ = atlas.shape
    x = min(w - 1, max(0, int(uv[0] % 1.0 * w)))
    y = min(h - 1, max(0, int(uv[1] % 1.0 * h)))
    return atlas[y, x]


def _head_top(g, cx):
    """The top of the head (helmet on): the highest slice of a central band
    still `HEAD_WIDE` across, so a whip antenna or a ruck rising above the
    helmet never sets the soldier's height."""
    tall = g[:, 2].max() - g[:, 2].min()
    band = g[np.abs(g[:, 0] - cx) < 0.07 * tall]
    step, z = 0.005 * tall, band[:, 2].max()
    while z > band[:, 2].min():
        sl = band[(band[:, 2] <= z) & (band[:, 2] > z - 2 * step)]
        if len(sl) and np.ptp(sl[:, 0]) > HEAD_WIDE * tall:
            return z
        z -= step
    return band[:, 2].max()


def _fit(gen, body, arm):
    """Scale and place the generated T-pose on the rig's rest pose: its height
    (helmet on, `_head_top`) to `HEIGHT_M`, its fingertip span to the body's, its feet on
    the ground and its torso centred on the body's."""
    g = np.array([v.co[:] for v in gen.data.vertices])
    mw = body.matrix_world
    b = np.array([(mw @ v.co)[:] for v in body.data.vertices])
    g_half = (g[:, 0].max() - g[:, 0].min()) / 2
    g_cx = (g[:, 0].max() + g[:, 0].min()) / 2
    sz = (HEIGHT_M / SCALE - b[:, 2].min()) / (_head_top(g, g_cx) - g[:, 2].min())
    sx = (b[:, 0].max() - b[:, 0].min()) / (2 * g_half)

    def torso_y(pts, lo, hi):
        band = pts[(np.abs(pts[:, 0] - (pts[:, 0].max() + pts[:, 0].min()) / 2) < 0.15 * (hi - lo))
                   & (pts[:, 2] > lo + 0.45 * (hi - lo)) & (pts[:, 2] < lo + 0.65 * (hi - lo))]
        return (band[:, 1].max() + band[:, 1].min()) / 2

    gy = torso_y(g, g[:, 2].min(), g[:, 2].max())
    by = torso_y(b, b[:, 2].min(), b[:, 2].max())
    fit = (Matrix.Translation(((b[:, 0].max() + b[:, 0].min()) / 2, by, b[:, 2].min()))
           @ Matrix.Diagonal((sx, sz, sz, 1))
           @ Matrix.Translation((-g_cx, -gy, -g[:, 2].min())))
    gen.data.transform(fit)
    # how well the arm line lands on the rig's (elbows), for the log
    g = np.array([v.co[:] for v in gen.data.vertices])
    bones = arm.data.bones
    for side in ("l", "r"):
        e = bones["lowerarm_" + side].head_local
        near = g[np.abs(g[:, 0] - e.x) < 0.02]
        print(f"GENERATED_BODY elbow_{side} rig z={e.z:.3f} sleeve z={near[:, 2].min():.3f}..{near[:, 2].max():.3f}")
    print(f"GENERATED_BODY fit sx={sx:.4f} sz={sz:.4f} top={g[:, 2].max():.3f} (bare body top {b[:, 2].max():.3f})")


def _align_arms(gen, arm):
    """Seat each arm on the rig's arm bones. A generator's T-pose arms droop a
    little, which would leave the elbow joint at the sleeve's top edge. Each
    1 cm slice of the arm moves, shape unchanged, so its centre sits on the line
    through the shoulder, elbow and wrist joints. The move fades in over the
    first `ARM_BLEND` past the shoulder so the armpit stays joined."""
    bones = arm.data.bones
    me = gen.data
    for side in ("l", "r"):
        joints = [bones[n + side].head_local for n in ("upperarm_", "lowerarm_", "hand_")]
        sign = 1.0 if joints[0].x > 0 else -1.0
        start, end = abs(joints[0].x), abs(joints[2].x) + 0.25
        floor = joints[1].z - 0.2
        verts = [v for v in me.vertices if v.co.x * sign > start and v.co.z > floor]
        slices = {}
        for v in verts:
            slices.setdefault(int(abs(v.co.x) * 100), []).append(v)
        xs = [abs(j.x) for j in joints]

        def target(ax):
            if ax <= xs[1]:
                a, b, t = joints[0], joints[1], (ax - xs[0]) / (xs[1] - xs[0])
            else:
                a, b, t = joints[1], joints[2], (ax - xs[1]) / (xs[2] - xs[1])
            return a.y + (b.y - a.y) * t, a.z + (b.z - a.z) * t

        moves = {}
        for k, vs in slices.items():
            if len(vs) < 4:
                continue
            ty, tz = target((k + 0.5) / 100)
            cy = (max(v.co.y for v in vs) + min(v.co.y for v in vs)) / 2
            cz = (max(v.co.z for v in vs) + min(v.co.z for v in vs)) / 2
            moves[k] = (ty - cy, tz - cz)
        keys = sorted(moves)
        for v in verts:
            k = int(abs(v.co.x) * 100)
            near = min(keys, key=lambda q: abs(q - k)) if keys else None
            if near is None:
                continue
            dy, dz = moves[near]
            t = min(1.0, (abs(v.co.x) - start) / ARM_BLEND)
            v.co.y += dy * t
            v.co.z += dz * t
        print(f"GENERATED_BODY arm_{side} moved by up to {max((abs(d[1]) for d in moves.values()), default=0):.3f} in z over {len(moves)} slices")


def _align_legs(gen, arm):
    """Seat each leg on the rig's leg bones, front to back. A generated leg's
    knee can sit a few centimetres ahead of the rig's knee joint (a knee pad,
    a different stance), which swings it out and up whenever the knee bends.
    Each 1 cm slice between the ankle and the hip moves along Y, shape
    unchanged, so its centre sits on the line through the hip, knee and ankle
    joints. The move fades out over `LEG_BLEND` at both ends: the hip keeps
    joining the torso, and the foot keeps its own shape."""
    bones = arm.data.bones
    me = gen.data
    for side in ("l", "r"):
        joints = [bones[n + side].head_local for n in ("thigh_", "calf_", "foot_")]
        sign = 1.0 if joints[0].x > 0 else -1.0
        top, bottom = joints[0].z - LEG_BLEND, joints[2].z + 0.02
        verts = [v for v in me.vertices if v.co.x * sign > 0.0 and bottom < v.co.z < joints[0].z]
        slices = {}
        for v in verts:
            slices.setdefault(int(v.co.z * 100), []).append(v)

        def line_y(z):
            a, b = (joints[0], joints[1]) if z >= joints[1].z else (joints[1], joints[2])
            return a.y + (b.y - a.y) * (z - a.z) / (b.z - a.z)

        moves = {k: line_y((k + 0.5) / 100) - (max(v.co.y for v in vs) + min(v.co.y for v in vs)) / 2
                 for k, vs in slices.items() if len(vs) >= 4}
        keys = sorted(moves)
        for v in verts:
            k = min(keys, key=lambda q: abs(q - int(v.co.z * 100)))
            fade = min(1.0, (joints[0].z - v.co.z) / LEG_BLEND, (v.co.z - bottom) / LEG_BLEND)
            v.co.y += moves[k] * max(0.0, fade)
        print(f"GENERATED_BODY leg_{side} moved by up to {max((abs(d) for d in moves.values()), default=0):.3f} in y over {len(moves)} slices")


def _cut_hands(gen, arm):
    """Drop everything past the wrists; the glove shell replaces it."""
    wrist = min(abs(arm.data.bones["hand_l"].head_local.x), abs(arm.data.bones["hand_r"].head_local.x))
    bm = bmesh.new()
    bm.from_mesh(gen.data)
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if abs(v.co.x) > wrist - 0.01], context="VERTS")
    bm.to_mesh(gen.data)
    bm.free()


def _weights(gen, body, arm, scn):
    """Nearest-face weights from the body; arm vertices only from the arm."""
    def source(name, keep):
        o = body.copy()
        o.data = body.data.copy()
        o.name = name
        scn.collection.objects.link(o)
        for md in list(o.modifiers):
            o.modifiers.remove(md)
        names = {g.index: g.name for g in o.vertex_groups}
        bm = bmesh.new()
        bm.from_mesh(o.data)
        dl = bm.verts.layers.deform.active
        drop = [v for v in bm.verts if not keep(max(v[dl].items(), key=lambda e: e[1], default=(None, 0))[0], names)]
        bmesh.ops.delete(bm, geom=drop, context="VERTS")
        bm.to_mesh(o.data)
        bm.free()
        return o

    full = source("weights_body", lambda gi, names: True)
    arms = source("weights_arms", lambda gi, names: gi is not None and names[gi].startswith(ARM_PREFIXES))
    shoulder = min(abs(arm.data.bones["upperarm_l"].head_local.x), abs(arm.data.bones["upperarm_r"].head_local.x))
    zone_arm = gen.vertex_groups.new(name="_zone_arm")
    zone_body = gen.vertex_groups.new(name="_zone_body")
    floor = arm.data.bones["lowerarm_l"].head_local.z - 0.2  # the arms' band: the feet stand as wide
    for v in gen.data.vertices:
        on_arm = abs(v.co.x) > shoulder + 0.03 and v.co.z > floor
        (zone_arm if on_arm else zone_body).add([v.index], 1.0, "REPLACE")
    for g in body.vertex_groups:
        if g.name not in gen.vertex_groups:
            gen.vertex_groups.new(name=g.name)
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = gen
    # names, not the groups: applying a modifier rebuilds the list under them
    for src, zone in ((full, "_zone_body"), (arms, "_zone_arm")):
        dt = gen.modifiers.new("weights", "DATA_TRANSFER")
        dt.object = src
        dt.use_vert_data = True
        dt.data_types_verts = {"VGROUP_WEIGHTS"}
        dt.vert_mapping = "POLYINTERP_NEAREST"
        dt.layers_vgroup_select_src = "ALL"
        dt.layers_vgroup_select_dst = "NAME"
        dt.mix_mode = "REPLACE"
        dt.vertex_group = zone
        bpy.ops.object.modifier_apply(modifier=dt.name)
    for name in ("_zone_arm", "_zone_body"):
        gen.vertex_groups.remove(gen.vertex_groups[name])
    vg = gen.vertex_groups
    moves = [(v.index, vg[e.group].name, e.weight) for v in gen.data.vertices for e in v.groups
             if vg[e.group].name.startswith(FINGERS) and e.weight > 0]
    for i, name, w in moves:
        vg["hand_" + name[-1]].add([i], w, "ADD")
        vg[name].remove([i])
    gen.select_set(True)
    bpy.ops.object.vertex_group_clean(group_select_mode="ALL", limit=0.001)
    bpy.ops.object.vertex_group_limit_total(group_select_mode="ALL", limit=4)
    bpy.ops.object.vertex_group_normalize_all(lock_active=False)
    for o in (full, arms):
        bpy.data.objects.remove(o, do_unlink=True)


def _skin_like(rgb_linear):
    """Bare skin of any tone: an orange-red hue at a skin's saturation, from
    fair to dark. Coyote and OCP tan sit at a yellower hue, a balaclava's green
    and black kit outside it."""
    r, g, b = (float(c) ** (1 / 2.2) for c in rgb_linear)
    hi, lo = max(r, g, b), min(r, g, b)
    if hi < 0.06 or hi == lo or hi != r:
        return False
    hue = 60.0 * (g - b) / (hi - lo)
    return 0.0 <= hue <= 32.0 and 0.15 <= (hi - lo) / hi <= 0.8


def _surface(gen, atlas, cloth_mean):
    """Split skin from cloth, regrade the atlas, and set both materials."""
    me = gen.data
    uv = me.uv_layers[0]
    uv.name = "UVMap"
    names = {g.index: g.name for g in gen.vertex_groups}
    cloth = mat("generated", (1, 1, 1), rough=0.9, tint=1.0, role="fabric")
    skin = mat("generated_skin", (1, 1, 1), rough=0.6, role="skin")
    me.materials.clear()
    me.materials.append(cloth)
    me.materials.append(skin)
    acc, area = np.zeros(3), 0.0
    for poly in me.polygons:
        centre = sum((Vector(uv.data[li].uv) for li in poly.loop_indices), Vector((0.0, 0.0))) / poly.loop_total
        rgb = _sample(atlas, centre)
        heads = 0.0
        for vi in poly.vertices:
            for e in me.vertices[vi].groups:
                if names.get(e.group) in SKIN_BONES:
                    heads += e.weight
        poly.material_index = 1 if heads / poly.loop_total > 0.5 and _skin_like(rgb) else 0
        if poly.material_index == 0:
            acc += rgb * poly.area
            area += poly.area
    target = np.array(cloth_mean)
    gain = target / (acc / area)
    print(f"GENERATED_BODY cloth mean {np.round(acc / area, 4).tolist()} -> {np.round(target, 4).tolist()} gain {np.round(gain, 3).tolist()}")
    k = atlas.shape[0] // ATLAS_PX
    small = atlas.reshape(ATLAS_PX, k, ATLAS_PX, k, 3).mean((1, 3)) if k > 1 else atlas
    skin_texels = _mask(me, uv, 1)
    graded = np.where(skin_texels[..., None], small, _clamp_flecks(small * gain, target))
    rows = np.clip(graded, 0, 1)[::-1]  # PNG rows run top first
    s = (ATLAS_PX, ATLAS_PX)
    albedo = np.concatenate([textures._u8(textures._srgb(rows)), np.full((*s, 1), 255, np.uint8)], -1)
    orm = np.stack([np.full(s, 255, np.uint8), np.full(s, 230, np.uint8),
                    np.zeros(s, np.uint8), np.full(s, 255, np.uint8)], -1)
    normal = np.zeros((4, 4, 4), np.uint8) + np.array([128, 128, 255, 255], np.uint8)
    images = {"albedo": textures.png(albedo), "orm": textures.png(orm), "normal": textures.png(normal)}
    textures.ATLASES[cloth.name] = images
    textures.ATLASES[skin.name] = images


def _mask(me, uv, material_index):
    """The `ATLAS_PX` texels the faces of one material cover, from their UVs
    (rows bottom first, as the atlas)."""
    mask = np.zeros((ATLAS_PX, ATLAS_PX), bool)
    for poly in me.polygons:
        if poly.material_index != material_index:
            continue
        pts = [np.array(uv.data[li].uv) * ATLAS_PX for li in poly.loop_indices]
        for i in range(1, len(pts) - 1):
            a, b, c = pts[0], pts[i], pts[i + 1]
            x0, y0 = np.floor(np.minimum(np.minimum(a, b), c)).astype(int)
            x1, y1 = np.ceil(np.maximum(np.maximum(a, b), c)).astype(int)
            x0, y0, x1, y1 = max(x0, 0), max(y0, 0), min(x1, ATLAS_PX), min(y1, ATLAS_PX)
            if x1 <= x0 or y1 <= y0:
                continue
            ys, xs = np.mgrid[y0:y1, x0:x1] + 0.5
            d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
            if abs(d) < 1e-12:
                continue
            w0 = ((b[1] - c[1]) * (xs - c[0]) + (c[0] - b[0]) * (ys - c[1])) / d
            w1 = ((c[1] - a[1]) * (xs - c[0]) + (a[0] - c[0]) * (ys - c[1])) / d
            mask[y0:y1, x0:x1] |= (w0 >= 0) & (w1 >= 0) & (w0 + w1 <= 1)
    return mask


def _clamp_flecks(rgb, target):
    """Pull cloth texels brighter than `FLECK` times the cloth mean down to it,
    halfway toward the cloth's own hue at that brightness."""
    luma = np.array((0.2126, 0.7152, 0.0722))
    cap = FLECK * float(target @ luma)
    lum = rgb @ luma
    over = lum > cap
    scaled = rgb[over] * (cap / lum[over])[:, None]
    rgb = rgb.copy()
    rgb[over] = 0.5 * scaled + 0.5 * target * (cap / float(target @ luma))
    return rgb


def wear(kit, pin, cloth_mean):
    """Build the pinned generated body into `kit` (in place of its cloth shells)
    and return it. Call before anything that seats kit on the clothing."""
    rig = kit.rig
    gen = _import(skin_pins.verified(pin), pin["front"])
    atlas = _atlas(gen)
    _decimate(gen)
    _fit(gen, kit.body, kit.arm)
    _align_arms(gen, kit.arm)
    _align_legs(gen, kit.arm)
    _cut_hands(gen, kit.arm)
    _weights(gen, kit.body, kit.arm, rig.scn)
    _surface(gen, atlas, cloth_mean)
    for img in list(bpy.data.images):
        bpy.data.images.remove(img)
    gen.parent = kit.arm
    kit.parts.append(gen)
    # the vertex colour is a multiplier at COLOUR_SCALE: plain white, so AO alone shades it
    kit.painters[gen.name] = lambda p, n: (1 / textures.COLOUR_SCALE,) * 3
    w = kit.w
    glove = kit.shell("gloves", lambda co, n, v, dl: w(v, dl, kit.g_hand) > 0.3, kit.m["gloves"],
                      lambda co, n: 0.004, smooth=1, rim=0.0, pin=False,
                      painter=lambda p, n: tuple(c * (0.85 + 0.3 * fbm(p, 50, 7)) for c in kit.look["gloves"]))
    glove.modifiers.new("lighten", "DECIMATE").ratio = 0.6
    return gen


