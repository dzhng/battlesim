"""Interior atlases: the rooms and shops seen behind a building's windows.

Each window of a city building draws a room box whose surfaces look one
picture up in an atlas, projected from a pinhole far in front of the window
(the flat-perspective lookup; the contract is in this folder's README,
"Interiors"). This script makes the pictures: it builds each room as simple
scripted geometry inside the box the lookup assumes, lights it with nothing
but the sky coming in through its own window wall, and renders it from the
lookup's pinhole. So a cell projected back onto its box is the room it was
rendered from.

Two sheets, `rooms.png` (apartments) and `shops.png` (ground floors), go to
`assets/source/city/interiors/`. Nothing in a room emits light, and nothing
carries a name, a brand or lettering.

Deterministic: Cycles on the CPU with a fixed seed and sample count and no
denoiser, the tone curve and the PNG in numpy. The same script writes the
same bytes.

    asset blender packages/scene-assets/blender/city/interiors.py -- [--out DIR]
"""
import math
import os
import random
import shutil
import sys
import tempfile
import zlib

import bmesh
import bpy
import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import common  # noqa: E402
import textures  # noqa: E402

# ---------------------------------------------------------------- the contract
# The room box every cell assumes: x across, y into the building from the
# inside face of the window wall, z up from the floor.
ROOM_W, ROOM_H, ROOM_D = 3.0, 3.0, 4.5
# The pinhole stands this far in front of the box's open face, on its axis,
# and frames that face exactly.
EYE_M = 16.0
COLS, ROWS = 2, 5
# 2 x 5 square cells under the renderer's 1024 px texture cap: 128 px is the
# largest power of two that fits five rows.
CELL = 128

SUPERSAMPLE = 4
SAMPLES = 96

# The daylight entering a room (W), the same whatever its opening: a shopfront
# is three times an apartment window's glass, and at the same sky its room
# would read as lit.
SKY_W = 117.0
# The picture's tone. The game shows a cell as a matte surface in sun shadow,
# so a cell is set against that, not against a display: its values are what a
# shaded surface of that colour would have. A high exposure under a low
# ceiling brings the back of a room up toward its front, so a window reads as
# a room from 30 to 100 m, and nothing passes `CEILING` (linear): the palest
# room, behind its glass, stays dimmer than the building's own wall in shade.
# Tune both in the facade scene's room frames, never on the sheet.
EXPOSURE = 1.0
CEILING = 0.15

WALL_T = 0.3

# ---------------------------------------------------------------- palette (linear albedo)
# Walls differ strongly in hue: from the game's distances a window is a few
# pixels, and its wall and floor colour are what tell one room from the next.
WHITE = (0.6, 0.59, 0.55)
CREAM = (0.6, 0.5, 0.3)
GREEN = (0.2, 0.4, 0.2)
BLUE = (0.17, 0.3, 0.5)
YELLOW = (0.62, 0.45, 0.12)
PINK = (0.56, 0.24, 0.24)
MINT = (0.26, 0.5, 0.4)
BEIGE = (0.5, 0.36, 0.22)
GREY = (0.36, 0.36, 0.35)
CONCRETE = (0.33, 0.325, 0.31)
BRICK = (0.3, 0.12, 0.08)
PLASTER = (0.48, 0.45, 0.4)
# Floors are light: from the steep game camera a window is mostly its floor.
PARQUET = (0.42, 0.25, 0.12)
BOARDS = (0.34, 0.23, 0.13)
LINO = (0.3, 0.38, 0.26)
TILE = (0.48, 0.47, 0.43)
CARPET = (0.42, 0.13, 0.1)
SCREED = (0.34, 0.33, 0.31)
WOOD = (0.19, 0.11, 0.055)
DARK_WOOD = (0.07, 0.04, 0.025)
PALE_WOOD = (0.36, 0.26, 0.15)
STEEL = (0.2, 0.2, 0.2)
ENAMEL = (0.5, 0.5, 0.48)
BLACK = (0.012, 0.012, 0.012)
DARK = (0.03, 0.03, 0.032)
CARDBOARD = (0.3, 0.2, 0.11)
RUBBLE = ((0.25, 0.24, 0.22), (0.18, 0.09, 0.06), (0.33, 0.31, 0.27), (0.12, 0.11, 0.1))
BOOKS = ((0.2, 0.05, 0.04), (0.05, 0.1, 0.16), (0.07, 0.14, 0.07), (0.3, 0.25, 0.14), (0.1, 0.09, 0.08),
         (0.32, 0.3, 0.26), (0.22, 0.12, 0.05), (0.14, 0.07, 0.14))
GOODS = ((0.4, 0.08, 0.05), (0.42, 0.33, 0.06), (0.08, 0.2, 0.36), (0.1, 0.3, 0.12), (0.45, 0.43, 0.38),
         (0.42, 0.2, 0.05), (0.2, 0.08, 0.2), (0.3, 0.3, 0.3))
CLOTH = ((0.06, 0.08, 0.16), (0.25, 0.06, 0.05), (0.3, 0.28, 0.22), (0.05, 0.05, 0.05), (0.1, 0.16, 0.1),
         (0.34, 0.2, 0.08), (0.18, 0.18, 0.2), (0.3, 0.14, 0.2))
BINS = ((0.06, 0.12, 0.3), (0.35, 0.07, 0.04), (0.38, 0.3, 0.05), (0.2, 0.2, 0.2))
GLASSWARE = ((0.03, 0.08, 0.04), (0.1, 0.05, 0.02), (0.18, 0.2, 0.2), (0.05, 0.05, 0.06))
BREAD = ((0.34, 0.19, 0.07), (0.27, 0.14, 0.05), (0.4, 0.27, 0.12))


def shade(col, k):
    return tuple(min(1.0, c * k) for c in col)


# ---------------------------------------------------------------- materials
def paint(col, rough=0.9, mottle=0.0, stripes=None):
    """A matt surface of linear albedo `col`. `mottle` darkens it in soft stains;
    `stripes` (second colour, pitch in metres) rules it in vertical bands."""
    name = "m_" + "_".join(f"{v:.3f}" for v in (*col, rough, mottle)) + ("" if not stripes else f"_s{stripes[1]:.2f}")
    m = bpy.data.materials.get(name)
    if m is not None:
        return m
    m = common.mat(name, col, rough)
    if not mottle and not stripes:
        return m
    nt = m.node_tree
    out = None
    pos = nt.nodes.new("ShaderNodeNewGeometry").outputs["Position"]
    if stripes:
        xyz = nt.nodes.new("ShaderNodeSeparateXYZ")
        nt.links.new(pos, xyz.inputs[0])
        last = xyz.outputs[0]
        for op, value in (("MULTIPLY", 1.0 / stripes[1]), ("FRACT", 0.0), ("GREATER_THAN", 0.5)):
            n = nt.nodes.new("ShaderNodeMath")
            n.operation = op
            n.inputs[1].default_value = value
            nt.links.new(last, n.inputs[0])
            last = n.outputs[0]
        band = nt.nodes.new("ShaderNodeMix")
        band.data_type = "RGBA"
        band.inputs[6].default_value = (*col, 1)
        band.inputs[7].default_value = (*stripes[0], 1)
        nt.links.new(last, band.inputs[0])
        out = band.outputs[2]
    if mottle:
        noise = nt.nodes.new("ShaderNodeTexNoise")
        noise.inputs["Scale"].default_value = 1.1
        noise.inputs["Detail"].default_value = 5.0
        nt.links.new(pos, noise.inputs["Vector"])
        stain = nt.nodes.new("ShaderNodeMix")
        stain.data_type = "RGBA"
        stain.blend_type = "MULTIPLY"
        stain.inputs[7].default_value = (1 - mottle, 1 - mottle, 1 - mottle * 0.9, 1)
        if out is None:
            stain.inputs[6].default_value = (*col, 1)
        else:
            nt.links.new(out, stain.inputs[6])
        nt.links.new(noise.outputs[0], stain.inputs[0])
        out = stain.outputs[2]
    nt.links.new(out, nt.nodes["Principled BSDF"].inputs["Base Color"])
    return m


