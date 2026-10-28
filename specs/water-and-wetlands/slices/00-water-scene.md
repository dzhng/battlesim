# 00: The water scene and the frozen reference

**Lane:** render. **Depends on:** nothing. **Visual variable:** none (this slice makes the targets).
**Status:** done (2026-10-27).

## Contract

There is one fixed place to judge water, shared by every visual slice and by
native:

- **Map:** `fixtures/maps/wetlands/`, a `capture` map (decision 31): a river
  that widens from 14 m at a country road's bridge to a 30 m stretch,
  carved with its bank, a dirt track, a small wood, and flat open ground
  south-east of the river, kept clear for slice 10's swamp. Its encounter
  `water` stands a capture rifle squad and a capture tank on the south bank,
  at the water's edge. It is a capture map, not a `test` map, because the
  capture tool and the graphics test film only capture maps; a lab, which
  opens only `test` maps, would need its own `test` copy of this ground
  ([saved maps](../../../fixtures/README.md#saved-maps)).
- **Shots:** `fixtures/captures/water.json` owns the three framings, as one
  scene of three held shots, in order: close (65 m at 0.85 rad), tactical
  (260 m at 0.95 rad) and low (80 m at 0.2 rad, looking east along the wide
  stretch). It is the ordinary capture format. Slice 16 adds wave warm-up to
  the same file.
- **Filming:** the graphics test films any reel that `reels.ts` names, chosen
  with `?capture=` (the menu's by default):
  `/labs/graphics-test?capture=water&plates=1` reads a control plate back at
  each shot's middle. `/labs/capture?capture=water&scene=0` plays the shots
  live.
- **Scene:** `bun run --cwd web scene -- water` (`web/scenes/water.mjs`) films
  the reel through the shared parity run (`web/scenes/_parityRun.mjs`, also
  the `native-parity-capture` tool's), replays it on the native client, and
  checks that every plate in both clients shows the simulation's river
  bluer than its bank. It writes `throwaway/evidence/water/water.png`, our
  plates beside the RedGPU stills. Run it under the GPU lock.
- **Water mask:** `web/scenes/_water.mjs` owns where the simulation says the
  water is (`waterInside`, over the exported river stretches) and which pixels
  of a parity frame show it. `river.mjs` installs the same function in its
  page. There is one copy.
- **Frozen reference:** RedGPU's live `lakeWithLandscape` and
  `characterWalking` demos, filmed at the same three framings, are in
  [`../assets/`](../assets/) as `redgpu-<landscape|beach>-<framing>.jpg`. The
  [assets README](../assets/README.md) records how they were captured.
- **Native draw check:** the water scene's native replay. It fails if native
  doesn't start, counts differently from the browser, or draws no water.

## What changed from the plan

- Wetlands is a capture map, so there is no `/labs/wetlands` lab. The water
  scene's page and the capture route stand in for it, and later slices point
  there.
- The shots are one scene, not three: the parity run names its folders by
  map, so three scenes on one map would overwrite each other.
- The graphics test runs on the wall clock, so a plate lands a few
  milliseconds apart from one run to the next. Grass and idle soldiers move a
  pixel or two, and two runs' stills are not byte-identical. They agree
  within a baseline's tolerance. Native replays the browser's own frame
  records, so the browser–native comparison stays exact.

## Verification (done)

1. Three runs gave the same framings, and the same water-against-bank reading
   to a tenth.
2. The native draw check passes on today's water. The browser and native
   plates match, and native's counts equal the browser's on every plate.
3. The `river` scene passes with the shared mask.
4. An unprimed `screenshot-critique` found that all three read as their
   framings, with water the clear subject in close and low and a secondary
   one in tactical. It noted the squad stood too far from the water to give
   the close shot scale, so the encounter moved the squad and tank to the
   bank.
5. Checkpoint: the sheet and plates were shown with `preview-shots`. No
   feedback came within the wait, so the framings stand.

## Must stay green

All battle digests and replays, every existing scene and baseline, and native
pipeline validation.

## Feedback that would change this slice

If the user wants a different hero framing, change the shot records. The
slices that follow read them.
