"""Roster exporters take each variant's physical frame from the resolved catalogs."""
import json
import tempfile
import unittest
from pathlib import Path

from catalog_frames import disabled_variant, family_variants, requested_variant, roster_variant


def unit(type_id, appearance, half, eye, mounts):
    return dict(
        id=type_id,
        appearance=appearance,
        name=type_id.title(),
        faction="us",
        roster=dict(variant="Mk " + type_id[-1]),
        body=dict(hull=dict(half_extents_m=half, eye_m=eye)),
        mounts=mounts,
    )


GUN = dict(name="cannon", on=None, pivot_m=[0.0, 0.0, 1.5], muzzle_m=[5.0, 0.0, 0.4], weapons=["ap"])
ROOF = dict(name="HMG", on="cannon", pivot_m=[-0.2, 0.5, 2.3], muzzle_m=[1.4, 0.0, 0.3], weapons=["hmg"])


class CatalogFramesTest(unittest.TestCase):
    def setUp(self):
        self.repo = Path(tempfile.mkdtemp())
        (self.repo / "fixtures").mkdir()
        (self.repo / "assets").mkdir()
        self.units = [
            unit("tank_2", "look_2", [4.0, 1.8, 1.2], 2.3, [GUN, ROOF]),
            unit("tank_1", "look_1", [3.5, 1.6, 1.1], 2.1, [GUN]),
            unit("truck_1", "truck_look", [5.2, 1.2, 1.5], 2.8, []),
        ]
        self.appearances = {
            "look_2": dict(unit="vehicle", source="assets/source/roster/tank/look_2.glb",
                           mounts={"cannon": "gun", "HMG": "hmg"}),
            "look_1": dict(unit="vehicle", source="assets/source/roster/tank/look_1.glb",
                           mounts={"cannon": "gun"}),
            "truck_look": dict(unit="vehicle", source="assets/source/roster/truck/truck_look.glb"),
            "unbound": dict(unit="vehicle", source="assets/source/roster/tank/unbound.glb"),
            "tree": dict(unit="scenery", states={}),
        }
        self.write()

    def write(self):
        (self.repo / "fixtures/catalog.json").write_text(json.dumps(dict(units=self.units)))
        (self.repo / "assets/catalog.json").write_text(json.dumps(dict(appearances=self.appearances)))

    def test_a_variant_carries_the_drawing_units_frame(self):
        variant = roster_variant("look_2", repo=self.repo)
        self.assertEqual(variant["id"], "tank_2")
        self.assertEqual(variant["name"], "Tank_2")
        self.assertEqual(variant["variant"], "Mk 2")
        self.assertEqual(variant["faction"], "us")
        self.assertEqual(variant["export"], "assets/source/roster/tank/look_2.glb")
        frame = variant["frame"]
        self.assertEqual(frame["half_extents_m"], [4.0, 1.8, 1.2])
        self.assertEqual(frame["body_dimensions_m"], [8.0, 3.6, 2.4])
        self.assertEqual(frame["eye_m"], 2.3)
        self.assertEqual(frame["mounts"], [
            dict(name="cannon", role="gun", on=None, pivot_m=[0.0, 0.0, 1.5], muzzle_m=[5.0, 0.0, 0.4]),
            dict(name="HMG", role="hmg", on="cannon", pivot_m=[-0.2, 0.5, 2.3], muzzle_m=[1.4, 0.0, 0.3]),
        ])

    def test_a_family_is_every_appearance_in_its_source_folder(self):
        del self.appearances["unbound"]
        self.write()
        ids = [v["id"] for v in family_variants("tank", repo=self.repo)]
        self.assertEqual(ids, ["tank_1", "tank_2"])

    def test_an_appearance_no_unit_draws_has_no_frame(self):
        with self.assertRaisesRegex(LookupError, "unbound"):
            roster_variant("unbound", repo=self.repo)
        with self.assertRaisesRegex(LookupError, "unbound"):
            family_variants("tank", repo=self.repo)

    def test_an_unknown_appearance_is_refused_by_name(self):
        with self.assertRaisesRegex(LookupError, "nowhere"):
            roster_variant("nowhere", repo=self.repo)
        with self.assertRaisesRegex(LookupError, "nowhere"):
            family_variants("nowhere", repo=self.repo)

    def test_an_exporter_is_asked_for_an_appearance_or_a_file_named_after_one(self):
        variant, out = requested_variant("tank", ["look_1", "--preview=x"], repo=self.repo)
        self.assertEqual(variant["id"], "tank_1")
        self.assertEqual(out, str(self.repo / "assets/source/roster/tank/look_1.glb"))
        variant, out = requested_variant("tank", ["--preview=x", "/tmp/scratch/look_2.glb"], repo=self.repo)
        self.assertEqual(variant["id"], "tank_2")
        self.assertEqual(out, "/tmp/scratch/look_2.glb")

    def test_a_family_of_one_needs_no_name(self):
        variant, out = requested_variant("truck", ["--preview=x"], repo=self.repo)
        self.assertEqual(variant["id"], "truck_1")
        self.assertEqual(out, str(self.repo / "assets/source/roster/truck/truck_look.glb"))
        del self.appearances["unbound"]
        self.write()
        with self.assertRaisesRegex(LookupError, "tank"):
            requested_variant("tank", [], repo=self.repo)

    def test_an_exporter_refuses_another_familys_appearance(self):
        with self.assertRaisesRegex(LookupError, "truck_look"):
            requested_variant("tank", ["truck_look"], repo=self.repo)

    def test_a_mount_the_appearance_does_not_rig_has_no_role(self):
        self.appearances["look_2"]["mounts"] = {"cannon": "gun"}
        self.write()
        mounts = roster_variant("look_2", repo=self.repo)["frame"]["mounts"]
        self.assertEqual([m["role"] for m in mounts], ["gun", None])


