"""Our own tower blocks: the post-war residential slab and point towers of a town, as one kit.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/towers.py

writes `assets/source/city/towers/kit.glb` and `templates.json` (the format is in
this folder's readme). They are plain panel-built housing (bare precast, painted
render, facing tile), so they stand beside any region's apartment blocks; they
carry the first shipping family's name until a family has towers of its own.

The physical box is the authority. A tower is one part: its walls stand on the
part's faces and its top is the parapet. Balconies, sills, the entrance's canopy
and step overhang by at most the set's side fit; the lift's bulkhead, vents,
tanks and aerials rise by at most its top fit. Every side is whole 3 m bays
between two 1 m corner piers (3n + 2 m), every floor is 3 m, and each opening
sits in its bay on its floor's datum, where a garrison's soldiers stand.

A tower is drawn by instancing. Near (tier 0), every bay of every floor is a row
placing one shared panel module (a window, a balcony door, a loggia, a stair
light, a blank), with balconies, curtains, washing and air conditioners as rows
of their own, so a row's tint colours one thing. From further off (tiers 1 to 3)
the panels' rows stop and the template's own shell carries the same grid as a
texture: one face a run of bays, sampling a facade recipe two bays by two floors
to the tile. The shell also holds what is the template's alone: the corner
piers, the parapet, the roof, and at tier 0 a closed core behind the panels.
At tiers 2 and 3 a tower is that one row: its balconies fold into the shell as
one textured stack a column, its roof's huts, plant and tanks and its canopies as
boxes. The roof is drawn the same at every tier: a field with its stains and
mended patches, inside a band the parapet shades.

A tower does not fall: it stands gutted. That state is the same tower built a
second time (`shell(..., burnt=True)`, the `gutted_*` panels): every opening an
empty dark hole with the soot of its fire fanned up the wall over it, balconies
burnt, broken or hanging, a few bays blown out to the bare floor slabs, the roof
and its huts burnt. The soot and the holes are in the burnt facade recipes, so
they hold at every tier, and each bay of each floor shows the same quarter of
the recipe near and far.
"""
import math
import os
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kit import *  # noqa: E402,F403

FAMILY = "china"
BAY_M, FLOOR_M = BAY_PITCH_M, 3.0
PIER_M = 1.0  # the blank corner at each end of a wall
PARAPET_M = 1.0  # the part's top above the top floor's ceiling
ROOF_M = 0.35  # the roof's surface above that ceiling, inside the parapet
COPING_M = 0.3  # the parapet's thickness
REVEAL_M = 0.14  # how far glass sits behind the wall's face
LOGGIA_M = 1.3  # a loggia's depth
CORE_M = 1.5  # the tier 0 core stands this far inside the faces: behind every loggia
FACADE_M = 6.0  # a facade recipe's tile: two bays by two floors
# An opening in its panel (x0, x1, z0, z1): x about the bay's centre, z above the floor's datum.
# The panel modules and the facade recipes are both cut to these.
OPENING = {"w": (-0.75, 0.75, 0.9, 2.4), "d": (-1.1, 1.1, 0.1, 2.3), "l": (-1.3, 1.3, 0.12, 2.7), "s": (-0.4, 0.4, 0.5, 2.6),
           "r": (-1.35, 1.35, 0.9, 2.4), "x": (-1.38, 1.38, 0.16, 2.84)}
KINDS = {"w": "window", "d": "door", "l": "loggia", "s": "stair", "b": "blank", "B": "balconies", "r": "ribbon", "x": "blown"}
SOOT = (0.02, 0.018, 0.016)
# How hard each of a burnt recipe's four bays burnt (which = bay + 2 * floor): no two alike, so a wall is not one grey.
BURNT = (1.0, 0.55, 0.85, 0.3)
BALCONY = (2.8, 1.2, 1.05)  # a balcony's width, its reach and its front's height
GLASS = (0.02, 0.025, 0.03)
STAIR_GLASS = (0.035, 0.045, 0.045)
FRAME = (0.46, 0.45, 0.42)
RAIL = (0.5, 0.5, 0.48)  # a loggia's balustrade, the sills, the coping: pale cast concrete
BASE = (0.2, 0.2, 0.19)  # the ground floor: darker concrete
FACADE_WALL = (0.72, 0.71, 0.68)
CLOTHS = ((0.3, 0.28, 0.22), (0.2, 0.24, 0.29), (0.31, 0.31, 0.3), (0.27, 0.17, 0.15))  # curtains seen through glass

kit = Kit("towers", "towers.py", fit_side_m=1.5, fit_top_m=4.0)


def linear(tint):
    return tuple(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in (v / 255 for v in tint))


# ---------------------------------------------------------------- the facade recipes
# Which of a facade recipe's four bays (bay + 2 * floor) has curtains drawn across its glass, and how:
# the rows that hang curtain modules at tier 0 follow the same table, so a window is the same near and far.
CURTAINED = {1: "curtain_pair", 3: "curtain_blind"}


