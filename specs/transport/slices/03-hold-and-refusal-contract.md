# 03 · The hold and refusal contract

**Owns:** L3 (in part), L5, L6; the refusal half of D7 and D9. **Depends on:** 02. **Outcome change:** none.

## Contract
The README's "Observation and publication" and "Refusals per unit" contracts, introduced in full here so the publication layout changes **once**:
- `OwnUnit.garrison` (`crates/contract/src/observation.rs:606`) becomes `OwnUnit.hold: Option<HoldState { host, phase, progress }>`, with `host: HostRef::Building { id, center, half } | HostRef::Carrier { id }`. The carrier variant is declared now and stays unused until slice 06.
- The 8 `garrison*` columns of the own row (`crates/sim/src/publication.rs:364-371`) become host columns (`hostKind`, `hostLo`/`hostHi`, `holdPhase`, `holdProgress`, `hostX`/`hostY`, `hostHalfX`/`hostHalfY`). `IDENTIFIED_FIELDS` doesn't change.
- `contract::command::Refusal { unit, reason: OrderError }`. `CommandAck` gains `refused: Vec<Refusal>`, and `ObservationFrame` gains `refusals: Vec<Refusal>` for the own side, for the tick it happens. Publication gains a `refusals` group with a table of reason names.

## Seam
- `crates/contract/src/observation.rs` and `command.rs`.
- `crates/sim/src/publication.rs` (`OWN_FIELDS`, `GROUP_LAYOUTS`, the explanatory comment in `layout_json`), `publication/reader.rs` (`fn own`, about `:836`), and the reader's view.
- `web/src/battle/sim/observationView.ts` and `protocol.ts`.
- Readers of the old field, migrated in the same commit:
  - `web/src/battle/present/panelRows.ts`
  - `web/src/battle/input/commandReach.ts`
  - `battleOverlay.ts`
  - `useBattleSession.ts`
  - `apps/battle-lab/src/routes/garrison.tsx`
  - `crates/presentation/src/presentation.rs:1493-1498`

  Find any others with grep.

## What a human can see
The garrison lab looks identical. A refused order still shows its text beside the cursor (`web/src/battle/present/rejectedOrder.tsx`), now fed per unit.

## Verification
- Re-bless the codec vectors in `crates/sim/tests/common/publication_codec.rs`.
- **Re-record every capture** under the GPU lock, native's included (`fixtures/README.md:250`; the `native-parity-capture` scene; `fixtures/parity/capture/counts`). This is the only slice that re-records.
- Must pass: `native/client/tests/counts.rs`, the publication reader tests, `web/tests/captureObservations.test.ts`, the garrison scene.
- Digests unchanged: rerun the slice 02 probe.

## Delegated
Field names inside the host columns; how the refusal reason table is packed; whether `HoldState.progress` stays a 0–1 fraction.

## Must stay green
Reader strictness (a stale layout is refused, never adapted), native and browser parity, every digest. No compatibility reader and no `serde(default)` shim.

## Feedback that would change it
None expected.
