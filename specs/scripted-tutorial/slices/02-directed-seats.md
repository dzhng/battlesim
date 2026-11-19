# 02 Directed seats

**Status: shipped 2026-11-19.**

**Unlocks:** an AI seat that does nothing on its own but fights at its units'
authored engagement, which is what the tutorial's red side needs.

**Contract (C1), as shipped:** `contract::players::ControllerKind` gains
`DirectedAi`.

- It issues nothing on its own: the skirmish AI and the non-skirmish
  controller turn both act only for `ActiveAi`.
- Its units keep their authored engagement (`PassiveAi` holds them at return
  fire; `put_under` leaves a directed seat's units alone).
- It counts as ready in preparation (`PlayerState::is_ready`), so it never
  holds a skirmish's preparation open, and the published player row says so.
- A client order for it is refused `ai_controlled`, as for any AI seat.
- Only the director (06) orders it.
- `ControllerKind::ALL` is the one list of controllers that the publication's
  layout table and player rows read. Before this there were two hand-written
  copies. The browser's `ControllerKind` union, the player view and battle
  addresses (`d`) mirror it.

**Why a controller kind, not the planned `Playbook` field:** controller kinds
already say what an AI seat does (`ActiveAi` plays, `PassiveAi` holds fire). A
separate playbook field would have been a second, overlapping owner of the same
question (see [decisions.md](../decisions.md#slice-02)).

**Evidence:** `crates/sim/tests/directed_ai.rs`:

- a directed seat issues no orders and its rifleman stays at fire at will;
- a client order for it is refused;
- a human's ready starts a skirmish with a directed red seat long before the
  countdown. Mutation-checked: with the readiness rule removed, this test
  fails.

The contract tests, the sim library tests and the sim tests for skirmish,
publication and side topology all pass. The browser typecheck shows no errors
in the touched files; its other errors come from the missing local WASM build.
