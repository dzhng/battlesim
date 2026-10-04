# Courtyards: paved, lived-in block interiors

A generated town's streets carry furniture, but the ground between its buildings is one unbroken lawn in every region ([before shots](assets/before/)). This plan paves the dense districts' blocks as each region's town would and dresses them with amenities that read as lived in and fight as cover, and gives garden suburbs and villages garden dressing on their lawns.

## Next Agent Prompt

*Status (2026-10-04): planned, nothing implemented. Worktree `.claude/worktrees/courtyards`, branch `courtyards` off `main` 2c9dee78.*

You are implementing this spec with [implement-spec](../../.agents/skills/implement-spec/SKILL.md). Start at [slice 00](slices/00-budget-and-stand-ins.md); it only measures and decides two numbers, and every later slice inherits them. Work the ladder in order unless the graph below says a slice can run beside another. Record every decision the slices leave open in [choices.md](choices.md) as you make it. Before ending a pass, update this section: status, the next pickup point, and the checklist.

- [x] 00 Measure the scenery download and prove stand-in bodies ([slice](slices/00-budget-and-stand-ins.md))
- [x] 01 The legality check gets its own module, no behaviour change ([slice](slices/01-field-module.md))
- [ ] 02 One paved kind for hard ground that is no way through ([slice](slices/02-paving-kind.md))
- [ ] 03 Dense blocks paved ([slice](slices/03-courts.md))
- [ ] 04 The map names its region; paving takes the region's finish ([slice](slices/04-map-region.md))
- [ ] 05 Court amenities as bodies, with stand-in art ([slice](slices/05-court-amenities.md))
- [ ] 06 Garden dressing in suburbs and villages ([slice](slices/06-gardens.md))
- [ ] 07 Regional appearances, loaded for the map's region only ([slice](slices/07-regional-appearances.md))
- [ ] 08 Art: shared core, then China, New York, Paris ([slice](slices/08-art.md))
- [ ] 09 Closeout: parity, full run, balance, close-spec ([slice](slices/09-closeout.md))

## Decided with the user (2026-10-04)

