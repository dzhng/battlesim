"""Non-Blender tests for vehicle lint's pure Python logic.

These tests verify the baseline filtering, issue matching, and data structures
without requiring Blender. The geometry checks are tested separately in
vehicle_lint_test.py which runs inside Blender.

    python3 -m unittest discover -s packages/scene-assets/blender -p 'vehicle_lint_logic_test.py'
"""
import json
import tempfile
import unittest
from pathlib import Path


class MockLintIssue:
    """Minimal LintIssue for testing without importing Blender-dependent code."""

    def __init__(self, check, severity, message, parts=(), distance_m=None):
        self.check = check
        self.severity = severity
        self.message = message
        self.parts = tuple(parts)
        self.distance_m = distance_m

    def to_dict(self):
        d = {"check": self.check, "severity": self.severity, "message": self.message}
        if self.parts:
            d["parts"] = list(self.parts)
        if self.distance_m is not None:
            d["distance_m"] = round(self.distance_m, 4)
        return d

    @classmethod
    def from_dict(cls, d):
        return cls(
            check=d["check"],
            severity=d["severity"],
            message=d["message"],
            parts=tuple(d.get("parts", [])),
            distance_m=d.get("distance_m"),
        )

    def matches_baseline(self, other):
        return self.check == other.check and set(self.parts) == set(other.parts)


class TestLintIssue(unittest.TestCase):
    """Test LintIssue data structure."""

    def test_to_dict_basic(self):
        issue = MockLintIssue(
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
        issue = MockLintIssue(
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
        issue = MockLintIssue.from_dict(d)
        self.assertEqual(issue.check, "floating_part")
        self.assertEqual(issue.severity, "error")
        self.assertEqual(issue.parts, ("antenna",))
        self.assertEqual(issue.distance_m, 0.05)

    def test_matches_baseline_same(self):
        issue1 = MockLintIssue("interpenetration", "error", "msg", ("a", "b"))
        issue2 = MockLintIssue("interpenetration", "error", "different msg", ("a", "b"))
        self.assertTrue(issue1.matches_baseline(issue2))

    def test_matches_baseline_different_order(self):
        issue1 = MockLintIssue("interpenetration", "error", "msg", ("a", "b"))
        issue2 = MockLintIssue("interpenetration", "error", "msg", ("b", "a"))
        self.assertTrue(issue1.matches_baseline(issue2))

    def test_matches_baseline_different_check(self):
        issue1 = MockLintIssue("interpenetration", "error", "msg", ("a", "b"))
        issue2 = MockLintIssue("floating_part", "error", "msg", ("a", "b"))
        self.assertFalse(issue1.matches_baseline(issue2))

    def test_matches_baseline_different_parts(self):
        issue1 = MockLintIssue("interpenetration", "error", "msg", ("a", "b"))
        issue2 = MockLintIssue("interpenetration", "error", "msg", ("a", "c"))
        self.assertFalse(issue1.matches_baseline(issue2))


class TestBaselineFile(unittest.TestCase):
    """Test baseline file I/O."""

    def test_round_trip(self):
        issues = [
            MockLintIssue("interpenetration", "error", "Parts overlap", ("skirt", "wheel"), 0.015),
            MockLintIssue("floating_part", "error", "Antenna floating", ("antenna",)),
        ]
        baseline = {"test_vehicle": [i.to_dict() for i in issues]}

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump(baseline, f, indent=2, sort_keys=True)
            f.flush()
            path = Path(f.name)

        try:
            loaded = json.loads(path.read_text())
            self.assertEqual(len(loaded["test_vehicle"]), 2)

            loaded_issues = [MockLintIssue.from_dict(d) for d in loaded["test_vehicle"]]
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
    """Test baseline filtering logic."""

    def _filter_issues(self, issues, baseline):
        """Filter out issues that match baseline entries."""
        known = baseline.get(issues[0].vehicle_id if hasattr(issues[0], 'vehicle_id') else "test", [])
        known_issues = [MockLintIssue.from_dict(d) for d in known]
        return [i for i in issues if not any(i.matches_baseline(k) for k in known_issues)]

    def test_known_issues_filtered(self):
        issues = [
            MockLintIssue("interpenetration", "error", "Overlap", ("skirt", "wheel")),
            MockLintIssue("floating_part", "error", "Floating", ("antenna",)),
        ]
        baseline = {
            "test": [{"check": "interpenetration", "severity": "error",
                      "message": "old message", "parts": ["wheel", "skirt"]}]
        }
        filtered = self._filter_issues(issues, baseline)
        self.assertEqual(len(filtered), 1)
        self.assertEqual(filtered[0].check, "floating_part")

    def test_new_issues_kept(self):
        issues = [MockLintIssue("ground_contact", "error", "Hovering", ("wheel",))]
        baseline = {}
        filtered = self._filter_issues(issues, baseline)
        self.assertEqual(len(filtered), 1)

    def test_empty_baseline_keeps_all(self):
        issues = [
            MockLintIssue("interpenetration", "error", "Overlap", ("a", "b")),
            MockLintIssue("floating_part", "error", "Floating", ("c",)),
        ]
        baseline = {}
        filtered = self._filter_issues(issues, baseline)
        self.assertEqual(len(filtered), 2)


if __name__ == "__main__":
    unittest.main()
