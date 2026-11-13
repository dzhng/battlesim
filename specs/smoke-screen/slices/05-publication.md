# 05: observation and publication contract

## Contract

- **Clouds:** each side's observation carries the clouds it can see (D12).
  Every one of its own units carries its smoke charges.
- **Publication layout:** the layout changes once, for the whole feature.
  Captures and vectors are re-made in this slice.
- **Privacy:** nothing leaks. A cloud record never names the unit that popped it
  or that unit's kind.

## Seam

- **Observation** (`crates/contract/src/observation.rs`):
  - `ObservationFrame.smoke_clouds: Vec<ObservedSmoke { id, centre, popped:
    Tick, spec: SmokeCloudSpec }>`;
  - `OwnUnit.smoke: Option<u32>`, which is the charges.

  Name it `smoke`, never `smoking`: that name already means damage smoke.
- **Publication**, in a new `crates/sim/src/publication/smoke.rs`, because
  `publication.rs` and `reader.rs` are over budget:
  - a group `rows("smokeClouds", "smokeCloudCount", &SMOKE_CLOUD_FIELDS)`, which
    takes `GROUPS` from 21 to 22;
  - an own-unit field `smokeCharges`, set to −1 when the unit has no smoke;
  - a `smokeLooks` names list, built like `roundKinds`.

  Read them in `reader.rs` and `reader/view.rs`, and mirror them in
  `web/src/battle/sim/observationView.ts`.
- **Wasm boundary:** the presentation-wasm JSON view follows `ObservationView`.

## What you can run

A reader round-trip of a smoke scenario, and the new captures.

## Tests and checks

- **Codec:** a stream with clouds and charges round-trips through the reader.
- **Privacy:** a side that sees only the cloud's rim gets the cloud record but
  no identified unit, and the record carries no unit id or kind.
- **Pull LFS first** (memory: full run needs LFS). Then re-record the browser
  captures `paris-corner` and `market-town` (`fixtures/README.md:293`), and
  re-bless the codec, fog-sight and effects vectors with `BLESS_PARITY`.
- **Native:** it replays the browser captures, so it parses the new layout.

## Delegated

You may choose the field order inside `SMOKE_CLOUD_FIELDS` and how the spec is
packed: inline fields, or an index into `smokeLooks` plus inline numbers.

## Must stay green

- The parity, codec and capture checks against the new recordings.
- Every pinned simulation digest. The publication layout is not part of the
  battle digest; check this and record it.

## Feedback that would change this slice

None is expected. This slice is mechanical once 03 and 04 are fixed.
