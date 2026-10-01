# Sim rules lane

A second session works this lane in parallel with the map lane. It is simulation logic: rules, costs and bodies. Nothing here needs art, and nothing here touches the generator or the renderer.

## Ownership

| This lane owns | The map lane owns (stay out) |
|---|---|
| `crates/sim/src/` except `world/` map loading | `crates/mapgen/`, `fixtures/map-presets.json`, `fixtures/prototype-building-templates.json` |
| Rules data in `fixtures/village.json`, `fixtures/props/`, `fixtures/units/` | `apps/`, `packages/`, `web/src/`, `web/scenes/` except scenes for this lane's labs |
| `crates/sim/tests/`, `crates/sim/examples/` | Map acquisition and preparation: C09, C60, C33, C55, C58, C59 |
| This file's Status section | `README.md`'s Next Agent Prompt and TODO |

A change this lane needs outside its column is a small, named commit, mentioned in Status.

## Work, in order

Each line is a slice file: it is the contract. Everything in [the README](README.md) applies (invariants, standing gates, budgets, firewalls), and so does the [systems handoff](systems-handoff.md): build the physical half, on physical bays and the prototype templates, and leave each slice's look to the visual pass.

1. [C40 floor-band seats](slices/C40-floor-band-seats.md) → [C41 facade eyes](slices/C41-facade-eyes.md)
2. [C42 low-rise lifecycle](slices/C42-low-rise-lifecycle.md) → [C43 tall buildings gutted](slices/C43-tall-gutted.md)
3. [SA6 movement gaps](slices/SA6-movement-gaps.md)
4. [SA5 sight and fog cost](slices/SA5-sight-cost.md)
5. [C44 street bodies](slices/C44-street-bodies.md) (the catalog rows only; placing them in generated towns is C46, in the map lane)
6. [C77 forest bodies](slices/C77-forest-bodies.md)
7. [C80 grass presets](slices/C80-grass-presets.md): the effective-height validator only
8. [C86 tree lines](slices/C86-tree-lines.md): the sim-real tree line only

Slices 1 and 2 share the garrison and structure owners, so they run one after the other. 3 and 4 are independent of them and of each other.

## How to work

Use [implement-spec](../../.agents/skills/implement-spec/SKILL.md). Branch from main; merge back at each green slice with `bun run check` and `bun run verify`. Add an Outcome to each slice file and its decisions to [`choices.md`](choices.md) under a heading for the slice. Generated maps for measuring come from `mapgen generate-map` ([generator README](../../crates/mapgen/README.md)); `city_report` usage is in [S1](spikes/S1.md).

## Status

**Final integration in progress (2026-10-01), branch `codex/city-sim-lane`.**
All lane physical code is implemented and independently reviewed. Main's later
camera/group-formation owners are integrated in `3c3752d8`, with a clean bounded
merge review. The complete pre-merge check passed. The merged-source check and refreshed native reports pass. Final browser
verification is queued on the shared GPU lock; source stays fixed while it runs. Frozen structure/movement recordings, comparisons and unprimed critiques
are complete. Prototype pixels accept no facade/source/terminal art.

**Next pickup:** complete stable browser verification, freeze the final gate/report
scopes in [closeout evidence](assets/sim-lane-closeout/README.md), then merge and
remove lane worktrees/build output. Parent map/art work and complete G0 stay open;
do not archive the parent spec. Optional forest-floor density remains zero until
accepted C78 drawing; its active-variant timing gate did not pass.

Individual `G0-SIM-*` rows disclose physical/resource unlocks in the shared gate
and decision ledger. SA5 preserves matched battle identities; its attributed
bracket excludes sight snapshotting. SA6's twelve-thousand-case physical oracle
passes, with the observed approach-cost tradeoff retained. None grants complete
active-world admission or art acceptance.

**Named outside-column seams:** C44 adds explicit `appearance.status: systems_only` without fitted art (`789e8f4b`). Building policy/scaled-integrity schema and cached immutable aggregate area are isolated in `9fdcdf5b`; TypeScript mirrors the JSON seam. SA5 changes runtime height/foliage lookup and aggregate discovery, without changing map loading. C77 adds optional forest-rule fields and bounded admission in the world builder. C80 shares effective-height constants with the renderer/culling bound and lowers two biome scales to satisfy the cap. Aggregate loading also rejects movable replacement states, preserving the existing no-composite-motion boundary. The generator and map preparation remain untouched.

Placement-context validation (`15a690ea`) adds a runtime-only catalog enum and
one chain checker, used by actual bindings/births and inherited replacement
ownership; the root README records that principle. Corpse support (`c920fe81`)
uses existing knowledge and observation owners without public layout changes;
its necessary renderer consumer refreshes dying, resting and fading anchors.
The matching shared renderer lesson is `37a4c0e` in the skills repository.

The necessary test-only closeout seam (`1ac913a7`) caches fixed rotations in the existing
full grass-prop oracle; all points, full scans, precision, comparisons and the
original deadline remain. It has no production consumer or schema change.
