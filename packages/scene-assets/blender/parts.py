"""Scripted parts for vehicles, buildings and props (the Muster technique).

Every model script builds its parts through these helpers:

- Primitives (`box`, `cyl`, `prism`, `loft`) make one mesh object per LOD
  tier, named `<part>_LOD<n>`. Segments fall with the tier, bevels exist only
  on the finer tiers, and a part can skip tiers (small detail drops out).
- `finish()` bakes the look into vertex colour, since bundles carry no
  textures: each material's paint (camouflage, plaster, tiles, rust, soot...),
  edge wear on convex edges, grime rising from the ground, and ambient
  occlusion ray-cast against the model itself. Large faces are split first
  so a pattern has vertices to live on.
- `export()` writes the GLB with custom properties (the articulation's
  extras) and the colour attribute.

Engine basis throughout: Z up, +X forward, origin on the ground at the unit's
position. Deterministic: same script, same arguments, same bytes.
"""
import bpy, bmesh, math, os, sys
from mathutils import Vector, Matrix, noise
from mathutils.bvhtree import BVHTree

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import textures  # noqa: E402

TIERS = (0, 1, 2, 3)
SEG_SCALE = (1.0, 0.6, 0.36, 0.2)
# Longest edge a painted face keeps, per tier: the pattern lives on vertices.
PAINT_EDGE_M = (0.28, 0.55, 1.2, 3.0)
MAX_TIER_SPLIT = 12


def script_args():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.unit_settings.system = "METRIC"


# ---------------------------------------------------------------- paints
# A paint is a function (world position, normal, edge, rng seed) -> linear
# albedo. Its material carries the roughness and metalness and a base colour
# of `BASE` grey; the vertex colour holds albedo / BASE, so dark paints keep
# precision in the bundle's 8-bit colour.
BASE = 0.5
PAINTS = {}


def lerp3(a, b, t):
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))


def smoothstep(e0, e1, x):
    t = max(0.0, min(1.0, (x - e0) / (e1 - e0)))
    return t * t * (3 - 2 * t)


def fbm(p, scale, octaves=3, seed=0.0):
    q = Vector((p.x * scale + seed, p.y * scale + seed * 0.7, p.z * scale + seed * 1.3))
    v, amp, tot = 0.0, 1.0, 0.0
    for _ in range(octaves):
        v += noise.noise(q, noise_basis="PERLIN_ORIGINAL") * amp
        tot += amp
        amp *= 0.5
        q = q * 2.03
    return v / tot  # about -1..1


def paint(name, rough=0.6, metal=0.0, coverage=None, interior=None):
    """Register a paint function under `name`, with its material. `coverage`
    makes it a cutout or blended and `interior` a room behind a window
    (`textures.surface`)."""

    def wrap(fn):
        PAINTS[name] = fn
        m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
        m.use_nodes = True
        b = m.node_tree.nodes["Principled BSDF"]
        b.inputs["Base Color"].default_value = (BASE, BASE, BASE, 1)
        b.inputs["Roughness"].default_value = rough
        b.inputs["Metallic"].default_value = metal
        return textures.surface(m, coverage, interior)

    return wrap


def flat_paint(name, colour, rough=0.6, metal=0.0, wear=0.0, grime=1.0, coverage=None, interior=None):
    def fn(p, n, edge):
        c = colour
        v = fbm(p, 3.0, 2, 11.0) * 0.08
        c = tuple(x * (1 + v) for x in c)
        if wear > 0 and edge > 0:
            c = lerp3(c, (0.36, 0.35, 0.33), min(1.0, edge * wear))
        return grime_rise(c, p, grime)

    paint(name, rough, metal, coverage, interior)(fn)
    return bpy.data.materials[name]


