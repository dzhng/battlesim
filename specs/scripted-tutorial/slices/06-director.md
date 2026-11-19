# 06 Director: contract and runtime

**Unlocks:** scenario progression as part of the battle's rules: chapters,
beats, red cues, grants and offers. It is deterministic, digested, and runs the
same live and in replay.

**Contract (C6):**

```rust
// crates/contract/src/scenario/director.rs
pub struct Director { pub chapters: Vec<Chapter> }
pub struct Chapter { pub id: String, pub beats: Vec<Beat> } // ends when its last beat fires
pub struct Beat {
    pub id: String,
    pub when: Trigger,
    #[serde(default)] pub audience: Vec<Side>, // sides told it fired (07)
    #[serde(default)] pub then: Vec<Action>,
}
pub enum Trigger {
    ChapterOpens,
    Fired { beat: String, after_s: f64 },
    Phase(Phase),
    Units { who: Selector, are: UnitFact, count: Count },
    Knows { side: Side, what: Selector, how: Knowledge }, // identified / contact
    All(Vec<Trigger>), Any(Vec<Trigger>),
    // later slices add variants: Objective, MatchResult, more UnitFacts
}
pub enum UnitFact { In(String), Engagement(Engagement), Hidden, Purchased /* + later */ }
pub enum Selector { Tag(String), Bought { player: String, variant: String } }
pub enum Action {
    Order { player: String, units: Selector, order: DirectedOrder }, // DirectedAi seats only
    Grant { player: String, credits: CreditGrant },
    Offer { player: String, add: Vec<PurchaseSpec> },
}
```

This slice ships only chapters 1–2's vocabulary. Each later chapter slice adds
the variants it needs, each proved on test units.

**Runtime rules:**

- **Facts are state, never commands.** A trigger reads what is true (a unit's
  engagement, its hold, its zone), so it holds again after a restart and can't
  be satisfied by a refused command.
- **Triggers read authoritative state**, as `Referee::judge` and objective
  capture do. Cueing red on blue's position is the scenario's authored drama,
  not red's knowledge. Nothing reaches a side's observation except a beat id
  (07). `Knows` reads a side's own knowledge, for beats like "blue identified
  the outpost".
- **Orders go through `Battle::apply_authored_order`**, extracted from the
  fixed-script block in `step_profiled` and called by both. They aren't written
  to the accepted log, because replay reruns the director. The function returns
  the refusal. The director keeps a per-beat refusal list outside the digest
  (`Battle::director_refusals()`) for the walkthrough; the page never sees it.
  An `Order` for a seat that isn't `DirectedAi` is skipped.
- **State** `DirectorState { chapter, fired: Vec<Option<Tick>> }` is digested
  only when a director exists, so other digests stay unchanged. Beats arm only
  while their chapter is open. A chapter closes when its last beat fires, and
  the next opens on the next tick.
- **Admission** at `ScenarioDocument::on`:
  - ids are unique;
  - a `Fired` names an earlier beat of the same or an earlier chapter (no
    cycles);
  - every tag, zone, objective, player and variant exists;
  - an `Order` names a `DirectedAi` player's units.

  Tags, zones and grants resolve into a `ResolvedDirector` (ids and numbers
  only).
- It runs once a tick, after orders apply and before the skirmish advances.
  Record the exact place in the ledger.

**Playable:** `scenario_report` prints the chapter and beat outline and, with
`--run <ticks>`, which beats fired when.

**Gates:**

- `cargo test -p contract --test director`: a cycle is refused; an unknown tag,
  zone or player is refused; an order to a non-`DirectedAi` player is refused; a
  `Fired` naming a later chapter is refused.
- New `crates/sim/tests/director.rs` on test units and a test map:
  - beats fire in chapter order, and only while their chapter is open;
  - a state trigger holds however the state was reached;
  - an `Order` reaches a `DirectedAi` seat and is skipped for a human seat;
  - director orders are absent from `replay().accepted`, and the replay matches
    the live digest at every tick;
  - `Grant` and `Offer` change the wallet and the offer.
- Fixed-script outcomes are unchanged: run the capture and street-encounter
  tests and confirm the same digests.

**Delegated:** module layout, the `Count` and `Knowledge` shapes, where in the tick
it runs (recorded).

**Must stay green:** every sim digest, fixed scripted orders, replay tests.