# ---------------------------------------------------------------- the set builder
class Set:
    """One room under construction. Furniture is built in its own frame (origin
    on the floor at the middle of its back edge, its front toward -y) under an
    empty that `at` puts in the room; yaw 0 faces the window."""

    def __init__(self, seed):
        self.rng = random.Random(seed)
        self.count = 0

    def _name(self):
        self.count += 1
        return f"p{self.count:04d}"

    def at(self, x, y, yaw=0.0, z=0.0, roll=0.0):
        e = common.empty(self._name(), (x, y, z))
        e.rotation_euler = (math.radians(roll), 0.0, math.radians(yaw))
        return e

    def box(self, lo, hi, col, parent=None, rot=None, rough=0.9, material=None):
        size = tuple(h - l for l, h in zip(lo, hi))
        loc = tuple((l + h) / 2 for l, h in zip(lo, hi))
        rot = tuple(math.radians(a) for a in rot) if rot else (0, 0, 0)
        return common.box(self._name(), size, loc, material or paint(col, rough), parent, rot=rot)

    def cyl(self, r, depth, loc, col, parent=None, axis="Z", r2=None, rough=0.9, seg=16):
        return common.cyl(self._name(), r, depth, loc, axis, paint(col, rough), parent, seg=seg, r2=r2)

    def pick(self, colours):
        return colours[self.rng.randrange(len(colours))]

    # ------------------------------------------------------------ fabric
    def curtain(self, x0, x1, z0, z1, col, y=0.16, folds_per_m=5.0, amp=0.01):
        """A hanging panel from x0 to x1, gathered into folds."""
        n = max(8, int(abs(x1 - x0) * folds_per_m * 6))
        bm = bmesh.new()
        phase = self.rng.uniform(0, math.tau)
        col_ = []
        for k in range(n + 1):
            u = k / n
            x = x0 + (x1 - x0) * u
            yy = y + math.sin(phase + u * abs(x1 - x0) * folds_per_m * math.tau) * amp
            col_.append((bm.verts.new((x, yy, z0)), bm.verts.new((x, yy, z1))))
        for a, b in zip(col_, col_[1:]):
            bm.faces.new((a[0], b[0], b[1], a[1])).smooth = True
        return common.obj_from_bm(self._name(), bm, [paint(col, 1.0)])

    def curtains(self, col, left=0.45, right=0.45, z0=0.75, z1=2.55, half=0.95, rod=True):
        """A pair of panels on a rod over the window: `left` and `right` are how
        far each is drawn across (metres from the rod's end; 0 for none)."""
        if left:
            self.curtain(-half, -half + left, z0, z1, col)
        if right:
            self.curtain(half - right, half, z0, z1, col)
        if rod:
            self.cyl(0.012, half * 2 + 0.1, (0, 0.16, z1 + 0.03), DARK_WOOD, axis="X", seg=8)

    # ------------------------------------------------------------ casework
    def cabinet(self, x, y, w, h, d, col, yaw=0.0, z=0.0, cols=1, rows=1, top=None, plinth=0.0, handles=True):
        """A closed cabinet: `cols` x `rows` doors or drawers, an optional worktop."""
        g = self.at(x, y, yaw, z)
        self.box((-w / 2, -d, plinth), (w / 2, 0, h), col, g)
        if plinth:
            self.box((-w / 2 + 0.03, -d + 0.05, 0), (w / 2 - 0.03, 0, plinth), shade(col, 0.3), g)
        seam = shade(col, 0.35)
        for i in range(1, cols):
            sx = -w / 2 + w * i / cols
            self.box((sx - 0.008, -d - 0.003, plinth), (sx + 0.008, -d, h), seam, g)
        for j in range(1, rows):
            sz = plinth + (h - plinth) * j / rows
            self.box((-w / 2, -d - 0.003, sz - 0.008), (w / 2, -d, sz + 0.008), seam, g)
        if handles:
            for i in range(cols):
                for j in range(rows):
                    hx = -w / 2 + w * (i + 0.5) / cols
                    hz = plinth + (h - plinth) * (j + 0.5) / rows
                    if rows == 1 and cols > 1:  # tall doors: handles by the meeting edge
                        hx = -w / 2 + w * (i + (0.85 if i % 2 == 0 else 0.15)) / cols
                    self.box((hx - 0.012, -d - 0.025, hz - 0.06), (hx + 0.012, -d, hz + 0.06), STEEL, g)
        if top:
            self.box((-w / 2 - 0.01, -d - 0.02, h), (w / 2 + 0.01, 0, h + 0.035), top, g)
        return g

    def shelving(self, x, y, w, h, d=0.32, shelves=5, fill=None, col=WOOD, yaw=0.0, z=0.0, density=0.9, back=True,
                 roll=0.0):
        """Open shelves, `fill`ed with books, goods, bins, bottles, loaves or folded cloth."""
        g = self.at(x, y, yaw, z, roll)
        t = 0.03
        for sx in (-w / 2, w / 2 - t):
            self.box((sx, -d, 0), (sx + t, 0, h), col, g)
        if back:
            self.box((-w / 2, -0.012, 0), (w / 2, 0, h), shade(col, 0.6), g)
        gap = (h - 0.08 - t) / shelves
        for i in range(shelves + 1):
            sz = 0.08 + i * gap
            self.box((-w / 2, -d, sz), (w / 2, 0, sz + t), col, g)
            if fill and i < shelves:
                self._fill(g, -w / 2 + t + 0.01, w / 2 - t - 0.01, sz + t, gap - t - 0.02, d, fill, density)
        return g

    def _fill(self, g, x0, x1, z, room, d, kind, density):
        rng = self.rng
        x = x0
        while x < x1 - 0.05:
            if rng.random() > density:
                x += rng.uniform(0.08, 0.3)
                continue
            if kind == "books":
                w = rng.uniform(0.07, 0.22)
                self.box((x, -d + 0.03, z), (min(x + w, x1), -0.03, z + room * rng.uniform(0.6, 0.95)),
                         shade(self.pick(BOOKS), rng.uniform(0.7, 1.1)), g)
            elif kind == "goods":
                w = rng.uniform(0.1, 0.26)
                self.box((x, -d + 0.01, z), (min(x + w, x1), -0.04, z + room * rng.uniform(0.45, 0.9)),
                         shade(self.pick(GOODS), rng.uniform(0.6, 1.0)), g)
            elif kind == "bins":
                w = 0.24
                self.box((x, -d - 0.02, z), (min(x + w, x1), -0.03, z + min(room * 0.7, 0.16)), self.pick(BINS), g)
            elif kind == "bottles":
                w = rng.uniform(0.08, 0.11)
                hgt = min(room * 0.9, rng.uniform(0.2, 0.3))
                self.cyl(w * 0.42, hgt, (x + w / 2, -d / 2, z + hgt / 2), self.pick(GLASSWARE), g, rough=0.5, seg=8)
            elif kind == "loaves":
                w = rng.uniform(0.2, 0.34)
                self.box((x, -d + 0.02, z), (min(x + w, x1), -0.05, z + min(room * 0.6, rng.uniform(0.08, 0.13))),
                         self.pick(BREAD), g)
            else:  # folded cloth
                w = rng.uniform(0.25, 0.35)
                for k in range(rng.randrange(1, 4)):
                    if 0.06 * (k + 1) < room:
                        self.box((x, -d + 0.02, z + 0.06 * k), (min(x + w, x1), -0.03, z + 0.06 * k + 0.05),
                                 self.pick(CLOTH), g)
            x += w + rng.uniform(0.005, 0.03)

    def counter(self, x, y, w, col, top=PALE_WOOD, yaw=0.0, h=0.95, d=0.6):
        g = self.cabinet(x, y, w, h, d, col, yaw, cols=max(1, round(w / 0.6)), top=top, plinth=0.08, handles=False)
        return g

    # ------------------------------------------------------------ seating and tables
    def sofa(self, x, y, w, col, yaw=0.0, roll=0.0, z=0.0):
        g = self.at(x, y, yaw, z, roll)
        self.box((-w / 2, -0.9, 0.08), (w / 2, 0, 0.4), col, g)
        self.box((-w / 2, -0.25, 0.4), (w / 2, 0, 0.85), col, g)
        for sx in (-w / 2, w / 2 - 0.18):
            self.box((sx, -0.9, 0.08), (sx + 0.18, 0, 0.62), shade(col, 0.9), g)
        n = max(1, round((w - 0.36) / 0.65))
        cw = (w - 0.36) / n
        for i in range(n):
            x0 = -w / 2 + 0.18 + i * cw
            self.box((x0 + 0.015, -0.88, 0.4), (x0 + cw - 0.015, -0.25, 0.5), shade(col, 1.15), g)
            self.box((x0 + 0.015, -0.36, 0.5), (x0 + cw - 0.015, -0.24, 0.8), shade(col, 1.1), g)
        return g

    def bed(self, x, y, w, blanket, yaw=0.0, frame=WOOD, length=2.0, linen=ENAMEL):
        """A bed with its headboard at the back of its frame."""
        g = self.at(x, y, yaw)
        self.box((-w / 2, -length, 0.12), (w / 2, 0, 0.3), frame, g)
        self.box((-w / 2, -0.04, 0), (w / 2, 0, 0.95), frame, g)
        self.box((-w / 2 + 0.03, -length + 0.03, 0.3), (w / 2 - 0.03, -0.04, 0.48), shade(linen, 0.8), g)
        self.box((-w / 2, -length, 0.26), (w / 2, -0.62, 0.52), blanket, g)
        n = 2 if w > 1.2 else 1
        pw = (w - 0.2) / n
        for i in range(n):
            x0 = -w / 2 + 0.1 + i * pw
            self.box((x0 + 0.04, -0.55, 0.48), (x0 + pw - 0.04, -0.1, 0.6), linen, g, rot=(8, 0, 0))
        return g

    def table(self, x, y, w, d, col, h=0.74, yaw=0.0, leg=0.05):
        g = self.at(x, y, yaw)
        self.box((-w / 2, -d, h - 0.04), (w / 2, 0, h), col, g)
        for sx in (-w / 2 + 0.04, w / 2 - 0.04 - leg):
            for sy in (-d + 0.04, -0.04 - leg):
                self.box((sx, sy, 0), (sx + leg, sy + leg, h - 0.04), shade(col, 0.8), g)
        return g

    def round_table(self, x, y, r, col, h=0.74):
        g = self.at(x, y)
        self.cyl(r, 0.035, (0, 0, h - 0.018), col, g, seg=24)
        self.cyl(0.035, h - 0.05, (0, 0, (h - 0.05) / 2 + 0.015), DARK, g, seg=8)
        self.cyl(0.22, 0.03, (0, 0, 0.015), DARK, g, seg=16)
        return g

    def chair(self, x, y, col, yaw=0.0, z=0.0, roll=0.0):
        g = self.at(x, y, yaw, z, roll)
        self.box((-0.21, -0.42, 0.43), (0.21, 0, 0.47), col, g)
        for sx in (-0.21, 0.175):
            self.box((sx, -0.42, 0), (sx + 0.035, -0.385, 0.43), col, g)
            self.box((sx, -0.035, 0), (sx + 0.035, 0, 0.9), col, g)
        self.box((-0.21, -0.03, 0.66), (0.21, -0.005, 0.86), col, g)
        return g

    def stool(self, x, y, col, h=0.48):
        g = self.at(x, y)
        self.cyl(0.17, 0.04, (0, 0, h - 0.02), col, g)
        for a in range(3):
            ang = a * math.tau / 3
            self.cyl(0.015, h, (0.11 * math.cos(ang), 0.11 * math.sin(ang), h / 2), DARK, g, seg=6)
        return g

    # ------------------------------------------------------------ walls and floor
    def door(self, x, col, ajar=0.0, frame=None, y=ROOM_D, w=0.85, h=2.05):
        """A door in the back wall; `ajar` degrees shows the dark hall behind."""
        frame = frame or shade(col, 0.8)
        self.box((x - w / 2 - 0.07, y - 0.03, 0), (x + w / 2 + 0.07, y, h + 0.07), frame)
        self.box((x - w / 2, y - 0.034, 0), (x + w / 2, y, h), BLACK)
        g = self.at(x - w / 2, y - 0.04, -ajar)
        self.box((0, -0.04, 0.01), (w, 0, h - 0.01), col, g)
        self.box((w - 0.12, -0.08, 1.0), (w - 0.04, -0.04, 1.03), STEEL, g)
        return g

    def picture(self, x, z, w, h, seed_colours, y=ROOM_D, frame=DARK_WOOD):
        """A framed picture on the back wall: bands of colour, no subject."""
        self.box((x - w / 2, y - 0.03, z - h / 2), (x + w / 2, y, z + h / 2), frame)
        a, b = seed_colours
        split = self.rng.uniform(0.35, 0.65)
        self.box((x - w / 2 + 0.04, y - 0.034, z - h / 2 + 0.04), (x + w / 2 - 0.04, y, z - h / 2 + 0.04 + (h - 0.08) * split), a)
        self.box((x - w / 2 + 0.04, y - 0.034, z - h / 2 + 0.04 + (h - 0.08) * split), (x + w / 2 - 0.04, y, z + h / 2 - 0.04), b)

    def rug(self, x0, x1, y0, y1, col, border=None):
        if border:
            self.box((x0, y0, 0), (x1, y1, 0.012), border)
            self.box((x0 + 0.12, y0 + 0.12, 0), (x1 - 0.12, y1 - 0.12, 0.016), col)
        else:
            self.box((x0, y0, 0), (x1, y1, 0.012), col)

    def radiator(self, y, side=-1, col=ENAMEL, w=0.9):
        """A ribbed radiator on a side wall."""
        x = side * (ROOM_W / 2 - 0.06)
        n = int(w / 0.09)
        for i in range(n):
            self.box((x - 0.05, y + i * 0.09, 0.15), (x + 0.05, y + i * 0.09 + 0.06, 0.75), col)

    # ------------------------------------------------------------ loose things
    def cartons(self, x, y, n, col=CARDBOARD, size=0.42, spread=0.25):
        """A loose stack of cardboard boxes."""
        rng = self.rng
        z = 0.0
        for i in range(n):
            s = size * rng.uniform(0.75, 1.1)
            hgt = s * rng.uniform(0.6, 0.9)
            cx, cy = x + rng.uniform(-spread, spread) * (0.3 if i else 0), y + rng.uniform(-spread, spread) * (0.3 if i else 0)
            self.box((cx - s / 2, cy - s / 2, z), (cx + s / 2, cy + s / 2, z + hgt), shade(col, rng.uniform(0.8, 1.1)),
                     rot=(0, 0, rng.uniform(-18, 18)))
            z += hgt

    def debris(self, x0, x1, y0, y1, n, colours=RUBBLE, big=0.32):
        """Broken plaster, brick and boards scattered over the floor."""
        rng = self.rng
        for _ in range(n):
            s = rng.uniform(0.06, big)
            cx, cy = rng.uniform(x0, x1), rng.uniform(y0, y1)
            self.box((cx - s / 2, cy - s * 0.35, 0), (cx + s / 2, cy + s * 0.35, s * rng.uniform(0.2, 0.6)),
                     shade(self.pick(colours), rng.uniform(0.7, 1.1)),
                     rot=(rng.uniform(-14, 14), rng.uniform(-14, 14), rng.uniform(0, 180)))

    def mound(self, x, y, r, col=RUBBLE[0]):
        """A heap of rubble."""
        rng = self.rng
        for _ in range(9):
            a, d = rng.uniform(0, math.tau), rng.uniform(0, r * 0.6)
            s = r * rng.uniform(0.5, 0.9)
            self.box((x + d * math.cos(a) - s / 2, y + d * math.sin(a) - s / 2, -s * 0.3),
                     (x + d * math.cos(a) + s / 2, y + d * math.sin(a) + s / 2, s * rng.uniform(0.25, 0.55) * (1 - d / r)),
                     shade(self.pick(RUBBLE), rng.uniform(0.7, 1.0)),
                     rot=(rng.uniform(-25, 25), rng.uniform(-25, 25), rng.uniform(0, 90)))

    def patches(self, col, n, wall="back", size=(0.3, 1.0), zr=(0.2, 2.8), tilt=1.0):
        """Flat patches on a wall: bare brick where plaster fell, or the pale
        ghosts of pictures and furniture."""
        rng = self.rng
        for _ in range(n):
            w, h = rng.uniform(*size), rng.uniform(*size)
            t = rng.uniform(0.006, 0.02)  # no two patches share a plane
            z = rng.uniform(zr[0], max(zr[0] + 0.01, zr[1] - h))
            if wall == "back":
                x = rng.uniform(-ROOM_W / 2, ROOM_W / 2 - w)
                self.box((x, ROOM_D - t, z), (x + w, ROOM_D, z + h), col, rot=(0, rng.uniform(-30, 30) * tilt, 0))
            else:
                side = -1 if wall == "left" else 1
                y = rng.uniform(0.2, ROOM_D - w)
                x = side * ROOM_W / 2
                self.box((min(x, x - side * t), y, z), (max(x, x - side * t), y + w, z + h), col,
                         rot=(rng.uniform(-30, 30) * tilt, 0, 0))

    def rail(self, x0, x1, y, z=1.65, garments=CLOTH, density=0.85):
        """A clothes rail along x with garments hanging edge-on."""
        rng = self.rng
        self.cyl(0.015, x1 - x0, ((x0 + x1) / 2, y, z), STEEL, axis="X", seg=8)
        for sx in (x0, x1):
            self.cyl(0.015, z, (sx, y, z / 2), STEEL, seg=8)
        x = x0 + 0.06
        while x < x1 - 0.08:
            if rng.random() < density:
                length = rng.uniform(0.6, 1.15)
                wd = rng.uniform(0.04, 0.07)
                self.box((x, y - 0.22, z - 0.06 - length), (x + wd, y + 0.22, z - 0.06), shade(self.pick(garments), rng.uniform(0.7, 1.1)))
                x += wd
            x += rng.uniform(0.015, 0.05)

    def dress_form(self, x, y, col):
        """A tailor's dummy: a torso on a pole."""
        g = self.at(x, y)
        self.cyl(0.17, 0.03, (0, 0, 0.015), DARK, g)
        self.cyl(0.015, 1.0, (0, 0, 0.5), STEEL, g, seg=8)
        self.cyl(0.15, 0.32, (0, 0, 1.12), col, g, r2=0.12)
        self.cyl(0.12, 0.3, (0, 0, 1.43), col, g, r2=0.18)
        self.box((-0.2, -0.09, 1.56), (0.2, 0.09, 1.62), col, g)
        self.cyl(0.045, 0.06, (0, 0, 1.65), PALE_WOOD, g, seg=8)
        return g

    def tyres(self, x, y, n):
        for i in range(n):
            self.cyl(0.31, 0.19, (x + self.rng.uniform(-0.03, 0.03), y, 0.1 + i * 0.2), (0.02, 0.02, 0.02), seg=20)

    def drum(self, x, y, col, h=0.88, r=0.29):
        self.cyl(r, h, (x, y, h / 2), col, seg=20, rough=0.6)
        for z in (h * 0.33, h * 0.66):
            self.cyl(r + 0.012, 0.03, (x, y, z), shade(col, 0.8), seg=20)

    def shutter(self, z_bottom, col=(0.02, 0.022, 0.022)):
        """A roller shutter in the shopfront, down as far as `z_bottom`. It hangs in
        the full daylight, so it is dark steel: a pale one would read as a lit panel."""
        ww, wh, sill = SHOPFRONT
        hw = ww / 2 + 0.04
        z = sill + wh + 0.04
        i = 0
        while z > z_bottom + 0.2:
            self.box((-hw, 0.03, z - 0.09), (hw, 0.06 if i % 2 else 0.075, z), shade(col, 1.0 if i % 2 else 0.7))
            z -= 0.09
            i += 1
        self.box((-hw, 0.02, z_bottom), (hw, 0.09, z), shade(col, 0.6))

    def pallet(self, x, y, loads, col=CARDBOARD, w=1.1, d=0.8):
        """A pallet with cartons stacked on it in courses."""
        rng = self.rng
        g = self.at(x, y, rng.uniform(-6, 6))
        for sy in (-d, -d / 2 - 0.05, -0.1):
            self.box((-w / 2, sy, 0), (w / 2, sy + 0.1, 0.1), PALE_WOOD, g)
        self.box((-w / 2, -d, 0.1), (w / 2, 0, 0.13), PALE_WOOD, g)
        z = 0.13
        for _ in range(loads):
            hgt = rng.uniform(0.28, 0.38)
            n = rng.randrange(2, 4)
            for i in range(n):
                if rng.random() < 0.88:
                    self.box((-w / 2 + i * w / n + 0.01, -d + 0.02, z), (-w / 2 + (i + 1) * w / n - 0.01, -0.02, z + hgt),
                             shade(col, rng.uniform(0.75, 1.1)), g)
            z += hgt

    def sacks(self, x, y, n, col=(0.33, 0.28, 0.18)):
        for i in range(n):
            self.box((x - 0.35, y - 0.22, i * 0.16), (x + 0.35, y + 0.22, i * 0.16 + 0.17), shade(col, self.rng.uniform(0.8, 1.05)),
                     rot=(0, 0, self.rng.uniform(-12, 12)))

    def ladder(self, x, y, lean=14.0, h=2.3):
        g = self.at(x, y, 0, 0, -lean)
        for sx in (-0.2, 0.17):
            self.box((sx, -0.03, 0), (sx + 0.03, 0.03, h), PALE_WOOD, g)
        for i in range(1, int(h / 0.28)):
            self.box((-0.2, -0.015, i * 0.28), (0.2, 0.015, i * 0.28 + 0.03), PALE_WOOD, g)

    def bucket(self, x, y, col=STEEL):
        self.cyl(0.13, 0.26, (x, y, 0.13), col, r2=0.16, rough=0.6)


