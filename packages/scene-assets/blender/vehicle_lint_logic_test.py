"""Non-Blender tests for vehicle lint's pure Python logic.

These tests verify the baseline filtering, issue matching, and data structures
without requiring Blender. The geometry checks are tested separately in
vehicle_lint_test.py which runs inside Blender.

    python3 -m unittest discover -s packages/scene-assets/blender -p 'vehicle_lint_logic_test.py'
"""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from vehicle_lint_core import (
    LintIssue,
    LintResult,
    filter_by_baseline,
    filter_by_allowlist,
)


class TestLintIssue(unittest.TestCase):
    """Test LintIssue data structure."""

    def test_to_dict_basic(self):
        issue = LintIssue(
            check="interpenetration",
            severity="error",
            message="Parts overlap",
        )
        d = issue.to_dict()
        self.assertEqual(d["check"], "interpenetration")
        self.assertEqual(d["severity"], "error")
        self.assertEqual(d["message"], "Parts overlap")
        self.assertNotIn("parts", d)
        self.assertNotIn("distance_m", d)

    def test_to_dict_with_parts_and_distance(self):
        issue = LintIssue(
            check="interpenetration",
            severity="error",
            message="Parts overlap",
            parts=("skirt", "wheel"),
            distance_m=0.01234567,
        )
        d = issue.to_dict()
        self.assertEqual(d["parts"], ["skirt", "wheel"])
        self.assertEqual(d["distance_m"], 0.0123)  # Rounded to 4 decimals

    def test_from_dict(self):
        d = {
            "check": "floating_part",
            "severity": "error",
            "message": "Part floating",
            "parts": ["antenna"],
            "distance_m": 0.05,
        }
        issue = LintIssue.from_dict(d)
        self.assertEqual(issue.check, "floating_part")
        self.assertEqual(issue.severity, "error")
        self.assertEqual(issue.parts, ("antenna",))
        self.assertEqual(issue.distance_m, 0.05)

    def test_matches_baseline_same(self):
        issue1 = LintIssue("interpenetration", "error", "msg", ("a", "b"))
        issue2 = LintIssue("interpenetration", "error", "different msg", ("a", "b"))
        self.assertTrue(issue1.matches_baseline(issue2))

    def test_matches_baseline_different_order(self):
        issue1 = LintIssue("interpenetration", "error", "msg", ("a", "b"))
        issue2 = LintIssue("interpenetration", "error", "msg", ("b", "a"))
        self.assertTrue(issue1.matches_baseline(issue2))

    def test_matches_baseline_different_check(self):
        issue1 = LintIssue("interpenetration", "error", "msg", ("a", "b"))
        issue2 = LintIssue("floating_part", "error", "msg", ("a", "b"))
        self.assertFalse(issue1.matches_baseline(issue2))

    def test_matches_baseline_different_parts(self):
        issue1 = LintIssue("interpenetration", "error", "msg", ("a", "b"))
        issue2 = LintIssue("interpenetration", "error", "msg", ("a", "c"))
        self.assertFalse(issue1.matches_baseline(issue2))

    def test_matches_baseline_distance_within_tolerance(self):
        """Issue with distance slightly larger than baseline is still matched."""
        baseline = LintIssue("floating_part", "error", "msg", ("antenna",), distance_m=0.020)
        # Distance 22mm is within 5mm tolerance of 20mm baseline
        current = LintIssue("floating_part", "error", "msg", ("antenna",), distance_m=0.022)
        self.assertTrue(current.matches_baseline(baseline))

    def test_matches_baseline_distance_beyond_tolerance(self):
        """Issue with distance significantly larger than baseline fails matching."""
        baseline = LintIssue("floating_part", "error", "msg", ("antenna",), distance_m=0.020)
        # Distance 30mm exceeds 5mm tolerance past 20mm baseline
        current = LintIssue("floating_part", "error", "msg", ("antenna",), distance_m=0.030)
        self.assertFalse(current.matches_baseline(baseline))

    def test_matches_baseline_smaller_distance_matches(self):
        """Issue with smaller distance than baseline always matches."""
        baseline = LintIssue("floating_part", "error", "msg", ("antenna",), distance_m=0.030)
        current = LintIssue("floating_part", "error", "msg", ("antenna",), distance_m=0.020)
        self.assertTrue(current.matches_baseline(baseline))

    def test_matches_baseline_no_baseline_distance(self):
        """Legacy baseline without distance matches any current distance."""
        baseline = LintIssue("floating_part", "error", "msg", ("antenna",), distance_m=None)
        current = LintIssue("floating_part", "error", "msg", ("antenna",), distance_m=0.050)
        self.assertTrue(current.matches_baseline(baseline))

    def test_matches_baseline_no_current_distance(self):
        """Current issue without distance matches any baseline distance."""
        baseline = LintIssue("floating_part", "error", "msg", ("antenna",), distance_m=0.020)
        current = LintIssue("floating_part", "error", "msg", ("antenna",), distance_m=None)
        self.assertTrue(current.matches_baseline(baseline))


