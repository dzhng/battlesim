# 01 — Backend admission and presentation manifest

**Contract:** choose the native Godot renderer backend and define `native-visual-presentation/v1`, the browser-owned evaluated presentation record consumed by native.

**Seam:** browser export beside `battle-presentation-capture/v1`; native `presentation_capture.gd` and a single adapter in `reel.gd`. Include workload/map/capture hashes, viewport/DPR/plate crop, camera lens and projection fingerprint, resolved geometry/appearance ids, environment values, fog/ground references, and diagnostic fallback/omission rows.

**Runnable checkpoint:** one display-backed market-town cut loads through the manifest; missing asset, LFS pointer, wrong hash, malformed manifest, or backend capability failure is explicit and stops parity mode. Headless remains lifecycle-only.

**Focused gates:** capture parser tests, native decoder/lifecycle tests, manifest hash/identity tests, digest/replay unchanged, backend capability probe with one building/grass/shadow/sky cut, and resource cleanup.

**Visual variable:** none beyond valid content and display backing. Terrain, camera and materials are out of scope.

**Required visual gates:** `compare-screenshots`, unprimed `screenshot-critique`, and non-blocking `preview-shots` review of the single checkpoint.

**Delegated decisions:** internal manifest packing and Godot resource cache shape. Do not delegate ownership, fallback semantics, or backend admission.
