# Choices

Decisions made during implementation where the spec was silent, each with a
verdict. [`decisions.md`](decisions.md) holds the user's givens; this file holds
what the build decided on their behalf. Banked entries are settled for later
passes.

## Slice 00: the water scene

### The wetlands map is a `capture` map, so there is no `/labs/wetlands`

- **When:** slice 00.
- **The choice:** the repository has two kinds of saved map: `test` maps,
  which labs and tests open, and `capture` maps, which the capture tool and
  the graphics test film. Each reader refuses the other kind. The spec asked
  for one map that is both a lab (`/labs/wetlands`) and a filmed capture
  (`water.json`), which the readers forbid. Wetlands is a capture map, so it
  can be filmed and replayed on native. Quick looks go through
  `/labs/capture?capture=water&scene=0` and the water scene instead of a lab.
- **The gap:** the spec didn't know about the category rule.
- **The reach:** slices 03, 10 and 15 named the lab. They now point at the
  water scene's page. Slice 15's debug buttons need a lab, which must stand on
  its own `test` copy of the wetlands ground.
- **Verdict:** sound. The repository's rule says to copy a map rather than
  share it across categories, and filming is what this map is for.
- **Confidence:** high.

### The graphics test films any reel named in `reels.ts`, picked with `?capture=`

- **When:** slice 00.
- **The choice:** the native client replays only runs the graphics test
  records, and the graphics test filmed only the main menu's reel. To get the
  water shots onto native, the test now takes `?capture=water`, the menu's
  reel by default. The module that named the menu's reel (`menuReel.ts`) is
  now `reels.ts` and names both. The alternative was a second recorder just
  for water, which would duplicate the parity format.
- **The gap:** the spec said to use "the existing capture format" and
  `native-parity-capture.mjs`, but not how a non-menu capture reaches them.
- **The reach:** any future filmed scene that native must replay adds one
  line to `REELS`. A water run's report names its own workload id, `water`,
  so it never passes for a menu-reel measurement.
- **Verdict:** sound.
- **Confidence:** high.

### The three framings are one scene of three held shots

- **When:** slice 00.
- **The choice:** the parity run writes each scene's files under the scene's
  map name, so three scenes on the wetlands map would overwrite each other.
  The reel is one scene with three held shots (close, tactical, low), and
  each gets a control plate at its middle.
- **The gap:** the spec said "three framings" and left their shape open.
- **The reach:** slice 16's wave warm-up and any added framing go into this
  one scene. The scene names shots by order (`SHOTS` in `water.mjs`), because
  the capture format has no shot names.
- **Verdict:** sound.
- **Confidence:** high.

### The water stills agree within a tolerance, not byte for byte

- **When:** slice 00.
- **The choice:** the graphics test runs on the wall clock. A plate lands at
  "the shot's middle" give or take a frame, about 10–20 ms of animation, so
  grass and idle soldiers move a pixel or two between runs. The spec's
  "identical stills" is met within a baseline's tolerance (it forgives where
  a thing is drawn by a pixel or two), not byte for byte. Native replays the
  browser's exact frame records, so the browser–native comparison is exact.
  The alternative was a fixed-step clock for plated runs, a change to the
  shared reel runner that the parity tool doesn't need.
- **The gap:** the spec assumed a deterministic still.
- **The reach:** a baseline blessed from this scene must use the existing
  position-forgiving match, never an exact one.
- **Verdict:** sound for now. Revisit if a slice needs exact stills, for
  example wave fields compared across runs in slice 16.
- **Confidence:** medium.

### "Native draws water" means the river is bluer than its bank

- **When:** slice 00.
- **The choice:** the native draw check projects the simulation's river into
  each plate and compares the river's mean colour with that of the dry
  ground 8–24 m outside its edge. The river must be at least 12 lower on
  CIELAB's yellow-blue axis (b*). Water measures 27–32 there, and the bare
  earth a missing water surface would show measures 5–7. A plain "different
  colour from the bank" test was rejected, because the brown bed already
  differs from grass.
- **The gap:** the spec said "fails if native draws no water", with no measure.
- **The reach:** later look slices must keep river water bluer than grass by
  that margin. A murky variant (decision 9) that drifts toward the bank's hue
  would trip it. If so, re-judge the measure; don't lower the floor to pass.
- **Verdict:** sound.
- **Confidence:** medium.

### The water scene is part of the harness, and it builds and runs native

- **When:** slice 00.
- **The choice:** `water` is a registered scene, not a `tool`. So `verify`
  runs it, and it builds the native client (`cargo build -p native-client`)
  and replays on it. It is marked `serial`, because native shares the GPU.
  Decision 19 says every slice proves native still runs, and a tool scene
  would let that check be skipped silently.
- **The gap:** the spec named the check, not where it runs.
- **The reach:** `verify` now needs the native client to build on the
  machine. It costs about 45 s, plus native's build when that is stale.
- **Verdict:** sound.
- **Confidence:** medium.

### RedGPU's reference comes from two live demos, at our framings

- **When:** slice 00.
- **The choice:** the frozen reference is `lakeWithLandscape` (a wooded lake
  by daylight) and `characterWalking` (a beach lake at sunset), filmed from
  their live GitHub Pages site at our three framings. The page's UI is
  hidden, the soldier is held still, and the camera is centred on deep water
  near the soldier. The live site may run a build newer than the spec's tag.
  The alternative was building the tag locally, which needs their toolchain.
- **The gap:** the spec left the capture method to the implementer.
- **The reach:** decision 20's side-by-side read uses these and the video
  stills. Their light differs from ours, so later comparisons judge
  technique, not palette.
- **Verdict:** sound.
- **Confidence:** medium.
