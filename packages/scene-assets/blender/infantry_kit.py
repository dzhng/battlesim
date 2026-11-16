"""Soldier assembly: a generated body on the rig, with the gloves, the weapon,
the sockets, ambient occlusion and the tiers, exported as one appearance
source. The roster's kits (`roster/infantry_equipment.py`) and the test
soldiers below are both built through it.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/infantry_kit.py rifle|recon|at|at_carried [a|b|c]

Writes the test soldier assets/source/test/<kind>.glb (look a) or
<kind>_<look>.glb (test unit art, never game content): the rig (scaled to the
simulation's soldier height, not turned) and one skinned mesh in four tiers
(`soldier_LOD0..3`), plus the `eye` socket under `Head` and the weapon's
`muzzle` socket under `hand_r`. A test soldier wears a pinned body
(`TEST_BODIES`) and keeps its test weapon and hold (`weapons.KINDS`).

Hard kit (the weapon, a slung launcher and its straps) is modelled in place
and skinned 100% to one bone at join. The weapon sits where its low-ready hold
puts it and rides `hand_r` in every clip of its hold family (`weapons.py`).
Materials that take the side colour carry a `tint` weight (`common.mat`).
"""

import math
import os
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import generated_body  # noqa: E402
import infantry_roster as roster  # noqa: E402
import skin_pins  # noqa: E402
import weapons  # noqa: E402
from common import REPO, box, empty, export_glb, fbm, mat, paint, script_args, texture_uvs  # noqa: E402
from infantry_rig import EYE_IN_HEAD, SCALE, SLUNG_MIDDLE, Rig, back_y, bone_of  # noqa: E402
from mesh_lods import canonical, make_tiers  # noqa: E402

# Each test soldier kind and the pinned body it wears, as (army, body kind);
# its look (a, b, c) is the body's.
TEST_BODIES = {
    "rifle": ("us_army_ocp", "rifle"),
    "at": ("us_army_ocp", "rifle"),
    "at_carried": ("us_army_ocp", "rifle"),
    "recon": ("us_army_ocp", "recon"),
}


# How much hand weight a body vertex needs to be glove: any at all, so the glove reaches
# back past the wrist, under the generated sleeve's cut end, so no wrist shows between.
GLOVE_CUFF = 0.0


def scale3(a, k):
    return tuple(x * k for x in a)


def clamp01(x):
    return 0.0 if x < 0 else 1.0 if x > 1 else x


class Kit:
    def __init__(self, rig, army):
        """`army` is the soldier's army (`roster/armies.json`): its gloves and webbing."""
        self.rig = rig
        self.army = army
        self.arm = rig.arm
        self.body = rig.body
        self.parts = []  # every mesh that ends up in the soldier
        self.painters = {}  # object name -> colour_at(p, n)
        # Material roles: the weapon's steel is steel, lenses dark glass, webbing
        # fabric; leather, plastics and polymer take none.
        m = self.m = {
            "pads": mat("pads", (1, 1, 1), rough=0.9, texture="hard_plastic"),
            "gloves": mat("gloves", (1, 1, 1), rough=0.85, texture="leather"),
            "lens": mat("lens", (1, 1, 1), rough=0.08, role="glass"),
            "gun_black": mat("gun_black", (1, 1, 1), rough=0.45, metal=0.3, texture="gunmetal", role="steel"),
            "gun_fde": mat("gun_fde", (1, 1, 1), rough=0.6, texture="polymer"),
            "gun_polymer": mat("gun_polymer", (1, 1, 1), rough=0.7, texture="polymer"),
            "webbing": mat("webbing", (1, 1, 1), rough=1.0, tint=0.6, texture="cordura", role="fabric"),
            "launcher": mat("launcher", (1, 1, 1), rough=0.7, texture="polymer"),
            "marking": mat("marking", (1, 1, 1), rough=0.75, role="marking"),
        }
        self.flat = {"pads": (0.03, 0.03, 0.028), "marking": (0.3, 0.29, 0.2), "gun_black": (0.03, 0.03, 0.03),
                     "gun_fde": (0.25, 0.20, 0.13), "gun_polymer": (0.03, 0.029, 0.027), "webbing": army["webbing"],
                     "launcher": (0.075, 0.08, 0.052), "lens": (0.012, 0.014, 0.014)}
        vg = self.body.vertex_groups
        self.g_hand = self.gidx([g.name for g in vg if g.name.startswith(("hand_", "index", "middle", "ring", "pinky", "thumb"))])
        self.slim_body()

    def slim_body(self):
        """The UBC body is a superhero. Pull each vertex toward its dominant bone's
        segment, by bone: narrower shoulders, thinner arms and neck, a shallower
        chest. The generated body takes its weights from this one. Joints stay
        where the rig has them."""
        pull = {"clavicle": 0.25, "upperarm": 0.4, "lowerarm": 0.3, "spine_03": 0.22, "spine_02": 0.14,
                "spine_01": 0.06, "neck_01": 0.2}
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

    def gidx(self, names):
        vg = self.body.vertex_groups
        return {vg[n].index for n in names if n in vg}

    @staticmethod
    def w(v, dl, groups):
        return sum(wt for gi, wt in v[dl].items() if gi in groups)

    def gloves(self):
        """The army's gloves: a thin shell cut from the body's hands, so they
        skin with it, lightened in the budget."""
        o = self.body.copy()
        o.data = self.body.data.copy()
        o.name = o.data.name = "gloves"
        self.rig.scn.collection.objects.link(o)
        for mod in list(o.modifiers):
            o.modifiers.remove(mod)
        bm = bmesh.new()
        bm.from_mesh(o.data)
        dl = bm.verts.layers.deform.active
        bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
        bm.normal_update()
        bmesh.ops.delete(bm, geom=[v for v in bm.verts if self.w(v, dl, self.g_hand) <= GLOVE_CUFF], context="VERTS")
        bmesh.ops.smooth_vert(bm, verts=list(bm.verts), factor=0.5, use_axis_x=True, use_axis_y=True, use_axis_z=True)
        bm.normal_update()
        for v, d in [(v, v.normal * 0.004) for v in bm.verts]:
            v.co += d
        bm.normal_update()
        for f in bm.faces:
            f.smooth = True
        bm.to_mesh(o.data)
        bm.free()
        o.data.materials.clear()
        o.data.materials.append(self.m["gloves"])
        o.parent = self.arm
        self.parts.append(o)
        rgb = self.army["gloves"]
        self.painters[o.name] = lambda p, n: tuple(c * (0.85 + 0.3 * fbm(p, 50, 7)) for c in rgb)
        o.modifiers.new("lighten", "DECIMATE").ratio = 0.6
        return o

    def rigid(self, o, bone, painter=None):
        self.rig.bone_parent(o, bone)
        self.parts.append(o)
        if painter:
            self.painters[o.name] = painter
        return o

    def sling(self, root, lift, standoff, name, muzzle_up=True, tip_deg=0.0, lean_deg=30.0, straps=(-.20, .30)):
        """Sling carried kit (`root`, a launcher's muzzle along its -Y) across the
        back at a `lean_deg` lean, muzzle up or down, tipped in by `tip_deg`: its
        origin `lift` high and `standoff` behind the back, on a retention strap at
        each of `straps` along it (`<name>_retention_<j>`). The kit and its straps
        ride the `sling` bone: the back while he lives, the ground beside him when
        he dies."""
        back = back_y(self.parts)
        rotation = Matrix.Rotation(math.radians(lean_deg), 4, "Y") @ Matrix.Rotation((-1 if muzzle_up else 1) * math.pi / 2, 4, "X")
        if tip_deg:
            rotation = Matrix.Rotation(math.radians(tip_deg), 4, "X") @ rotation
        root.matrix_world = Matrix.Translation((0, back + standoff, lift)) @ rotation
        self.rig.bone_parent(root, "sling")
        self.parts.extend(c for c in root.children_recursive if c.type == "MESH")
        for j, along in enumerate(straps):
            lug = root.matrix_world @ Vector((-.045, along, .10))
            anchor = Vector((-.09 if j == 0 else .09, back, 1.1 - along * .85))
            reach = lug - anchor
            strap = box(f"{name}_retention_{j}", (.035, .014, reach.length), (anchor + lug) / 2, self.m["webbing"],
                        rot=reach.to_track_quat("Z", "Y").to_euler())
            self.rigid(strap, "sling")

    # ---------------------------------------------------------------- weapon, sockets
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