def grime_rise(c, p, amount, top=1.2, colour=(0.13, 0.11, 0.08)):
    """Dust and mud rising from the ground, broken by noise."""
    if amount <= 0:
        return c
    rise = 1.0 - smoothstep(0.05, top, p.z)
    brk = 0.5 + 0.5 * fbm(p, 2.2, 3, 5.0)
    return lerp3(c, colour, max(0.0, min(0.85, rise * amount * (0.55 + brk))))


def streaks(c, p, n, amount, colour=(0.05, 0.045, 0.04)):
    """Vertical rain streaks and grime on walls (not on up-facing faces)."""
    if amount <= 0 or abs(n.z) > 0.7:
        return c
    s = noise.noise(Vector((p.x * 7.0 + p.y * 7.0, 0.0, p.z * 0.35)), noise_basis="PERLIN_ORIGINAL")
    t = max(0.0, s) * amount
    return lerp3(c, colour, min(0.6, t))


def material(name):
    return bpy.data.materials[name]


# ---------------------------------------------------------------- textured paints
# A textured material samples a baked recipe (`textures.py`) through box-
# projected UVs; its vertex colour holds only what the model knows, relative to
# the recipe's mean: ambient occlusion, grime and hue (`macro`), and in alpha
# how worn the surface is (edges chip, mud rises from the ground), which the
# texture's wear threshold breaks up into crisp chips and spatter.
TEXTURED = {}  # material name -> recipe name
# Textured faces need vertices only for the macro look: split them coarser.
TEXTURED_EDGE_SCALE = 1.6
# Summer field dust and dried mud, and a fire's ash (linear albedo).
DUST = (0.15, 0.13, 0.1)
ASH = (0.16, 0.155, 0.145)
RUST = (0.075, 0.036, 0.016)  # steel whose paint burnt off, weathered to rust
LICHEN = (0.085, 0.095, 0.05)  # grey-green crust and moss on old stone
# Where a burnt-out vehicle's fire vented (world point, reach in metres): its
# script sets them, and every textured paint blackens round them.
SCORCH = []


