"""Tests for vehicle lint checks using synthetic fixtures.

Run inside Blender:
    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint_test.py
"""
import math
import os
import sys
import unittest

import bpy
import bmesh
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from vehicle_lint import (
    LintResult,
    check_interpenetration,
    check_floating_parts,
    check_ground_contact,
    INTERPENETRATION_TOLERANCE,
    FLOATING_PART_THRESHOLD,
    GROUND_CONTACT_TOLERANCE,
)


def reset_scene():
    """Clear the scene for a fresh test."""
    bpy.ops.wm.read_factory_settings(use_empty=True)


def make_box(name, size, location, parent=None):
    """Create a box mesh at the given location."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * size[0], v.co.y * size[1], v.co.z * size[2]))
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    obj.location = location
    if parent:
        obj.parent = parent
    return obj


def make_empty(name, location=(0, 0, 0)):
    """Create an empty object."""
    obj = bpy.data.objects.new(name, None)
    bpy.context.scene.collection.objects.link(obj)
    obj.location = location
    return obj


class TestInterpenetration(unittest.TestCase):
    """Test the interpenetration check."""

    def setUp(self):
        reset_scene()
        self.hull = make_empty("hull")

    def test_no_issues_with_separate_boxes(self):
        """Two boxes that don't touch should pass."""
        make_box("part_A_LOD0", (1, 1, 1), (0, 0, 0.5), self.hull)
        make_box("part_B_LOD0", (1, 1, 1), (3, 0, 0.5), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_interpenetration(result)
        
        issues = [i for i in result.issues if i.check == "interpenetration"]
        self.assertEqual(len(issues), 0, f"Expected no issues, got: {issues}")

    def test_no_issues_with_touching_boxes(self):
        """Two boxes that just touch (within tolerance) should pass."""
        make_box("part_A_LOD0", (1, 1, 1), (0, 0, 0.5), self.hull)
        make_box("part_B_LOD0", (1, 1, 1), (1.0 + INTERPENETRATION_TOLERANCE * 0.5, 0, 0.5), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_interpenetration(result)
        
        issues = [i for i in result.issues if i.check == "interpenetration"]
        self.assertEqual(len(issues), 0, f"Expected no issues, got: {issues}")

    def test_detects_overlapping_boxes(self):
        """Two boxes that significantly overlap should fail."""
        make_box("side_skirt_LOD0", (2, 0.1, 0.5), (0, 1.5, 0.5), self.hull)
        # This box penetrates 0.1m into the skirt
        make_box("wheel_L_1_tyre_LOD0", (0.5, 0.5, 0.5), (0, 1.45, 0.5), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_interpenetration(result)
        
        issues = [i for i in result.issues if i.check == "interpenetration"]
        self.assertGreaterEqual(len(issues), 1, "Expected interpenetration to be detected")
        self.assertEqual(issues[0].severity, "error")

    def test_same_part_group_not_flagged(self):
        """Parts from the same group (like wheel components) should not flag each other."""
        wheel = make_empty("wheel_L_1")
        make_box("wheel_L_1_tyre_LOD0", (0.5, 0.2, 0.5), (0, 0, 0), wheel)
        make_box("wheel_L_1_rim_LOD0", (0.3, 0.2, 0.3), (0, 0, 0), wheel)  # Inside the tyre
        
        result = LintResult(vehicle_id="test")
        check_interpenetration(result)
        
        # Should not flag wheel components overlapping themselves
        issues = [i for i in result.issues if i.check == "interpenetration"]
        self.assertEqual(len(issues), 0, f"Same-group parts flagged: {issues}")


class TestFloatingParts(unittest.TestCase):
    """Test the floating parts check."""

    def setUp(self):
        reset_scene()
        self.hull = make_empty("hull")

    def test_no_issues_with_connected_parts(self):
        """Parts that are connected should pass."""
        make_box("hull_body_LOD0", (2, 1, 0.5), (0, 0, 0.75), self.hull)
        make_box("turret_LOD0", (1, 0.8, 0.4), (0.3, 0, 1.2), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_floating_parts(result)
        
        issues = [i for i in result.issues if i.check == "floating_part"]
        self.assertEqual(len(issues), 0, f"Expected no issues, got: {issues}")

    def test_detects_floating_part(self):
        """A part far from others should be flagged."""
        make_box("hull_body_LOD0", (2, 1, 0.5), (0, 0, 0.75), self.hull)
        # This part is 5cm away from hull
        far_distance = FLOATING_PART_THRESHOLD * 2
        make_box("detached_panel_LOD0", (0.3, 0.3, 0.1), (0, 0, 1.5 + far_distance), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_floating_parts(result)
        
        issues = [i for i in result.issues if i.check == "floating_part"]
        self.assertGreaterEqual(len(issues), 1, "Expected floating part to be detected")
        self.assertIn("detached_panel", issues[0].parts[0])

    def test_nearby_parts_pass(self):
        """Parts within threshold distance should pass."""
        make_box("hull_body_LOD0", (2, 1, 0.5), (0, 0, 0.75), self.hull)
        # Just within threshold
        make_box("stowage_LOD0", (0.3, 0.3, 0.2), (0, 0, 1.0 + FLOATING_PART_THRESHOLD * 0.5), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_floating_parts(result)
        
        issues = [i for i in result.issues if i.check == "floating_part"]
        self.assertEqual(len(issues), 0, f"Parts within threshold flagged: {issues}")


class TestGroundContact(unittest.TestCase):
    """Test the ground contact check."""

    def setUp(self):
        reset_scene()
        self.hull = make_empty("hull")

    def test_passes_with_wheels_on_ground(self):
        """Wheels touching ground should pass."""
        wheel_radius = 0.3
        # Wheel bottom at z=0 (on ground)
        make_box("wheel_L_1_tyre_LOD0", (wheel_radius * 2, 0.2, wheel_radius * 2),
                 (0, 1, wheel_radius), self.hull)
        make_box("wheel_R_1_tyre_LOD0", (wheel_radius * 2, 0.2, wheel_radius * 2),
                 (0, -1, wheel_radius), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_ground_contact(result)
        
        issues = [i for i in result.issues if i.check == "ground_contact"]
        self.assertEqual(len(issues), 0, f"Expected no issues, got: {issues}")

    def test_detects_hovering_vehicle(self):
        """Vehicle hovering above ground should be flagged."""
        wheel_radius = 0.3
        hover_height = GROUND_CONTACT_TOLERANCE * 3
        # Wheel bottom at hover_height above ground
        make_box("wheel_L_1_tyre_LOD0", (wheel_radius * 2, 0.2, wheel_radius * 2),
                 (0, 1, wheel_radius + hover_height), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_ground_contact(result)
        
        issues = [i for i in result.issues if i.check == "ground_contact"]
        self.assertGreaterEqual(len(issues), 1, "Expected hovering vehicle to be detected")
        self.assertIn("hovers", issues[0].message.lower())

    def test_detects_sinking_vehicle(self):
        """Vehicle sinking below ground should be flagged."""
        wheel_radius = 0.3
        sink_depth = GROUND_CONTACT_TOLERANCE * 3
        # Wheel bottom below ground
        make_box("wheel_L_1_tyre_LOD0", (wheel_radius * 2, 0.2, wheel_radius * 2),
                 (0, 1, wheel_radius - sink_depth), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_ground_contact(result)
        
        issues = [i for i in result.issues if i.check == "ground_contact"]
        self.assertGreaterEqual(len(issues), 1, "Expected sinking vehicle to be detected")
        self.assertIn("sinks", issues[0].message.lower())

    def test_within_tolerance_passes(self):
        """Wheels very slightly off ground (within tolerance) should pass."""
        wheel_radius = 0.3
        # Barely hovering, within tolerance
        make_box("wheel_L_1_tyre_LOD0", (wheel_radius * 2, 0.2, wheel_radius * 2),
                 (0, 1, wheel_radius + GROUND_CONTACT_TOLERANCE * 0.5), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_ground_contact(result)
        
        issues = [i for i in result.issues if i.check == "ground_contact"]
        self.assertEqual(len(issues), 0, f"Within-tolerance flagged: {issues}")

    def test_tracks_also_checked(self):
        """Track belts should also be checked for ground contact."""
        # Track belt hovering
        hover_height = GROUND_CONTACT_TOLERANCE * 3
        make_box("track_L_band_LOD0", (3, 0.5, 0.1), (0, 1.5, hover_height + 0.05), self.hull)
        
        result = LintResult(vehicle_id="test")
        check_ground_contact(result)
        
        issues = [i for i in result.issues if i.check == "ground_contact"]
        self.assertGreaterEqual(len(issues), 1, "Expected hovering track to be detected")


class TestDressingExclusion(unittest.TestCase):
    """Test that dressing nodes are excluded from checks."""

    def setUp(self):
        reset_scene()
        self.hull = make_empty("hull")

    def test_dressing_parts_ignored_in_floating_check(self):
        """Parts under dressing_* nodes should not be flagged as floating."""
        make_box("hull_body_LOD0", (2, 1, 0.5), (0, 0, 0.75), self.hull)
        
        # Antenna on dressing node - far from hull
        dressing = make_empty("dressing_antenna", (0, 0, 0))
        dressing.parent = self.hull
        make_box("antenna_whip_LOD0", (0.02, 0.02, 2), (0, 0, 3), dressing)
        
        result = LintResult(vehicle_id="test")
        check_floating_parts(result)
        
        issues = [i for i in result.issues if i.check == "floating_part"]
        self.assertEqual(len(issues), 0, "Dressing parts should not be flagged as floating")


class TestBaselineFiltering(unittest.TestCase):
    """Test baseline filtering logic."""

    def test_known_issues_filtered(self):
        """Issues in baseline should be filtered out."""
        from vehicle_lint import LintIssue, filter_by_baseline
        
        result = LintResult(vehicle_id="test_vehicle")
        result.add(LintIssue(
            check="interpenetration",
            severity="error",
            message="Parts 'skirt' and 'wheel' interpenetrate",
            parts=("skirt", "wheel"),
            distance_m=0.015,
        ))
        result.add(LintIssue(
            check="floating_part",
            severity="error",
            message="Part 'antenna' is floating",
            parts=("antenna",),
        ))
        
        baseline = {
            "test_vehicle": [
                {"check": "interpenetration", "severity": "error", 
                 "message": "old message", "parts": ["wheel", "skirt"]}  # Note: order swapped
            ]
        }
        
        filtered = filter_by_baseline(result, baseline)
        self.assertEqual(len(filtered.issues), 1)
        self.assertEqual(filtered.issues[0].check, "floating_part")


def run_tests():
    """Run all tests and return exit code."""
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromModule(sys.modules[__name__])
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    exit(run_tests())
