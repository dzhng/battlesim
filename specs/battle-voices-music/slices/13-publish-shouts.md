# 13 — Publish shouts (the single named re-bless)

**Contract this unlocks:** the browser receives each side's shouts in the packed publication.

## Seam

**`crates/sim/src/publication.rs`**
- A new `rows("shouts", "shoutCount", SHOUT_FIELDS)` group in `GROUP_LAYOUTS` (`:417-470`), modelled on `audible` (`:234`, `:451`).
- Fields:
  - the speaker's tag (own/seen) and its id, as 16-bit limbs (`:14-17`);
  - `kind`;
  - `position`;
  - `trigger`;
  - `intensity`.
- `layout_json` (`:541`) publishes the trigger and intensity tag names, so TypeScript never hard-codes them.

**`crates/sim/src/publication/reader.rs`:** the matching reader. A mismatch is refused (`:286`).

**`web/src/battle/sim/observationView.ts`:** exposes `shouts`.

**`crates/contract/src/observation.rs:14-15`:** amend the `IdentifiedUnit` comment to name the audible exception: a seen enemy's shouts reveal stress and suppression (decision D5).

**`SoundCategory::Shout`** (added and emitted in slice 12) travels in the existing `audible` group. Its bytes are covered by this same re-bless, so the re-bless happens exactly once.

**Contract test:** the layout's trigger tags equal `BARK_TRIGGERS` in `voiceCatalog.ts`, and every role in `voices.json` scripts every tag.

## The re-bless (by name, exactly once)
1. Run `BLESS_PARITY=1` (`fixtures/README.md:220`).
2. The git diff must show **only** `publication_sha256` lines in `fixtures/parity/publication/*.json` changing, plus the new rows in `codec-vectors.json`.
3. Any `digest` line changing is a bug. Stop and fix it.
4. Record the commit in `choices.md`.

## Verification
- `crates/sim/tests/publication.rs`: a round trip, overflow, and a test that an unseen enemy's shout is never published as a `Shout`, and reaches the other side only as an anonymous `audible` cue.
- `web/tests/observation.test.ts`: codec vectors.
- `web/tests/fogDelivery.test.ts`: the Wasm side.

## Delegated to you
Row field order, following the existing convention.

## Must stay green
Every publication and parity test after the re-bless; every digest.
