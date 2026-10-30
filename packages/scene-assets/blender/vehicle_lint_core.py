"""Pure Python logic for vehicle lint - no Blender dependencies.

This module contains the data structures and filtering logic that can be
tested without Blender. The geometry checks that require bpy live in
vehicle_lint.py and import this module.
"""
import fnmatch
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class LintIssue:
    """A single lint finding."""
    check: str
    severity: str
    message: str
    parts: tuple[str, ...] = ()
    distance_m: Optional[float] = None

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

    def matches_baseline(self, other, distance_tolerance=0.005):
        """Check if this issue matches a baseline entry.
        
        Matches if check and parts are the same, AND the distance hasn't grown
        more than distance_tolerance (5mm default) past the baseline value.
        A part whose gap grows past its baselined value fails.
        """
        if self.check != other.check or set(self.parts) != set(other.parts):
            return False
        if other.distance_m is None:
            return True
        if self.distance_m is None:
            return True
        return self.distance_m <= other.distance_m + distance_tolerance


@dataclass
class LintResult:
    """Results from linting one vehicle."""
    vehicle_id: str
    issues: list[LintIssue] = field(default_factory=list)
    passed: bool = True

    def add(self, issue: LintIssue):
        self.issues.append(issue)
        if issue.severity == "error":
            self.passed = False


def load_baseline(path: Path) -> dict[str, list[dict]]:
    """Load the baseline file of known issues."""
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text())
    except (json.JSONDecodeError, IOError):
        return {}


def save_baseline(path: Path, baseline: dict[str, list[dict]]):
    """Save the baseline file."""
    path.write_text(json.dumps(baseline, indent=2, sort_keys=True) + "\n")


def load_allowlist(path: Path) -> dict[str, list[dict]]:
    """Load the allowlist file of intentionally accepted issues."""
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text())
    except (json.JSONDecodeError, IOError):
        return {}


def _matches_pattern(part_name: str, pattern: str) -> bool:
    """Check if a part name matches an allowlist pattern.
    
    Supports glob-style patterns with * wildcards:
    - "wheel_*_sprocket_hub" matches "wheel_L_sprocket_hub", "wheel_R_sprocket_hub"
    - "dash*" matches "dash", "dash_-1", "dash_1"
    """
    return fnmatch.fnmatch(part_name.lower(), pattern.lower())


def filter_by_baseline(result: LintResult, baseline: dict[str, list[dict]]) -> LintResult:
    """Filter out issues that are in the baseline."""
    known = baseline.get(result.vehicle_id, [])
    known_issues = [LintIssue.from_dict(d) for d in known]
    
    filtered = LintResult(vehicle_id=result.vehicle_id)
    for issue in result.issues:
        is_known = any(issue.matches_baseline(k) for k in known_issues)
        if not is_known:
            filtered.add(issue)
    return filtered


def filter_by_allowlist(result: LintResult, allowlist: dict) -> LintResult:
    """Filter out issues that are in the allowlist (intentional gaps).
    
    The allowlist supports two formats:
    1. Per-vehicle exact matches: {"vehicle_id": [{"check": ..., "parts": [...]}]}
    2. Global pattern matches: {"patterns": [{"part_pattern": "...", "reason": "..."}]}
    """
    patterns = []
    if "patterns" in allowlist:
        patterns = [p["part_pattern"] for p in allowlist["patterns"] if "part_pattern" in p]
    
    vehicle_allowed = allowlist.get(result.vehicle_id, [])
    vehicle_issues = [LintIssue.from_dict(d) for d in vehicle_allowed if isinstance(d, dict) and "check" in d]
    
    filtered = LintResult(vehicle_id=result.vehicle_id)
    for issue in result.issues:
        is_exact_match = any(issue.matches_baseline(a) for a in vehicle_issues)
        if is_exact_match:
            continue
        
        is_pattern_match = False
        for part in issue.parts:
            if any(_matches_pattern(part, pattern) for pattern in patterns):
                is_pattern_match = True
                break
        
        if not is_pattern_match:
            filtered.add(issue)
    
    return filtered


def format_issues(issues: list[LintIssue]) -> str:
    """Format issues for printing."""
    lines = []
    for issue in issues:
        parts_str = f" ({', '.join(issue.parts)})" if issue.parts else ""
        dist_str = f" [{issue.distance_m*1000:.1f}mm]" if issue.distance_m is not None else ""
        lines.append(f"  {issue.severity.upper()}: [{issue.check}]{parts_str} {issue.message}{dist_str}")
    return "\n".join(lines)
