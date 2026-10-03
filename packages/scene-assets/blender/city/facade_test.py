"""The source facade must show an opening at every declared fighting position."""
import unittest

from facade import missing_openings, overlapping_openings


class FacadeTest(unittest.TestCase):
    def test_window_panes_do_not_overlap_on_the_same_wall(self):
        low = ("front", -1.25, 1.25, 1.3, 2.8)
        high = ("front", -1.25, 1.25, 2.0, 3.5)
        beside = ("front", 1.25, 3.75, 1.3, 2.8)
        above = ("front", -1.25, 1.25, 2.8, 4.3)
        back = ("back", -1.25, 1.25, 2.0, 3.5)
        self.assertEqual(overlapping_openings([low, high, beside, back]), [(low, high)])
        self.assertEqual(overlapping_openings([low, beside, back, above]), [])

    def test_each_floor_needs_its_own_opening(self):
        descriptor = dict(
            parts=[dict(id="wall", base_z=0.0, half_extents=[3.0, 2.0, 3.0])],
            floor_heights_m=[0.0, 3.0],
            edges=[dict(id="front", part="wall", exposed=True, span_m=[-3.0, 3.0],
                        bays=dict(pitch_m=3.0, phase_m=1.5))],
        )
        upper = [("front", -2.1, -0.9, 3.9, 5.4), ("front", 0.9, 2.1, 3.9, 5.4)]
        self.assertEqual(missing_openings(descriptor, upper, (1.4, 1.6)),
                         [("front", -1.5, 0.0), ("front", 1.5, 0.0)])
        ground = [("front", -2.1, -0.9, 0.9, 2.4), ("front", 0.9, 2.1, 0.9, 2.4)]
        self.assertEqual(missing_openings(descriptor, ground + upper, (1.4, 1.6)), [])

    def test_only_held_bands_below_each_part_top_need_openings(self):
        descriptor = dict(
            parts=[dict(id="low", base_z=0.0, half_extents=[3.0, 2.0, 1.45]),
                   dict(id="tower", base_z=0.0, half_extents=[3.0, 2.0, 6.5])],
            floor_heights_m=[0.0, 3.0, 6.0, 9.0],
            edges=[dict(id=name, part=name, exposed=True, span_m=[-3.0, 3.0],
                        bays=dict(pitch_m=3.0, phase_m=1.5)) for name in ("low", "tower")],
        )
        openings = [("low", -3.0, 3.0, 0.0, 2.9), ("tower", -3.0, 3.0, 0.0, 8.0)]
        self.assertEqual(missing_openings(descriptor, openings, (1.4, 1.6)), [])

    def test_a_door_or_open_front_counts_but_a_high_window_does_not(self):
        descriptor = dict(
            parts=[dict(id="shed", base_z=0.0, half_extents=[4.5, 3.0, 1.5])],
            floor_heights_m=[0.0],
            edges=[dict(id="front", part="shed", exposed=True, span_m=[-4.5, 4.5],
                        bays=dict(pitch_m=3.0, phase_m=0.0))],
        )
        openings = [("front", -4.0, -2.0, 0.0, 2.1), ("front", -1.0, 1.0, 2.0, 2.8)]
        self.assertEqual(missing_openings(descriptor, openings, (1.4, 1.6)),
                         [("front", 0.0, 0.0), ("front", 3.0, 0.0)])
        self.assertEqual(missing_openings(descriptor, [("front", -4.5, 4.5, 0.0, 3.0)], (1.4, 1.6)), [])


if __name__ == "__main__":
    unittest.main()
