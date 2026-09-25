# Ownership and contracts

Status: the architecture as built through the village checkpoint (slices 01–16); continuation slices extend it. Gameplay authority is `requirements.md`. No backward compatibility or migrations.

## Natural monorepo shape

| Owner | Sole responsibility | Consumers |
|---|---|---|
| `crates/contract` | IDs, scenario/config records, commands/acks, observation records and export layout descriptors | sim, WASM, fixture readers |
| `crates/sim` | One Battle: world geometry, time, bodies, sensors, knowledge, weapons, projectiles, impacts, orders, services | thin WASM boundary, native behavioral fixtures |
| `crates/game-wasm` | Commands, stepping, side-observation export, diagnostic exports only for lab | worker |
| `packages/renderer-core` | Camera/projection, reverse-Z, GPU device/capabilities/lifetime primitives | renderer and picking |
| `packages/battle-renderer` | Scene resources, primitive meshes, visible projectiles, fog and contact presentation | web scene host, lab |
| `apps/battle-lab` | Source-only fixture UI mounted by web Vite: lab routes, the shared `BattleView`, and the village player routes (`/battle/village`, `/replay/village`) | development and village play |
| `web/src/battle/sim` | One worker owner, ordered transport, bounded publications, lifecycle | browser shell |
| `web` | Input (`useUnitControl`), player readouts, filtered audio, the worker client and observation decoding | lab and player routes |
| `fixtures` | Versioned authored maps/units/config and deterministic command scripts | sim, labs, browser tests |

Sim modules as built: `world`, `navigation`, `movement`, `sensing`, `visibility`, `hearing`, `knowledge`, `weapons`, `flight`, `damage`, `garrison`, `deployment`, `supply`, `publication`, `village` (the encounter) and `endurance` (the stress load); orders live on `units` and `movement`; create a separate crate only if it gains an actual independent consumer. Do not create a generic engine framework, ECS migration, service/event bus, or speculative networking abstraction.

## Single owners

- Geometry: Rust world mesh/colliders are authoritative. Renderer receives those mesh vertices and prop definitions; no separately exaggerated relief. XY ground, +Z up; metres, seconds, radians. Projection stays solely in renderer-core.
- Knowledge: sensing generates evidence; knowledge converts it to side observations. Target choice, AI, UI, sound, and picking consume permitted observation; physics retains truth for actual collision.
- Actions: each weapon owns aim/reload/target; unit owns engagement mode, command queue and deployment. Progress rings read state, never advance it.
- Randomness: sim-owned seeded streams, separated by combat, observation uncertainty, and scenario policy. Camera/render/audio choices do not consume combat RNG.
- Data: Rust contract owns semantic fields and packed layout descriptor. TypeScript boundary types describe the external API; offsets, strides and enum tags come from WASM exports, never duplicated hardcoded constants. The producer publishes only already-filtered side data.
- Timing: fixed 30 Hz simulation, presentation interpolates visible geometry between completed ticks. No interpolation through hidden state or across identification transitions. Same-build replay must match tick digests; do not promise cross-compiler bit identity.
- Test oracle: independent analytic physics examples and consumer outcomes, not a second gameplay implementation. Export diagnostic calculations from their owner for geometry consistency checks.

## Typed seam sketches

These signatures constrain ownership and shape; naming and internal storage are delegated.