# ---------------------------------------------------------------- apartments
ROOMS = []
SHOPS = []
APARTMENT_WINDOW = (1.3, 1.5, 0.9)  # the opening the daylight comes through: width, height, sill
SHOPFRONT = (2.6, 2.3, 0.35)


def room(walls, floor, ceiling=WHITE, window=APARTMENT_WINDOW, skirting=None, mottle=0.25, stripes=None, sheet=ROOMS):
    def wrap(fn):
        sheet.append({"name": fn.__name__, "build": fn, "walls": walls, "floor": floor, "ceiling": ceiling,
                      "window": window, "skirting": skirting, "mottle": mottle, "stripes": stripes})
        return fn

    return wrap


def shop(walls, floor, **kw):
    return room(walls, floor, window=SHOPFRONT, sheet=SHOPS, **kw)


@room(GREEN, PARQUET, skirting=WHITE)
def living_room(s):
    s.sofa(-0.35, 4.42, 1.9, (0.2, 0.17, 0.12))
    s.picture(-0.35, 1.75, 0.9, 0.6, ((0.1, 0.16, 0.22), (0.3, 0.26, 0.16)))
    s.shelving(1.05, 4.5, 0.8, 2.0, fill="books", col=DARK_WOOD)
    s.table(-0.3, 3.0, 1.0, 0.55, WOOD, h=0.42)
    s.rug(-1.2, 0.7, 1.6, 3.3, (0.4, 0.14, 0.1), border=(0.2, 0.15, 0.1))
    s.cabinet(-1.22, 1.6, 0.9, 0.7, 0.4, WOOD, yaw=90, cols=2)
    s.curtains((0.11, 0.085, 0.04), left=0.5, right=0.42)


