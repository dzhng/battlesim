"""A wreck's thrown debris: `wreckage.scatter` throws it clear of the hull and
low, from what the vehicle carries; `burn` keeps it; `cut_to` separates it
from the whole that stays. Runs inside Blender (it builds scenes):

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/wreckage_test.py
"""
import math
import os
import sys
import unittest

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import box, empty, reset, tier_of  # noqa: E402
from vehicle_export import materials  # noqa: E402
from vehicle_parts import tyre_wheel  # noqa: E402
import wreckage  # noqa: E402
from wreckage import DEBRIS, burn, cut_to, scatter  # noqa: E402

HALF = (3.0, 1.5)
# The validator's allowance (SCENERY_KINDS.wreck.debris in scenery.ts).
REACH_M, TOP_M = 8.0, 1.0


def vehicle(kind):
    """A hull box on its running gear (`tracked` or `wheeled`), with a
    jerrycan and an antenna on dressing, as a family's build leaves it."""
    reset()
    mats = materials("us_desert_tan")
    root = empty("vehicle")
    hull = empty("hull", parent=root)
    box("hull_body", (2 * HALF[0], 2 * HALF[1], 1.6), (0, 0, 1.2), mats["paint"], hull)
    if kind == "tracked":
        for side in ("L", "R"):
            belt = empty(f"track_{side}", parent=hull)
            y = 1.2 if side == "L" else -1.2
            box(f"belt_{side}", (6.0, 0.6, 0.08), (0, y, 0.04), mats["track"], belt)
    else:
        for k, (x, y) in enumerate(((1.8, 1.2), (-1.8, 1.2), (1.8, -1.2), (-1.8, -1.2))):
            tyre_wheel(f"wheel_{k}", (x, y, 0.5), 0.5, 0.35, 1 if y > 0 else -1, mats, hull)
    dressing = empty("dressing_stowage", parent=hull)
    box("jerrycan_rack", (0.4, 0.4, 0.5), (-2.5, 0, 2.2), mats["paint"], dressing)
    box("antenna", (0.02, 0.02, 2.0), (-2.0, 0.5, 3.0), mats["steel"], dressing)
    return mats


def debris_meshes():
    return [o for o in bpy.data.objects if o.type == "MESH" and wreckage.is_debris(o)]


def world_points(objs):
    bpy.context.view_layer.update()
    return [o.matrix_world @ v.co for o in objs for v in o.data.vertices]


class Scatter(unittest.TestCase):
    def check_thrown(self):
        meshes = debris_meshes()
        self.assertTrue(meshes, "no debris thrown")
        for p in world_points(meshes):
            dx, dy = max(0.0, abs(p.x) - HALF[0]), max(0.0, abs(p.y) - HALF[1])
            # Clear of the hull, never on it; inside the allowance; lying low.
            self.assertGreater(math.hypot(dx, dy), 0.2, f"debris on the hull at {p}")
            self.assertLessEqual(math.hypot(dx, dy), REACH_M)
            self.assertLessEqual(p.z, TOP_M)
            self.assertGreaterEqual(p.z, -1e-4)
        # Every tier draws some of it, so the debris state is a whole bundle.
        self.assertEqual({tier_of(o) for o in meshes}, {0, 1, 2, 3})
        # Every debris tree hangs from the scene's root, not the hull: a wreck
        # tilted on its broken suspension does not lift it off the ground.
        for o in bpy.data.objects:
            if o.name.startswith(DEBRIS):
                self.assertIsNone(o.parent)
        return meshes

    def test_a_tracked_hull_throws_a_track_run_and_no_wheel(self):
        mats = vehicle("tracked")
        scatter(HALF, mats, seed=5)
        meshes = self.check_thrown()
        names = " ".join(o.name for o in meshes)
        self.assertIn("track", names)
        self.assertNotIn("tyre", names)

    def test_a_wheeled_hull_throws_a_wheel_and_no_track_run(self):
        mats = vehicle("wheeled")
        scatter(HALF, mats, seed=5)
        meshes = self.check_thrown()
        self.assertTrue(any(o.data.materials[0] == mats["rubber"] for o in meshes))
        self.assertNotIn("track", " ".join(o.name for o in meshes))

    def test_the_same_seed_throws_the_same_debris(self):
        def thrown(seed):
            vehicle("tracked")
            scatter(HALF, materials("us_desert_tan"), seed=seed)
            return [tuple(round(c, 6) for c in p) for p in world_points(debris_meshes())]

        self.assertEqual(thrown(5), thrown(5))
        self.assertNotEqual(thrown(5), thrown(6))


class Burn(unittest.TestCase):
    def test_burn_keeps_debris_and_drops_dressing(self):
        mats = vehicle("tracked")
        scatter(HALF, mats, seed=5)
        before = sorted(o.name for o in debris_meshes())
        burn()
        self.assertEqual(sorted(o.name for o in debris_meshes()), before)
        self.assertFalse([o for o in bpy.data.objects if o.name.startswith(("dressing_", "jerrycan_rack"))])


class Cut(unittest.TestCase):
    def test_the_whole_keeps_no_debris_and_the_debris_state_only_debris_where_it_lay(self):
        mats = vehicle("wheeled")
        scatter(HALF, mats, seed=5)
        bpy.context.view_layer.update()
        lay = sorted(tuple(round(c, 5) for c in p) for p in world_points(debris_meshes()))
        path = os.path.join(bpy.app.tempdir or "/tmp", "wreckage_test.blend")
        bpy.ops.wm.save_as_mainfile(filepath=path, copy=True)
        cut_to("default")
        self.assertFalse(debris_meshes())
        self.assertTrue(any(o.name.startswith("hull_body") for o in bpy.data.objects))
        bpy.ops.wm.open_mainfile(filepath=path)
        cut_to("debris")
        meshes = [o for o in bpy.data.objects if o.type == "MESH"]
        self.assertEqual(meshes, debris_meshes())
        self.assertEqual(sorted(tuple(round(c, 5) for c in p) for p in world_points(meshes)), lay)
        os.remove(path)


if __name__ == "__main__":
    result = unittest.main(argv=["wreckage_test"], exit=False).result
    sys.exit(0 if result.wasSuccessful() else 1)
