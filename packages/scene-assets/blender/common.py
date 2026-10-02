"""Blender helpers shared by the appearance scripts: scene reset, flat PBR
materials, primitive parts, empties, vertex colour and export.

Engine-facing materials are plain Principled BSDFs: a base colour, roughness
and metallic, which is all a bundle carries (`scene-assets` `Material`).
Surface pattern (camouflage, wear, dirt) is baked into the `Col` colour
attribute, which the bundle carries per vertex and multiplies by the base
colour. A material whose custom property `tint` is set is the side-tint mask
(`Material.tint`): the renderer recolours it per side.

Ported from spike 03's `common.py` (specs/done/battle-look/assets/spikes/03/scripts),
minus its Cycles node materials and render rig, which our renderer cannot use.
"""

import math
import os
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../.."))

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import textures  # noqa: E402


def script_args():
    import sys

    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def reset(fps=30):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scn = bpy.context.scene
    scn.unit_settings.system = "METRIC"
    scn.render.fps = fps
    return scn


TEXTURED = {}  # material name -> recipe (textures.py)


def mat(name, base=(0.5, 0.5, 0.5), rough=0.6, metal=0.0, tint=0.0, texture=None, coverage=None, interior=None):
    """A flat material. `tint` > 0 marks it as the side-tint mask, with that weight.
    `texture` names a recipe the material samples (textures.py): its images carry
    colour, roughness and metalness, and the painted vertex colour becomes a
    multiplier relative to the recipe's mean (`paint`). `coverage` makes it a
    cutout or blended and `interior` a room behind a window (`textures.surface`)."""
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (*base, 1)
    b.inputs["Roughness"].default_value = rough
    b.inputs["Metallic"].default_value = metal
    if tint:
        m["tint"] = float(tint)
    if texture:
        TEXTURED[name] = texture
    return textures.surface(m, coverage, interior)


def texture_mean(material):
    """The linear mean albedo of a textured material's recipe (what a painter
    returns for "the texture as it is")."""
    return textures.baked(TEXTURED[material.name]).mean()


def texture_uvs(objects):
    """Box-projected UVs on every textured face of `objects`, at its recipe's tile."""
    bpy.context.view_layer.update()
    for o in objects:
        tiles = textures.material_tiles(o, TEXTURED)
        if tiles:
            textures.box_uv(o, tiles)


def obj_from_bm(name, bm, mats=None, parent=None):
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    o = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(o)
    for m in mats or []:
        me.materials.append(m)
    if parent:
        o.parent = parent
    return o


def bevel(o, width, segments=2, angle=None):
    bv = o.modifiers.new("bevel", "BEVEL")
    bv.width = width
    bv.segments = segments
    bv.limit_method = "ANGLE"
    if angle is not None:
        bv.angle_limit = math.radians(angle)
    return bv


def box(name, size, loc=(0, 0, 0), mat_=None, parent=None, bevel_=0.0, rot=(0, 0, 0), segments=2):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * size[0], v.co.y * size[1], v.co.z * size[2]))
    o = obj_from_bm(name, bm, [mat_] if mat_ else None, parent)
    o.location = loc
    o.rotation_euler = rot
    if bevel_ > 0:
        bevel(o, bevel_, segments)
    return o


def cyl(name, r, depth, loc=(0, 0, 0), axis="Z", mat_=None, parent=None, seg=24, bevel_=0.0, r2=None, rot=None):
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r, radius2=r if r2 is None else r2, depth=depth)
    turn = {"Z": Matrix.Identity(4), "X": Matrix.Rotation(math.pi / 2, 4, "Y"), "Y": Matrix.Rotation(math.pi / 2, 4, "X")}[axis]
    bmesh.ops.transform(bm, matrix=turn, verts=bm.verts)
    for f in bm.faces:
        f.smooth = True
    o = obj_from_bm(name, bm, [mat_] if mat_ else None, parent)
    o.location = loc
    if rot is not None:
        o.rotation_euler = rot
    if bevel_ > 0:
        bevel(o, bevel_, 2, 50)
    return o


def prism(name, profile_xz, width, loc=(0, 0, 0), mat_=None, parent=None, bevel_=0.0):
    """Extrude an XZ profile polygon across Y (centred)."""
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
    if bevel_ > 0:
        bevel(o, bevel_)
    return o


