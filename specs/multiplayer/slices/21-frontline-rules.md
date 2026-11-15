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

## As built (2026-11-15)

- **The rule field** is `MatchRules.all_objectives_victory`, true by
  default: taking every objective uncontested wins on the spot. When it is
  true it is left out of the serialised rules, so every existing scenario,
  fixture, digest and replay is byte for byte what it was. A replay
  recorded with it on is refused by a battle with it off.
- **The Frontline document** is `fixtures/frontline.json`:
  `majority_s` 3780, `victory_score` 2100, `all_objectives_victory` false.
- **One place lays a document over the base:**
  `sim::skirmish::resolve_rules`. Native tools, the page and the server all
  get their match rules from it.
- **How a battle selects it:** `BattleSetup.rules` to the lobby, to the
  battle request, to preparation, which resolves the rules into the
  scenario. A request names a rule document only when it overrides the
  base, so a room battle on skirmish rules asks for exactly what an offline
  lobby asks for. (The pass as delivered gave skirmish two spellings,
  absent and `"skirmish"`, and an existing room test caught the
  difference; fixed at integration.)

**The length, measured.** Three generated 5v5s of AIs on mixed XL maps
(seven objectives, so a majority is four), with the instant win off and no
reachable end, each played for an hour. The leading side's
objective-holding time at the hour:

| Seed | Leader | Holding time at 60 min | Its score at skirmish pace | The other side's |
|---|---|---|---|---|
| 1 | Blue | 14,119 objective-seconds | 1,961 | 1,254 |
| 2 | Blue | 15,543 | 2,159 | 966 |
| 5 | Red | 16,080 | 2,233 | 977 |

The mean is 15,247 objective-seconds, which is a majority of four held for
3,812 seconds, or 2,118 points at a skirmish's pace. Rounded to a target of
2,100 points, which is 3,780 seconds. So on such a map the leading side of
an average game reaches the target at about an hour. Two more seeds (3 and
4) were refused by the map generator before play and are not in the mean.

These are the user's dial for how long a Frontline game lasts. They were
measured on the largest maps with one pair of factions; a smaller map has
fewer objectives and its games will run to a different length.

Tests, none naming a value: with the field off, a side holding every
objective keeps scoring and the match ends when its holding time reaches
what the rules require; holding everything scores faster than a bare
majority; both rule sets show the same points for the same holding time;
the Frontline document changes exactly three fields; page and server
prepare the same map and the same rules for either selection.

