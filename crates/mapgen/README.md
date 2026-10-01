# Map generation and physical compilation

Two steps, one crate. The **layout generator** turns a seed, a map type and a size
into a `MapPlan`. The **compiler** turns any plan, generated or authored, into the
contract's final map. The battle and renderer consume that compiled geometry; they
never reinterpret a plan or look up a template catalogue. The crate imports no
simulation or appearance library.

## Layout generation (`layout`)

`generate_layout(&GenerationRequest, &PresetDefinitions)` writes where settlements,
roads and forests go. Its numbers are data: [`fixtures/map-presets.json`](../../fixtures/map-presets.json),
validated at load, whose `revision` a request pins beside `GENERATOR_VERSION`. A
request that names another revision or generator is refused, so an old request
cannot quietly yield a new map. The only constants in code are the three playable
extents, which are a user decision (M04 in the [map brief](../../specs/city-maps/procedural-maps.md)).

- **The same request gives the same plan bytes on every target.** Each consumer draws
  from its own named stream of the seed (`rng`), so a change to one (forests, a
  town's districts) cannot move what another placed. Arithmetic is `+ − × ÷` and
  `libm`, as in `contract::curve`; plan coordinates are whole centimetres.
- **A rule is met because the finished plan measures as meeting it.** `measure` reads
  only the plan's geometry: top/bottom areas, open approaches, the road graph's reach
  and journey times. The generator steers toward each rule as it builds, then holds
  its output to `measure` and refuses a plan that fails.
- **A search that runs out is a named refusal.** The diagnostic names the feature,
  the preset cell and the seed. The generator never tries another seed and never
  returns a thinner map than the presets describe.
- **Settlements are plan-level.** `MapPlan.settlements` holds each settlement's
  outline and its districts: single-use pieces of ground (one dominant building
  category), each with a stable id, an area and an anchor point. `MapPlan.approaches`
  holds the measured wedges of open ground. Neither reaches the map: the parcel pass
  (C53) turns districts into `buildings`, and the encounter planner reads both.
- **Rivers are not generated.** The compiler does not admit them yet (C69). They
  belong after sites and before roads, on a stream of their own.

What the presets mean, and why they hold the values they do, is in the
[C52 slice](../../specs/city-maps/slices/C52-procedural-generator.md) and its
entries in the [choices ledger](../../specs/city-maps/choices.md).

## Compilation

Generation configuration pins the lossless map seed, generator/preset versions,
canonical plan and physical catalogue. Execution limits are separate preparation
policy. The map's content hash identifies physical output independently of its
execution allowance or presentation assets.

The shared identity owner lives in `contract::identity`. A seed is canonical decimal
text at JSON boundaries. Typed field order defines canonical bytes; authored sequence
order is meaningful because implicit IDs occupy the remaining dense namespace. Input
whitespace and object-key order do not change identity. Version labels are supplied
by preparation callers, so this is content identity rather than verified source provenance.

The compiler admits physical buildings, ordinary authored bodies, and ground: roads,
tracks, sidewalks and forests in the contract's shared shapes, which pass into the map
unchanged. Explicit body IDs enter the contract-owned dense namespace. A ground shape's
authored points must lie inside the playable rectangle (a stroke may overhang the edge
by its width). Rivers, land regions and source/art fit remain prerequisites for their
compiler arms; a requested unsupported feature produces a named error rather than
disappearing from output.

`MapPlan.size` is the playable rectangle. Admission follows the architecture envelope
in the city-map scale policy; it defines no rendered surroundings.
Physical box bounds use the template contract's numeric evaluator for admission.
Simulation collision arithmetic and its remaining cross-runtime proof are independent.

## Boundaries

The library's complete outcome is shared by the CLI (`mapgen`, whose usage text lists
its commands) and WASM. A refusal returns diagnostics and no plan or map. Successful
file preparation writes the final map and the shared `MapSources` envelope. Supplied
request, preset and catalogue receipts hash exactly the bytes used and assert no Git
history; generation identity and stdout outcomes are unchanged. Acquisition and
catalogue publication belong to C09/C60. The required execution limits cover authored
parts, emitted bay positions and ground points (polygon vertices plus rounded stroke
samples) before materialization. They do not claim a bound on all input bytes, terrain,
navigation, runtime trees or the complete battle's memory.

`mapgen inspect` draws a plan's layers at metre scale for review, with the numbers
`measure` reports. The `layout_sweep` example runs every type and size over a range
of seeds; it is how a preset change is judged.
