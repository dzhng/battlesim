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
