# 02 Seat playbooks

**Unlocks:** an AI seat that does nothing on its own but fights at its units'
authored engagement, which is what the tutorial's red side needs. It also gives
opponent dispatch one owner.

**Contract (C1):**

```rust
// crates/contract/src/players.rs
pub struct Player {
    // existing fields …
    #[serde(default, skip_serializing_if = "Playbook::is_doctrine")]
    pub playbook: Playbook,
}
pub enum Playbook { Doctrine, Directed } // default Doctrine
```

- `Doctrine` is today's skirmish AI. An absent field means `Doctrine`, so every
  existing setup, digest and replay is unchanged.
- `Directed` issues nothing on its own. Its units keep their authored engagement
  (`PassiveAi` forces return fire; this doesn't). It is ready from setup
  (`Skirmish::new` marks it), so it never holds preparation open. A client order
  for it is refused `ai_controlled`, as for any AI seat. Only the director
  (06) orders it.

**Seam:** `Battle::ai_turn` asks each AI seat's playbook what to do.
`skirmish_ai_turn` becomes the `Doctrine` arm; `Directed` is an empty arm. Don't
add a publication table for playbooks: the page doesn't need one.

**Playable:** none; this is a sim seam. The test is the evidence.

**Gates (sim domain only):** new `crates/sim/tests/playbooks.rs` on test units:

- a `Directed` seat issues no orders;
- it keeps a unit's authored fire at will and fires on what it sees;
- a client order for it is refused `ai_controlled`;
- it counts as ready in preparation.

`cargo test -p sim` for the crate is the gate, because this touches the AI turn
every skirmish test runs. Every existing digest must stay unchanged. Also run
`cargo test -p contract` for the players type.

**Delegated:** the dispatcher's internal shape.

**Must stay green:** the skirmish parity fixtures (`fixtures/parity/`) and the
skirmish AI tests.

**Feedback that changes it:** if the user wants the encounter `Defender` folded
in now, it becomes a third arm here, and `crates/sim/tests/encounter*.rs` joins
the gate.