def empty(name, loc=(0, 0, 0), parent=None, size=0.1):
    e = bpy.data.objects.new(name, None)
    e.empty_display_size = size
    bpy.context.scene.collection.objects.link(e)
    if parent:
        e.parent = parent
    e.location = loc
    return e


def reparent_keep(o, parent):
    bpy.context.view_layer.update()
    mw = o.matrix_world.copy()
    o.parent = parent
    bpy.context.view_layer.update()
    o.matrix_world = mw


def bone_parent(o, arm, bone):
    """Parent `o` to a bone without moving it: the bake skins it rigidly to that joint."""
    bpy.context.view_layer.update()
    mw = o.matrix_world.copy()
    o.parent = arm
    o.parent_type = "BONE"
    o.parent_bone = bone
    bpy.context.view_layer.update()
    o.matrix_world = mw


# ---------------------------------------------------------------- baked surface colour

def _hash3(x, y, z, seed):
    n = math.sin(x * 127.1 + y * 311.7 + z * 74.7 + seed * 19.19) * 43758.5453
    return n - math.floor(n)


def value_noise(p, scale, seed=0.0):
    """Smooth 3D value noise in [0, 1): deterministic, no Blender texture nodes."""
    x, y, z = p[0] * scale, p[1] * scale, p[2] * scale
    xi, yi, zi = math.floor(x), math.floor(y), math.floor(z)
    xf, yf, zf = x - xi, y - yi, z - zi
    u, v, w = (t * t * (3 - 2 * t) for t in (xf, yf, zf))
    acc = 0.0
    for dx in (0, 1):
        for dy in (0, 1):
            for dz in (0, 1):
                h = _hash3(xi + dx, yi + dy, zi + dz, seed)
                acc += h * (u if dx else 1 - u) * (v if dy else 1 - v) * (w if dz else 1 - w)
    return acc


def fbm(p, scale, seed=0.0, octaves=3):
    total, amp, norm = 0.0, 1.0, 0.0
    for o in range(octaves):
        total += amp * value_noise(p, scale * (2 ** o), seed + o * 7.3)
        norm += amp
        amp *= 0.5
    return total / norm


def paint(o, colour_at, coords="rest"):
    """Write the `Col` colour attribute per face corner: colour_at(position, normal) ->
    (r, g, b) in linear space, or ((r, g, b), wear). Positions are the mesh's world rest
    positions, so a pattern is continuous across separate pieces of kit. On a textured
    material the colour is stored relative to its recipe's mean and the wear (how worn:
    0 clean, 1 fully) in alpha."""
    me = o.data
    material = me.materials[0] if me.materials else None
    recipe = TEXTURED.get(material.name) if material is not None else None
    for a in list(me.color_attributes):
        me.color_attributes.remove(a)
    while len(me.uv_layers) > 1:
        me.uv_layers.remove(me.uv_layers[-1])
    layer = me.color_attributes.new("Col", "BYTE_COLOR", "CORNER")
    me.color_attributes.active_color = layer
    mw = o.matrix_world
    nm = mw.to_3x3().inverted_safe().transposed()
    cache = {}
    for poly in me.polygons:
        for li in poly.loop_indices:
            vi = me.loops[li].vertex_index
            if vi not in cache:
                v = me.vertices[vi]
                c = colour_at(mw @ v.co, (nm @ v.normal).normalized())
                c, wear = c if len(c) == 2 else (c, 0.0)
                cache[vi] = (*textures.macro(c, recipe), wear) if recipe else (*c, 1.0)
            layer.data[li].color = cache[vi]


def paint_flat(o, rgb=(1, 1, 1)):
    paint(o, lambda p, n: rgb)


# ---------------------------------------------------------------- export

def export_glb(path, objects, animations=False):
    """Export exactly `objects` (meshes, their armature and empties) as a GLB,
    +Y up, modifiers applied, colour attributes and custom properties kept."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    bpy.ops.object.select_all(action="DESELECT")
    for o in objects:
        o.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=path,
        use_selection=True,
        export_tangents=bool(TEXTURED),
        export_format="GLB",
        export_yup=True,
        export_apply=True,
        export_animations=animations,
        export_animation_mode="ACTIONS",
        export_vertex_color="ACTIVE",
        export_all_vertex_colors=False,
        export_extras=True,
        export_image_format="NONE",
        export_texcoords=True,
        export_normals=True,
        export_skins=True,
        export_morph=False,
        export_def_bones=False,
    )
    textures.attach(path, TEXTURED)
    return path
