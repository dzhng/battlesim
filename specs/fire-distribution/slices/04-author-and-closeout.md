# 04 Author the flags and close out

## Contract

The game's rows opt in (D3), and the whole feature is checked once.

## Steps

1. Set `avoids_overkill: true` on `atgm`, `tank_ap`, `sead_missile` and
   `grenade` in `fixtures/rules.json`. Regenerate `fixtures/catalog.json` with
   the catalog's bless step. Confirm the parity records (`fixtures/parity/`)
   carry their own rules and do not move; if any does, read why before
   re-recording.
2. Confirm the mechanics editor shows and saves the new row flag and the
   `targeting` section; fix it if it drops either.
3. Run the full repository closeout checks once (the check ladder's step 3).
   Read every moved outcome; never loosen a test.
4. Inspect the menu reel. If it moved, re-record it, check it for regressions
   and approve it.
5. Run the full balance report once. Record the shift in this README; do not
   retune in the same pass (tweak-mechanics step 8).
6. Update `crates/sim/README.md`'s battle-rules section with both rules in
   principle form, naming tuning keys, never values.
7. Run the `review` skill, then `close-spec` to archive this spec to
   `specs/done/fire-distribution/` as rationale.

## Stays green

Everything in the closeout run.

## Delegated

- The wording of the README paragraph, within the write-docs skill.

## Feedback that would change it

The balance report or the user watching a battle: flag list changes are data
(one line per root row); spread strength is the two tuning keys.
