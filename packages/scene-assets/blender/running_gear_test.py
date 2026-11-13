"""A tracked vehicle's belt as `running_gear` builds it: its top run sags
between the wheels that carry it, it never cuts into a wheel, however low the
rollers or steep the sag, and its `track_length_m` is the length the links
scroll round. Runs inside Blender (it builds scenes):

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/running_gear_test.py
"""
import math
import os
import sys
import unittest

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import empty, reset  # noqa: E402
import running_gear as RG  # noqa: E402
from vehicle_export import materials  # noqa: E402

ROAD_X = [2.0 - 0.8 * k for k in range(6)]
ROAD_Z, ROAD_R = 0.40, 0.33
# Return rollers well above the road wheels, and rollers barely clear of them.
HIGH = [(1.6, 0.88, 0.10), (0.0, 0.88, 0.10), (-1.6, 0.88, 0.10)]
LOW = [(1.6, 0.68, 0.06), (0.0, 0.68, 0.06), (-1.6, 0.68, 0.06)]
# The lint's contact tolerance: a belt vertex this close to a wheel's rim
# touches it, as the belt touches every wheel it wraps.
TOUCH = 0.005


def build(returns, sag=RG.TRACK_SAG):
    """One side's belt wrapped round its road wheels, sprocket, idler and
    `returns` rollers; the belt's tier-0 points as (x, z), its node and the
    wheels as (x, z, radius) circles."""
    reset()
    hull = empty("hull", parent=empty("vehicle"))
    wheels = [(x, ROAD_Z, ROAD_R) for x in ROAD_X] + [(-2.9, 0.5, 0.3), (2.9, 0.5, 0.3)] + list(returns)
    RG.track_loop("track_L", wheels, 1.3, 0.5, 0.17, materials("us_desert_tan"), hull, sag=sag)
    bpy.context.view_layer.update()
    node = bpy.data.objects["track_L"]
    belt = next(o for o in node.children if o.name.endswith("LOD0"))
    points = [(belt.matrix_world @ v.co) for v in belt.data.vertices]
    return [(p.x, p.z) for p in points], node, wheels


def top_at(points, x, reach=0.05):
    """The belt's highest point within `reach` of `x` along the hull."""
    return max(z for px, z in points if abs(px - x) < reach)


class TrackSag(unittest.TestCase):
    def test_top_run_sags_between_its_rollers(self):
        points, _, _ = build(HIGH)
        for (a, za, _), (b, _, _) in zip(HIGH, HIGH[1:]):
            self.assertLess(top_at(points, (a + b) / 2), top_at(points, a) - 0.01)

    def test_belt_never_enters_a_wheel(self):
        for returns in (HIGH, LOW):
            for sag in (RG.TRACK_SAG, 0.2):
                points, _, wheels = build(returns, sag)
                for x, z, r in wheels:
                    inside = max(r - math.hypot(px - x, pz - z) for px, pz in points)
                    self.assertLess(inside, TOUCH, f"rollers {returns[0][1]}, sag {sag}: wheel at {x:.2f}")

    def test_length_is_the_belt_the_links_scroll_round(self):
        points, node, _ = build(HIGH)
        # Each ring of four vertices sits round one point of the centre line.
        centre = [(sum(p[0] for p in points[i:i + 4]) / 4, sum(p[1] for p in points[i:i + 4]) / 4)
                  for i in range(0, len(points), 4)]
        drawn = sum(math.dist(centre[i], centre[i - 1]) for i in range(len(centre)))
        self.assertAlmostEqual(drawn / node["track_length_m"], 1.0, delta=0.005)


if __name__ == "__main__":
    result = unittest.main(argv=["running_gear_test"], exit=False).result
    sys.exit(0 if result.wasSuccessful() else 1)
