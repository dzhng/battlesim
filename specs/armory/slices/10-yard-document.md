# 10 · Yard document and spots, with a placeholder yard

Map rows: Q5, L14, plus P2. Depends on 04.

## Contract

K8. The yard is a presentation document. The simulation never admits it. It
names its spots and lists prop kinds, which are drawn with their catalog
appearance.

**Spots are chosen by body (P2):**

| Body | Spot |
|---|---|
| A squad | `tents` |
| A hull with a flight model | `apron` |
| Any other hull | `motor_pool` |

**Until the military set lands (slice 16), the yard is built from existing
props:** `sandbags`, `jersey_barrier`, `crate`, `pallet_stack`, `site_cabin`,
`heras_fence` and `tooth`.

## Seam

**The data.** `fixtures/armory/yard.json` is a new file of authored
presentation data, like `fixtures/presentation.json` and
`fixtures/captures/menu.json`. It isn't a catalog change, so L1 doesn't hold
it. Add a short "yard" entry to `fixtures/README.md`.

**The type and reader**, in `apps/battle-lab/src/armory/yard.ts`:

```ts
type SpotId = "apron" | "motor_pool" | "tents";
type Body = "squad" | "aircraft" | "ground";
interface YardDocument {
  ground: { half_m: number; tint: Rgba };
  spots: Record<SpotId, { at: [number, number]; yaw: number; clear_m: number; camera: CameraPose }>;
  bodies: Record<Body, SpotId>;                       // P2
  props: { kind: string; at: [number, number]; yaw: number }[];
}
export function bodyOf(unit: UnitType): Body;         // squad body / flight model / else
export function yardModels(doc: YardDocument, installed: InstalledAppearances): LabModel[]; // static poses
export function spotFor(doc: YardDocument, unit: UnitType | null): Spot | null;  // null: planned card
```

- **Appearances.** `propsDrawnBy` (`packages/scene-assets/src/scenery.ts`) maps
  a kind to its appearance. Call it; don't copy it.
- **The lab.** `/labs/showcase?yard=1` stands the showcased unit on its spot in
  the yard.
- **References.** Gather the yard references now, into `assets/references/`
  with `references.json`: WARNO and Broken Arrow armory screens, and real
  flight-line and motor-pool photos.

## The human runs or sees

`/labs/showcase?yard=1&unit=<kind>`, once for a tank, a helicopter, a jet and a
squad.

## Verification

Write these first, in `web/tests/armoryYard.test.ts`:
- `every yard prop is a catalog prop kind with an appearance`;
- `no yard prop stands inside any spot's clear disc`;
- `every body has a spot`;
- `every unit fits its spot's clear disc` (generative): hulls by
  `half_extents_m`, squads by their farthest `Showground.squad` spot;
- `a recon squad stands at the tents and a recon vehicle in the motor pool`,
  using fabricated cards. This proves that P2 reads the body.

**Visual gate.**
- **The one variable:** composition per spot. Dressing stands behind and beside
  the unit, never between it and the default camera.
- **Crop:** each spot's default camera frame, with the UI masked.
- `compare-screenshots` against the gathered WARNO and Broken Arrow references
  and `assets/mock/a-tank.png`.
- `screenshot-critique` last.
- Then the non-blocking `preview-shots` checkpoint.

## Delegated to the implementer

- Prop placement.
- The ground tint.
- The camera poses for each spot.

## Must stay green

- The scene from slice 04.
- `catalogFootprints.test.ts`. The yard only places catalog kinds, so it adds
  no prop type.

## Feedback that would change it

The layout is data in `yard.json`.
