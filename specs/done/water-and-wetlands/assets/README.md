# RedGPU reference stills

The targets the build judged our water against
([decision 20](../decisions.md#scope-and-acceptance)). Nothing here is ours,
and none of it ships.

- **`redgpu-depth-colour`, `-overhead-shadows`, `-wading-shore`, `-wake-rings`:**
  frames of the RedGPU V4.4.0-Alpha release tweet's 41 s video, at its own
  cameras (overhead, and near the water). They show the look at its best.
- **`redgpu-<demo>-<framing>`:** the live demos on GitHub Pages
  (`redcamel.github.io/RedGPU/examples/3d/water/lake/<demo>/`), filmed on
  2026-10-27 at our water reel's first three framings
  ([`fixtures/captures/water.json`](../../../../fixtures/captures/water.json)):
  close (65 m at 0.85 rad), tactical (260 m at 0.95 rad) and low (80 m at
  0.2 rad). `landscape` is `lakeWithLandscape` (a 300 m lake on a 400 m
  wooded landscape, by daylight), centred on the deep water nearest its
  soldier. `beach` is `characterWalking` (a 240 m lake over a sandy beach,
  under a sunset sky), centred the same way. Captured in headless Chromium
  148 with WebGPU (`web/browser.mjs`'s browser) at 1280×720, with the page's
  UI hidden and the soldier held still. The script served each demo's own
  `index.js` patched only to reach its orbit camera; it was a scratch script,
  not in the repository. The pages serve whatever RedGPU last deployed, which may be
  newer than the `95aaf9a9ecb3c5ccfcfc824ccad1a6489392cf68` tag the spec
  reads the code at.

Their sky and light differ from ours (the beach is lit by a sunset), so judge
the techniques against them — depth colour, the shore, the bed through the
water, highlights — and not their palette.