@room(BLUE, BOARDS, skirting=WHITE)
def bedroom(s):
    s.bed(-0.45, 4.45, 1.5, (0.3, 0.1, 0.08))
    s.cabinet(0.55, 4.5, 0.42, 0.5, 0.4, WOOD, rows=2)
    s.cabinet(1.1, 4.5, 0.8, 2.1, 0.58, PALE_WOOD, cols=2)
    s.picture(-0.45, 1.7, 0.6, 0.45, ((0.25, 0.25, 0.2), (0.1, 0.12, 0.1)))
    s.rug(-1.3, 0.3, 1.2, 2.3, (0.42, 0.38, 0.3))
    s.chair(1.05, 1.9, DARK_WOOD, yaw=-60)
    s.box((0.9, 1.5, 0.47), (1.25, 1.85, 0.56), (0.1, 0.12, 0.2), rot=(0, 0, 20))  # clothes left on the chair


@room(CREAM, TILE, skirting=None)
def kitchen(s):
    s.cabinet(-0.6, 4.5, 1.8, 0.88, 0.6, WHITE, cols=3, top=(0.12, 0.11, 0.1), plinth=0.1)
    s.cabinet(-0.6, 4.5, 1.8, 0.7, 0.34, WHITE, z=1.45, cols=3)
    s.box((-1.5, 4.49, 0.92), (0.3, 4.5, 1.45), (0.36, 0.4, 0.38))  # tiled splashback
    s.cabinet(0.95, 4.5, 0.62, 1.75, 0.62, ENAMEL, rows=2)  # fridge
    s.box((-0.95, 3.93, 0.915), (-0.45, 4.43, 0.93), BLACK)  # hob
    s.cyl(0.11, 0.16, (-0.7, 4.2, 1.0), STEEL, rough=0.5)  # a pan left on it
    s.shelving(-1.34, 2.9, 0.9, 0.5, d=0.2, shelves=2, fill="bottles", col=WOOD, yaw=90, z=1.4, back=False)
    s.cabinet(1.48, 2.3, 0.6, 0.85, 0.6, ENAMEL, yaw=-90, handles=False)  # a washing machine
    g = s.at(-0.2, 2.6, 12)  # a clothes airer, the washing still on it
    for sx in (-0.5, 0.47):
        s.box((sx, -0.6, 0), (sx + 0.03, -0.57, 0.95), STEEL, g, rot=(-14, 0, 0))
        s.box((sx, -0.03, 0), (sx + 0.03, 0, 0.95), STEEL, g, rot=(14, 0, 0))
    for i in range(5):
        y = -0.52 + i * 0.11
        s.box((-0.5, y, 0.93), (0.5, y + 0.012, 0.942), STEEL, g)
        if i != 2:
            x = s.rng.uniform(-0.45, 0.0)
            s.box((x, y - 0.012, 0.42 + s.rng.uniform(0, 0.25)), (x + s.rng.uniform(0.3, 0.45), y + 0.024, 0.95), s.pick(CLOTH), g)
    s.bucket(0.75, 1.5, (0.1, 0.14, 0.25))