def dressed(army, look, kind):
    """A fresh rig wearing `army`'s pinned generated body (`look`, `kind`): the
    `Kit` ready for its weapon. A body with no pin is an error naming its key."""
    pin = skin_pins.required("infantry", army=army, look=look, kind=kind)
    rig = Rig()
    for n in ("Eyes", "Eyebrows"):
        if n in bpy.data.objects:
            bpy.data.objects.remove(bpy.data.objects[n], do_unlink=True)
    data = roster.armies()[army]
    kit = Kit(rig, data)
    generated_body.wear(kit, pin, data["cloth_mean"])
    return kit


def export_soldier(kit, path):
    """Finish `kit` (eye socket, paint, join, AO, tiers, UVs) and export it to
    `path` with its rig and sockets. Returns the tiers."""
    rig = kit.rig
    kit.eye()
    kit.paint_all()
    soldier = kit.join()
    kit.bake_ao(soldier)
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
    os.makedirs(os.path.dirname(path), exist_ok=True)
    export_glb(path, [rig.arm, *tiers, bpy.data.objects["eye"], bpy.data.objects["muzzle"]])
    return tiers


def carried_launcher(kit):
    """Sling the test launcher as a roster missile tube is slung: muzzle up, its
    CLU to the side, its middle at `SLUNG_MIDDLE`, its CLU filling the gap to
    the back."""
    root = weapons.launcher(kit.m, "carried_launcher")
    for c in list(root.children_recursive):
        if c.type == "EMPTY":
            bpy.data.objects.remove(c, do_unlink=True)
    kit.sling(root, SLUNG_MIDDLE + bpy.data.objects["carried_launcher_tube"].location.y, -.02, "carried_launcher")


def main():
    args = script_args()
    kind = args[0] if args else "rifle"
    look = args[1] if len(args) > 1 else "a"
    if kind not in TEST_BODIES or look not in ("a", "b", "c"):
        raise SystemExit(f"Usage: infantry_kit.py {'|'.join(TEST_BODIES)} [a|b|c]")
    army, body = TEST_BODIES[kind]
    kit = dressed(army, look, body)
    if kind == "at_carried":
        carried_launcher(kit)
    kit.weapon(weapons.KINDS[kind])
    rel = f"assets/source/test/{kind if look == 'a' else f'{kind}_{look}'}.glb"
    export_soldier(kit, os.path.join(REPO, rel))
    print(f"wrote {rel}")


if __name__ == "__main__":
    main()
