# 04 · Showcase lab: the spike, end to end

Map rows: Q1′, Q6, D1. Depends on 02 and 03.

## Contract

One TypeScript owner drives the showcase on a `BrowserScene`: `ShowcaseStage`.
Slice 11 mounts the same class. This is where the human first sees idle life.

## Seam

`web/src/battle/render/showcaseStage.ts`:

```ts
export interface StageUnit { kind: string; faction: Faction; at: Vec3; yaw: number }
export class ShowcaseStage {
  static create(o: { gpu: BrowserGpu; canvas: HTMLCanvasElement; world: WorldLayers;
                     appearances: InstalledAppearances; fielding: FieldingInput;
                     showground: Showground }): Promise<ShowcaseStage>;
  show(unit: StageUnit | null, seed: number): IdleLoop[];   // returns the loops to play
  frame(time: number, camera: PresentedCamera): void;
  dispose(): void;
}
```

- **Presentation wrapper.** `ScenePresentation` (`web/src/battle/sim/presentation.ts:217`) gains `setShowcase` and `clearShowcase`, beside `setLabModels` (`:367`).
- **Ground.** The flat ground is `flatTerrain(benchGround(false))` from `apps/battle-lab/src/workbench/benchWorld.ts:68,104`. Reuse it; don't copy it.
- **GPU.** Use `appResources.gpu()`. Never call `BrowserGpu.new()`.
- **The lab page.**
  - It is a test page: fixture `showcase` in `apps/battle-lab/src/fixtures.json`, route `/labs/showcase?unit=<kind>&faction=<f>`, page `apps/battle-lab/src/routes/showcase.tsx`, scene `web/scenes/showcase.mjs`.
  - It may use `LabViewport` for its camera. The Armory camera belongs to slice 12.
- **Not a duplicate.** `SheetRenderer` (`workbench/sheet.ts`) stays the workbench's one-shot sheet readback, with page-posed models. That is a different job.

## The human runs or sees

`/labs/showcase` shows three units side by side, each chosen by a property,
never by an id:
- the first tracked hull;
- the first rotorcraft;
- the first squad that has a special mount.

`?unit=` picks any kind.

## Verification

Probes in `web/scenes/showcase.mjs`:
- **Rotor:** the pixel delta between two frames inside the rotor-disc mask is
  greater than 0.
- **Squad:**
  - the soldiers aren't collinear (the scatter);
  - poses differ between soldiers;
  - the Javelin or launcher operator shows his carried kit.
- **Grounded:** every model's lowest point is within 2 cm of the ground.
- **Parked aircraft** (the map's "Confirm before coding" item 2, which needs
  `git lfs pull` for the aircraft bundles):
  - For every aircraft appearance in its rest pose, no non-gear node lies
    within 2 cm of the ground.
  - Write the report to `throwaway/armory/04/aircraft-ground.json`.
  - Name any failure to the user. Its art fix belongs to that model's next
    pass, not here.

`web/tests/browserModule.test.ts` covers `setShowcase` and `clearShowcase`
through the real wasm.

**Visual gate.**
- **The one variable:** idle life reads as alive and grounded.
- **Crops:**
  - the helicopter's bounds, side-on, rotors in frame;
  - the squad from 30° above.
- **Compare:**
  - `compare-screenshots` the helicopter against a reference photo of a parked
    attack or utility helicopter with its rotors turning, stored in
    `assets/references/`;
  - compare the rotor disc against the battle's airborne blur (the `air-hover`
    scene), the prior look.
- `screenshot-critique` is the last check on each crop.
- Then the non-blocking `preview-shots` checkpoint.

## Delegated to the implementer

- The lab's controls.
- The camera framing.
- The choice of reference photo.

## Must stay green

- `web/tests/browserScene.test.ts` and `sceneAppearances.test.ts`.
- The `workbench`, `air-hover` and `air-crash` scenes.
- `_appJourney.mjs`'s street-lab digest.

## Feedback that would change it

**"The rotors look wrong parked"** (for example, too fast).
- That value is `presentation.json` `rotor.tip_mps`, which the battle shares.
- Retuning it for the Armory alone would make a second owner. Raise it with the
  user as a decision.

**"The soldiers look like statues."** Tune the driver's idle-feel data
(`presentation.json` `pose.rest`). Never add page code.