```rust
struct CommandEnvelope { side: SideId, seq: u64, order: Order, queued: bool }
struct CommandAck { seq: u64, applied_tick: Tick, result: Result<(), OrderError> }
enum TargetRef { Identified(ObservedTargetId), Contact(ContactId), Ground(Vec3) }
// ObservedTargetId and ContactId are side-scoped, not raw enemy entity handles.
enum Order { UpgradeMove { gesture: u64, route: RoutePolicy }, Move { units: Vec<UnitId>, gesture: u64, goal: Vec2, route: RoutePolicy },
  Attack { units: Vec<UnitId>, target: TargetRef }, AttackMove { units: Vec<UnitId>, goal: Vec2 },
  Stop { units: Vec<UnitId> }, SetEngagement { units: Vec<UnitId>, policy: Engagement },
  Garrison { units: Vec<UnitId>, building: PropId }, ExitBuilding { units: Vec<UnitId> },
  SetDeployment { units: Vec<UnitId>, deployed: bool } }
// Load/Unload, LowFlight, Sortie and Purchase are added with their owning continuation slice.
impl Battle {
  fn new(setup: ScenarioDefinition, seed: u64) -> Self;
  fn accept(&mut self, command: CommandEnvelope) -> CommandAck;
  fn step(&mut self) -> Tick;
  fn observe(&self, side: SideId) -> &ObservationFrame;
}
struct ObservationFrame {
  tick: Tick, own: Vec<OwnUnit>, identified: Vec<IdentifiedUnit>,
  contacts: Vec<ApproximateContact>, audible: Vec<SoundCue>,
  visible_projectile_segments: Vec<VisibleSegment>, known_props: Vec<PropChange>,
  ground_visibility: VisibilityField, reasons: Vec<ActionReason>
}
struct ApproximateContact { id: ContactId, area: Circle, layer: Layer,
  evidence_tick: Tick, expires_tick: Tick, confidence: f32 }
struct Replay { scenario_digest: String, config_digest: String, seed: u64,
  accepted: Vec<(Tick, CommandEnvelope)> // both sides, including bot commands }
```

Large visible arrays live in reusable Rust export buffers; worker reads views into WASM memory then copies a complete filtered tick into transferable publication storage. Re-fetch pointers after memory growth. A bounded two-publication credit pool follows the sibling pattern. Field layout is exported from Rust. JSON is acceptable for small rare commands/setup and diagnostics, not a separate giant truth snapshot each draw. Capacity grows only at declared admission points; no silent truncation. No main-thread Game-shaped proxy and no direct raw pointer use by renderer/UI.

A Move gesture token maps to one submitted group gesture and the corresponding per-unit queue entries; UpgradeMove updates only entries still belonging to that token on that side. Commands validate side ownership and side-visible target references at the authority. Invalid commands return a reason and do not mutate orders. Replay stores accepted commands from both sides with application ticks. During replay disable player input and every command-generating bot/controller; feed the logged accepted commands only. Do not rerun a bot on top of its recorded commands. Scenario initial assignments remain fixed setup, not extra replay-time commands. A real-time scheduler may shed wall-clock debt and report slow simulation but cannot skip logical ticks/events. Hidden tabs suspend; loading does not tick. Failure and device loss produce actionable UI, not an endless loading state.

## Observation and world updates

Static map geometry is known from the start; dynamic wreck/building changes enter a side's remembered geometry only when observed or physically encountered by its units. Path planning uses that known geometry; physical collision uses true geometry and reports a newly encountered obstruction without leaking unrelated hidden objects. Remembered remains do not disappear merely because sight is lost. Hidden updates cannot change player payloads, hit previews, minimap, ambient light, sound origins, debug picking, or enemy health bars.

A development-only truth inspector is a different lab surface with explicit omniscient labeling. It cannot be imported into the player route or opposing controller. The defender policy receives the same side observation API, plus its own authored initial assignments. Filtering inside a local browser is design separation, not anti-cheat security; a future trusted server must own hidden truth.

## Asset replacement

Use generated asymmetric silhouettes: infantry capsules with facing cue, vehicles with nose/turret, distinct supply body, box buildings, explicit forest volumes and sparse trunks. Canonical local forward is +X, up +Z, feet/ground contact at z=0; visual bounds and selection anchor are declared in an asset record. Future meshes replace visuals without changing simulation colliders or weapon sockets. No downloaded art, credentials, Blender dependency, animation rig, photoreal lighting, or external service is required for the first village.

## Refactor-clean audit

One owner exists for every concept above; packages are direct consumers, not forwarding wrappers. Fixture scripts issue real commands through the same owner. Scenario setup is permitted to create authored states; once started, it never mutates live gameplay behind the APIs. Any toy data in the renderer replication route is a permanent render-only fixture, not a second simulation. When the worker arrives it consumes ObservationFrame; no legacy population adapter survives. A new state machine or duplicate geometry function is a spec gap, not an internal naming freedom.