def facade(kind, seed, burnt=False, front=None):
    """A tower's wall from a distance: two bays by two floors of pale panels the
    building's tint colours, each with `kind`'s opening where the panel module has
    it, and no two of the four dressed alike. `burnt` is the same wall gutted: each
    opening an empty dark hole, the soot of its fire fanned up the panels over it (and
    on up into the floor above), the facing spalled off in patches. `x` is a bay blown
    out: the bare edges of its floor slabs and the dark of the floor between them.
    `front` (sRGB) paints a column of balconies' fronts in the recipe itself, which then
    takes no tint: a stack is one face, and only its fronts are that colour."""
    S = textures.SIZE
    ss, mix = textures.smoothstep, textures.mix
    rows, cols = np.mgrid[0:S, 0:S].astype(float)
    x, z = (cols + 0.5) / S * FACADE_M, (1.0 - (rows + 0.5) / S) * FACADE_M  # image rows run down the wall
    bx, bz = x % BAY_M - BAY_M / 2, z % FLOOR_M
    which = (x // BAY_M).astype(int) + 2 * (z // FLOOR_M).astype(int)
    px = FACADE_M / S

    def rect(x0, x1, z0, z1):
        return ss(x0 - px, x0, bx) * ss(x1 + px, x1, bx) * ss(z0 - px, z0, bz) * ss(z1 + px, z1, bz)

    def bars(at, across, width=0.05):
        """Glazing bars at each of `at` along `across` (bx or bz)."""
        out = np.zeros((S, S))
        for a in at:
            out = np.maximum(out, ss(width / 2 + px, width / 2, np.abs(across - a)))
        return out

    tone = 0.84 + 0.22 * textures.fbm(3, seed, 5) + 0.06 * (textures.fbm(64, seed + 1, 3) - 0.5)
    wall = np.array(FACADE_WALL) * tone[..., None]
    joint = np.maximum(ss(0.035, 0.012, BAY_M / 2 - np.abs(bx)), ss(0.035, 0.012, np.minimum(bz, FLOOR_M - bz)))
    wall = mix(wall, (0.2, 0.2, 0.19), joint * 0.6)
    x0, x1, z0, z1 = OPENING["d" if kind == "B" else kind]
    hole = rect(x0, x1, z0, z1)
    lintel = 1.0 - 0.5 * ss(z1 - 0.3, z1 - 0.1, bz)  # the head's shadow on what is behind it
    rough = np.full((S, S), 0.5)
    if burnt:
        return gutted(kind, seed, wall, tone, joint, hole, which, bx, bz, rect)
    if kind == "B":
        # a column of balconies as one face: each floor its front, the shade over it, the slab above
        h, slab = BALCONY[2], FLOOR_M - 0.14
        hole = ss(h - px, h, bz) * ss(slab + px, slab, bz)
        inside = wall * 0.22
        inside = mix(inside, GLASS, rect(x0, x1, h, z1))
        sash = (which == 1) * (1.0 - bars((-1.37, -0.47, 0.47, 1.37), bx, 0.06)) * ss(slab - 0.08, slab - 0.08 - px, bz)
        inside = mix(inside, (0.035, 0.042, 0.05), sash)  # one in four glazed in
        inside = mix(inside, FRAME, (which == 1) * (1.0 - sash) * 0.9)
        for k, (at, colour) in enumerate(((-0.9, CLOTHS[1]), (-0.2, CLOTHS[2]), (0.5, CLOTHS[3]))):  # one hung with washing
            inside = mix(inside, colour, (which == 2) * rect(at, at + 0.45, 1.25, 2.05 - 0.15 * k))
        rough = np.where(sash > 0.5, 0.12, 0.9)
        joint = np.zeros((S, S))  # no panel joints: a front, then the slab's edge in its own tone
        paint = np.array(FACADE_WALL) * (np.array(linear(front)) if front else 1.0)
        wall = mix(paint * tone[..., None], np.array(RAIL) * tone[..., None], ss(slab - px, slab, bz))
    elif kind == "l":
        # a recess in shade with a glazed door at its back, behind a pale balustrade
        inside = wall * 0.22
        door = rect(-0.9, 0.9, z0, 2.3) * (1.0 - bars((-0.9, 0.0, 0.9), bx, 0.07))
        inside = mix(inside, GLASS, door)
        glazed = (which == 1) * (1.0 - bars((x0 + 0.03, -0.43, 0.43, x1 - 0.03), bx, 0.06)) * (1.0 - bars((z1 - 0.03,), bz, 0.06))
        inside = mix(inside, (0.035, 0.042, 0.05), glazed)  # one in four glazed in
        inside = mix(inside, FRAME, (which == 1) * (1.0 - glazed) * 0.9)
        for k, (at, colour) in enumerate(((-0.7, CLOTHS[3]), (-0.1, CLOTHS[2]), (0.55, CLOTHS[1]))):  # one hung with washing
            inside = mix(inside, colour, (which == 2) * rect(at, at + 0.45, 1.25, 2.1 - 0.15 * k))
        inside = inside * lintel[..., None]
        rough = np.where(door + glazed > 0.5, 0.12, 0.9)
        inside = mix(inside, np.array(RAIL) * tone[..., None], rect(x0, x1, z0, z0 + 0.98))
        inside = mix(inside, (0.1, 0.1, 0.1), rect(x0, x1, z0 + 0.98, z0 + 1.04))
    elif kind == "s":
        frac = (bz - z0) % 0.42
        pane = rect(x0 + 0.05, x1 - 0.05, z0 + 0.05, z1 - 0.05) * (1.0 - ss(0.045, 0.025, np.minimum(frac, 0.42 - frac)))
        inside = mix(FRAME, STAIR_GLASS, pane) * lintel[..., None]
        rough = np.where(pane > 0.5, 0.2, 0.5)
    else:
        pane = rect(x0 + 0.06, x1 - 0.06, z0 + 0.06, z1 - 0.06) * (1.0 - bars({"w": (0.0,), "r": (-0.45, 0.45)}.get(kind, (-0.25,)), bx))
        inside = mix(FRAME, GLASS, pane)
        mid, half = (x0 + x1) / 2, (x1 - x0) / 2
        scale = half / 0.75  # the curtain modules are cut to a window's opening, and stretched to a wider one
        drawn = np.select([which == 1, which == 3], [np.abs(bx - mid) > half - 0.42 * scale, bz > z0 + 0.7], False)
        cloth = 0.55 * np.array(CLOTHS)[which]  # in the reveal's shade, as the panel's curtain modules are
        inside = mix(inside, cloth, pane * drawn) * lintel[..., None]
        rough = np.where(pane > 0.5, 0.12, 0.5)
    col = mix(wall, inside, hole)
    height = textures.blur(-2.0 * hole - 0.5 * joint)
    wear = np.where(hole > 0.5, 1.0, 0.3 + 0.7 * textures.fbm(6, seed + 3, 5))
    return textures.Baked(col, wear, textures.normals_from_height(height, 1.2), 1.0 - 0.25 * joint,
                          mix(np.full((S, S, 1), 0.9), rough[..., None], hole)[..., 0], tint=0.0 if front else 1.0 - hole)


def balconies_recipe(front):
    """The name of the balcony column's recipe with its fronts painted `front` (sRGB), made when first asked for."""
    name = "facade_balconies_%02x%02x%02x" % front
    if name not in textures.RECIPES:
        textures.recipe(name, tile=FACADE_M, wear=(0.12, 0.11, 0.09, 1.0))(lambda: facade("B", 3501, front=front))
    return name


def gutted(kind, seed, wall, tone, joint, hole, which, bx, bz, rect):
    """`facade`'s wall after the fire (its `burnt`), from the wall, joints and opening it had laid out."""
    S = textures.SIZE
    ss, mix = textures.smoothstep, textures.mix
    x0, x1, z0, z1 = OPENING["d" if kind == "B" else kind]
    px = FACADE_M / S
    void = np.array((0.016, 0.015, 0.014)) * (0.6 + 0.8 * textures.fbm(24, seed + 57, 3))[..., None]
    inside = void
    if kind == "B":
        inside = np.array(FACADE_WALL) * 0.12 * tone[..., None]  # the recess behind a balcony: in shade, not a hole  # a column of balconies: fronts scorched, one in four gone and one half gone
        h, slab = BALCONY[2], FLOOR_M - 0.14
        gone = (which == 1) & (bx < 0.2)
        hole = ss(np.where(gone, 0.06, h) - px, np.where(gone, 0.06, h), bz) * ss(slab + px, slab, bz)
        z1 = slab
        joint = np.zeros((S, S))
        wall = mix(np.array(FACADE_WALL) * tone[..., None], np.array(RAIL) * tone[..., None], ss(slab - px, slab, bz))
    elif kind == "l":  # a loggia: its balustrade standing in two of the four, scorched
        stands = ((which == 0) | (which == 2)) * rect(x0, x1, z0, z0 + 0.98)
        inside = mix(void, np.array(RAIL) * 0.3 * tone[..., None], stands)
    elif kind == "x":  # blown out: the slabs' edges, the dark floor between, rubble along its lip
        hole = rect(x0, x1, z0, z1)
        lip = ss(z0 + 0.5, z0, bz) * ss(0.35, 0.6, textures.fbm(20, seed + 61, 3))
        inside = mix(void, np.array((0.2, 0.19, 0.175)) * tone[..., None], 0.7 * lip)
        wall = np.array(RAIL) * 0.75 * tone[..., None]  # bare concrete: the slab's edge and the columns either side
        joint = np.zeros((S, S))
    # the soot each opening's fire left on the wall over it, and on up the floor above
    # (a tongue of it: black over the head, narrowing and wavering as it climbs, a floor or more for a hard fire)
    wobble = textures.fbm(8, seed + 71, 4) - 0.5
    mid, half = (x0 + x1) / 2, (x1 - x0) / 2
    soot = np.zeros((S, S))
    for rise, bay in ((bz - z1, which), (bz + FLOOR_M - z1, (which + 2) % 4)):
        strength = np.array(BURNT)[bay] * (0.5 if kind == "B" else 1.0)
        reach = 0.45 + 1.7 * strength
        up = np.clip(rise, 0, None)
        width = half * (1.0 - 0.45 * np.clip(up / (2.2 * reach), 0, 1)) + 0.08
        plume = np.minimum(1.0, 1.3 * strength) * np.exp(-np.clip(up - 0.35, 0, None) / reach)
        plume = plume * ss(width + 0.25, width - 0.1, np.abs(bx - mid) + wobble * (0.35 + 0.5 * up))
        soot = np.maximum(soot, np.where(rise > 0, plume, 0.0))
    soot = np.clip(soot, 0, 1) * (0.85 + 0.15 * textures.fbm(32, seed + 73, 3))
    spall = ss(0.68, 0.75, textures.fbm(7, seed + 79, 4)) * (1.0 - hole)
    wall = mix(wall, np.array((0.24, 0.235, 0.22)) * tone[..., None], 0.85 * spall)
    wall = mix(wall, (0.2, 0.2, 0.19), joint * 0.6)
    wall = mix(wall * 0.88, SOOT, 0.95 * soot)
    col = mix(wall, inside, hole)
    height = textures.blur(-2.0 * hole - 0.5 * joint - 0.6 * spall)
    wear = np.where(hole > 0.5, 1.0, 0.3 + 0.7 * textures.fbm(6, seed + 3, 5))
    return textures.Baked(col, wear, textures.normals_from_height(height, 1.2), 1.0 - 0.25 * joint, np.full((S, S), 0.95),
                          tint=(1.0 - hole) * (0.0 if kind == "x" else 1.0))


for kind, seed in (("w", 3101), ("d", 3201), ("l", 3301), ("s", 3401), ("B", 3501), ("r", 3601)):
    textures.recipe(f"facade_{KINDS[kind]}", tile=FACADE_M, wear=(0.12, 0.11, 0.09, 1.0))(
        lambda kind=kind, seed=seed: facade(kind, seed))
    textures.recipe(f"facade_{KINDS[kind]}_burnt", tile=FACADE_M, wear=(0.12, 0.11, 0.09, 1.0))(
        lambda kind=kind, seed=seed: facade(kind, seed, burnt=True))
textures.recipe("facade_blown_burnt", tile=FACADE_M, wear=(0.12, 0.11, 0.09, 1.0))(lambda: facade("x", 3701, burnt=True))

# ---------------------------------------------------------------- materials
# Walls are pale and tint-masked: a row's tint is the tower's colour, or a column's.
FINISHES = {"precast": "precast", "render": "plaster", "tile": "mosaic"}  # finish -> its recipe
WALLS = {finish: textured(f"tower_wall_{finish}", recipe, tint=1.0, dirt=0.0, chip=0.0, streak=0.0)
         for finish, recipe in FINISHES.items()}
base_m = textured("tower_base", "concrete", colour=BASE, dirt=0.8, chip=0.0, streak=0.2, rise=1.2)
rail_m = textured("tower_cast", "concrete", colour=RAIL, dirt=0.0, chip=0.0, streak=0.2)
roof_m = textured("tower_roof_felt", "asphalt", colour=(0.1, 0.1, 0.096), dirt=0.0, chip=0.0, streak=0.0)
front_m = textured("tower_balcony_front", "concrete", tint=1.0, colour=FACADE_WALL, dirt=0.0, chip=0.0, streak=0.3)  # painted: no cracks
hut_m = textured("tower_roof_hut", "plaster", tint=1.0, dirt=0.4, chip=0.0, streak=0.4, rise=0.6, dust=(0.1, 0.1, 0.096))
glass_m = flat_paint("tower_glass", GLASS, rough=0.08, grime=0.0)
stair_glass_m = flat_paint("tower_stair_glass", STAIR_GLASS, rough=0.2, grime=0.0)
frame_m = flat_paint("tower_frame", FRAME, rough=0.6, grime=0.0)
metal_m = flat_paint("tower_metal", (0.07, 0.075, 0.08), rough=0.5, metal=0.6, grime=0.0)
tank_m = flat_paint("tower_tank", (0.24, 0.27, 0.28), rough=0.6, metal=0.3, grime=0.0)
unit_m = flat_paint("tower_ac", (0.5, 0.5, 0.48), rough=0.6, grime=0.0)
linen_m = flat_paint("tower_linen", (0.55, 0.55, 0.53), rough=0.9, grime=0.0)
cloth_m = flat_paint("tower_cloth", (0.5, 0.5, 0.48), rough=0.9, grime=0.0)
cloth_m["tint"] = 1.0
drape_m = flat_paint("tower_drape", (0.3, 0.3, 0.29), rough=0.5, grime=0.0)  # a curtain, dimmed by the glass before it
drape_m["tint"] = 1.0
patch_m = textured("tower_roof_patch", "asphalt", colour=(0.2, 0.2, 0.195), dirt=0.0, chip=0.0, streak=0.0, seed=2.0)  # felt laid since
void_m = flat_paint("tower_void", (0.012, 0.011, 0.01), rough=1.0, grime=0.0)  # a gutted floor, seen through its openings
ash_m = flat_paint("tower_ash", (0.21, 0.205, 0.195), rough=1.0, grime=0.0)  # where a roof's felt burnt away: pale, so not a shadow


def stained(mat, seed=0.0):
    """Lay a flat roof's weather over its felt's paint: damp darkening it in pools some
    metres across, and the pale silt each leaves as it dries. Wider than two of the
    roof's cells, so every tier's vertices find the same marks."""
    base = PAINTS[mat.name]

    def fn(p, n, edge):
        c, wear = base(p, n, edge)
        if n.z < 0.5:
            return c, wear
        level = fbm(p, 0.08, 2, 97.0 + seed)
        damp = smoothstep(0.05, 0.4, level)
        silt = math.exp(-((level - 0.02) / 0.08) ** 2)
        return lerp3(tuple(x * (1.0 - 0.5 * damp) for x in c), (0.3, 0.29, 0.26), 0.5 * silt), wear

    PAINTS[mat.name] = fn
    return mat


stained(roof_m)
# The same, after the fire. Soot rises with a wall; a roof, a hut and a balcony are sooted all over, in patches.
ROOF_BURNT, HUT_BURNT = scorched(roof_m, 1e6, floor=0.75), scorched(hut_m, 1e6, 3.0, floor=0.85)
FRONT_BURNT, CAST_BURNT = scorched(front_m, 1e6, 5.0, floor=0.75), scorched(rail_m, 1e6, 7.0, floor=0.6)


def smoked(z):
    """How much of a gutted tower's wall the smoke took, `z` metres up: more the higher."""
    return 0.06 + 0.4 * smoothstep(5.0, 50.0, z)


def dimmed(tint, z):
    """A row's tint (sRGB) as the smoke left it at height `z`: what `smoked` does to the shell's walls, for a panel."""
    k = 1.0 - smoked(z)
    return tuple(round(255 * (12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055)) for c in (v * k for v in linear(tint)))

# Tints, sRGB: bodies, accents, balcony fronts, curtains.
GREY, CREAM, WHITE, PALE = (172, 168, 158), (226, 210, 178), (228, 230, 224), (214, 210, 200)
TERRACOTTA, AQUA, SLATE = (178, 114, 88), (156, 190, 184), (96, 118, 140)
FRONTS = ((142, 164, 178), (198, 178, 136), (162, 178, 152))
REPAINTS = ((226, 222, 208), (122, 150, 134), (176, 120, 96))  # what an owner repainted his balcony's front
CURTAINS = ((236, 224, 196), (168, 190, 214), (232, 232, 228), (214, 150, 136), (176, 200, 160), (222, 196, 120))
WASHING = ((84, 112, 160), (176, 70, 62), (226, 196, 96), (90, 140, 110))


_STRIPS = {}


def strip_mat(recipe, tint=None, tone=1.0, smoke=False, plain=False):
    """A shell wall's material on `recipe`: tint-masked for the row's tint (the tower's
    colour), or with `tint` (sRGB) baked in for a column of another colour, or `plain`
    for a recipe that carries its own colours. `smoke` is a gutted tower's: the wall
    darker the higher it stands (`smoked`)."""
    key = (recipe, tint, tone, smoke, plain)
    if key not in _STRIPS:
        mean = textures.baked(recipe).mean()
        name = (f"tower_shell_{recipe}" + ("" if tint is None else "_%02x%02x%02x" % tint) + ("" if tone == 1.0 else "_base")
                + ("_smoked" if smoke else ""))
        colour = None if tint is None and tone == 1.0 else tuple(m * c * tone for m, c in zip(mean, linear(tint or (255, 255, 255))))
        _STRIPS[key] = textured(name, recipe, tint=1.0 if tint is None and tone == 1.0 and not plain else 0.0, colour=colour, dirt=0.5,
                                chip=0.0, streak=0.3, rise=2.5)
        if smoke:
            base = PAINTS[name]
            PAINTS[name] = lambda p, n, edge: (lambda c, wear: (tuple(x * (1.0 - smoked(p.z)) for x in c), wear))(*base(p, n, edge))
    return _STRIPS[key]


BASE_TONE = BASE[0] / FACADE_WALL[0]  # the ground floor's panels, from far off: the facade recipe darkened to the base


# ---------------------------------------------------------------- faces by hand
def sheet(name, mat, parent, quads, lods=TIERS, tile=None, shift=(0.0, 0.0)):
    """A mesh of flat faces: [(points, a direction its normal leans toward, uvs or None)].
    With `tile`, a face without uvs is mapped in metres over it as a panel's is: u from
    the bay's left edge along the wall, v up from the floor, `shift` metres along."""

    def panel_uv(p, n):
        if abs(n.y) >= max(abs(n.x), abs(n.z)):
            return (p.x + BAY_M / 2 + shift[0]) / tile, (p.z + shift[1]) / tile
        return ((p.y + shift[0]) / tile, (p.z + shift[1]) / tile) if abs(n.x) > abs(n.z) else ((p.x + BAY_M / 2 + shift[0]) / tile, p.y / tile)

    def build(bm, lod):
        layer = bm.loops.layers.uv.new("UVMap") if tile or any(q[2] for q in quads) else None
        for points, toward, uvs in quads:
            verts = [bm.verts.new(p) for p in points]
            f = bm.faces.new(verts)
            f.normal_update()
            if f.normal.dot(Vector(toward)) < 0:
                f.normal_flip()
            if layer is not None:
                given = dict(zip(verts, uvs)) if uvs else None
                for loop in f.loops:
                    loop[layer].uv = given[loop.vert] if given else panel_uv(loop.vert.co, f.normal)

    return mesh_part(name, build, mat, parent, lods)


def flat(x0, x1, z0, z1, y=0.0, uvs=None):
    """A face on a wall-mounted module's plane, `y` behind its wall's face, looking out."""
    return [(x0, y, z0), (x1, y, z0), (x1, y, z1), (x0, y, z1)], (0, -1, 0), uvs


def opening(x0, x1, z0, z1, depth=REVEAL_M):
    """A 3 m panel's face round an opening, and the opening's reveal back to `depth`."""
    half = BAY_M / 2
    out = [flat(-half, x0, 0, FLOOR_M), flat(x1, half, 0, FLOOR_M), flat(x0, x1, 0, z0), flat(x0, x1, z1, FLOOR_M)]
    out += [([(x0, 0, z0), (x1, 0, z0), (x1, depth, z0), (x0, depth, z0)], (0, 0, 1), None),
            ([(x0, 0, z1), (x1, 0, z1), (x1, depth, z1), (x0, depth, z1)], (0, 0, -1), None),
            ([(x0, 0, z0), (x0, depth, z0), (x0, depth, z1), (x0, 0, z1)], (1, 0, 0), None),
            ([(x1, 0, z0), (x1, depth, z0), (x1, depth, z1), (x1, 0, z1)], (-1, 0, 0), None)]
    return out


def glazing(m, x0, x1, z0, z1, y, uprights=(), rails=(), glass=None, bar=0.05):
    """Glass in an opening at `y`, with a frame round it and bars across it, a little before it."""
    sheet(m.n("glass"), glass or glass_m, m.root, [flat(x0, x1, z0, z1, y)], lods=(0,))
    at = y - 0.03
    quads = [flat(x0, x0 + bar, z0, z1, at), flat(x1 - bar, x1, z0, z1, at), flat(x0 + bar, x1 - bar, z0, z0 + bar, at),
             flat(x0 + bar, x1 - bar, z1 - bar, z1, at)]
    quads += [flat(x - bar / 2, x + bar / 2, z0 + bar, z1 - bar, at) for x in uprights]
    quads += [flat(x0 + bar, x1 - bar, z - bar / 2, z + bar / 2, at) for z in rails]
    sheet(m.n("frame"), frame_m, m.root, quads, lods=(0,))


def card(m, mat, tile, lods=(1, 2, 3), which=0):
    """The whole panel as one face: what the module is past tier 0, where no row draws it.
    `which` is the quarter of a facade recipe it shows (bay + 2 * floor)."""
    k, u, v = BAY_M / tile, (which % 2) * BAY_M / tile, (which // 2) * BAY_M / tile
    sheet(m.n("card"), mat, m.root, [flat(-BAY_M / 2, BAY_M / 2, 0, FLOOR_M, uvs=[(u, v), (u + k, v), (u + k, v + k), (u, v + k)])], lods=lods)


# ---------------------------------------------------------------- panels: one bay wide, one floor high
# A panel faces -Y with its bay's centre on the wall's face at the floor's datum.
PANEL = dict(ao_distance=0.5, paint_scale=10.0)  # no face needs splitting for paint
_PANELS = {}


def panel(finish, kind):
    """The id of `finish`'s panel module of `kind`, made the first time it is asked for."""
    name = f"{finish}_{KINDS[kind]}"
    if name in _PANELS:
        return name
    wall, tile = WALLS[finish], textures.tile_of(FINISHES[finish])
    m = _PANELS[name] = kit.module(name, **(PANEL | (dict(ao_distance=1.4, ao_strength=0.7) if kind == "l" else {})))
    if kind == "b":
        card(m, wall, tile, lods=TIERS)
        return name
    x0, x1, z0, z1 = OPENING[kind]
    if kind == "l":
        d = LOGGIA_M
        sheet(m.n("wall"), wall, m.root, opening(x0, x1, z0, z1, d) + [flat(x0, x1, z0, z1, d)], lods=(0,), tile=tile)
        glazing(m, -0.9, 0.9, z0, 2.3, d - 0.03, uprights=(0.0,), bar=0.07)
        box(m.n("balustrade"), (x1 - x0, 0.1, 0.98), (0, 0.05, z0 + 0.49), rail_m, m.root, lods=(0,))
        box(m.n("handrail"), (x1 - x0, 0.06, 0.05), (0, 0.05, z0 + 1.01), metal_m, m.root, lods=(0,))
    else:
        sheet(m.n("wall"), wall, m.root, opening(x0, x1, z0, z1), lods=(0,), tile=tile)
        if kind == "s":
            glazing(m, x0, x1, z0, z1, REVEAL_M, rails=[z0 + 0.42 * k for k in range(1, 5)], glass=stair_glass_m)
        else:
            glazing(m, x0, x1, z0, z1, REVEAL_M, uprights={"w": (0.0,), "r": (-0.45, 0.45)}.get(kind, (-0.25,)))
        if kind in "wr":
            box(m.n("sill"), (x1 - x0 + 0.16, 0.12, 0.06), (0, -0.01, z0 - 0.03), rail_m, m.root, lods=(0,))
    card(m, strip_mat(f"facade_{KINDS[kind]}"), FACADE_M)
    return name


def gutted_panel(kind, which, ground=False):
    """The id of the gutted panel of `kind` that shows quarter `which` of its burnt recipe
    (bay + 2 * floor, as the shell's wall does there): the wall round an empty opening,
    and the dark of the burnt-out floor behind it. One set serves every finish: the
    burnt recipe is the wall. `ground` is the ground floor's, in its darker concrete."""
    name = f"gutted_{'ground' if ground else KINDS[kind]}_{which}"
    if name in _PANELS:
        return name
    m = _PANELS[name] = kit.module(name, ground=ground, **PANEL)
    mat = strip_mat(f"facade_{KINDS[kind]}_burnt", tone=BASE_TONE if ground else 1.0)
    shift = ((which % 2) * BAY_M, (which // 2) * FLOOR_M)
    x0, x1, z0, z1 = OPENING[kind]
    if kind == "x":  # blown out: the slab's edge under and over, the floor running back into the dark
        sheet(m.n("edges"), mat, m.root, [flat(-BAY_M / 2, BAY_M / 2, 0, z0), flat(-BAY_M / 2, BAY_M / 2, z1, FLOOR_M),
                                          flat(-BAY_M / 2, x0, z0, z1), flat(x1, BAY_M / 2, z0, z1)], lods=(0,), tile=FACADE_M, shift=shift)
        sheet(m.n("floor"), CAST_BURNT, m.root, [([(x0, 0, z0), (x1, 0, z0), (x1, CORE_M - 0.1, z0), (x0, CORE_M - 0.1, z0)], (0, 0, 1), None),
                                                  ([(x0, 0, z1), (x1, 0, z1), (x1, CORE_M - 0.1, z1), (x0, CORE_M - 0.1, z1)], (0, 0, -1), None)],
              lods=(0,))
        depth = CORE_M - 0.1
    else:
        depth = LOGGIA_M + 0.4 if kind == "l" else 0.7
        sheet(m.n("wall"), mat, m.root, opening(x0, x1, z0, z1, 0.2), lods=(0,), tile=FACADE_M, shift=shift)
        if kind == "l" and which in (0, 2):  # the balustrade the recipe leaves standing in these two
            box(m.n("balustrade"), (x1 - x0, 0.1, 0.98), (0, 0.05, z0 + 0.49), CAST_BURNT, m.root, lods=(0,))
    sheet(m.n("void"), void_m, m.root, [flat(-BAY_M / 2, BAY_M / 2, 0, FLOOR_M, depth)], lods=(0,))
    card(m, mat, FACADE_M, which=which)
    return name


# The ground floor is the same on every tower: darker cast concrete, a window a bay.
m = kit.module("ground_window", ground=True, **PANEL)
sheet(m.n("wall"), base_m, m.root, opening(*OPENING["w"]), lods=(0,), tile=textures.tile_of("concrete"))
glazing(m, *OPENING["w"], REVEAL_M, uprights=(0.0,))
box(m.n("sill"), (1.66, 0.12, 0.06), (0, -0.01, 0.87), rail_m, m.root, lods=(0,))
card(m, strip_mat("facade_window", tone=BASE_TONE), FACADE_M)

m = kit.module("ground_blank", ground=True, **PANEL)
card(m, base_m, textures.tile_of("concrete"), lods=TIERS)

# The way in: glazed double doors up a step, under a canopy on two posts. Past tier 0 the
# shell's wall is behind it, so only the doors' glass, the canopy and the step are drawn.
DOOR = (-0.9, 0.9, 0.15, 2.3)
m = kit.module("entrance", ground=True, **PANEL)
sheet(m.n("wall"), base_m, m.root, opening(*DOOR), lods=(0,), tile=textures.tile_of("concrete"))
glazing(m, *DOOR, REVEAL_M, uprights=(0.0,), rails=(0.5,), bar=0.08)
sheet(m.n("doors"), glass_m, m.root, [flat(*DOOR, -0.03)], lods=(1, 2))
box(m.n("canopy"), (2.9, 1.4, 0.14), (0, -0.7, 2.52), rail_m, m.root)
box(m.n("step"), (2.9, 1.3, 0.15), (0, -0.65, 0.075), base_m, m.root, lods=(0, 1, 2))
for s in (-1, 1):
    box(m.n(f"post_{'ab'[s > 0]}"), (0.14, 0.14, 2.3), (s * 1.3, -1.25, 1.3), rail_m, m.root, lods=(0, 1))

m = kit.module("gutted_entrance", ground=True, **PANEL)  # the way in, burnt out: the canopy stands over an empty doorway
sheet(m.n("wall"), base_m, m.root, opening(*DOOR, 0.2), lods=(0,), tile=textures.tile_of("concrete"))
sheet(m.n("void"), void_m, m.root, [flat(-BAY_M / 2, BAY_M / 2, 0, FLOOR_M, 0.7)], lods=(0,))
sheet(m.n("doors"), void_m, m.root, [flat(*DOOR, -0.03)], lods=(1, 2))
box(m.n("canopy"), (2.9, 1.4, 0.14), (0, -0.7, 2.52), CAST_BURNT, m.root)
box(m.n("step"), (2.9, 1.3, 0.15), (0, -0.65, 0.075), base_m, m.root, lods=(0, 1, 2))
for s in (-1, 1):
    box(m.n(f"post_{'ab'[s > 0]}"), (0.14, 0.14, 2.3), (s * 1.3, -1.25, 1.3), CAST_BURNT, m.root, lods=(0, 1))

# ---------------------------------------------------------------- what hangs on a panel
# A balcony before a door panel: a slab at the floor's datum and a solid front a row tints.
def balcony(name, glazed):
    m = kit.module(name, **PANEL)
    w, reach, h = BALCONY
    box(m.n("slab"), (w, reach, 0.14), (0, -reach / 2, -0.07), rail_m, m.root, lods=(0, 1, 2))
    box(m.n("front"), (w, 0.08, h), (0, -reach + 0.04, h / 2), front_m, m.root)
    for s in (-1, 1):
        box(m.n(f"cheek_{'ab'[s > 0]}"), (0.08, reach - 0.08, h), (s * (w / 2 - 0.04), -(reach - 0.08) / 2, h / 2), front_m, m.root,
            lods=(0, 1))
    if not glazed:
        return
    # glazed in by its owner: sashes on the front and cheeks up to a lid under the slab above
    top, y = 2.72, -reach + 0.02
    sheet(m.n("sash"), glass_m, m.root, [flat(-w / 2, w / 2, h, top, y)], lods=(0, 1, 2))
    sheet(m.n("sash_bars"), frame_m, m.root,
          [flat(x - 0.03, x + 0.03, h, top, y - 0.02) for x in (-w / 2 + 0.03, -0.47, 0.47, w / 2 - 0.03)]
          + [flat(-w / 2, w / 2, top - 0.06, top, y - 0.02)], lods=(0,))
    for s in (-1, 1):
        x = s * (w / 2 - 0.01)
        sheet(m.n(f"side_{'ab'[s > 0]}"), glass_m, m.root, [([(x, 0, h), (x, -reach, h), (x, -reach, top), (x, 0, top)], (s, 0, 0), None)],
              lods=(0, 1))
    box(m.n("lid"), (w, reach, 0.08), (0, -reach / 2, top + 0.04), rail_m, m.root, lods=(0, 1, 2))


balcony("balcony", False)
balcony("balcony_glazed", True)

# The same balcony after the fire: scorched, or its front half gone, or hanging by its inner edge.
w_, reach_, h_ = BALCONY
m = kit.module("balcony_burnt", **PANEL)
box(m.n("slab"), (w_, reach_, 0.14), (0, -reach_ / 2, -0.07), CAST_BURNT, m.root, lods=(0, 1, 2))
box(m.n("front"), (w_, 0.08, h_), (0, -reach_ + 0.04, h_ / 2), FRONT_BURNT, m.root)
for s_ in (-1, 1):
    box(m.n(f"cheek_{'ab'[s_ > 0]}"), (0.08, reach_ - 0.08, h_), (s_ * (w_ / 2 - 0.04), -(reach_ - 0.08) / 2, h_ / 2), FRONT_BURNT, m.root,
        lods=(0, 1))
m = kit.module("balcony_broken", **PANEL)
box(m.n("slab"), (w_, reach_, 0.14), (0, -reach_ / 2, -0.07), CAST_BURNT, m.root)
box(m.n("front"), (w_ * 0.45, 0.08, h_), (-w_ * 0.275, -reach_ + 0.04, h_ / 2), FRONT_BURNT, m.root, lods=(0, 1, 2))
box(m.n("cheek"), (0.08, reach_ - 0.08, h_), (-(w_ / 2 - 0.04), -(reach_ - 0.08) / 2, h_ / 2), FRONT_BURNT, m.root, lods=(0, 1))
box(m.n("fallen"), (w_ * 0.4, 0.08, h_ * 0.8), (w_ * 0.25, -reach_ * 0.6, 0.12), FRONT_BURNT, m.root, rot=(1.35, 0, 0.2), lods=(0,))
m = kit.module("balcony_hanging", **PANEL)
box(m.n("slab"), (w_, reach_, 0.14), (0, -0.5 * reach_ * math.cos(0.9), -0.07 - 0.5 * reach_ * math.sin(0.9)), CAST_BURNT, m.root,
    rot=(-0.9, 0, 0))

# A loggia glazed in: sashes on the wall's face over the balustrade.
m = kit.module("loggia_sash", **PANEL)
x0, x1, z0, z1 = OPENING["l"]
sheet(m.n("glass"), glass_m, m.root, [flat(x0, x1, z0 + 1.04, z1, 0.03)])
sheet(m.n("bars"), frame_m, m.root, [flat(x - 0.03, x + 0.03, z0 + 1.04, z1, 0.01) for x in (x0 + 0.03, -0.43, 0.43, x1 - 0.03)]
      + [flat(x0, x1, z1 - 0.06, z1, 0.01)], lods=(0,))

# Curtains behind a window's glass, in the window panel's frame: a row's tint is their colour.
x0, x1, z0, z1 = OPENING["w"]
m = kit.module("curtain_pair", **PANEL)
sheet(m.n("drapes"), drape_m, m.root, [flat(x0 + 0.05, x0 + 0.42, z0 + 0.05, z1 - 0.05, REVEAL_M - 0.015),
                                       flat(x1 - 0.42, x1 - 0.05, z0 + 0.05, z1 - 0.05, REVEAL_M - 0.015)])
m = kit.module("curtain_blind", **PANEL)
sheet(m.n("blind"), drape_m, m.root, [flat(x0 + 0.05, x1 - 0.05, z0 + 0.7, z1 - 0.05, REVEAL_M - 0.015)])

# An air conditioner's outdoor unit on brackets beside a window.
m = kit.module("ac_unit", **PANEL)
box(m.n("case"), (0.62, 0.28, 0.5), (1.12, -0.17, 1.2), unit_m, m.root)
sheet(m.n("grille"), metal_m, m.root, [flat(0.87, 1.25, 1.02, 1.38, -0.315)], lods=(0,))
box(m.n("bracket"), (0.66, 0.3, 0.04), (1.12, -0.16, 0.93), metal_m, m.root, lods=(0,))

# Washing on a line across a balcony, 0.6 m before the wall: two things take the row's tint.
m = kit.module("washing", **PANEL)
box(m.n("line"), (2.5, 0.015, 0.015), (0, -0.6, 2.05), metal_m, m.root, lods=(0,))
for k, (x, w, drop, mat) in enumerate(((-0.85, 0.5, 0.75, cloth_m), (-0.2, 0.42, 0.55, linen_m), (0.35, 0.36, 0.8, cloth_m),
                                       (0.9, 0.5, 0.6, linen_m))):
    box(m.n(f"cloth_{k}"), (w, 0.02, drop), (x, -0.6, 2.04 - drop / 2), mat, m.root, lods=TIERS if k == 0 else (0,))

# ---------------------------------------------------------------- the roof's furniture
ROOFTOP = dict(ao_distance=0.8, paint_scale=10.0)


def hut(name, w, d, h, burnt=False):
    """A room on the roof (a lift's machine room, a stair's head), standing on the origin, its door
    facing -Y. `burnt`, it is sooted, and its door and louvre are holes."""
    m = kit.module(name, **ROOFTOP)
    box(m.n("room"), (w, d, h), (0, 0, h / 2), HUT_BURNT if burnt else hut_m, m.root)
    box(m.n("lid"), (w + 0.3, d + 0.3, 0.14), (0, 0, h + 0.07), CAST_BURNT if burnt else rail_m, m.root, lods=(0, 1, 2))
    box(m.n("door"), (0.9, 0.05, 2.0), (-w / 2 + 1.0, -d / 2 - 0.02, 1.0), void_m if burnt else metal_m, m.root, lods=(0, 1))
    box(m.n("louvre"), (0.05, 1.2, 0.6), (w / 2 + 0.02, 0, h - 0.7), void_m if burnt else metal_m, m.root, lods=(0, 1))
    return h + 0.14


HUTS = {"bulkhead": hut("bulkhead", 6.0, 4.5, 2.7), "stairhead": hut("stairhead", 3.2, 4.2, 2.3)}  # each one's height
hut("bulkhead_burnt", 6.0, 4.5, 2.7, burnt=True)
hut("stairhead_burnt", 3.2, 4.2, 2.3, burnt=True)

PLANT = (2.4, 1.4, 1.2)
UNIT_BURNT = scorched(unit_m, 1e6, 9.0, floor=0.8)
for name_, case_ in (("roof_plant", unit_m), ("roof_plant_burnt", UNIT_BURNT)):  # a packaged chiller on skids, its fan on top
    m = kit.module(name_, **ROOFTOP)
    box(m.n("case"), (PLANT[0], PLANT[1], PLANT[2] - 0.2), (0, 0, 0.2 + (PLANT[2] - 0.2) / 2), case_, m.root)
    cyl(m.n("fan"), 0.45, 0.06, (-0.5, 0, PLANT[2] + 0.03), "Z", metal_m, m.root, seg=12, lods=(0, 1))
    box(m.n("grille"), (1.7, 0.03, 0.55), (0, -PLANT[1] / 2 - 0.015, 0.75), metal_m, m.root, lods=(0, 1))
    for s in (-1, 1):
        box(m.n(f"skid_{'ab'[s > 0]}"), (0.12, PLANT[1], 0.2), (s * 0.9, 0, 0.1), metal_m, m.root, lods=(0, 1))

for name_, cast_ in (("roof_vent", rail_m), ("roof_vent_burnt", CAST_BURNT)):
    m = kit.module(name_, **ROOFTOP)
    box(m.n("stack"), (0.7, 0.7, 0.75), (0, 0, 0.375), cast_, m.root)
    box(m.n("cowl"), (0.95, 0.95, 0.08), (0, 0, 0.9), cast_, m.root, lods=(0, 1))

m = kit.module("roof_tank", **ROOFTOP)
cyl(m.n("drum"), 1.1, 1.8, (0, 0, 1.45), "Z", tank_m, m.root, seg=20)
box(m.n("deck"), (2.4, 2.4, 0.1), (0, 0, 0.5), rail_m, m.root, lods=(0, 1, 2))
for k, (sx, sy) in enumerate(((-1, -1), (1, -1), (1, 1), (-1, 1))):
    box(m.n(f"leg_{k}"), (0.14, 0.14, 0.45), (sx * 1.0, sy * 1.0, 0.225), rail_m, m.root, lods=(0, 1))

AERIAL_M = 1.6
m = kit.module("roof_aerial", **ROOFTOP)
cyl(m.n("mast"), 0.03, AERIAL_M, (0, 0, AERIAL_M / 2), "Z", metal_m, m.root, seg=6, caps=False)
for k, (z, w) in enumerate(((1.5, 0.9), (1.3, 0.7), (1.1, 1.0))):
    box(m.n(f"bar_{k}"), (w, 0.02, 0.02), (0, 0, z), metal_m, m.root, lods=(0, 1))


# ---------------------------------------------------------------- a tower
def roll(*key):
    """A fixed number in [0, 1) for `key`: which window has curtains, which balcony washing."""
    return zlib.crc32(repr(key).encode()) / 2 ** 32


def runs(keys):
    """`keys` as (first index, one past the last, key) for each stretch of equal neighbours."""
    out, start = [], 0
    for k in range(1, len(keys) + 1):
        if k == len(keys) or keys[k] != keys[start]:
            out.append((start, k, keys[start]))
            start = k
    return out


# What a roof's module is from far off, folded into the shell as a box (width, depth, height; None a
# drum), and the tiers it is folded at: a vent is too small for the coarsest.
FOLDED = {"bulkhead": ((6.0, 4.5, HUTS["bulkhead"]), (2, 3)), "stairhead": ((3.2, 4.2, HUTS["stairhead"]), (2, 3)),
          "roof_tank": (None, (2, 3)), "roof_plant": (PLANT, (2, 3)), "roof_vent": ((0.7, 0.7, 0.83), (2,))}
WALLED = (1, 2, 3)  # the tiers a tower's walls are its shell's own
ONE_ROW = (2, 3)  # the tiers a tower is its shell and nothing else
ROOF_BAND_M = 1.2  # the roof's edge inside the parapet, which the parapet shades
ROOF_CELL_M = 4.6  # the roof's stains live on vertices: this keeps them down to the third tier


def shell(t, tag, finish, columns, accents, doors, fronts, rooftop, crown=None, burnt=False, blown=()):
    """The template's own module: corner piers, parapet and roof in every tier; a closed
    core behind the panels at tier 0; from tier 1 the walls themselves, each run of
    like bays one face of its facade recipe; and at tiers 2 and 3 what the other rows
    drew: each balcony column a stack (`fronts`: its colour by side and bay), the
    canopies and the roof's huts, plant and tanks (`rooftop`) as boxes. `crown` tints
    the top floor and the parapet another colour, under a slab that stands out round
    the roof's edge. `burnt` is the tower gutted, in the burnt recipes: `blown` are the
    stretches of wall blown out, as (side, first bay, bays, first floor, floors)."""
    m = kit.module(f"{tag}_{'gutted' if burnt else 'shell'}", ground=True, paint_scale=6.0)
    floors = len(t.floor_heights)
    ceiling, top = floors * FLOOR_M, floors * FLOOR_M + PARAPET_M
    recipe = FINISHES[finish]
    tile = textures.tile_of(recipe)
    groups = {}  # (tiers, material) -> faces
    after = "_burnt" if burnt else ""

    def strip(recipe_, tint=None, tone=1.0, plain=False):
        return strip_mat(recipe_, tint, tone, smoke=burnt, plain=plain)

    def wall(lods, mat, edge, o0, o1, z0, z1, left, size, inset=0.0):
        a, b = t.at(edge, o0, 0.0, -inset), t.at(edge, o1, 0.0, -inset)
        normal = SIDES[t.edges()[edge]["side"]][1]
        u0, u1, v0, v1 = (o0 - left) / size, (o1 - left) / size, z0 / size, z1 / size
        groups.setdefault((lods, mat.name), (mat, []))[1].append(
            ([(a[0], a[1], z0), (b[0], b[1], z0), (b[0], b[1], z1), (a[0], a[1], z1)], (normal[0], normal[1], 0),
             [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]))

    def stack(edge, o, tint):
        """A column of balconies at bay `o` as one box, each face the balcony recipe a floor to the floor."""
        w, reach, h = BALCONY
        z0, z1 = FLOOR_M - 0.14, (floors - 1) * FLOOR_M + h
        mat = strip("facade_balconies_burnt" if burnt else balconies_recipe(tint), plain=True)  # burnt, a front has no colour left
        wall(ONE_ROW, mat, edge, o - w / 2, o + w / 2, z0, z1, o - BAY_M / 2, FACADE_M, inset=-reach)
        along = SIDES[t.edges()[edge]["side"]][2]
        for s in (-1, 1):
            (ax, ay, *_), (bx, by, *_) = t.at(edge, o + s * w / 2, 0.0, 0.0), t.at(edge, o + s * w / 2, 0.0, reach)
            groups[(ONE_ROW, mat.name)][1].append(([(ax, ay, z0), (bx, by, z0), (bx, by, z1), (ax, ay, z1)], (s * along[0], s * along[1], 0),
                                              [(0, z0 / FACADE_M), (reach / FACADE_M, z0 / FACADE_M), (reach / FACADE_M, z1 / FACADE_M),
                                               (0, z1 / FACADE_M)]))
        (ax, ay, *_), (bx, by, *_) = t.at(edge, o - w / 2, 0.0, 0.0), t.at(edge, o + w / 2, 0.0, 0.0)
        (cx, cy, *_), (dx, dy, *_) = t.at(edge, o + w / 2, 0.0, reach), t.at(edge, o - w / 2, 0.0, reach)
        lids.append(([(ax, ay, z1), (bx, by, z1), (cx, cy, z1), (dx, dy, z1)], (0, 0, 1), None))

    lids = []
    cast = CAST_BURNT if burnt else rail_m
    band = ceiling - FLOOR_M if crown else ceiling  # where the top floor's own colour starts

    def storeys(lods, kind, edge, k0, k1, left):
        """The wall of bays `k0` to `k1` from the first floor up: `kind`'s recipe, the crown's colour on the top floor."""
        o0, o1 = left + BAY_M * k0, left + BAY_M * k1
        name, size = (recipe, tile) if kind == "b" else (f"facade_{KINDS[kind]}{after}", FACADE_M)
        wall(lods, strip(name, accents.get(kind)), edge, o0, o1, FLOOR_M, band, left, size)
        if crown:
            wall(lods, strip(name, crown), edge, o0, o1, band, ceiling, left, size)

    body, base = strip(recipe), strip("concrete", tone=BASE[0] / textures.baked("concrete").mean()[0])
    head = strip(recipe, crown) if crown else body
    for side, cols in sorted(columns.items()):
        edge = f"body-{side}"
        (a, b), bays = t.edges()[edge]["span"], t.bays(edge)
        left = bays[0] - BAY_M / 2
        right = left + BAY_M * len(bays)
        for o0, o1 in ((a, left), (right, b)):  # the piers
            wall(TIERS, base, edge, o0, o1, 0.0, FLOOR_M, left, textures.tile_of("concrete"))
            wall(TIERS, body, edge, o0, o1, FLOOR_M, band, left, tile)
            if crown:
                wall(TIERS, head, edge, o0, o1, band, ceiling, left, tile)
        wall(TIERS, head, edge, a, b, ceiling, top, left, tile)  # the parapet
        wall((0,), void_m if burnt else body, edge, a + CORE_M, b - CORE_M, 0.0, ceiling + ROOF_M, left, tile, inset=CORE_M)
        ground = ["e" if side == "south" and k in doors else "b" if kind == "b" else "g" for k, kind in enumerate(cols)]
        for k0, k1, key in runs(ground):
            if key == "g":
                wall(WALLED, strip("facade_window" + after, tone=BASE_TONE), edge, left + BAY_M * k0, left + BAY_M * k1, 0.0, FLOOR_M, left,
                     FACADE_M)
            else:
                wall(WALLED, base, edge, left + BAY_M * k0, left + BAY_M * k1, 0.0, FLOOR_M, left, textures.tile_of("concrete"))
        holes = [(k, n, f, h) for where, k, n, f, h in blown if where == side]
        for k0, k1, kind in runs(list(cols)):
            cut = [(k, n, f, h) for k, n, f, h in holes if k0 <= k and k + n <= k1]
            if len(cut) != len([hole for hole in holes if k0 <= hole[0] < k1]):
                raise SystemExit(f"{t.id}: a blown stretch of {side} crosses two kinds of bay")
            at = k0
            for k, n, f, h in sorted(cut):  # the run in stretches: whole to the hole, then the hole's own bays under, in and over it
                if k > at:
                    storeys(WALLED, kind, edge, at, k, left)
                o0, o1 = left + BAY_M * k, left + BAY_M * (k + n)
                mat = strip(recipe if kind == "b" else f"facade_{KINDS[kind]}{after}", accents.get(kind))
                size = tile if kind == "b" else FACADE_M
                if f > 1:
                    wall(WALLED, mat, edge, o0, o1, FLOOR_M, f * FLOOR_M, left, size)
                wall(WALLED, strip("facade_blown_burnt"), edge, o0, o1, f * FLOOR_M, (f + h) * FLOOR_M, left, FACADE_M)
                if (f + h) * FLOOR_M < ceiling:
                    wall(WALLED, mat, edge, o0, o1, (f + h) * FLOOR_M, ceiling, left, size)
                at = k + n
            if at < k1:
                storeys(WALLED, kind, edge, at, k1, left)
        for k, (o, kind) in enumerate(zip(bays, cols)):
            if kind == "d":
                stack(edge, o, fronts[(side, k)])
            if side == "south" and k in doors:
                x, y, _, yaw = t.at(edge, o, 0.0, 0.7)
                box(m.n(f"canopy_{k}"), (2.9, 1.4, 0.14), (x, y, 2.52), cast, m.root, rot=(0, 0, yaw), lods=ONE_ROW)
    for k, ((lods, _), (mat, quads)) in enumerate(sorted(groups.items())):
        sheet(m.n(f"wall_{k}"), mat, m.root, quads, lods=lods)
    if lids and not burnt:
        sheet(m.n("balcony_lids"), rail_m, m.root, lids, lods=ONE_ROW)
    deck = ceiling + ROOF_M
    for k, (module, x, y, yaw) in enumerate(rooftop):
        if module not in FOLDED:
            continue
        size, lods = FOLDED[module]
        if size and module in HUTS:  # a hut keeps what it is near: its walls the tower's colour, its lid concrete, its door dark
            w, d, h = size
            c, s_ = math.cos(yaw), math.sin(yaw)
            box(m.n(f"hut_{k}"), (w, d, h - 0.14), (x, y, deck + (h - 0.14) / 2), HUT_BURNT if burnt else hut_m, m.root, rot=(0, 0, yaw), lods=lods)
            box(m.n(f"hut_lid_{k}"), (w + 0.3, d + 0.3, 0.14), (x, y, deck + h - 0.07), cast, m.root, rot=(0, 0, yaw), lods=lods)
            dx, dy = -w / 2 + 1.0, -d / 2 - 0.02
            if not burnt:  # (a burnt hut's doorway is a dark hole in a dark wall)
                box(m.n(f"hut_door_{k}"), (0.9, 0.05, 2.0), (x + dx * c - dy * s_, y + dx * s_ + dy * c, deck + 1.0), metal_m, m.root,
                    rot=(0, 0, yaw), lods=(2,))
        elif size:
            mat = cast if module != "roof_plant" else UNIT_BURNT if burnt else unit_m
            box(m.n(f"hut_{k}"), size, (x, y, deck + size[2] / 2), mat, m.root, rot=(0, 0, yaw), lods=lods)
        else:
            cyl(m.n(f"tank_{k}"), 1.1, 1.8, (x, y, deck + 1.45), "Z", tank_m, m.root, seg=20, lods=lods[:1] if burnt else lods)
    # The roof inside the parapet: a field that carries its stains on a grid of vertices down to the
    # third tier, inside a band the parapet shades (so the field is lit the same at every tier),
    # with what was mended since laid on it; then the parapet's inner faces and its coping.
    p = t.parts[0]
    x0, x1, y0, y1 = p["x0"], p["x1"], p["y0"], p["y1"]
    i0, i1, j0, j1 = x0 + COPING_M, x1 - COPING_M, y0 + COPING_M, y1 - COPING_M
    f0, f1, g0, g1 = i0 + ROOF_BAND_M, i1 - ROOF_BAND_M, j0 + ROOF_BAND_M, j1 - ROOF_BAND_M
    felt = ROOF_BURNT if burnt else roof_m
    nx, ny = max(1, math.ceil((f1 - f0) / ROOF_CELL_M)), max(1, math.ceil((g1 - g0) / ROOF_CELL_M))
    cells = [([(f0 + (f1 - f0) * i / nx, g0 + (g1 - g0) * j / ny, deck), (f0 + (f1 - f0) * (i + 1) / nx, g0 + (g1 - g0) * j / ny, deck),
               (f0 + (f1 - f0) * (i + 1) / nx, g0 + (g1 - g0) * (j + 1) / ny, deck), (f0 + (f1 - f0) * i / nx, g0 + (g1 - g0) * (j + 1) / ny, deck)],
              (0, 0, 1), None) for j in range(ny) for i in range(nx)]
    sheet(m.n("roof"), felt, m.root, cells, lods=(0, 1, 2))
    sheet(m.n("roof_far"), felt, m.root, [([(f0, g0, deck), (f1, g0, deck), (f1, g1, deck), (f0, g1, deck)], (0, 0, 1), None)], lods=(3,))
    outer, inner, field = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)], [(i0, j0), (i1, j0), (i1, j1), (i0, j1)], [(f0, g0), (f1, g0), (f1, g1), (f0, g1)]
    sheet(m.n("roof_band"), felt, m.root, [([(*inner[k], deck), (*inner[(k + 1) % 4], deck), (*field[(k + 1) % 4], deck), (*field[k], deck)],
                                            (0, 0, 1), None) for k in range(4)])
    patches = []
    for k in range(3 + round((f1 - f0) * (g1 - g0) / 300)):  # felt laid since, in lengths of a roll or two
        w, d = (2.0 + 4.0 * roll(t.id, "patch", k, "w"), 1.0 + 2.0 * roll(t.id, "patch", k, "d"))[::1 if k % 2 else -1]
        x, y = f0 + w / 2 + roll(t.id, "patch", k, "x") * (f1 - f0 - w), g0 + d / 2 + roll(t.id, "patch", k, "y") * (g1 - g0 - d)
        patches.append(([(x - w / 2, y - d / 2, deck + 0.02), (x + w / 2, y - d / 2, deck + 0.02), (x + w / 2, y + d / 2, deck + 0.02),
                         (x - w / 2, y + d / 2, deck + 0.02)], (0, 0, 1), None))
    sheet(m.n("roof_patches"), ash_m if burnt else patch_m, m.root, patches)  # burnt, the patches are ash where the felt burnt away
    quads = []
    for k in range(4):
        (ax, ay), (bx, by), (cx, cy), (dx, dy) = outer[k], outer[(k + 1) % 4], inner[(k + 1) % 4], inner[k]
        quads.append(([(ax, ay, top), (bx, by, top), (cx, cy, top), (dx, dy, top)], (0, 0, 1), None))
        inward = ((i0 + i1) / 2 - (cx + dx) / 2, (j0 + j1) / 2 - (cy + dy) / 2, 0)
        quads.append(([(dx, dy, deck), (cx, cy, deck), (cx, cy, top), (dx, dy, top)], inward, None))
    sheet(m.n("coping"), cast, m.root, quads)
    if crown:  # the roof's slab, standing out round the building over the top floor
        box(m.n("eave"), (x1 - x0 + 1.1, y1 - y0 + 1.1, 0.24), ((x0 + x1) / 2, (y0 + y1) / 2, ceiling + 0.12), cast, m.root)
    return m.name


def tower(id_, tag, w, d, floors, finish, body, columns, doors, rooftop, accents=None, fronts=(WHITE,), glazed=0.25, recipe=None,
          crown=None, blown=()):
    """A tower that is one box: its template, its shell and a row for every panel, and the same gutted.

    `columns` gives each side's bays as letters, read from its left as you face it:
    w a window, r a ribbon of glazing the bay's width, d a balcony door (with its
    balcony), l a loggia, s a stair light, b blank. A column is one kind from the
    first floor up. `doors` are the street (south) side's bays with a way in.
    `rooftop` is what stands on the roof: (module, x, y, yaw), and for an aerial the
    hut it stands on. `accents` tints a kind's columns another colour; `fronts` are the
    balcony fronts' colours, one a column in turn; `crown` the top floor's. `blown` are
    the stretches of wall its fire blew out: (side, first bay, bays, first floor, floors)."""
    accents = accents or {}
    hx, hy = w / 2, d / 2
    t = kit.template(id_, "highrise", FAMILY, dict(Width=w, Depth=d, Floors=floors, Finish=finish, **(recipe or {}),
                                                   **{side.capitalize(): cols for side, cols in sorted(columns.items())}))
    t.part("body", -hx, hx, -hy, hy, floors * FLOOR_M + PARAPET_M)
    t.floors(*(k * FLOOR_M for k in range(floors)))
    for side, cols in columns.items():
        length = w if side in ("south", "north") else d
        if len(cols) * BAY_M + 2 * PIER_M != length:
            raise SystemExit(f"{id_}: {side} is {length} m, not {len(cols)} bays between two piers")
        t.lattice(f"body-{side}", 0.0 if len(cols) % 2 else BAY_M / 2)
    for k in doors:
        t.entrance("body-south", t.bays("body-south")[k])
    doors_of = [(side, k) for side, cols in sorted(columns.items()) for k, kind in enumerate(cols) if kind == "d"]
    column = {key: fronts[n % len(fronts)] for n, key in enumerate(doors_of)}
    huts = [(module, x, y, yaw) for module, x, y, yaw, *_ in rooftop]
    t.place(shell(t, tag, finish, columns, accents, doors, column, huts, crown), tint=body)
    t.place(shell(t, tag, finish, columns, accents, doors, column, huts, crown, burnt=True, blown=blown), tint=body, state="gutted")
    gone = {(side, k + i, f + j) for side, k, n, f, h in blown for i in range(n) for j in range(h)}
    deck = floors * FLOOR_M + ROOF_M
    for module, x, y, yaw, *on in rooftop:
        hut = module in HUTS
        z, tint = deck + (HUTS[on[0]] if on else 0.0), body if hut else WHITE
        t.place(module, x, y, z, yaw, tiers=TIERS_0_TO_1, tint=tint)
        if module != "roof_aerial":  # burnt, a hut, a vent and the plant are their sooted twins, and the aerials are down
            t.place(module if module == "roof_tank" else module + "_burnt", x, y, z, yaw, tiers=TIERS_0_TO_1, tint=tint, state="gutted")
    for side, cols in sorted(columns.items()):
        edge = f"body-{side}"
        for k, (o, kind) in enumerate(zip(t.bays(edge), cols)):
            if side == "south" and k in doors:
                t.mount("entrance", edge, o, tiers=TIERS_0_TO_1)
                t.mount("gutted_entrance", edge, o, tiers=TIERS_0_TO_1, state="gutted")
            else:
                t.mount("ground_blank" if kind == "b" else "ground_window", edge, o, tiers=TIER_0)
                t.mount("ground_blank" if kind == "b" else gutted_panel("w", k % 2, ground=True), edge, o, tiers=TIER_0,
                        tint=dimmed(WHITE, FLOOR_M / 2), state="gutted")
            for f in range(1, floors):
                z, key = f * FLOOR_M, (id_, side, k, f)
                tint = crown if crown and f == floors - 1 else accents.get(kind, body)
                t.mount(panel(finish, kind), edge, o, z=z, tiers=TIER_0, tint=tint)
                if kind in "wr":
                    if k % 2 + 2 * (f % 2) in CURTAINED:  # the bays the shell's recipe draws curtains in
                        t.mount(CURTAINED[k % 2 + 2 * (f % 2)], edge, o, z=z, tiers=TIER_0,
                                scale=(1.8 if kind == "r" else 1.0, 1.0, 1.0), tint=CURTAINS[int(roll(key, "cloth") * len(CURTAINS))])
                    if kind == "w" and roll(key, "ac") < 0.12:
                        t.mount("ac_unit", edge, o, z=z, tiers=TIER_0)
                elif kind == "d":
                    shut = roll(key, "glazed") < glazed
                    # a column's fronts are one colour, but for the odd one its owner repainted
                    front = column[(side, k)] if roll(key, "repaint") > 0.08 else REPAINTS[int(roll(key, "paint") * len(REPAINTS))]
                    t.mount("balcony_glazed" if shut else "balcony", edge, o, z=z, tiers=TIERS_0_TO_1, tint=front)
                    if not shut and roll(key, "washing") < 0.2:
                        t.mount("washing", edge, o, z=z, tiers=TIER_0, tint=WASHING[int(roll(key, "wash") * len(WASHING))])
                elif kind == "l":
                    if roll(key, "glazed") < glazed:
                        t.mount("loggia_sash", edge, o, z=z, tiers=TIER_0)
                    elif roll(key, "washing") < 0.2:
                        t.mount("washing", edge, o, z=z, out=-1.2, tiers=TIER_0, tint=WASHING[int(roll(key, "wash") * len(WASHING))])
                # the same bay, gutted: the quarter of the burnt recipe the shell shows there
                if (side, k, f) in gone:
                    t.mount(gutted_panel("x", k % 2 + 2 * (f % 2)), edge, o, z=z, tiers=TIER_0, state="gutted")
                    continue
                t.mount(panel(finish, kind) if kind == "b" else gutted_panel(kind, k % 2 + 2 * (f % 2)), edge, o, z=z, tiers=TIER_0,
                        tint=dimmed(tint, z + FLOOR_M / 2), state="gutted")
                if kind == "d":
                    fate = roll(key, "burnt")
                    if fate < 0.9:  # one in ten fell
                        t.mount("balcony_burnt" if fate < 0.55 else "balcony_broken" if fate < 0.78 else "balcony_hanging", edge, o, z=z,
                                tiers=TIERS_0_TO_1, tint=column[(side, k)], state="gutted")
    return t


# ---------------------------------------------------------------- the four
# A ten-floor slab of bare grey precast, three stairs to the street: balconies paired
# where two sections meet, each section's fronts their own faded paint, blank gable ends.
tower("china-tower-slab-10f", "slab_10f", 56, 14, 10, "precast", GREY,
      dict(south="dwwswd" * 3, north="wdwwdw" * 3, east="bwwb", west="bwwb"), doors=(3, 9, 15),
      rooftop=[row for x in (-16.5, 1.5, 19.5) for row in (
          ("stairhead", x, -3.6, 0.0), ("roof_aerial", x + 1.0, -3.0, 0.0, "stairhead"), ("roof_vent", x - 6.0, 2.5, 0.0),
          ("roof_vent", x + 4.5, 3.0, 0.0), ("roof_vent", x - 3.0, -3.5, 0.0))]
      + [("roof_plant", -24.0, 2.0, 0.0), ("roof_plant", 9.5, 2.5, math.pi / 2), ("roof_tank", 25.0, 1.5, 0.0)],
      fronts=[FRONTS[k] for k in (1, 1, 2, 2, 0, 0, 0, 0, 2, 2, 1, 1)], recipe=dict(Sections=3),
      blown=[("south", 7, 2, 6, 2), ("south", 13, 1, 3, 2), ("north", 2, 2, 7, 2)])

# A twelve-floor point tower in cream render: loggias at the corners behind white
# balustrades, the stair and lift lights a terracotta stripe up the street front, and
# the top floor a terracotta band under a roof slab that stands out all round.
tower("china-tower-12f", "tower_12f", 26, 26, 12, "render", CREAM,
      dict(south="lwwsswwl", north="lwlwwlwl", east="lwwlwwwl", west="lwwwlwwl"), doors=(4,), accents={"s": TERRACOTTA},
      rooftop=[("bulkhead", 0.0, -6.0, 0.0), ("roof_aerial", 1.8, -6.5, 0.0, "bulkhead"), ("roof_tank", 6.5, 5.5, 0.0),
               ("roof_vent", -7.0, 6.0, 0.0), ("roof_vent", -7.0, -2.0, 0.0), ("roof_vent", 7.5, -3.0, 0.0),
               ("roof_plant", -3.0, 5.0, 0.0), ("roof_vent", 1.5, 1.0, 0.0), ("roof_vent", -8.5, -8.0, 0.0)],
      crown=TERRACOTTA, recipe=dict(Crown="terracotta"),
      blown=[("south", 1, 2, 8, 2), ("east", 4, 2, 4, 2)])

# Sixteen floors faced in white tile, its narrow side to the street: ribbons of
# glazing from balcony column to balcony column, the balconies' fronts pale aqua,
# every third bay down the long sides.
tower("china-tower-16f", "tower_16f", 23, 32, 16, "tile", WHITE,
      dict(south="drrsrrd", north="drrrrrd", east="drrdrrdrrd", west="drrdrrdrrd"), doors=(3,),
      accents={"s": AQUA}, fronts=(AQUA,), glazed=0.35,
      rooftop=[("bulkhead", 0.0, -9.0, 0.0), ("roof_aerial", -1.5, -9.0, 0.0, "bulkhead"), ("stairhead", 0.0, 9.5, math.pi),
               ("roof_aerial", 0.5, 9.5, 0.0, "stairhead"), ("roof_vent", -6.5, 0.0, 0.0), ("roof_vent", 6.5, 3.0, 0.0),
               ("roof_vent", -6.5, -11.0, 0.0), ("roof_vent", 6.0, -12.0, 0.0), ("roof_plant", 5.5, -3.5, math.pi / 2),
               ("roof_plant", -5.5, 5.0, math.pi / 2), ("roof_tank", -6.0, 11.5, 0.0), ("roof_vent", 2.0, 2.5, 0.0)],
      blown=[("east", 1, 2, 10, 2), ("south", 1, 2, 5, 2), ("west", 4, 2, 12, 2)])

# Twenty floors of pale precast, the town's landmark: loggias two bays deep at every
# corner, the stair light and a blank column each side picked out in slate blue.
tower("china-tower-20f", "tower_20f", 29, 29, 20, "precast", PALE,
      dict(south="llwwswwll", north="llwwbwwll", east="llwwbwwll", west="llwwbwwll"), doors=(4,),
      accents={"s": SLATE, "b": SLATE}, glazed=0.3,
      rooftop=[("bulkhead", 0.0, -5.0, 0.0), ("roof_aerial", 0.0, -5.0, 0.0, "bulkhead"), ("roof_aerial", 2.2, -5.8, 0.0, "bulkhead"),
               ("roof_tank", -7.5, 6.5, 0.0), ("roof_tank", 7.5, 6.5, 0.0), ("roof_vent", -8.0, -6.0, 0.0), ("roof_vent", 8.0, -6.0, 0.0),
               ("roof_vent", 0.0, 7.0, 0.0), ("roof_vent", -3.0, 1.0, 0.0), ("roof_plant", 9.0, 0.5, math.pi / 2),
               ("roof_plant", -9.5, 0.0, math.pi / 2), ("roof_vent", 4.0, 11.0, 0.0), ("roof_vent", -10.5, 10.5, 0.0)],
      blown=[("south", 2, 2, 13, 2), ("east", 5, 2, 7, 2), ("south", 5, 2, 4, 2), ("north", 2, 2, 16, 2)])

kit.write(next(iter(script_args()), None))  # an argument writes the two files somewhere else