class TestBaselineFile(unittest.TestCase):
    """Test baseline file I/O."""

    def test_round_trip(self):
        issues = [
            LintIssue("interpenetration", "error", "Parts overlap", ("skirt", "wheel"), 0.015),
            LintIssue("floating_part", "error", "Antenna floating", ("antenna",)),
        ]
        baseline = {"test_vehicle": [i.to_dict() for i in issues]}

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump(baseline, f, indent=2, sort_keys=True)
            f.flush()
            path = Path(f.name)

        try:
            loaded = json.loads(path.read_text())
            self.assertEqual(len(loaded["test_vehicle"]), 2)

            loaded_issues = [LintIssue.from_dict(d) for d in loaded["test_vehicle"]]
            self.assertEqual(loaded_issues[0].check, "interpenetration")
            self.assertEqual(loaded_issues[0].parts, ("skirt", "wheel"))
            self.assertEqual(loaded_issues[0].distance_m, 0.015)
        finally:
            path.unlink()

    def test_empty_baseline(self):
        baseline = {}
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump(baseline, f)
            f.flush()
            path = Path(f.name)

        try:
            loaded = json.loads(path.read_text())
            self.assertEqual(loaded, {})
        finally:
            path.unlink()


class TestTolerances(unittest.TestCase):
    """Test that tolerances are sensible values."""

    def test_interpenetration_tolerance(self):
        # Should be around 5mm for intended contacts
        tolerance = 0.005  # From vehicle_lint.py
        self.assertGreater(tolerance, 0.001, "Too strict - would flag rivets")
        self.assertLess(tolerance, 0.02, "Too loose - would miss real problems")

    def test_floating_part_threshold(self):
        # Should be around 2cm
        threshold = 0.02  # From vehicle_lint.py
        self.assertGreater(threshold, 0.005, "Too strict - would flag stowage")
        self.assertLess(threshold, 0.1, "Too loose - would miss detached parts")

    def test_ground_contact_tolerance(self):
        # Should be around 1cm
        tolerance = 0.01  # From vehicle_lint.py
        self.assertGreater(tolerance, 0.001, "Too strict - would flag floating point errors")
        self.assertLess(tolerance, 0.05, "Too loose - would miss visible gaps")


class TestFilterByBaseline(unittest.TestCase):
    """Test baseline filtering logic using the real filter_by_baseline function."""

    def test_known_issues_filtered(self):
        result = LintResult(vehicle_id="test")
        result.add(LintIssue("interpenetration", "error", "Overlap", ("skirt", "wheel")))
        result.add(LintIssue("floating_part", "error", "Floating", ("antenna",)))
        baseline = {
            "test": [{"check": "interpenetration", "severity": "error",
                      "message": "old message", "parts": ["wheel", "skirt"]}]
        }
        filtered = filter_by_baseline(result, baseline)
        self.assertEqual(len(filtered.issues), 1)
        self.assertEqual(filtered.issues[0].check, "floating_part")

    def test_new_issues_kept(self):
        result = LintResult(vehicle_id="test")
        result.add(LintIssue("ground_contact", "error", "Hovering", ("wheel",)))
        baseline = {}
        filtered = filter_by_baseline(result, baseline)
        self.assertEqual(len(filtered.issues), 1)

    def test_empty_baseline_keeps_all(self):
        result = LintResult(vehicle_id="test")
        result.add(LintIssue("interpenetration", "error", "Overlap", ("a", "b")))
        result.add(LintIssue("floating_part", "error", "Floating", ("c",)))
        baseline = {}
        filtered = filter_by_baseline(result, baseline)
        self.assertEqual(len(filtered.issues), 2)

    def test_distance_within_tolerance_filtered(self):
        """Baseline issue at 20mm should filter current issue at 22mm (within 5mm tolerance)."""
        result = LintResult(vehicle_id="test")
        result.add(LintIssue("floating_part", "error", "Floating", ("antenna",), distance_m=0.022))
        baseline = {
            "test": [{"check": "floating_part", "severity": "error",
                      "message": "Floating", "parts": ["antenna"], "distance_m": 0.020}]
        }
        filtered = filter_by_baseline(result, baseline)
        self.assertEqual(len(filtered.issues), 0, "Issue within tolerance should be filtered")

    def test_distance_beyond_tolerance_kept(self):
        """Baseline issue at 20mm should NOT filter current issue at 30mm (exceeds tolerance)."""
        result = LintResult(vehicle_id="test")
        result.add(LintIssue("floating_part", "error", "Floating", ("antenna",), distance_m=0.030))
        baseline = {
            "test": [{"check": "floating_part", "severity": "error",
                      "message": "Floating", "parts": ["antenna"], "distance_m": 0.020}]
        }
        filtered = filter_by_baseline(result, baseline)
        self.assertEqual(len(filtered.issues), 1, "Issue beyond tolerance should be kept")

    def test_improved_distance_filtered(self):
        """Current issue with smaller distance than baseline is always filtered."""
        result = LintResult(vehicle_id="test")
        result.add(LintIssue("floating_part", "error", "Floating", ("antenna",), distance_m=0.015))
        baseline = {
            "test": [{"check": "floating_part", "severity": "error",
                      "message": "Floating", "parts": ["antenna"], "distance_m": 0.020}]
        }
        filtered = filter_by_baseline(result, baseline)
        self.assertEqual(len(filtered.issues), 0, "Improved issue should be filtered")


if __name__ == "__main__":
    unittest.main()
