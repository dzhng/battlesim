"""A hull's side outline and the panels kept inside it: `hull_shape.Outline`
refuses a notched profile, and `armour_parts.side_skirt` and `track_fender`
clipped to a sloped glacis never leave it. Runs inside Blender (it builds
scenes):

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/hull_shape_test.py
"""
import os
import sys
import unittest

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import armour_parts as AR  # noqa: E402
from hull_shape import Outline  # noqa: E402
from parts import empty, reset  # noqa: E402
from vehicle_export import materials  # noqa: E402

# A hull 7 m long in side elevation: a level roof, a long glacis falling to
# a short nose plate, and a narrower lower hull under it.
HALF, ROOF, GLACIS_HEAD, NOSE_TOP = 3.5, 1.8, 1.0, 1.1
UPPER = [(-HALF, 0.9), (3.2, 0.9), (HALF, 1.0), (HALF, NOSE_TOP), (GLACIS_HEAD, ROOF), (-HALF, ROOF)]
LOWER = [(-3.1, 0.45), (2.9, 0.45), (3.3, 0.9), (-HALF, 0.9)]
# The tolerance a vertex may stray past the outline: float32 storage of
# metre-scale coordinates.
EPS = 1e-5


def world_xz(objects):
    bpy.context.view_layer.update()
    return [(p.x, p.z) for o in objects if o.type == "MESH" for p in (o.matrix_world @ v.co for v in o.data.vertices)]


def scene():
    reset()
    return materials("us_desert_tan"), empty("hull")


class OutlineShape(unittest.TestCase):
    def test_reads_the_hull_side(self):
        outline = Outline(UPPER, LOWER)
        self.assertAlmostEqual(outline.top(0.0), ROOF)
        self.assertLess(outline.top(3.0), ROOF)
        self.assertAlmostEqual(outline.front(1.05), HALF)
        self.assertAlmostEqual(outline.rear(0.5), -3.1 + (-HALF + 3.1) * (0.05 / 0.45), places=6)
        self.assertIsNone(outline.top(HALF + 0.1))
        self.assertIsNone(outline.front(ROOF + 0.1))

    def test_refuses_a_notched_profile(self):
        stepped = [(-HALF, 0.9), (HALF, 0.9), (HALF, 1.4), (0.0, 1.4), (0.0, ROOF), (-HALF, ROOF)]
        with self.assertRaises(ValueError):
            Outline(stepped)
        with self.assertRaises(ValueError):
            Outline.from_rings([(0.9, [(-HALF, 1), (HALF, 1)]), (1.4, [(-HALF, 1), (HALF, 1)]),
                                (1.41, [(-HALF, 1), (0.0, 1)]), (ROOF, [(-HALF, 1), (0.0, 1)])])
        self.assertEqual(Outline(stepped, envelope=True).top(HALF), 1.4)


class PanelsStayInside(unittest.TestCase):
    def check_inside(self, objects, outline, inset):
        for x, z in world_xz(objects):
            self.assertLessEqual(z, outline.top(x) - inset + EPS, f"above the outline at x={x:.3f}")
            front, rear = outline.front(z), outline.rear(z)
            if front is not None:
                self.assertLessEqual(x, front - inset + EPS, f"ahead of the outline at z={z:.3f}")
                self.assertGreaterEqual(x, rear + inset - EPS, f"behind the outline at z={z:.3f}")

    def test_a_skirt_run_stops_where_the_glacis_takes_over(self):
        mats, hull = scene()
        outline = Outline(UPPER, LOWER)
        inset = 0.03
        # Panels high on the side, run on past the nose: the glacis cuts the
        # front ones down, and the last is only a sliver.
        panels = 14
        made = AR.side_skirt("skirt", outline, 1, 1.6, -3.05, 3.95, 1.0, mats, hull, panels=panels, inset=inset)
        plates = [o for o in made if "_bolt_" not in o.name]
        self.check_inside(plates, outline, inset)
        self.check_inside(made, outline, 0.0)
        heights = []
        for k in range(panels):
            mesh = [o for o in plates if o.name.startswith(f"skirt_{k}_LOD")]
            if mesh:
                zs = [z for _, z in world_xz(mesh)]
                heights.append(max(zs) - min(zs))
        self.assertTrue(all(h >= AR.MIN_PANEL_M - EPS for h in heights), heights)
        self.assertLess(len(heights), panels, "no panel was dropped at the nose")
        self.assertLess(min(heights), max(heights) - 0.1, "no panel was cut down by the glacis")

    def test_a_grid_with_a_rising_foot_stays_inside(self):
        mats, hull = scene()
        outline = Outline(UPPER, LOWER)
        made = AR.side_skirt("tiles", outline, -1, 1.6, -3.3, 3.2, [(-3.3, 0.6), (2.6, 0.6), (3.2, 1.2)], mats,
                             hull, top=1.75, panels=6, rows=2, depth=0.08, bolts=(2, 2))
        self.assertTrue(made)
        self.check_inside([o for o in made if "_bolt_" not in o.name], outline, 0.03)
        self.check_inside(made, outline, 0.0)

    def test_a_fender_ends_at_the_nose(self):
        mats, hull = scene()
        outline = Outline(UPPER, LOWER)
        made = AR.track_fender("fender", outline, 1, 1.0, 1.6, 2.8, 3.9, 1.05, mats, hull, slope=-0.3)
        self.assertTrue(made)
        self.check_inside(made, outline, 0.0)
        self.assertGreater(max(x for x, _ in world_xz(made)), HALF - 0.4, "the fender no longer reaches the nose")


if __name__ == "__main__":
    result = unittest.main(argv=["hull_shape_test"], exit=False).result
    sys.exit(0 if result.wasSuccessful() else 1)
