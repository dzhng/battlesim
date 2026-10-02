"""The facade lab's kit: the fixture the renderer's cutout, glass and room surfaces are judged on.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/facade_lab.py

writes `assets/source/city/facade_lab/kit.glb`: a kit with no templates. No map places it;
the lab route `/lab/facade` stands its modules on a plain ground. It is the worked example of
the three things a facade material can say beyond "opaque" (`textures.surface`):

- a cutout (`coverage=("cutout", cutoff)` on a recipe with a coverage image): the grille and
  the perforated sheet;
- glass (`coverage=("blended", opacity)`): one face (`sheet`), never a thin box;
- a room (`room`, `room_box`): the open box behind a window, 2.9 m wide and tall so that the
  boxes of neighbouring bays never share a wall's plane.

`facade_lab_render.py` photographs the kit in Blender where the lab stands it: the picture the
lab's frames are compared with.
"""
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
from parts import *  # noqa: E402,F401,F403

REPO = os.path.abspath(os.path.join(HERE, "../../../.."))
OUT = os.path.join(REPO, "assets", "source", "city", "facade_lab", "kit.glb")
BAY_M, FLOOR_M, WALL_M = 3.0, 3.0, 0.3  # a bay's width, a floor's height, the wall's thickness
ROOM_M, ROOM_DEPTH_M = 2.9, 4.5  # a room box's width and height, and its depth
SILL_M, WINDOW_W, WINDOW_H = 0.95, 1.4, 1.5
SHOP_W, SHOP_H, SHOP_SILL_M = 2.4, 2.4, 0.25
PANEL_W, PANEL_H = 3.0, 2.0
BLOCK_BAYS, BLOCK_FLOORS = 3, 3
BAKE = dict(ao_distance=0.4, ao_strength=0.5, ao_rays=10, paint_scale=8.0)

reset()
plaster_m = textured("wall_plaster", "plaster", tint=1.0, dirt=0.5, chip=0.5, streak=0.3, rise=0.8)
concrete_m = textured("trim_concrete", "concrete", dirt=0.0, chip=0.3, streak=0.2)
frame_m = flat_paint("window_frame", (0.46, 0.45, 0.42), rough=0.6, grime=0.0)
steel_m = flat_paint("frame_steel", (0.05, 0.055, 0.05), rough=0.5, grime=0.0)
glass_m = flat_paint("window_glass", (0.02, 0.025, 0.03), rough=0.08, grime=0.0, coverage=("blended", 0.5))
grille_m = textured("grille_steel", "grille", dirt=0.0, chip=0.6, streak=0.0, coverage=("cutout", 0.5))
sheet_m = textured("sheet_perforated", "perforated", dirt=0.3, chip=0.4, streak=0.3, coverage=("cutout", 0.5))
rooms_m = room("room_rooms", "rooms")
shops_m = room("room_shops", "shops")

MODULES = {}  # name -> (root, stands on the ground)


def module(name, ground=False):
    """A module's root, ten metres along +X from the last: a whole number of every recipe's tile."""
    root = empty(name, loc=(10.0 * len(MODULES), 0, 0))
    MODULES[name] = (root, ground)
    return root, lambda part: f"{name}_{part}"


def framed(n, root, w, h, z, y, mat, bar=0.06):
    """A rectangular frame of four bars round an opening `w` by `h` whose foot is at `z`."""
    for s in (-1, 1):
        box(n(f"stile_{'ab'[s > 0]}"), (bar, bar, h + 2 * bar), (s * (w + bar) / 2, y, z + h / 2), mat, root)
    box(n("foot"), (w, bar, bar), (0, y, z - bar / 2), mat, root)
    box(n("head"), (w, bar, bar), (0, y, z + h + bar / 2), mat, root)


# ---------------------------------------------------------------- cutouts
# A fence panel of each cutout, standing on the ground so its shadow falls where it can be read.
for name, mat in (("grille_panel", grille_m), ("sheet_panel", sheet_m)):
    root, n = module(name, ground=True)
    framed(n, root, PANEL_W, PANEL_H, 0.1, 0.0, steel_m)
    sheet(n("cutout"), PANEL_W, PANEL_H, (0, 0, 0.1), mat, root)