if __name__ == "__main__":
    unittest.main()


class DisabledFramesTest(unittest.TestCase):
    """A disabled card has no unit type, so its frame comes from one of two
    named sources: its archived manifest frame, or dimensions its exporter
    states from references. Neither is guessed."""

    def setUp(self):
        self.repo = Path(tempfile.mkdtemp())
        (self.repo / "fixtures/units/roster").mkdir(parents=True)
        (self.repo / "specs/done/unit-roster/manifests").mkdir(parents=True)
        entries = [
            dict(id="tank_x", category="veh", source_path="assets/source/roster/disabled/tank_x.glb"),
            dict(id="jet_y", category="air", source_path="assets/source/roster/disabled/jet_y.glb"),
        ]
        (self.repo / "fixtures/units/model-manifest.json").write_text(json.dumps(dict(entries=entries)))
        cards = dict(units=dict(tank_x=dict(name="Tank X", faction="eastern"), jet_y=dict(name="Jet Y", faction="us")))
        (self.repo / "fixtures/units/roster/cards.json").write_text(json.dumps(cards))
        archived = dict(variants=[dict(id="tank_x", physical_authoring=dict(
            body_dimensions_m=[8.0, 3.4, 3.0], half_extents_m=[4.0, 1.7, 1.5], eye_m=3.0,
            mounts=[dict(name="cannon", role="gun", on=None, pivot_m=[0, 0, 2], muzzle_m=[5, 0, 0.5])]))])
        (self.repo / "specs/done/unit-roster/manifests/tank.json").write_text(json.dumps(archived))

    def test_a_card_with_an_archived_frame_takes_it_and_names_it(self):
        v = disabled_variant("tank_x", repo=self.repo)
        self.assertEqual(v["frame"]["half_extents_m"], [4.0, 1.7, 1.5])
        self.assertEqual(v["frame"]["mounts"][0]["role"], "gun")
        self.assertEqual(v["frame_source"], "specs/done/unit-roster/manifests/tank.json")
        self.assertEqual((v["faction"], v["export"]), ("eastern", "assets/source/roster/disabled/tank_x.glb"))

    def test_a_card_without_one_takes_the_dimensions_its_exporter_states(self):
        v = disabled_variant("jet_y", dimensions=[19.0, 13.0, 4.8], repo=self.repo)
        self.assertEqual(v["frame"]["body_dimensions_m"], [19.0, 13.0, 4.8])
        self.assertEqual(v["frame"]["half_extents_m"], [9.5, 6.5, 2.4])
        self.assertEqual(v["frame_source"], "references")

    def test_a_card_with_neither_is_refused_by_name(self):
        with self.assertRaisesRegex(LookupError, "jet_y"):
            disabled_variant("jet_y", repo=self.repo)
        with self.assertRaisesRegex(LookupError, "nope"):
            disabled_variant("nope", repo=self.repo)
