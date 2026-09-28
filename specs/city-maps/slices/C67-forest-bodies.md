# C67: forest bodies

**Depends on:** C64, C44 (pattern). **Kind:** slice.

## Question
Do fallen logs and large boulders behave as cover (Q-G12)?

## Contract it unlocks
- Catalog rows (`fixtures/props/nature.json` or a new `fixtures/props/forest/`): `log` and `boulder`, written by tweak-mechanics (stop rounds, cover tier, block vehicles, not pushable).
- Sparse placement inside forests by map rules. The fallen-trunk model is shared with C65.
- **Named village digest change** if the village's forests get them.

## API seam
Catalog rows, map placement rule (in the map fixture or the importer).

## What the human can run or see
A GIF of a squad taking cover behind a log.

## Verification
- Native tests per row.
- `--quick` report.

## Delegated to the implementer
Placement density. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The props-are-bodies rule (rows, no names).
