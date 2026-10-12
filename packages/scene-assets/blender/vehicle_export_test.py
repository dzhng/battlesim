"""Exporter selection and receipts without launching Blender or writing art."""
import importlib.util
import json
import tempfile
import sys
import types
import unittest
from pathlib import Path
from unittest.mock import patch


def exporter():
    parts = types.ModuleType("parts")
    for name in ("bare_steel", "empty", "finish", "flat_paint", "glass", "reset",
                 "script_args", "textured", "track_steel", "tyre", "triangles_by_tier"):
        setattr(parts, name, None)
    wreckage = types.ModuleType("wreckage")
    for name in ("WRECK_ARG", "burn", "export_wreck", "scatter"):
        setattr(wreckage, name, None)
    mathutils = types.ModuleType("mathutils")
    mathutils.Vector = None
    spec = importlib.util.spec_from_file_location("vehicle_export", Path(__file__).with_name("vehicle_export.py"))
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {"bpy": types.ModuleType("bpy"), "mathutils": mathutils,
                                  "parts": parts, "wreckage": wreckage}):
        spec.loader.exec_module(module)
    return module


class VehicleExportTest(unittest.TestCase):
    def test_one_export_and_receipt_cover_variants_in_several_source_folders(self):
        module = exporter()
        with tempfile.TemporaryDirectory() as folder:
            repo = Path(folder)
            (repo / "fixtures").mkdir()
            (repo / "assets").mkdir()
            units = [dict(id=name, appearance=name, name=name, faction="us", roster=dict(variant=name),
                          body=dict(hull=dict(half_extents_m=[2, 1, 1], eye_m=1.5)), mounts=[])
                     for name in ("first", "second")]
            (repo / "fixtures/catalog.json").write_text(json.dumps(dict(units=units)))
            (repo / "assets/catalog.json").write_text(json.dumps(dict(appearances={
                "first": dict(source="assets/source/roster/one/first.glb"),
                "second": dict(source="assets/source/roster/two/second.glb")})))
            receipt = [dict(id="first", sha256="a"), dict(id="second", sha256="b")]
            with patch.object(module, "REPO", repo), \
                 patch.object(module, "_export", return_value=receipt) as build, \
                 patch.object(module, "_receipt") as save:
                module.run("one", "paint", None, None,
                           appearances=["first", "second"], script="shared.py",
                           references=["assets/references/one/references.json", "assets/references/two/references.json"])
        self.assertEqual(build.call_count, 1)
        self.assertEqual([v["id"] for v in build.call_args.args[0]], ["first", "second"])
        self.assertEqual([v["frame"]["body_dimensions_m"] for v in build.call_args.args[0]], [[4, 2, 2], [4, 2, 2]])
        self.assertEqual(save.call_args.args[:4],
                         ("assets/source/roster/one/source-receipt.json", "shared.py", receipt, "fixtures/catalog.json"))


if __name__ == "__main__":
    unittest.main()