1. **Where.** Dense districts (`small_centre`, `centre`, `apartments`, `core` in [map-presets.json](../../fixtures/map-presets.json)) get paved courts dressed with amenities. Garden suburbs and villages keep grass and gain garden dressing: sheds, fences, hedges, washing lines, garden furniture. (Planner's call, confirm at slice 03 review: farms unchanged, because their open ground is the open approach the open-country coverage proof is tuned for; industry is paved through its aprons and keeps its own yard stock, with no amenities.)
2. **Physics.** Every amenity is an ordinary catalog prop with a body, blocking and giving cover by its physical properties, the same rule as street furniture. Town battles change; parity records are re-blessed.
3. **Art.** A shared core (benches, bins, bike racks, playground) dressed per region, plus signature pieces per region: China (bike shed, concrete ping-pong tables, outdoor gym, laundry poles), New York (basketball court, chain-link, garages, a dumpster yard where the fire escapes look down), Paris (gravel square, kiosk, pétanque, plane trees).
4. **No backward compatibility.** Hard cutover: the presets revision bumps, parity is re-blessed, and old generated replays stop matching.
5. **Reachability (from the user's question whether units can enter a court).** Today any open ground, lawns included, is walkable and drivable; only bodies block (`crates/sim/src/navigation.rs`). Dressing a court must not seal it: every court keeps a way in for infantry and one for vehicles, held by a test against the simulation's own navigation.

## Design

Three independent drafts (fewest slices, risk first, seam quality) were merged into this plan; where they split, the call and the losing alternative are below.

- **The block interior is the district.** A district is one block a road's depth deep, with its streets laid inside it (`layout/towns/mod.rs`, `parcels/streets.rs`). Paving the district ring as one polygon puts courts everywhere between a block's buildings without polygon booleans (the repo has no clipper); its streets draw over it because a carriageway wins where paved kinds overlap. Edges of the ring that face open country and carry no carriageway are drawn in to the last lot's rear, so paving never runs out into the fields. Parks and greens are not districts and stay grass. *Rejected:* the convex hull of a district's lots (a district need not be convex), and a 1 m raster of the ring clear of carriageways (correct but a new algorithm and many ground points for edges the street paving already hides).
- **One paved kind for hard ground that is no way through: `paving`.** The terrain shader holds exactly four paved kinds (`roadOrder: vec4u`, `groundPaved`, in `frame/terrainMaterial.ts`); a fifth means rewriting it. Generated plans never emit `SurfaceKind::Sidewalk` (the renderer draws walks off road strokes), so that slot is renamed `paving` and takes courts and the parcel pass's aprons, which today are tagged `Road`: one owner for paved ground you do not drive along. `is_road()` becomes an explicit match on the carriageway kinds. *Rejected:* four material kinds (`concrete`, `blacktop`, `gravel`, `setts`), and one new `court` kind; both need the shader widened, and material is a look, not physics.
- **The region's finish is a look.** The biome's `paving` row gains per-family overrides (China concrete, New York concrete slabs and blacktop, Paris pale stone). Gravel squares and court floors are walk-on props (pitches) that block nothing.
- **The map names its region.** `MapDefinition` gains `regional_family`, set by mapgen from `parcels::family()` and validated against every building's. It is the one thing the renderer and the loader read. *Rejected:* inferring it from the buildings in the renderer (two owners, and a map without buildings has none).
- **A shared amenity is one kind with per-region appearances; a signature piece is its own kind.** A bench is the same body everywhere, so its regional looks are appearances tagged with a family, chosen in `PropAppearances` (`models/propAppearance.ts`), which already picks among a scenery kind's appearances. A concrete ping-pong table and a chain-link fence are physically different from anything else, so they are their own catalog kinds. *Rejected:* per-region kinds for shared pieces (duplicate bodies that must agree).
- **Load only the map's region.** Every appearance except building kits loads on every map today (`scene-assets/src/loader.ts` `load`): about 267 MiB raw, with street props near 2 MiB each, and the 50 MiB per-family gate counts kits only. Regional appearances therefore load only for the map's region, and the budget counts what a family's map actually fetches.
- **Measured (slice 00).** Every map fetches 267 MiB raw of non-kit scenery today (79 MiB if gzipped; scenery has no gzip transport, kits do), most of it soldiers (141 MiB) and wrecks. A street prop is about 2 MiB raw (0.6 MiB gzipped), almost all baked texture, stored once per bundle, so two props sharing a recipe each carry it. **Cap: a new piece is at most 1 MiB raw**, by baking its recipes at the smallest size that reads at a court station and reusing the street set's recipes. About 8 shared pieces plus about 5 per region then add roughly 13 MiB raw a map (5 %); no lazy loading beyond slice 07's region filter.
- **Placement has one legality check.** `Field` and its `legal` move into their own module (slice 01) and every new consumer goes through them. Groups (a playground, a basketball court, a garage row) are placed all or nothing. One fence routine serves building sites, fenced courts and garden boundaries (today's fence loop in `site()`). Each consumer draws from its own named stream.
- **Amenities are bodies, decided with [tweak-mechanics](../../.agents/skills/tweak-mechanics/SKILL.md).** The catalog's existing columns describe every piece: garages and concrete tables stop rounds, sheds and kiosks give light cover, chain-link and gym frames block movers but not rounds or sight, laundry and hedges conceal, pitches block nothing. A piece that needs a new column is a rule change and goes back to the user.
- **Stand-ins first, art last.** A catalog kind with no art draws as a tinted stand-in box, so the physics and placement slices ship and are judged before any Blender work.

## Slice graph

```
00 ─┬─ 01 ─┬─ 05 ─┬─ 07 ── 08 ── 09
    │      └─ 06 ─┘
    └─ 02 ── 03 ── 04 ──┘
```

02–04 and 01 can run in parallel lanes; 05 needs 01 and 03; 06 needs 01; 07 needs 04 and 05; 08 needs 07 (06 for garden art).

## Single owners

| Concept | Owner |
|---|---|
| Block interior (court rings) | `crates/mapgen/src/parcels/courts.rs` (new) |
| Hard ground that is no way through | `SurfaceKind::Paving` |
| Legality of a body | `crates/mapgen/src/street_props/field.rs` (moved) |
| Fences round an area, with a gate | one routine in `street_props` |
| Dressing data | presets: `street_props.bodies`, groups, and each district's `props.courts` / `props.gardens` |
| A body's physics | its catalog row |
| The map's region | `MapDefinition.regional_family` |
| A region's look | appearance `regional_family`; the biome's per-family paving rows |
| What a map downloads | the loader's download set, counted by the budget test |

## Verification standing for every slice

- Tests first ([write-tests](../../.agents/skills/write-tests/SKILL.md)), at the seam the slice names.
- Determinism: the same request gives the same plan bytes native and wasm; parity records change only in slices that say so.
- Any visual shot: [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md) unprimed as the last check, and [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md) against [the before shots](assets/before/) or the previous slice's.
- Review checkpoints are non-blocking: open the shots with [preview-shots](../../.agents/skills/preview-shots/SKILL.md), wait about five minutes, then decide on the evidence, record it in the choices ledger and continue.
- In-game shots: the ground rig's stations draw the real battle route with fog, HUD and effects off (`web/scenes/_groundStations.mjs` `openStations`, `shoot` with `models: true`); a region is chosen by `&region=` on the route.

## End state

Reads as designed today: one paved kind, one legality check, one fence routine, one owner of the map's region, regional art as appearances. Nothing left behind for compatibility, no second paving owner, no per-region copies of a shared body.
