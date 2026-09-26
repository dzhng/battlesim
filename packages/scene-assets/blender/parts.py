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


def paint(name, rough=0.6, metal=0.0):
    """Register a paint function under `name`, with its material."""

    def wrap(fn):
        PAINTS[name] = fn
        m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
        m.use_nodes = True
        b = m.node_tree.nodes["Principled BSDF"]
        b.inputs["Base Color"].default_value = (BASE, BASE, BASE, 1)
        b.inputs["Roughness"].default_value = rough
        b.inputs["Metallic"].default_value = metal
        return m

    return wrap


def flat_paint(name, colour, rough=0.6, metal=0.0, wear=0.0, grime=1.0):
    def fn(p, n, edge):
        c = colour
        v = fbm(p, 3.0, 2, 11.0) * 0.08
        c = tuple(x * (1 + v) for x in c)
        if wear > 0 and edge > 0:
            c = lerp3(c, (0.36, 0.35, 0.33), min(1.0, edge * wear))
        return grime_rise(c, p, grime)

    paint(name, rough, metal)(fn)
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


def rest_on_ground():
    """Push any vertex below the ground up onto it (tumbled rubble, a tilted wreck)."""
    bpy.context.view_layer.update()
    for o in bpy.data.objects:
        if o.type != "MESH":
            continue
        mw = o.matrix_world
        inv = mw.inverted_safe()
        for v in o.data.vertices:
            w = mw @ v.co
            if w.z < 0:
                w.z = 0.0
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


def finish(ao_distance=1.2, ao_strength=0.8, ao_rays=12, ground_ao=True, paint_scale=1.0):
    """Bake paint, wear, grime and occlusion into every tier's vertex colour.
    `paint_scale` stretches the longest painted edge (buildings are larger)."""
    bpy.context.view_layer.update()
    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    for o in meshes:
        t = tier_of(o)
        mat = o.data.materials[0] if o.data.materials else None
        if mat is not None and mat.name in PAINTS and t is not None:
            _split_long_edges(o.data, PAINT_EDGE_M[t] * paint_scale)
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
        cols.append(tuple(max(0.0, min(1.0, c[i] * k / BASE)) for i in range(3)))
    bm.free()
    attr = me.color_attributes.new("Color", "FLOAT_COLOR", "POINT")
    for i, c in enumerate(cols):
        attr.data[i].color = (c[0], c[1], c[2], 1.0)
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


def export(path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    for o in list(bpy.data.objects):
        if o.type in ("CAMERA", "LIGHT"):
            bpy.data.objects.remove(o, do_unlink=True)
    bpy.ops.export_scene.gltf(
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
    print("GLB", path, os.path.getsize(path))
