# 04 Scenario document and its resolution

**Unlocks:** a general authored-scenario source that resolves under today's
rules and roster, plus the check that every placement is still legal.

**Contract (C3), without director contents (06) or offer enforcement (05):**

```rust
// crates/contract/src/scenario/authored.rs (type name may change, but must not
// collide with ScenarioDefinition, the resolved runtime setup)
#[serde(deny_unknown_fields)]
pub struct ScenarioDocument {
    pub battle_seed: u64,
    pub players: Vec<Player>,                       // with playbooks (02)
    pub match_rules: serde_json::Map<String, Value>, // patch over skirmish.json
    pub sites: SkirmishSites,                       // entries and objectives
    pub forces: Vec<TaggedUnit>,                    // UnitSetup + optional tag
    pub opening_credits: BTreeMap<String, CreditGrant>,
    pub offered: BTreeMap<String, Vec<PurchaseSpec>>,
    pub zones: BTreeMap<String, Zone>,              // named public places
    pub director: Director,                         // empty here; 06 fills it
}
pub enum CreditGrant { Covering(Vec<PurchaseSpec>), Exactly(u64) }
pub struct PurchaseSpec { pub variant: String, pub carrier: Option<String> }
impl ScenarioDocument {
    pub fn on(self, map: MapDefinition, rules: Rules) -> Result<ScenarioDefinition, ScenarioError>;
    pub fn public(&self) -> ScenarioPublic; // chapters, beats + audience, zones, objective ids
}
```

- **No `overrides` field, by type.**
- `credits_covering(catalog, specs) -> u64` is one function in contract, and
  the resolved number enters the `ScenarioDefinition` and its digest.
- **The match patch reuses one resolver.** Extend `sim::skirmish::resolve_rules`
  (Frontline's patch mechanism) to take a patch rather than adding a second
  resolver. A key outside `MatchRules` is refused.
- **Resolution refuses**, each refusal naming the document path as
  `EncounterDefinition::on` does:
  - a unit outside its player's faction, or not in the game catalog set (no
    `test_*` or `capture_*`);
  - an unknown or duplicate tag;
  - an unknown zone;
  - a lab-only `EventAction`;
  - a bad match-rules key.
- Tags resolve to `UnitId`s here (authored units take ids in order), so the
  runtime never handles tag strings.
- `SkirmishSetup` gains per-player `opening_credits` (an absent player gets the
  rules' opening wallet), read by `Skirmish::new`.
- **Loaders mirror encounters:** native `MapCatalog::scenario_document(id, name)`;
  browser and Node `loadScenario` in `web/src/maps/{browser,node}.ts`; a WASM
  resolver beside `encounter_scenario`.
- **Rename** `MapCatalog::scenario` → `encounter_scenario`.

**Placement legality** (the interview's "a check proves every placement is
still legal"): `crates/sim/tests/scenarios.rs` finds every scenario on every
`scenario` map by directory (no list), resolves it, and asks the encounter
planner's legality owner (`crates/sim/src/encounter/legality.rs`) about every
force:

- it stands on its ground;
- its footprint fits;
- a squad in a building fits that building's seats;
- every zone and site is inside the map;
- the sites pass skirmish site admission with route evidence.

It pins nothing about Ferryman.

**Content:** `fixtures/maps/ferryman-valley/scenarios/operation-ferryman.json`:

- players: blue human `us`, red `Directed` `eastern`;
- sites: one blue road, one red road, three objectives;
- chapter 1–2 forces: `outpost` and `east_guns`;
- the [story](../story.md) zones;
- opening credits covering chapter 1's purchases;
- a match patch with a long preparation and a long `majority_s`.

**Playable:** `cargo run -p sim --example scenario_report -- ferryman-valley operation-ferryman`
prints the resolved players, credits, forces by tag and legality verdicts. It
also writes the skirmish picture SVG with forces and zones drawn on it (extend
`mapgen`'s skirmish picture to take marks).

**Gates:**

- `cargo test -p contract --test scenario_document`: each refusal above, and a
  covering grant that follows a price change on a fake catalog.
- `cargo test -p sim --test sim scenarios` (placement legality).
- The report's SVG shown inline (non-blocking).

**Delegated:** type and module names, zone shape representation (circle or
polygon), the report's layout.

**Must stay green:** encounter loading (renamed call sites), skirmish rule
resolution (Frontline included), contract tests.
