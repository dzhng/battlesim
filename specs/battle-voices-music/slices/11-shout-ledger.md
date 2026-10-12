# 11 — Shout ledger (simulation, not yet published)

**Contract this unlocks:** the simulation decides, every tick, which units shout which bark at which intensity. This covers every bark whose evidence already exists. The publication doesn't change yet.

Load [`tweak-mechanics`](../../../.agents/skills/tweak-mechanics/SKILL.md) before starting. Write each test first ([`write-tests`](../../../.agents/skills/write-tests/SKILL.md)).

## Seam

**Contract crate (`crates/contract/src/observation.rs`):**
- `ShoutTrigger`: the 17 barks, snake_case.
- `Intensity`: `Steady` or `Stressed`.
- `ShoutSpeaker`: `Own(UnitId)` or `Seen(ObservedTargetId)`.
- `Shout { speaker, kind: TypeIndex, position: [f64;3], trigger, intensity }`.
- `ObservationFrame.shouts: Vec<Shout>`, marked `#[serde(default)]` the way `crashes` is (`:634`).

**`crates/contract/src/scenario.rs`:**
- `ShoutRules { cooldown_s: 8.0, talk_s: { <trigger>: seconds } }` in `Rules`, authored in `fixtures/game.json`.
- `talk_s` holds **nominal** talk times; the generation tool flags any take that runs over (slice 02).

**`crates/sim/src/shouts.rs` (new module; this is the side observation builder's seam):**
- **`ShoutLedger`, one per side:** `last: BTreeMap<(UnitId, ShoutTrigger), Tick>` and `talking_until: BTreeMap<UnitId, Tick>`. It lives in `Battle` as `shouts: [ShoutLedger; 2]` next to `observations` (`battle.rs:280`). **It is never folded into `Battle::digest`** (`battle.rs:3683-3776`).
- **`ShoutEvidence`, per tick:** hp before and after, member deaths, mount `ActionReason` transitions, launches, kills, and newly identified enemies by layer. Battle phases write it; it is never digested.
- **`build(side, &evidence, &units, &knowledge, &rules) -> Vec<Shout>`** is called from `observe_all` (`battle.rs:3358`). It only reads `fired` (`battle.rs:3708`, which is digested) and never writes it.

**Who hears:**
- every shout from a side's own units;
- an enemy's shout only while that enemy is identified this tick (`knowledge.identifies`), with `speaker = Seen(id)`.

**Intensity:** `Stressed` when health ≤ ½ or suppression isn't `none`. For squads, health is living members ÷ starting members.

**Barks in this slice:**

| Bark | Evidence |
|---|---|
| B1 / B2 / B3 contact (infantry / vehicle / air) | an enemy enters `identified` for this side, classified by unit body and altitude layer. The nearest seeing own unit shouts. |
| B4 opens fire | a mount's reason goes to `Firing` on a new target |
| B6 suppressed · B7 pinned | the suppression tier rises |
| B8 hit | hp or a member's hp dropped and the unit survived |
| B9 man down | a member died and the squad lives |
| B10 badly damaged | health crosses ⅓ downward |
| B11 kill | this unit's round destroyed an enemy unit |
| B12 reloading | the reason goes to `Reloading`; infantry only, by script |
| B13 out of ammo | the reason goes to `OutOfAmmo` |
| B14 missile away | a guided launch by this unit |
| B16 destroyed | this unit was destroyed this tick; it speaks from its last `kind` and `position` |

**Ledger rules:**
- A unit repeats the same trigger only after `cooldown_s`.
- A unit never starts a shout while `talking_until > now`.
- Ties within a tick go to the more specific trigger, in a fixed order kept as data.

**No AI reads `shouts`.** Both AIs read observation frames (`battle.rs:1736`, `:1768`). Add a test that asserts their inputs ignore `shouts`, for example a grep test or a type that can't reach it.

## What the human sees
A test that prints the shouts per tick for a short scripted skirmish.

## Verification
- `crates/sim/tests/shouts.rs`: one test per bark, plus the cooldown, no-overlap, seen-only enemy and intensity tests.
- **Digests unchanged:** `crates/sim/tests/menu_reel.rs` and every pinned `digest` in `fixtures/parity/*.json` pass without a re-bless.
- **`config_digest` changes** because `ShoutRules` is in `Rules`. That is expected and named in `choices.md`.

## Delegated to you
- the ledger's internals;
- the tie-break order;
- the nominal `talk_s` values (start at the reference clips' lengths, about 1–3 s).

## Must stay green
- every digest;
- `menu_reel.rs`;
- `publication.rs` tests (the publication is unchanged).
