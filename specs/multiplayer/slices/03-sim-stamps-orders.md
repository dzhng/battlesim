# 03 The sim stamps orders

**Stage:** Foundation. **Needs:** nothing. **Carries:** decisions 22, 28, P3.

## Contract

A page never names a sequence number, a side or a player that the battle then
trusts. It sends an order; the battle stamps who sent it and its place in that
player's sequence.

Today the page counts its own sequence (`web/src/battle/sim/client.ts:359`)
and the battle refuses an order whose number is not the next one
(`crates/sim/src/battle.rs:1117`). A seat's AI advances the same counter, so
the first order after a human takes a seat back is refused as out of sequence.

## Seam

- **`crates/game-wasm/src/lib.rs`:** new
  `accept_order(side, player: Option<String>, order_json, queued) -> ack`,
  which fills the sequence from the battle's own counters. The binding's
  `accept(command_json)` is removed. Rust's `Battle::accept(CommandEnvelope)`
  stays, because a replay still carries stamped envelopes.
- **`web/src/battle/sim/protocol.ts`:** the request becomes
  `{ type: "command"; id: number; order: Order; queued: boolean }` and the
  reply `{ type: "ack"; id: number; ack: CommandAck }`. Side and player are
  the channel's, fixed at `init`.
- **`web/src/battle/sim/client.ts`:** matches acknowledgements by its local
  `id`, and no longer counts a sequence.

Owner of the stamp: the simulation. This holds for local play too; there is
one path.

## Run or see

Single-player skirmish plays as before.

## Verification

- New test at the authority seam: a seat is handed to its AI, the AI issues
  orders, the seat returns to the human, and the human's next order is
  applied, not refused.
- A recorded battle's replay ends on the same digest as before this slice:
  the recorded envelope still carries its sequence.
- Two orders sent back to back are applied in the order given. The double
  right-click that upgrades a move names the first click's order, so this
  ordering is load-bearing.

## Must stay green

Every existing digest and replay. Every lab and test that issues an order;
each moves to the new request in this slice.

## Delegated

Names; how the tests that build a `CommandEnvelope` by hand are re-pointed.
Use [refactor-change](../../../.agents/skills/refactor-change/SKILL.md): the
page-side counter is deleted, not kept beside the new one.

## As built

`Battle::accept_order(side, player, order, queued)` stamps the next sequence
number for that player (or, in a battle without players, that side) and hands
the envelope to `Battle::accept`, so a live order and a replayed one share
every check after the stamp. The authority takes the commanding side from
the first side it publishes and the player from `init`; slice 06 changes
that for a host serving several seats.

A page can no longer send an order out of sequence, so the authority test
that expected that refusal now shows the page's id returning on the ack
beside the battle's own sequence number. The refusal itself remains for
replays and is still tested in `crates/sim`.

