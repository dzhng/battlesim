# C62: ground evidence rig

**Depends on:** C60. **Kind:** slice.

## Question
Can every ground slice shoot one frozen station, get an exact mask of the ground class it judges, and get the same pixels twice?

## Contract it unlocks
- **Stations:** `web/scenes/_groundStations.mjs` holds named poses per map id. For the village:
  - `bend-25/65/120/250` at the (420,420) road corner;
  - `forest-edge-65` and `forest-deep-25` in the west wood;
  - `field-65/250` and `patchwork-1100`.

  Lab maps add theirs.
- **A `FrameView` `ground-classes`,** written by the terrain material's own shading function (not hand-drawn). Per pixel it encodes road signed-distance bands, river bands, forest (inside, verge or none), plot kind and a hashed plot id.
- A `suppressTrees` toggle beside `suppressModels` (trees are scenery).
- The frozen set: fixture light, `setClock(T0)`, biome seed, no units, scars and paint suppressed, DPR 1 at 1920×1080.
- Baseline shots of every station on main before any ground change. These are each sub-lane's first "before".

## API seam
`packages/battle-renderer/src/scene.ts` (`FrameView`), `apps/battle-lab/src/LabViewport.tsx`, `web/scenes/_groundStations.mjs`.

## What the human can run or see
A station sheet: each shot beside its class mask.

## Verification
- Two runs give byte-identical masks and final shots within tolerance.
- The mask agrees with the sim export at 1,000 sampled pixels.
- The mask view adds 0 ms to the final view.

## Delegated to the implementer
Mask encoding; stations beyond the village minimum. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The final view's pixels; every scene.

## Feedback that would change this slice
A camera station that hides the judged variable changes the evidence rig, preserving fixed comparison inputs.

## Outcome

**Changed since.** [C67](C67-road-shoulder.md), [C70](C70-river-bank-bands.md) and [C30](C30-markings.md) widened the reach the Open item names: the mask's distances are now exact to 4.3 m from a road and 5 m from water. The rig's module also owns what the later slices' checks share (a station's three frames, the mask's pixels, a lab switch's paired frame cost), with the colour measures in `web/scenes/_colour.mjs`.

**Built.** `web/scenes/_groundStations.mjs` is the rig: named poses per map, one frozen page (`openStations`), one way to take a frame (`shoot`), and the mask's decoder (`classAt`). `STATIONS=village,river,generated bun run --cwd web scene -- ground` writes every station's shot, its bare ground (no grass, no trees), its class mask and a sheet per map into `throwaway/evidence/ground/`.

**The seam.**
- **`FrameView` "ground-classes"** (`scene.ts`). The terrain fragment writes `groundClasses` in place of its lit colour, from the same site, water and footprint `groundColour` paints by (`terrainMaterial.ts`); the encoding is `terrain/groundClasses.ts`. Three bytes: metres outside the paved edge, metres outside the water's edge (eighths of a metre, negative inside, held at about 15.9 either way), then forest (none, verge, inside), the plot's kind and two hashed bits of its index. Black is "not ground": a pixel any part of which is a building, a tree, a unit or past the map's edge. Grass and the water surface are left out of that view so the ground shows.
- The bytes reach the screen untouched: the fog mask pass passes them through where every sample of the pixel is ground, and post gained a third mode beside the look and the tone-map-only masks (`PostMode` "raw").
- **`BattleFrame.setTreesShown`** and the lab's `suppressTrees`: the forest's and the backdrop's trees and their shadows, apart from the models' switch.
- **Stations.** Village: `bend-25/65/120/250`, `forest-edge-65`, `forest-deep-25`, `field-65/250`, `patchwork-1100`. River lab: `bend-25/65`, `wide-65`, `track-25/65`, `junction-65`, `bridge-65`, `wood-65`. Generated (`mixed`, `medium`, seed 2): `overview-2500`, `town-250/65`, `country-250/65/25`, placed on what the map's preparation reports (the objective town, blue's start), so they follow the generator.
- **The frozen set:** the route's light, tick 12 with the clock at rest on it, 1920 × 1080 at DPR 1; models, effects, cast lights, scars, paint, fog and the overlay glow off.

**Verified** (three checks in the `ground` scene, `RIG_ONLY=1` runs them alone in 11 s):
- A station shot twice gives the same mask and the same bare ground, byte for byte (two stations). Frames with grass are not compared: blade depth ties differ between runs of one build (C63).
- The mask agrees with the simulation's export at 1,002 seeded ground pixels over three stations (65 on the road, 36 in the wood): the road distance within a byte's step and the pixel's own width, the side of the road's edge and the forest's edge exactly.
- The trees switch off on their own (40% of the wood-edge frame changes; 687 trunks placed).
- The final view: one uniform test in the terrain and backdrop fragments and one branch on the CPU. Not measured: it is below what a paired run resolves on this machine.

**What the baseline shows** (the lane's first "before"; sheets regenerate from main at `ca7bb972` plus this slice):
- Roads are one flat pale band with a green verge line each side, the same on the dirt track and the country road, and on town streets.
- The forest floor is a blocky brown and green camouflage at every distance, and reads as noise through the crowns.
- Crowns are smooth blobs; a wood from above is one texture.
- The river's bank is a thin brown line; water is flat grey from above.
- Generated maps: the patchwork is drawn, but fan-shaped round the map's centre, and the town's streets are the same pale band between massing boxes.

**Open.** The mask's distances are exact only as far as the ground's look reads them (`groundReach`: under 2 m from a road at a play-camera pixel) and hold their side beyond; a slice that paints a wider band (C67's shoulder, C70's bank) widens the reach and the mask follows. `forest-deep-25` stands above the canopy at 0.6 rad: at the ground view's pitch the eye sits inside a crown.
