# 02 — Camera, projection, and framing parity

**Contract:** native and browser project the same resolved world pose to the same viewport.

**Seam:** `camera3d.ts`, `cameraUniform.ts`, `cameraController.ts`, `gameCamera.ts`, menu reel capture, and native `_captured_pose`/`_apply_pose`. Carry target elevation, distance, yaw, pitch, FOV, near/far/depth convention, viewport/DPR, follow lag, wall-clock interpolation, and coordinate conversion explicitly.

**Runnable checkpoint:** matched 1280×720 browser/native cuts with camera anchors, horizon, ground split, and subject boxes overlaid.

**Focused gates:** projection tests against known points; camera timeline/frame matching; custom up-vector and z-up conversion tests; no clearance drift; no crop-after-render comparison; digest/replay unchanged.

**Visual variable:** framing and projection only. Materials, grass density, lighting, sky, and effects are out of scope.

**Acceptance evidence:** fixed pixel tolerances for anchors and subject boxes, paired broad and close crops, `compare-screenshots`, unprimed `screenshot-critique`, and `preview-shots`.