@room(WHITE, PARQUET, skirting=PALE_WOOD, mottle=0.3)
def empty_room(s):
    s.door(0.75, ENAMEL, ajar=28)
    s.patches(shade(WHITE, 0.94), 3, "back", size=(0.4, 0.8), zr=(1.1, 2.4), tilt=0)  # where the pictures hung
    s.box((-1.4, 4.497, 0.08), (-0.2, 4.5, 2.0), shade(WHITE, 0.92))  # and where the wardrobe stood
    s.cartons(-0.9, 3.4, 2)
    s.cyl(0.11, 1.5, (-0.3, 4.1, 0.11), (0.22, 0.1, 0.08), axis="X")  # a rolled carpet
    s.radiator(1.4, side=1)
    s.curtains(None, left=0, right=0)  # the bare rod


@room(CONCRETE, SCREED, ceiling=CONCRETE, mottle=0.5)
def stripped_room(s):
    s.patches(BRICK, 5, "back", size=(0.5, 1.4), zr=(0.0, 3.0))
    s.patches(BRICK, 2, "left", size=(0.5, 1.3))
    s.patches(PLASTER, 4, "back", size=(0.3, 0.9))
    s.patches(PLASTER, 2, "right", size=(0.4, 1.0))
    s.box((0.3, 4.47, 0), (1.2, 4.5, 2.05), BLACK)  # a doorway with no door
    s.box((-0.1, 4.2, 0.02), (0.75, 4.26, 2.05), (0.16, 0.14, 0.11), rot=(-13, 0, 6))  # the door, off, against the wall
    for i in range(4):  # joists where the ceiling came down
        s.box((-1.5, 0.6 + i * 1.05, 2.86), (1.5, 0.72 + i * 1.05, 3.0), DARK_WOOD)
    s.mound(-0.8, 3.4, 0.7)
    s.debris(-1.4, 1.4, 0.4, 4.2, 26)
    s.box((-1.35, 1.2, 0.0), (-0.45, 3.0, 0.16), (0.3, 0.27, 0.2), rot=(0, 0, 8))  # a mattress


@room(BEIGE, CARPET, skirting=DARK_WOOD)
def sitting_room(s):
    s.door(1.0, DARK_WOOD, ajar=0)
    s.cabinet(-0.55, 4.5, 1.6, 0.6, 0.45, DARK_WOOD, cols=3, plinth=0.06)
    s.box((-1.0, 4.2, 0.62), (-0.1, 4.27, 1.15), BLACK, rough=0.4)  # a television, off
    s.box((-1.35, 4.47, 1.5), (0.25, 4.5, 1.56), DARK_WOOD)  # a shelf of ornaments over it
    for _ in range(6):
        x, k = s.rng.uniform(-1.3, 0.1), s.rng.uniform(0.06, 0.14)
        s.box((x, 4.4, 1.56), (x + k, 4.48, 1.56 + k * s.rng.uniform(1.0, 2.2)), s.pick(GOODS))
    s.sofa(-0.85, 2.1, 0.95, (0.1, 0.13, 0.1), yaw=150)
    s.sofa(0.8, 2.2, 0.95, (0.1, 0.13, 0.1), yaw=-150)
    s.round_table(0.0, 3.45, 0.32, WOOD, h=0.5)
    s.rug(-0.9, 0.9, 2.4, 3.9, (0.3, 0.26, 0.18), border=(0.42, 0.36, 0.22))
    s.curtains((0.08, 0.02, 0.02), left=0, right=0.75, z0=0.75)


@room(YELLOW, LINO, skirting=WOOD, mottle=0.35)
def box_room(s):
    s.bed(1.45, 3.9, 0.9, (0.12, 0.2, 0.16), yaw=-90, frame=STEEL)  # along the back wall
    s.picture(0.5, 1.7, 0.7, 0.45, ((0.3, 0.2, 0.1), (0.14, 0.2, 0.22)))
    s.cabinet(-1.05, 4.5, 0.8, 1.1, 0.5, WOOD, rows=4)
    s.box((-1.3, 4.1, 1.1), (-0.95, 4.4, 1.35), (0.1, 0.1, 0.1))  # a radio, silent
    s.table(-1.48, 2.3, 1.2, 0.6, PALE_WOOD, yaw=90)
    s.chair(-0.45, 2.3, DARK_WOOD, yaw=-80)
    s.shelving(-1.48, 2.3, 1.0, 0.6, d=0.22, shelves=2, fill="books", col=PALE_WOOD, yaw=90, z=1.3, back=False)
    s.box((-1.3, 1.95, 0.74), (-0.95, 2.3, 0.78), (0.3, 0.3, 0.27), rot=(0, 0, 12))  # papers on the desk
    s.cartons(0.9, 2.2, 2, size=0.4)
    s.curtains((0.085, 0.085, 0.08), left=0.6, right=0.3, z0=0.8)