def textured(name, recipe, rough=None, metal=None, tint=0.0, colour=None, dirt=0.7, chip=0.8, streak=0.3,
             rise=1.0, seed=0.0, soot=0.0, ash=0.0, dust=DUST, mottle=0.0, lichen=0.0, coverage=None, grain=0.06):
    """A material that samples `recipe`: `colour` (linear) tints the recipe's mean,
    `chip` scales wear on convex edges, `dirt` the dust (colour `dust`) and mud rising
    from the ground to `rise` metres, `streak` rain streaks on walls, `soot` blackens
    walls and undersides (a fire's smoke), `ash` greys what faces up, `mottle` varies the tone
    piece to piece and `lichen` greens what faces the sky. `grain` is the tone's own fine
    variation, vertex to vertex: 0 on a big flat surface seen from far off, where it
    shows as the mesh's grid. `coverage` makes it a cutout or blended, by the recipe's
    coverage image (`textures.surface`)."""
    mean = textures.baked(recipe).mean()
    hue = tuple(c / m for c, m in zip(colour, mean)) if colour else (1.0, 1.0, 1.0)

    def fn(p, n, edge):
        k = 1.0 + grain * fbm(p, 3.0, 2, 17.0 + seed)
        if mottle:  # piece-to-piece tone: blotches about a stone or a board across
            k *= 1.0 + mottle * fbm(p, 3.5, 1, 53.0 + seed)
        wall = 1.0 - smoothstep(0.3, 0.9, abs(n.z))
        if soot:  # smoke rose up the walls: black above, the paint burnt to rust below
            patchy = 0.6 + 0.4 * (0.5 + 0.5 * fbm(p, 1.6, 3, 29.0 + seed))
            smoke = soot * wall * patchy * smoothstep(0.6, 1.6, p.z)
            k *= 1.0 - min(0.9, smoke)
        c = tuple(m * h * k for m, h in zip(mean, hue))
        if soot:
            burnt = soot * (0.35 + 0.65 * wall) * (1.0 - smoothstep(1.0, 2.4, p.z)) * smoothstep(-0.25, 0.35, fbm(p, 1.8, 3, 41.0 + seed))
            c = lerp3(c, RUST, min(0.9, burnt * 2.2))
        c = streaks(c, p, n, streak)
        ground = (1.0 - smoothstep(0.05, rise, p.z)) * (0.55 + 0.45 * (0.5 + 0.5 * fbm(p, 2.2, 3, 5.0 + seed)))
        # dust and dried mud film the lower parts, lighter than dark paint
        c = lerp3(c, dust, ground * dirt * 0.7)
        if lichen:  # lichen and moss on the tops and the damp foot
            grow = max(smoothstep(0.3, 0.9, n.z), 1.0 - smoothstep(0.0, 0.35, p.z))
            c = lerp3(c, LICHEN, lichen * grow * smoothstep(-0.1, 0.5, fbm(p, 4.0, 3, 61.0 + seed)))
        if ash:  # grey ash settled on everything that faces up
            c = lerp3(c, ASH, ash * smoothstep(0.4, 0.9, n.z) * (0.5 + 0.5 * (0.5 + 0.5 * fbm(p, 2.5, 3, 31.0 + seed))))
        for vent, reach in SCORCH:  # black fans round where the fire vented, over the ash, in a
            d = (p - vent).length  # wider ring of paint blistered off to rust
            blister = (1.0 - smoothstep(0.7 * reach, 1.8 * reach, d)) * smoothstep(-0.3, 0.4, fbm(p, 2.2, 3, 43.0))
            c = lerp3(c, RUST, 0.75 * blister)
            burn = 0.85 * (1.0 - smoothstep(0.2 * reach, reach, d)) * (0.7 + 0.3 * (0.5 + 0.5 * fbm(p, 4.0, 2, 37.0)))
            c = tuple(x * (1.0 - burn) for x in c)
        # edges chip, but never through: a thin part is all edge
        chipped = min(0.62, edge * edge * chip * (0.7 + 0.3 * (0.5 + 0.5 * fbm(p, 9.0, 2, 1.0 + seed))))
        chipped *= smoothstep(0.3, 0.7, 0.5 + 0.5 * fbm(p, 1.3, 2, 3.0 + seed))  # some edges clean, some battered
        wear = max(chipped, ground * dirt * 1.25)
        return c, max(0.0, min(1.0, wear))

    PAINTS[name] = fn
    TEXTURED[name] = recipe
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (1, 1, 1, 1)
    b.inputs["Roughness"].default_value = 1.0 if rough is None else rough
    b.inputs["Metallic"].default_value = 1.0 if metal is None else metal
    if tint:
        m["tint"] = float(tint)
    return textures.surface(m, coverage)


# NATO three-colour camouflage, linear albedo: the spike's hues, darkened and
# desaturated after the workbench's sun showed them as bright toy greens
WOODLAND = ((0.062, 0.082, 0.045), (0.085, 0.062, 0.04), (0.013, 0.013, 0.012))


def woodland(name, colours=WOODLAND, scale=0.6, wear=0.4, dirt=0.6, rough=0.62, seed=2.0):
    """Spike 03's disruptive camouflage (two noise fields, constant bands),
    with edge wear and ground dirt: here as a vertex paint."""

    def fn(p, n, edge):
        f = 0.5 + 0.5 * fbm(p, scale, 3, seed)
        g = 0.5 + 0.5 * fbm(p, scale * 1.7, 2, 7.3 + seed)
        c = colours[0] if f < 0.46 else colours[1] if f < 0.58 else colours[2]
        if len(colours) > 3 and g > 0.66:
            c = colours[3]
        # sun-faded top surfaces, a little paint variation
        v = fbm(p, 4.0, 2, 3.0) * 0.07
        c = tuple(x * (1.0 + v + (0.05 if n.z > 0.7 else 0.0)) for x in c)
        if wear > 0 and edge > 0.15:
            w = edge * (0.4 + 0.6 * (0.5 + 0.5 * fbm(p, 18, 2, 1.0))) * wear * 1.6
            c = lerp3(c, (0.2, 0.19, 0.17), min(0.9, w))
        c = streaks(c, p, n, 0.35)
        return grime_rise(c, p, dirt)

    return paint(name, rough)(fn)


