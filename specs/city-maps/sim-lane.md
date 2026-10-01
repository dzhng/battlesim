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

**In progress (2026-10-01), branch `codex/city-sim-lane`.** C44 catalog and C40–43 physical garrison/lifecycle checkpoints are committed. Focused checks pass; integrated `check`, `verify` and named village rule-change reports remain pending before merge. C80 height validator passes its nine checks and has matched production-route shots; its bounded systems visual disposition is recorded, with full appearance still open. SA5 is finishing exact winner confirmation. SA6's opposing-column regression is now green (eight of eight arrive); broader movement coverage and six river probes are in progress. C77 core and C86 physical tree-line commits await integration; C77 is adding a bounded local navigation connectivity guard; paired cost/evidence follows SA5.

**Next pickup:** integrate the reviewed SA5 winner, then C77/C86 and SA6 when the last column regression is green; regenerate shared catalog output and run final integration gates. Parent map/art gates remain open.

**Named outside-column seams:** C44 adds explicit `appearance.status: systems_only` without fitted art (`789e8f4b`). Building policy/scaled-integrity schema and cached immutable aggregate area are isolated in `9fdcdf5b`; TypeScript mirrors the JSON seam. C80 shares effective-height constants with the renderer/culling bound and lowers two biome scales to satisfy the cap. The generator and map preparation remain untouched. This lane does not archive the parent spec.
