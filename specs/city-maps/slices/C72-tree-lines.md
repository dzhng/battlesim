# C72: tree lines

**Depends on:** C64, C60. **Kind:** slice.

## Question
Can a map optionally have tree lines between fields that are real to the sim (Q-G15)?

## Contract it unlocks
- A map option (in `meta.json` tags or the map's generation rules) places **thin forest strips** along chosen plot edges inside the playable area, under the one forest rule, so they block sight as drawn.
- The backdrop hedgerows past the map edge are unchanged.
- The village's choice is its own: if it gets tree lines, that's a **named digest change.**

## API seam
Map data (forest strips), `terrain/plots.ts` edge choice.

## What the human can run or see
A lab map with tree lines at 65 m, and a GIF of sight blocked by one.

## Verification
- Native test: a strip blocks sight like any forest.
- Critique.

## Delegated to the implementer
Which edges; strip width. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The one forest rule.
