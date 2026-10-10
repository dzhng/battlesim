# Godot reference-scene spike

This is a deliberately small native rendering probe. It establishes that a
pinned Godot runtime can open a desktop project, batch thousands of 3D instances
through `MultiMeshInstance3D`, move a camera over them, and emit the same basic
average/minimum/maximum/1%-low frame-rate fields used by the browser report.

It is not the battle renderer and it does not claim simulation or visual parity.
The Rust binding and full menu-reel comparison remain the later slices in
[`specs/native-rendering`](../../specs/native-rendering/README.md).

Run it with the pinned local binary:

```text
throwaway/tools/godot-4.7.2/Godot.app/Contents/MacOS/Godot --path native/godot-spike --editor --quit --no-header
throwaway/tools/godot-4.7.2/Godot.app/Contents/MacOS/Godot --path native/godot-spike
```

The interactive run writes `user://godot-spike-report.json` and prints the JSON
summary. The spike intentionally keeps its report separate from the browser
benchmark until the workload and binding seams are matched.


## Authored menu-reel camera probe

`reel.tscn` reads the browser-owned `fixtures/menu-backdrop.json` directly (or
 the path in `GODOT_REEL_SOURCE`) and plays every scene and shot with the same
wall-clock camera interpolation. It is a seam check, not a renderer result:
the world is still the synthetic cube field, and its report sets
`comparison_ready` to `false` until Godot consumes a real
`battle-presentation-capture/v1` publication and the authored map assets.

For a short lifecycle check, run with `GODOT_REEL_TIME_SCALE=0.01` and write
the report outside `user://` with
`GODOT_REEL_REPORT=throwaway/godot-reel.json`.

To exercise the authored-scene path, hydrate one scene asset with Git LFS and
copy it into the Godot project (Godot imports `.glb` as a `PackedScene`):

```text
git lfs pull --include="assets/source/city/homes/kit.glb"
mkdir -p native/godot-spike/authoring
cp assets/source/city/homes/kit.glb native/godot-spike/authoring/homes-kit.glb
GODOT_AUTHORED_SCENE=res://authoring/homes-kit.glb \
  GODOT_PRESENTATION_CAPTURE_DIR=throwaway/browser-captures \
  GODOT_REEL_REPORT=throwaway/godot-reel.json \
  godot --headless --path native/godot-spike --scene res://reel.tscn --quit-after 600 --no-header
```

The capture directory contains one browser-exported JSON file per scene, named
after the menu map. The copied asset is local evidence and stays outside the
committed spike. The report records `authored_asset_loaded`, the number of
capture scenes consumed, the number of map placements and the authored scene
path; `comparison_ready` remains false until the terrain, props and observed
units are composed with the actual menu-world asset catalog. Set
`GODOT_AUTHORED_BUILDING_LIMIT` to a positive value for a deliberately bounded
diagnostic run; an unset value or `0` loads every admitted building.

The reel consumes the authoritative saved-map exports directly by default
(`fixtures/maps`); `GODOT_AUTHORED_MAP_DIR` can point at another export. It loads the complete terrain, road,
building, forest, and prop geometry from `map.json`, and decodes sampled unit poses
from the presentation capture. `GODOT_MAP_BUILDING_LIMIT`,
`GODOT_MAP_PROP_LIMIT`, and `GODOT_MAP_ROAD_LIMIT` are optional explicit
diagnostic caps; an unset value or `0` means the full export. The report records
those limits and whether the synthetic proxy field was used. These primitives
are composition evidence, not authored-material or visual-parity evidence.

For menu composition, pass `GODOT_AUTHORED_SCENES` with the apartment, homes,
farmsteads, industry, and towers kits. The loader selects a shell from the
template id, so one kit is never instantiated wholesale at every building.

Catalog scenery and unit GLBs must be staged under the Godot project so its
importer can turn them into `PackedScene` resources. Set
`GODOT_NATIVE_ASSET_ROOT=res://native-assets` when using that staged tree;
the report leaves an asset instance at zero when a source is only a repository
file or an unfetched Git-LFS pointer. `GODOT_NATIVE_ASSET_INSTANCE_LIMIT`
controls authored props, and `GODOT_NATIVE_GRASS_INSTANCE_LIMIT` controls the
separate grass budget. Grass samples follow admitted forest polygons from the
saved map and are presentation geometry only. After a human has inspected the
display-backed cuts, set `GODOT_AUTHORED_MATERIALS_REVIEWED=1` on the evidence
run. The report records that attestation separately from `comparison_ready`:
the former says the authored materials were inspected, while the latter stays
false when the visual comparison still rejects parity.
