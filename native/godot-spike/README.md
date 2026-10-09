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

