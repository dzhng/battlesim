# 21 Frontline rules

**Stage:** Frontline. **Needs:** 01 (for the target), 13. **Carries:**
decisions 2, 7, 54, P24.

Use [tweak-mechanics](../../../.agents/skills/tweak-mechanics/SKILL.md)
before proposing anything here.

## Contract

Frontline is the normal rules with two values changed:

- **A higher points target.** Its length is not a requirement; the target
  starts at about an hour's play.
- **No instant win.** Holding every objective uncontested ends a skirmish on
  the spot (`crates/sim/src/objectives.rs:159`). In Frontline it just scores
  fastest. Without this a higher target does not make a longer game.

Rooms and offline skirmish keep both values as they are.

## Seam

- `crates/contract/src/skirmish.rs`, `MatchRules`: one new field for the
  instant win. It is rule data, not a mode flag.
- A Frontline rules document under `fixtures/` holding only the two
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
- The target's value is data. No test names it.

Set the target from slice 01's two-hour run: the score an average 5v5 reaches
in about an hour. Record the number and the run it came from in decisions.md.

## Must stay green

All skirmish digests, the balance tooling's inputs.

## Delegated

The field's name, after tweak-mechanics.

## Feedback that would change this

The target is the user's dial for how long a Frontline game lasts.
