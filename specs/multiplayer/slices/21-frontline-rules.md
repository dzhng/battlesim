# 21 Frontline rules

**Stage:** Frontline. **Needs:** 01 (for the target), 13. **Carries:**
decisions 2, 7, 54, P24.

Use [tweak-mechanics](../../../.agents/skills/tweak-mechanics/SKILL.md)
before proposing anything here.

## Contract

Frontline is the normal rules with a longer game and no instant win:

- **A higher points target, reached at the same pace.** Its length is not a
  requirement; the target starts at about an hour's play.
- **No instant win.** Holding every objective uncontested ends a skirmish on
  the spot (`crates/sim/src/objectives.rs:159`). In Frontline it just scores
  fastest. Without this a higher target does not make a longer game.

Rooms and offline skirmish keep every value as it is.

**Corrected on 2026-11-15.** The plan said "two values". The first pass at
this slice stopped, rightly, on finding that the points target alone cannot
lengthen a game. A side wins when its time holding objectives reaches
`majority_s` (times the number of objectives in a majority); `victory_score`
is only the number that moment is shown as. Raising `victory_score` makes
points climb faster by the same factor, and the match ends at the same
tick. So the length of a game is `majority_s`, and Frontline overrides
three values:

- `majority_s`: how long a game is.
- `victory_score`: raised by the same factor, so points climb on screen at
  the pace a skirmish's do and the target reads higher.
- the new field that turns the instant win off.

## Seam

- `crates/contract/src/skirmish.rs`, `MatchRules`: one new field for the
  instant win. It is rule data, not a mode flag.
- A Frontline rules document under `fixtures/` holding only the three
  overrides, applied over `fixtures/skirmish.json` (P24).
- `BattleSetup.rules` in [wire.md](../wire.md) selects it.

`MatchRules` refuses unknown fields and every replay records a digest of its
configuration. So the new field must leave every existing battle's digest and
configuration digest unchanged when it holds the skirmish value. Prove that
first.

## Run or see

A native run of a small match under Frontline rules in which one side takes
every objective: the match continues, and that side's score climbs fastest.

## Verification

- Under skirmish rules, holding every objective still ends the match.
- Under Frontline rules it does not, and the match ends when a side reaches
  the target.
- Every existing digest and replay is unchanged.
- The target's and the length's values are data. No test names them.

Set the length by measuring, since slice 01's two-hour run has no scores in
it (its rules let no objective be captured): play a generated 5v5 of AIs
with no instant win and no reachable end for an hour on several seeds, and
read how much objective-holding time the leading side has at the hour.
`majority_s` is the value that makes that the winning amount, and
`victory_score` scales with it. Record the numbers and the runs they came
from in decisions.md.

## Must stay green

All skirmish digests, the balance tooling's inputs.

## Delegated

The field's name, after tweak-mechanics.

## Feedback that would change this

The target is the user's dial for how long a Frontline game lasts.