@room(MINT, TILE, skirting=None, mottle=0.3)
def dining_kitchen(s):
    s.cabinet(0.45, 4.5, 2.1, 0.88, 0.6, (0.14, 0.2, 0.17), cols=4, top=PALE_WOOD, plinth=0.1)
    s.cabinet(0.75, 4.5, 1.5, 0.6, 0.34, (0.14, 0.2, 0.17), z=1.5, cols=3)
    s.box((0.0, 3.95, 1.75), (0.6, 4.5, 1.95), STEEL)  # a hood over the stove
    s.box((-0.02, 3.9, 0.0), (0.62, 4.5, 0.9), ENAMEL)  # the stove
    s.box((0.02, 3.895, 0.2), (0.58, 3.9, 0.62), BLACK, rough=0.4)
    s.cabinet(-1.1, 4.5, 0.7, 1.9, 0.5, PALE_WOOD, cols=2, rows=2)  # a dresser
    s.table(-0.25, 2.9, 1.5, 0.85, WOOD)
    for x, y, yaw in ((-0.7, 3.1, 5), (0.15, 3.1, -8), (-0.7, 1.9, 175), (0.2, 1.75, 200)):
        s.chair(x, y, WOOD, yaw=yaw)
    s.box((-0.6, 2.3, 0.74), (0.1, 2.7, 0.745), (0.35, 0.33, 0.28))  # a cloth
    s.cyl(0.12, 0.09, (-0.25, 2.5, 0.79), (0.2, 0.22, 0.3))


@room(PINK, BOARDS, skirting=WHITE, stripes=(shade(PINK, 0.8), 0.24))
def childrens_room(s):
    g = s.at(-0.95, 4.45)  # a bunk bed
    for sx in (-0.5, 0.45):
        for sy in (-1.95, -0.05):
            s.box((sx, sy, 0), (sx + 0.05, sy + 0.05, 1.75), PALE_WOOD, g)
    for z, blanket in ((0.3, (0.08, 0.14, 0.3)), (1.25, (0.32, 0.25, 0.06))):
        s.box((-0.5, -1.95, z), (0.5, 0, z + 0.1), PALE_WOOD, g)
        s.box((-0.46, -1.93, z + 0.1), (0.46, -0.02, z + 0.24), blanket, g)
        s.box((-0.5, -1.97, z + 0.1), (0.5, -1.95, z + 0.38), PALE_WOOD, g)
    s.ladder(-0.95, 2.4, lean=5, h=1.7)
    s.shelving(0.9, 4.5, 1.0, 1.4, fill="goods", col=WHITE, shelves=3, density=0.7)
    s.picture(0.9, 2.0, 0.55, 0.4, ((0.12, 0.2, 0.3), (0.34, 0.3, 0.12)))
    s.table(0.9, 2.7, 0.8, 0.5, (0.3, 0.12, 0.08), h=0.5)
    s.stool(0.5, 2.1, (0.1, 0.2, 0.3), h=0.3)
    s.rug(-0.2, 1.3, 1.0, 2.6, (0.16, 0.3, 0.42))
    for _ in range(7):  # toys left where they fell
        x, y, k = s.rng.uniform(-0.6, 1.2), s.rng.uniform(0.7, 2.4), s.rng.uniform(0.08, 0.16)
        s.box((x, y, 0.012), (x + k, y + k, 0.012 + k), s.pick(GOODS), rot=(0, 0, s.rng.uniform(0, 90)))
    s.curtains((0.028, 0.045, 0.09), left=0.4, right=0.55)


@room((0.36, 0.3, 0.17), PARQUET, skirting=DARK_WOOD, mottle=0.5, ceiling=PLASTER, stripes=((0.27, 0.24, 0.15), 0.3))
def abandoned_room(s):
    s.box((-1.25, 4.47, 0), (-0.4, 4.5, 2.05), BLACK)  # the hall, the door gone
    s.box((-1.32, 4.47, 0), (-1.25, 4.5, 2.1), DARK_WOOD)
    s.box((-0.4, 4.47, 0), (-0.33, 4.5, 2.1), DARK_WOOD)
    s.patches((0.42, 0.36, 0.22), 2, "back", size=(0.4, 0.7), zr=(1.2, 2.3), tilt=0)  # where pictures hung
    s.patches((0.2, 0.17, 0.1), 2, "right", size=(0.5, 1.2))  # damp
    s.sofa(0.55, 3.3, 1.7, (0.14, 0.12, 0.1), yaw=14, roll=-96, z=0.1)  # tipped onto its back
    s.shelving(1.2, 4.3, 0.8, 1.9, fill="books", col=WOOD, density=0.2, roll=7, yaw=-6)
    s.table(-0.6, 2.2, 0.9, 0.6, WOOD, h=0.45, yaw=25)
    s.chair(-0.9, 1.4, DARK_WOOD, yaw=40, roll=-90)  # a chair on its back
    s.debris(-1.3, 1.3, 0.5, 4.0, 14, colours=((0.36, 0.35, 0.3), (0.3, 0.2, 0.11), (0.2, 0.05, 0.04), (0.1, 0.1, 0.1)), big=0.24)
    s.curtain(-0.95, -0.55, 1.2, 2.55, (0.085, 0.08, 0.065))  # one curtain left, torn short
    s.cyl(0.012, 2.0, (0, 0.16, 2.58), DARK_WOOD, axis="X", seg=8)


# ---------------------------------------------------------------- shops
@shop(WHITE, TILE, mottle=0.3)
def grocery(s):
    s.shelving(-0.5, 4.5, 1.9, 2.2, d=0.36, fill="goods", col=ENAMEL, shelves=6)
    s.shelving(1.15, 4.5, 0.65, 2.2, d=0.36, fill="bottles", col=ENAMEL, shelves=6)
    s.shelving(-0.55, 2.9, 1.5, 1.3, d=0.4, fill="goods", col=ENAMEL, shelves=4, back=True)
    s.counter(0.95, 1.9, 0.9, (0.14, 0.2, 0.14))
    s.box((0.75, 1.5, 0.985), (1.1, 1.8, 1.2), STEEL)  # scales
    for i, col in enumerate(((0.3, 0.07, 0.04), (0.33, 0.25, 0.05), (0.1, 0.22, 0.07))):  # crates of produce
        s.box((-1.35 + i * 0.5, 0.9, 0.0), (-0.9 + i * 0.5, 1.3, 0.25), PALE_WOOD)
        s.box((-1.32 + i * 0.5, 0.93, 0.25), (-0.93 + i * 0.5, 1.27, 0.31), col)


@shop(CREAM, PARQUET, skirting=DARK_WOOD)
def clothes_shop(s):
    s.rail(-1.35, 0.2, 4.2)
    s.shelving(0.95, 4.5, 1.0, 2.1, d=0.4, fill="folded", col=PALE_WOOD, shelves=5)
    s.rail(-1.3, -0.1, 2.6, z=1.4, density=0.7)
    s.dress_form(-0.85, 1.0, (0.25, 0.06, 0.05))
    s.dress_form(0.1, 1.3, (0.06, 0.09, 0.16))
    s.counter(1.0, 2.6, 0.8, DARK_WOOD, top=WOOD)
    s.rug(-0.6, 0.7, 1.6, 3.4, (0.36, 0.2, 0.14))


@shop(GREY, SCREED, ceiling=GREY, mottle=0.5)
def workshop(s):
    s.table(-0.55, 4.45, 1.8, 0.7, WOOD, h=0.9, leg=0.09)
    s.box((-1.45, 4.47, 1.05), (0.35, 4.5, 2.1), (0.2, 0.15, 0.09))  # a tool board
    for _ in range(16):  # the tools on it
        x, z = s.rng.uniform(-1.35, 0.2), s.rng.uniform(1.12, 1.95)
        w, h = s.rng.choice(((0.04, 0.3), (0.25, 0.05), (0.1, 0.12), (0.03, 0.2)))
        s.box((x, 4.455, z), (x + w, 4.47, min(z + h, 2.08)), s.pick((STEEL, BLACK, (0.3, 0.06, 0.04), DARK_WOOD)))
    s.box((-1.2, 3.85, 0.9), (-0.9, 4.1, 1.08), (0.08, 0.12, 0.2))  # a vice
    s.box((-0.3, 3.9, 0.9), (0.2, 4.3, 1.1), (0.3, 0.06, 0.04))  # a tool chest
    s.shelving(1.1, 4.5, 0.75, 2.1, d=0.4, fill="bins", col=STEEL, shelves=5)
    s.tyres(1.05, 2.8, 4)
    s.tyres(0.95, 2.1, 2)
    s.drum(-1.1, 2.6, (0.06, 0.1, 0.18))
    s.drum(-1.15, 1.95, (0.25, 0.07, 0.04))
    s.box((-0.4, 1.7, 0.0), (0.4, 2.9, 0.012), (0.03, 0.03, 0.03))  # an oil stain
    s.box((-0.25, 2.0, 0.012), (0.3, 2.7, 0.3), (0.12, 0.12, 0.13))  # an engine block on the floor
    s.cyl(0.1, 0.3, (0.05, 2.35, 0.42), STEEL, rough=0.5)