def burnt(name, rough=0.9, dirt=0.3, seed=4.0):
    """The wreck variant: a soot-black and charcoal shell, grey ash settled on the
    top faces, rust breaking through in patches and on worn edges, and a ghost of
    the paint low down where the fire did not reach."""

    def fn(p, n, edge):
        f = 0.5 + 0.5 * fbm(p, 0.9, 3, seed)
        g = 0.5 + 0.5 * fbm(p, 2.6, 3, seed + 9.0)
        soot = (0.014, 0.0135, 0.013)
        charcoal = (0.045, 0.043, 0.041)
        rust = (0.085, 0.036, 0.017)
        ash = (0.14, 0.135, 0.125)
        paint_ghost = (0.05, 0.055, 0.035)
        c = lerp3(soot, charcoal, smoothstep(0.3, 0.7, g))
        c = lerp3(c, rust, smoothstep(0.66, 0.8, f) * 0.85)
        c = lerp3(c, paint_ghost, (1 - smoothstep(0.4, 1.0, p.z)) * smoothstep(0.55, 0.75, g) * 0.6)
        if n.z > 0.6:
            c = lerp3(c, ash, smoothstep(0.45, 0.8, g) * 0.6)
        if edge > 0.3:
            c = lerp3(c, rust, min(0.35, edge * 0.35))
        return grime_rise(c, p, dirt)

    return paint(name, rough)(fn)


# ---------------------------------------------------------------- primitives
def _obj(name, bm, mat, parent, loc, rot, smooth_all=False):
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    if smooth_all:
        me.shade_smooth()
    o = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(o)
    if mat is not None:
        me.materials.append(mat)
    if parent is not None:
        o.parent = parent
    o.location = loc
    o.rotation_euler = rot
    return o


def _bevel(bm, width, segments, angle_deg=30):
    if width <= 0 or segments <= 0:
        return
    edges = [e for e in bm.edges if len(e.link_faces) == 2 and e.calc_face_angle(0) > math.radians(angle_deg)]
    if edges:
        bmesh.ops.bevel(bm, geom=edges, offset=width, offset_type="OFFSET", segments=segments, profile=0.5,
                        affect="EDGES", clamp_overlap=True)


def _bevel_for(lod, bevel):
    if bevel <= 0 or lod >= 2:
        return 0, 0
    return bevel, (2 if lod == 0 else 1)


def _each(name, lods, make):
    out = []
    for lod in lods:
        o = make(lod, f"{name}_LOD{lod}")
        if o is not None:
            out.append(o)
    return out


def box(name, size, loc=(0, 0, 0), mat=None, parent=None, bevel=0.0, rot=(0, 0, 0), lods=TIERS, taper=None):
    """A box of full `size`. `taper` (tx, ty) scales the top face's x and y."""

    def make(lod, n):
        bm = bmesh.new()
        bmesh.ops.create_cube(bm, size=1.0)
        for v in bm.verts:
            sx, sy = 1.0, 1.0
            if taper and v.co.z > 0:
                sx, sy = taper
            v.co = Vector((v.co.x * size[0] * sx, v.co.y * size[1] * sy, v.co.z * size[2]))
        w, s = _bevel_for(lod, bevel)
        _bevel(bm, w, s)
        return _obj(n, bm, mat, parent, loc, rot)

    return _each(name, lods, make)