# A window guard: the grille over a window's opening, a hand's breadth proud of the wall.
root, n = module("window_grille")
framed(n, root, WINDOW_W, WINDOW_H, 0.0, -0.05, steel_m, bar=0.04)
sheet(n("cutout"), WINDOW_W, WINDOW_H, (0, -0.05, 0), grille_m, root)

# ---------------------------------------------------------------- glass
# A free-standing pane in a frame: glass against glass, and against what stands behind it.
root, n = module("pane", ground=True)
framed(n, root, 1.5, 2.0, 0.1, 0.0, frame_m)
sheet(n("glass"), 1.5, 2.0, (0, 0, 0.1), glass_m, root)


# ---------------------------------------------------------------- the facade
def bay(name, w, h, sill, interior):
    """A bay of wall, a floor high, with one opening `w` by `h`: frame, pane, and the room behind."""
    root, n = module(name)
    side = (BAY_M - w) / 2
    for s in (-1, 1):
        box(n(f"pier_{'ab'[s > 0]}"), (side, WALL_M, FLOOR_M), (s * (w + side) / 2, WALL_M / 2, FLOOR_M / 2), plaster_m, root)
    box(n("apron"), (w, WALL_M, sill), (0, WALL_M / 2, sill / 2), plaster_m, root)
    top = FLOOR_M - sill - h
    box(n("lintel"), (w, WALL_M, top), (0, WALL_M / 2, FLOOR_M - top / 2), plaster_m, root)
    box(n("sill"), (w + 0.2, 0.2, 0.06), (0, 0.04, sill - 0.03), concrete_m, root)
    framed(n, root, w - 0.1, h - 0.1, sill + 0.05, 0.13, frame_m, bar=0.05)
    sheet(n("glass"), w - 0.1, h - 0.1, (0, 0.13, sill + 0.05), glass_m, root)
    room_box(n("room"), ROOM_M, ROOM_M, ROOM_DEPTH_M, interior, root, (0, WALL_M, (FLOOR_M - ROOM_M) / 2))


bay("bay_window", WINDOW_W, WINDOW_H, SILL_M, rooms_m)
bay("bay_shop", SHOP_W, SHOP_H, SHOP_SILL_M, shops_m)

# What stands round the bays of a block three bays wide and three floors high: side walls,
# a back wall and a roof, clear of every room box.
root, n = module("block_shell", ground=True)
half, height, depth = BLOCK_BAYS * BAY_M / 2, BLOCK_FLOORS * FLOOR_M, WALL_M + ROOM_DEPTH_M + 0.4
for s in (-1, 1):
    box(n(f"side_{'ab'[s > 0]}"), (WALL_M, depth, height), (s * (half + WALL_M / 2), depth / 2, height / 2), plaster_m, root)
box(n("back"), (2 * half, WALL_M, height), (0, depth - WALL_M / 2, height / 2), plaster_m, root)
box(n("roof"), (2 * half + 2 * WALL_M, depth, 0.3), (0, depth / 2, height + 0.15), concrete_m, root)


# ---------------------------------------------------------------- bake and write
def meshes_of(root):
    return sorted((o for o in root.children_recursive if o.type == "MESH"), key=lambda o: o.name)


bpy.context.view_layer.update()
for name in sorted(MODULES):
    root, ground = MODULES[name]
    finish(objects=meshes_of(root), ground_ao=ground, **BAKE)

export(OUT)
print(f"\nKIT facade_lab: {len(MODULES)} modules")
for name in sorted(MODULES):
    count = 0
    for o in meshes_of(MODULES[name][0]):
        if tier_of(o) == 0:
            o.data.calc_loop_triangles()
            count += len(o.data.loop_triangles)
    print(f"{name:20} {count:6d} triangles at tier 0")