@shop(YELLOW, TILE, skirting=DARK_WOOD, mottle=0.3)
def cafe(s):
    s.counter(-0.5, 3.7, 1.8, DARK_WOOD, top=(0.1, 0.1, 0.1), h=1.05)
    s.shelving(-0.5, 4.5, 1.8, 1.0, d=0.22, fill="bottles", col=DARK_WOOD, shelves=3, z=1.15)
    s.box((-1.2, 3.25, 1.085), (-0.8, 3.6, 1.45), (0.07, 0.07, 0.075), rough=0.5)  # a coffee machine, cold
    s.door(1.0, DARK_WOOD, ajar=0)
    for x, y in ((-0.75, 1.9), (0.55, 2.3)):
        s.round_table(x, y, 0.36, WOOD)
        for k, yaw in enumerate((20, 200)):  # chairs upturned on the tables
            s.chair(x + (0.12 if k else -0.12), y - 0.02 + (0.2 if k else -0.2), PALE_WOOD, yaw=yaw, z=0.74 + 0.47, roll=180)
    for i in range(5):  # and the rest stacked
        s.chair(1.15, 1.35, PALE_WOOD, yaw=-80, z=i * 0.13)
    s.stool(-1.15, 2.9, DARK_WOOD, h=0.75)
    s.stool(-0.2, 3.0, DARK_WOOD, h=0.75)


@shop(WHITE, SCREED, mottle=0.45)
def empty_unit(s):
    s.shutter(1.45)
    s.door(-0.9, GREY, ajar=0)
    s.cartons(0.9, 3.7, 3)
    s.cartons(0.3, 3.9, 1)
    s.ladder(1.2, 3.9)
    s.bucket(0.6, 2.6)
    s.bucket(0.25, 2.75, (0.4, 0.4, 0.38))
    s.patches(shade(WHITE, 0.75), 3, "back", size=(0.5, 1.1), zr=(0.4, 2.6), tilt=0)  # where the fittings were
    s.box((-1.3, 1.0, 0.0), (0.2, 2.6, 0.008), (0.25, 0.24, 0.22), rot=(0, 0, 9))  # a dust sheet
    s.debris(-1.2, 1.2, 0.6, 3.6, 8, big=0.14)


@shop(CREAM, TILE, skirting=None)
def bakery(s):
    s.counter(-0.2, 2.9, 2.3, WHITE, top=PALE_WOOD, h=1.0, d=0.7)
    s.shelving(-0.2, 2.85, 2.2, 0.4, d=0.5, shelves=1, fill="loaves", col=STEEL, z=1.035, back=False)  # the display on it
    for i in range(4):  # racks on the back wall
        g = s.at(-0.35, 4.5, z=0.7 + i * 0.42)
        s.box((-1.05, -0.4, 0), (1.05, 0, 0.03), PALE_WOOD, g)
        s._fill(g, -1.0, 1.0, 0.03, 0.3, 0.4, "loaves", 0.85)
    s.cabinet(1.15, 4.5, 0.6, 2.0, 0.6, STEEL, rows=2)  # an oven
    s.box((0.88, 3.895, 1.1), (1.42, 3.9, 1.8), BLACK, rough=0.4)
    for i in range(3):  # stacked trays
        s.box((0.85, 1.2, 0.0 + i * 0.16), (1.35, 1.9, 0.14 + i * 0.16), (0.3, 0.12, 0.05) if i % 2 else (0.2, 0.2, 0.2))
    s.sacks(-1.1, 3.9, 3)


@shop(GREEN, LINO, skirting=DARK_WOOD)
def ironmonger(s):
    s.cabinet(-0.3, 4.5, 2.3, 2.3, 0.4, WOOD, cols=10, rows=11, plinth=0.1)  # a wall of small drawers
    s.shelving(1.2, 4.5, 0.55, 2.3, d=0.4, fill="bins", col=WOOD, shelves=6)
    s.counter(-0.2, 3.0, 2.2, DARK_WOOD, top=WOOD)
    s.box((0.5, 2.5, 0.985), (0.85, 2.85, 1.25), DARK)  # a till, no figures
    s.shelving(-1.32, 1.9, 1.2, 1.8, d=0.3, fill="goods", col=WOOD, yaw=90, shelves=5)
    for i in range(4):  # brooms and spades by the door
        s.box((0.9 + i * 0.12, 1.0, 0.0), (0.93 + i * 0.12, 1.03, 1.5), PALE_WOOD, rot=(0, 6 - i * 3, 0))
    s.bucket(1.0, 1.5)
    s.bucket(1.2, 1.9, (0.1, 0.14, 0.25))


@shop(BEIGE, BOARDS, skirting=DARK_WOOD)
def bookshop(s):
    s.shelving(-0.75, 4.5, 1.4, 2.5, fill="books", col=DARK_WOOD, shelves=7, density=0.95)
    s.shelving(0.75, 4.5, 1.4, 2.5, fill="books", col=DARK_WOOD, shelves=7, density=0.95)
    s.shelving(-1.33, 2.6, 1.6, 2.2, fill="books", col=DARK_WOOD, shelves=6, yaw=90)
    s.table(0.25, 2.9, 1.4, 0.8, WOOD)
    for _ in range(7):  # stacks on the table
        x, y = s.rng.uniform(-0.35, 0.7), s.rng.uniform(2.2, 2.75)
        s.box((x, y, 0.74), (x + 0.2, y + 0.14, 0.74 + s.rng.uniform(0.04, 0.25)), s.pick(BOOKS), rot=(0, 0, s.rng.uniform(-20, 20)))
    s.cartons(1.15, 1.4, 2, size=0.36)
    s.stool(-0.6, 1.5, WOOD)


@shop(BLUE, TILE, skirting=None, mottle=0.3)
def barber(s):
    for x in (-0.75, 0.45):
        s.box((x - 0.4, 4.46, 1.0), (x + 0.4, 4.48, 2.0), (0.05, 0.06, 0.07), rough=0.5)  # mirrors, showing only the dark room
        s.box((x - 0.44, 4.475, 0.96), (x + 0.44, 4.5, 2.04), PALE_WOOD)
        s.box((x - 0.45, 4.25, 0.86), (x + 0.45, 4.5, 0.9), WHITE)
        g = s.at(x, 3.3, 180 + (x * 20))  # a barber's chair
        s.cyl(0.26, 0.04, (0, -0.25, 0.02), STEEL, g, rough=0.5)
        s.cyl(0.05, 0.4, (0, -0.25, 0.22), STEEL, g, rough=0.5, seg=8)
        s.box((-0.27, -0.52, 0.42), (0.27, 0.0, 0.54), (0.2, 0.04, 0.04), g)
        s.box((-0.25, -0.06, 0.54), (0.25, 0.04, 1.15), (0.2, 0.04, 0.04), g)
        for sx in (-0.33, 0.27):
            s.box((sx, -0.5, 0.54), (sx + 0.06, 0.0, 0.72), STEEL, g)
    s.cabinet(1.2, 4.5, 0.5, 0.9, 0.5, WHITE, top=ENAMEL)  # a basin
    s.box((-1.45, 1.2, 0.0), (-1.0, 2.9, 0.45), (0.1, 0.1, 0.1))  # a bench to wait on
    s.box((-1.5, 1.2, 0.45), (-1.4, 2.9, 0.9), (0.1, 0.1, 0.1))
    s.shelving(1.36, 2.3, 0.9, 0.5, d=0.2, shelves=2, fill="bottles", col=WHITE, yaw=-90, z=1.2, back=False)
    s.picture(-0.15, 2.45, 0.3, 0.3, ((0.2, 0.2, 0.2), (0.3, 0.28, 0.22)))