def cyl(name, r, depth, loc=(0, 0, 0), axis="Z", mat=None, parent=None, seg=24, bevel=0.0, r2=None, lods=TIERS,
        rot=(0, 0, 0), caps=True, min_seg=6):
    def make(lod, n):
        s = max(min_seg, int(round(seg * SEG_SCALE[lod])))
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=caps, segments=s, radius1=r, radius2=r if r2 is None else r2, depth=depth)
        m = {"Z": Matrix.Identity(4), "X": Matrix.Rotation(math.pi / 2, 4, "Y"), "Y": Matrix.Rotation(math.pi / 2, 4, "X")}[axis]
        bmesh.ops.transform(bm, matrix=m, verts=bm.verts)
        w, sg = _bevel_for(lod, bevel)
        _bevel(bm, w, sg, 50)
        # sides smooth, caps flat
        ax = {"Z": Vector((0, 0, 1)), "X": Vector((1, 0, 0)), "Y": Vector((0, 1, 0))}[axis]
        for f in bm.faces:
            f.smooth = abs(f.normal.dot(ax)) < 0.9
        return _obj(n, bm, mat, parent, loc, rot)

    return _each(name, lods, make)


def prism(name, profile_xz, width, loc=(0, 0, 0), mat=None, parent=None, bevel=0.0, lods=TIERS, rot=(0, 0, 0),
          taper_y=None):
    """Extrude an XZ profile across Y (centred). `taper_y(z)` scales the half width by height."""

    def make(lod, n):
        bm = bmesh.new()
        hw = width / 2
        k = (lambda z: 1.0) if taper_y is None else taper_y
        a = [bm.verts.new((x, -hw * k(z), z)) for x, z in profile_xz]
        b = [bm.verts.new((x, hw * k(z), z)) for x, z in profile_xz]
        m = len(profile_xz)
        bm.faces.new(a[::-1])
        bm.faces.new(b)
        for i in range(m):
            j = (i + 1) % m
            bm.faces.new((a[i], a[j], b[j], b[i]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        w, s = _bevel_for(lod, bevel)
        _bevel(bm, w, s)
        return _obj(n, bm, mat, parent, loc, rot)

    return _each(name, lods, make)


def loft(name, rings, mat=None, parent=None, bevel=0.0, lods=TIERS, loc=(0, 0, 0), rot=(0, 0, 0)):
    """A closed solid through horizontal rings (lists of (x, y) at a z): [(z, [(x, y)...]), ...]."""

    def make(lod, n):
        bm = bmesh.new()
        vs = [[bm.verts.new((x, y, z)) for x, y in pts] for z, pts in rings]
        bm.faces.new(vs[0][::-1])
        bm.faces.new(vs[-1])
        for r in range(len(vs) - 1):
            a, b = vs[r], vs[r + 1]
            m = len(a)
            for i in range(m):
                j = (i + 1) % m
                bm.faces.new((a[i], a[j], b[j], b[i]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        w, s = _bevel_for(lod, bevel)
        _bevel(bm, w, s)
        return _obj(n, bm, mat, parent, loc, rot)

    return _each(name, lods, make)


def mesh_part(name, build, mat=None, parent=None, lods=TIERS, loc=(0, 0, 0), rot=(0, 0, 0), smooth=False):
    """A part from a function `build(bm, lod)` that fills a bmesh."""

    def make(lod, n):
        bm = bmesh.new()
        if build(bm, lod) is False:
            bm.free()
            return None
        return _obj(n, bm, mat, parent, loc, rot, smooth_all=smooth)

    return _each(name, lods, make)


def stencil(name, text, height, loc, rot, mat, parent=None, lods=(0, 1), depth=0.003):
    """Painted markings: `text` (Blender's built-in font, `\\n` for lines) as a thin
    raised mesh of letters `height` tall, lying in its local XY plane facing +Z,
    centred on `loc` and turned by `rot`."""

    def make(lod, n):
        cu = bpy.data.curves.new(n + "_text", "FONT")
        cu.body = text
        cu.size = height
        cu.extrude = depth / 2
        cu.align_x = "CENTER"
        cu.align_y = "CENTER"
        cu.resolution_u = 2
        tmp = bpy.data.objects.new(n + "_curve", cu)
        bpy.context.scene.collection.objects.link(tmp)
        bpy.context.view_layer.update()
        me = bpy.data.meshes.new_from_object(tmp.evaluated_get(bpy.context.evaluated_depsgraph_get()))
        bpy.data.objects.remove(tmp, do_unlink=True)
        bm = bmesh.new()
        bm.from_mesh(me)
        bmesh.ops.translate(bm, verts=bm.verts, vec=(0, 0, depth / 2))  # the back face on the surface
        return _obj(n, bm, mat, parent, loc, rot)

    return _each(name, lods, make)


def sheet(name, w, h, loc=(0, 0, 0), mat=None, parent=None, lods=TIERS, rot=(0, 0, 0)):
    """One face `w` wide and `h` tall, standing in its local XZ plane with its foot's middle at
    `loc`: a pane of glass, a grille, a perforated panel. It is drawn from both sides, so a
    blended or cutout surface is one face and never a thin box (two layers, and their edges)."""

    def make(lod, n):
        bm = bmesh.new()
        bm.faces.new([bm.verts.new(p) for p in ((-w / 2, 0, 0), (w / 2, 0, 0), (w / 2, 0, h), (-w / 2, 0, h))])
        return _obj(n, bm, mat, parent, loc, rot)

    return _each(name, lods, make)


def room(name, sheet_name):
    """The material of a room behind a window: it shows a cell of the interior atlas sheet
    `sheet_name` ("rooms", "shops") and has no look of its own (`textures.surface`)."""
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (0.03, 0.03, 0.03, 1)
    b.inputs["Roughness"].default_value = 1.0
    return textures.surface(m, interior=sheet_name)


def room_box(name, w, h, d, mat, parent=None, loc=(0, 0, 0), rot=(0, 0, 0), lods=TIERS):
    """The open box behind a window, `w` wide, `h` tall and `d` deep: a back wall, a floor, a
    ceiling and two side walls, running into the building along local +Y from the middle of
    its open face's foot at `loc`. `mat` is a `room` material.

    Its UVs are the box unfolded round its back wall, which is the unit square (u across,
    v up); the floor, the ceiling and the side walls hang off the back wall's four edges and
    reach one unit out at the open face. They are straight in the box's own space, so the
    shader's pinhole lookup (city/README.md, "Interiors") is exact at every pixel, and a box
    of any size shows the whole of its cell."""

    def make(lod, n):
        bm = bmesh.new()
        uv = bm.loops.layers.uv.new("UVMap")
        x0, x1 = -w / 2, w / 2

        def face(points):
            f = bm.faces.new([bm.verts.new(p) for p, _ in points])
            for loop, (_, at) in zip(f.loops, points):
                loop[uv].uv = at

        face((((x0, d, 0), (0, 0)), ((x1, d, 0), (1, 0)), ((x1, d, h), (1, 1)), ((x0, d, h), (0, 1))))  # back
        face((((x0, 0, 0), (0, -1)), ((x1, 0, 0), (1, -1)), ((x1, d, 0), (1, 0)), ((x0, d, 0), (0, 0))))  # floor
        face((((x0, d, h), (0, 1)), ((x1, d, h), (1, 1)), ((x1, 0, h), (1, 2)), ((x0, 0, h), (0, 2))))  # ceiling
        face((((x0, 0, 0), (-1, 0)), ((x0, d, 0), (0, 0)), ((x0, d, h), (0, 1)), ((x0, 0, h), (-1, 1))))  # left
        face((((x1, d, 0), (1, 0)), ((x1, 0, 0), (2, 0)), ((x1, 0, h), (2, 1)), ((x1, d, h), (1, 1))))  # right
        return _obj(n, bm, mat, parent, loc, rot)

    return _each(name, lods, make)


def empty(name, loc=(0, 0, 0), parent=None, rot=(0, 0, 0), props=None):
    e = bpy.data.objects.new(name, None)
    e.empty_display_size = 0.2
    bpy.context.scene.collection.objects.link(e)
    if parent is not None:
        e.parent = parent
    e.location = loc
    e.rotation_euler = rot
    for k, v in (props or {}).items():
        e[k] = v
    return e


# ---------------------------------------------------------------- finishing
def tier_of(o):
    n = o.name
    i = n.rfind("_LOD")
    return int(n[i + 4:]) if i >= 0 and n[i + 4:].isdigit() else None


def rest_on_ground(lift=0.0, keep=()):
    """Push any vertex below the ground up onto it (tumbled rubble, a tilted wreck).
    `lift` rests it that far above: a part lying flat (a thrown track) must not share
    the ground's plane, or the two fight for depth. Parts named by `keep` stand below
    the ground on purpose (a bridge's piers)."""
    bpy.context.view_layer.update()
    for o in bpy.data.objects:
        if o.type != "MESH" or (keep and o.name.startswith(keep)):
            continue
        mw = o.matrix_world
        inv = mw.inverted_safe()
        for v in o.data.vertices:
            w = mw @ v.co
            if w.z < lift:
                w.z = lift
                v.co = inv @ w


def _split_long_edges(me, limit):
    bm = bmesh.new()
    bm.from_mesh(me)
    bmesh.ops.triangulate(bm, faces=[f for f in bm.faces if len(f.verts) > 4])
    for _ in range(MAX_TIER_SPLIT):
        long_ = [e for e in bm.edges if e.calc_length() > limit]
        if not long_:
            break
        bmesh.ops.subdivide_edges(bm, edges=long_, cuts=1, use_grid_fill=True)
        bmesh.ops.triangulate(bm, faces=[f for f in bm.faces if len(f.verts) > 4])
    bm.to_mesh(me)
    bm.free()


def _edge_factor(bm):
    """Per vertex: how sharply convex its surroundings are (0 flat, ~1 edge)."""
    out = {}
    for v in bm.verts:
        best = 0.0
        for e in v.link_edges:
            if len(e.link_faces) != 2 or not e.is_convex:
                continue
            a = e.calc_face_angle(0)
            best = max(best, a)
        out[v.index] = min(1.0, best / math.radians(45))
    return out


def _hemisphere(k):
    dirs = []
    golden = math.pi * (3 - math.sqrt(5))
    for i in range(k):
        z = 1 - (i + 0.5) / k
        r = math.sqrt(max(0, 1 - z * z))
        a = i * golden
        dirs.append(Vector((math.cos(a) * r, math.sin(a) * r, z)))
    return dirs


def finish(ao_distance=1.2, ao_strength=0.8, ao_rays=12, ground_ao=True, paint_scale=1.0, objects=None):
    """Bake paint, wear, grime and occlusion into every tier's vertex colour.
    `paint_scale` stretches the longest painted edge (buildings are larger).
    `objects` bakes those alone, occluded only by each other (a kit's module is
    baked on its own: it is drawn on many buildings); the default is the scene."""
    bpy.context.view_layer.update()
    meshes = [o for o in (bpy.data.objects if objects is None else objects) if o.type == "MESH"]
    # two parts given one name: Blender renamed one `name.001`, which loses its tier
    clashes = [o.name for o in meshes if tier_of(o) is None and "_LOD" in o.name]
    if clashes:
        raise SystemExit(f"parts share a name: {clashes[:5]}")
    for o in meshes:
        t = tier_of(o)
        mat = o.data.materials[0] if o.data.materials else None
        if mat is not None and mat.name in PAINTS and t is not None:
            coarse = TEXTURED_EDGE_SCALE if mat.name in TEXTURED else 1.0
            _split_long_edges(o.data, PAINT_EDGE_M[t] * paint_scale * coarse)
    dirs = _hemisphere(ao_rays)
    for tier in TIERS:
        group = [o for o in meshes if tier_of(o) in (tier, None)]
        verts, polys = [], []
        for o in group:
            mw = o.matrix_world
            base = len(verts)
            verts.extend(mw @ v.co for v in o.data.vertices)
            polys.extend([base + i for i in p.vertices] for p in o.data.polygons)
        if ground_ao:
            base = len(verts)
            verts.extend([Vector((-40, -40, -0.001)), Vector((40, -40, -0.001)), Vector((40, 40, -0.001)), Vector((-40, 40, -0.001))])
            polys.append([base, base + 1, base + 2, base + 3])
        tree = BVHTree.FromPolygons(verts, polys, epsilon=0.0)
        for o in group:
            if tier_of(o) != tier:
                continue
            _colour(o, tree, dirs, ao_distance, ao_strength)


def _colour(o, tree, dirs, dist, strength):
    me = o.data
    mat = me.materials[0] if me.materials else None
    fn = PAINTS.get(mat.name) if mat is not None else None
    recipe = TEXTURED.get(mat.name) if mat is not None else None
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.verts.ensure_lookup_table()
    edge = _edge_factor(bm)
    mw = o.matrix_world
    nm = mw.to_3x3().inverted_safe().transposed()
    cols = []
    for v in bm.verts:
        p = mw @ v.co
        n = (nm @ v.normal).normalized()
        c = fn(p, n, edge[v.index]) if fn else (1.0, 1.0, 1.0)
        wear = 1.0
        if recipe:
            c, wear = c
        # occlusion: rays over the normal's hemisphere
        t = n.orthogonal().normalized()
        b = n.cross(t)
        start = p + n * 0.012
        hit = 0
        for d in dirs:
            w = t * d.x + b * d.y + n * d.z
            if tree.ray_cast(start, w, dist)[0] is not None:
                hit += 1
        occ = hit / len(dirs)
        k = 1.0 - strength * occ
        if recipe:
            m = textures.macro(c, recipe)
            cols.append((m[0] * k, m[1] * k, m[2] * k, wear))
        else:
            cols.append(tuple(max(0.0, min(1.0, c[i] * k / BASE)) for i in range(3)) + (1.0,))
    bm.free()
    attr = me.color_attributes.new("Color", "FLOAT_COLOR", "POINT")
    for i, c in enumerate(cols):
        attr.data[i].color = c
    me.color_attributes.active_color = attr


# ---------------------------------------------------------------- stats and export
def triangles_by_tier():
    out = [0, 0, 0, 0]
    for o in bpy.data.objects:
        if o.type != "MESH":
            continue
        me = o.data
        me.calc_loop_triangles()
        n = len(me.loop_triangles)
        t = tier_of(o)
        for k in TIERS:
            if t is None or t == k:
                out[k] += n
    return out


def export(path, worn=True):
    """Export the GLB: textured parts take box-projected UVs (unless they carry
    their own, as a track's links do) and tangents, then the recipes' images are
    written into it. `worn` false is for surfaces that never wear (`textures.attach`)."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    for o in list(bpy.data.objects):
        if o.type in ("CAMERA", "LIGHT"):
            bpy.data.objects.remove(o, do_unlink=True)
    bpy.context.view_layer.update()
    for o in bpy.data.objects:
        mat = o.data.materials[0] if o.type == "MESH" and o.data.materials else None
        if mat is not None and mat.name in TEXTURED and not o.data.uv_layers:
            textures.box_uv(o, textures.tile_of(TEXTURED[mat.name]))
    bpy.ops.export_scene.gltf(
        export_tangents=True,
        filepath=path,
        export_format="GLB",
        export_apply=True,
        export_yup=True,
        export_extras=True,
        export_vertex_color="ACTIVE",
        export_all_vertex_colors=False,
        export_animations=False,
        export_texcoords=True,
        export_normals=True,
        export_materials="EXPORT",
        export_image_format="NONE",
    )
    textures.attach(path, TEXTURED, worn)
    print("GLB", path, os.path.getsize(path))