@shop(CONCRETE, SCREED, ceiling=CONCRETE, mottle=0.45)
def storeroom(s):
    s.pallet(-0.75, 4.3, 4)
    s.pallet(0.6, 4.35, 3)
    s.pallet(-0.6, 2.9, 2, col=(0.33, 0.3, 0.24))
    s.sacks(0.9, 2.6, 4)
    s.sacks(1.0, 1.7, 2)
    s.shelving(-1.3, 1.9, 1.3, 2.0, d=0.4, fill="bins", col=STEEL, yaw=90, shelves=4, density=0.5)
    g = s.at(0.2, 1.7, 20)  # a sack truck
    s.box((-0.2, -0.02, 0.1), (0.2, 0.02, 1.2), (0.3, 0.06, 0.04), g, rot=(-12, 0, 0))
    s.box((-0.2, -0.3, 0.0), (0.2, 0.0, 0.03), (0.3, 0.06, 0.04), g)
    for sx in (-0.26, 0.2):
        s.cyl(0.1, 0.06, (sx + 0.03, 0.08, 0.1), BLACK, g, axis="X")
    s.debris(-1.0, 1.2, 0.5, 2.0, 6, colours=(CARDBOARD, (0.3, 0.3, 0.28)), big=0.2)


# ---------------------------------------------------------------- stage, render, sheet
def stage(spec):
    """The room box, its window wall and the sky in the window, then the room's own things."""
    common.reset()
    hw, t = ROOM_W / 2, WALL_T
    s = Set(zlib.crc32(spec["name"].encode()))
    walls = paint(spec["walls"], mottle=spec["mottle"])
    back = paint(spec["walls"], mottle=spec["mottle"], stripes=spec["stripes"]) if spec["stripes"] else walls
    s.box((-hw - t, -t, -t), (hw + t, ROOM_D + t, 0), None, material=paint(spec["floor"], 0.8, mottle=spec["mottle"] * 0.6))
    s.box((-hw - t, -t, ROOM_H), (hw + t, ROOM_D + t, ROOM_H + t), None, material=paint(spec["ceiling"], mottle=spec["mottle"] * 0.5))
    s.box((-hw - t, -t, 0), (-hw, ROOM_D + t, ROOM_H), None, material=walls)
    s.box((hw, -t, 0), (hw + t, ROOM_D + t, ROOM_H), None, material=walls)
    s.box((-hw, ROOM_D, 0), (hw, ROOM_D + t, ROOM_H), None, material=back)
    if spec["skirting"]:
        k = spec["skirting"]
        s.box((-hw, ROOM_D - 0.02, 0), (hw, ROOM_D, 0.1), k)
        s.box((-hw, 0, 0), (-hw + 0.02, ROOM_D, 0.1), k)
        s.box((hw - 0.02, 0, 0), (hw, ROOM_D, 0.1), k)
    # The window wall: the viewer looks through it, so the camera does not see
    # it, but it shapes the light and returns the room's own bounce.
    ww, wh, sill = spec["window"]
    for lo, hi in (((-hw, -t, 0), (-ww / 2, 0, ROOM_H)), ((ww / 2, -t, 0), (hw, 0, ROOM_H)),
                   ((-ww / 2, -t, 0), (ww / 2, 0, sill)), ((-ww / 2, -t, sill + wh), (ww / 2, 0, ROOM_H))):
        s.box(lo, hi, None, material=walls).visible_camera = False
    sky = bpy.data.lights.new("sky", "AREA")
    sky.shape = "RECTANGLE"
    sky.size, sky.size_y = ww, wh
    sky.energy = SKY_W
    sky.color = (0.9, 0.95, 1.0)
    lamp = bpy.data.objects.new("sky", sky)
    lamp.location = (0, -t - 0.01, sill + wh / 2)
    lamp.rotation_euler = (math.pi / 2, 0, 0)
    lamp.visible_camera = False
    bpy.context.scene.collection.objects.link(lamp)
    spec["build"](s)

    cam = bpy.data.cameras.new("eye")
    cam.sensor_fit = "HORIZONTAL"
    cam.angle = 2 * math.atan(ROOM_W / 2 / EYE_M)
    cam.clip_start, cam.clip_end = 1.0, 100.0
    eye = bpy.data.objects.new("eye", cam)
    eye.location = (0, -EYE_M, ROOM_H / 2)
    eye.rotation_euler = (math.pi / 2, 0, 0)
    bpy.context.scene.collection.objects.link(eye)
    bpy.context.scene.camera = eye


def render(tmp):
    """Render the staged room: linear radiance, (size, size, 3), top row first."""
    scn = bpy.context.scene
    size = CELL * SUPERSAMPLE
    scn.render.engine = "CYCLES"
    scn.cycles.device = "CPU"
    scn.cycles.samples = SAMPLES
    scn.cycles.use_adaptive_sampling = False
    scn.cycles.use_denoising = False
    scn.cycles.seed = 0
    scn.cycles.max_bounces = 12
    scn.cycles.diffuse_bounces = 12
    scn.cycles.caustics_reflective = False
    scn.cycles.caustics_refractive = False
    scn.render.resolution_x = scn.render.resolution_y = size
    scn.render.resolution_percentage = 100
    scn.render.pixel_aspect_x = scn.render.pixel_aspect_y = 1.0
    scn.world = bpy.data.worlds.new("dark")  # no light but the window's
    scn.world.use_nodes = True
    scn.world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.0
    scn.render.image_settings.file_format = "OPEN_EXR"
    scn.render.image_settings.color_depth = "32"
    scn.render.filepath = os.path.join(tmp, "cell.exr")
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(scn.render.filepath)
    px = np.empty(size * size * 4, dtype=np.float32)
    img.pixels.foreach_get(px)
    return px.reshape(size, size, 4)[::-1, :, :3].astype(np.float64)


def tone(radiance):
    """Linear radiance to linear display colour: exposure under a soft ceiling."""
    x = np.maximum(radiance * EXPOSURE, 0.0)
    return CEILING * (1.0 - np.exp(-x / CEILING))


def box_down(img, k):
    h, w, c = img.shape
    return img.reshape(h // k, k, w // k, k, c).mean(axis=(1, 3))


def encode(linear):
    rgb = textures._u8(textures._srgb(linear))
    return textures.png(np.concatenate([rgb, np.full(rgb.shape[:2] + (1,), 255, np.uint8)], -1))


def luminance(linear):
    return linear @ np.array((0.2126, 0.7152, 0.0722))


def cell(spec, tmp):
    """One room's picture at render size: linear display colour, top row first."""
    stage(spec)
    return tone(render(tmp))


def sheet(name, specs, out, tmp):
    """Render one atlas: cell i at column i % COLS, row i // COLS, from the top-left."""
    if len(specs) != COLS * ROWS:
        raise SystemExit(f"{name}: {len(specs)} cells, the atlas holds {COLS * ROWS}")
    atlas = np.zeros((ROWS * CELL, COLS * CELL, 3))
    for i, spec in enumerate(specs):
        r, c = divmod(i, COLS)
        small = box_down(cell(spec, tmp), SUPERSAMPLE)
        atlas[r * CELL:(r + 1) * CELL, c * CELL:(c + 1) * CELL] = small
        print(f"CELL {name} {i} {spec['name']}: mean {textures._srgb(luminance(small)).mean():.3f} sRGB, "
              f"brightest {textures._srgb(small).max():.3f}")
    path = os.path.join(out, f"{name}.png")
    data = encode(atlas)
    open(path, "wb").write(data)
    lum = luminance(atlas)
    print(f"SHEET {path}: {atlas.shape[1]}x{atlas.shape[0]} px, {len(data)} bytes, mean luminance "
          f"{lum.mean():.4f} linear ({textures._srgb(lum).mean():.3f} sRGB), brightest pixel {textures._srgb(atlas).max():.3f} sRGB")


def main():
    args = common.script_args()
    out = os.path.join(common.REPO, "assets/source/city/interiors")
    if "--out" in args:
        out = os.path.abspath(args[args.index("--out") + 1])
    os.makedirs(out, exist_ok=True)
    tmp = tempfile.mkdtemp(prefix="interiors-")
    try:
        sheet("rooms", ROOMS, out, tmp)
        sheet("shops", SHOPS, out, tmp)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
