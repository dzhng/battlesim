# 12 — Physical barks (B5, B15, B17) and the unseen shout cue

**Contract this unlocks:** under fire, incoming missile and friendly destroyed nearby, each built from physical evidence the simulation already has. The rules are in `choices.md` → "The three physical barks". Load [`tweak-mechanics`](../../../.agents/skills/tweak-mechanics/SKILL.md). Write a test for each ruled-out moment, not only the happy path.

## Seam

All three extend `ShoutEvidence` and `shouts.rs` from slice 11.

**B5, under fire**
- **Evidence:** this tick's `FlightEvent::NearMiss`, `Impact` and `Ricochet` from **enemy** rounds, scored for distance with the weapon's `suppression_radius_m`, the same as `near_miss()` (`damage.rs:667-672`). It applies to every unit, **hulls included**.
- **Capture it before** `rounds.retain` (`battle.rs:1977`), because rounds that ended this tick are gone after it.
- **Do not reuse** suppression's squad-only filter (`damage.rs:388`).
- **First, confirm that hulls receive `NearMiss`.** Standoff is checked at `flight/mod.rs:1014` and `:1289`. If they don't, take a hull's evidence from impacts and ricochets on it plus near-miss geometry against its hull extents. Record which in `choices.md`.

**B15, incoming missile**
- **Evidence:** every enemy guided round in flight whose `Guidance.point` (`flight/mod.rs:364-370`) lies within the unit's footprint (hull extents, or the squad area's radius).
- **The unit must see it:** `sensing::sees_point(world, sensing::eye(unit), missile_position, sight::of(unit), 1.0, &rules.sensors)` (`sensing.rs:67`, `:102`).

**B17, friendly destroyed nearby**
- **Evidence:** this tick's unit deaths.
- **Who shouts:** the nearest living friendly for which `sees_point` to the dead unit's position holds. Only one shouts.

**Unseen shout cue** (the simulation half of the user's murmur decision; the client half is slice 15). It lives here so slice 13's single re-bless covers its bytes.
- In `crates/sim/src/hearing.rs` (`:16-64`), an unseen enemy's shouts since the last sound bucket (hearing runs only on bucket ticks, `battle.rs:1380-1392`) produce **one** cue per source per bucket.
- The cue uses the new `SoundCategory::Shout`. It is heard by the nearest listener within the source's `loudness_m`, and carries **only** `listener`, `sector` and `band`: no trigger, set, kind or position.
- Unseen shouts still obey the ledger's cooldowns.
- Add `SoundCategory::Shout` to the contract enum (`observation.rs:124-128`) in this slice. Slice 13 publishes it.

## Ruled-out moments (each one is a test)

| Bark | Moment | Expected |
|---|---|---|
| B5 | own-side rounds pass close by | silent |
| B5 | the unit's own rounds | silent |
| B5 | a squad already Suppressed or Pinned | silent (B6/B7 own it) |
| B5 | a tank pinged by rifle fire, with ricochets | shouts |
| B5 | a sabot passes a tank | shouts |
| B5 | the shooter is unseen | shouts |
| B15 | the missile is aimed at the neighbouring tank | only the neighbour shouts |
| B15 | the missile is behind a ridge | silent until it crests into sight |
| B15 | the unit's own missile | silent |
| B15 | a released (unsupported) missile whose fixed point lies on the unit | still shouts if seen |
| B17 | a death nobody can see | silent |
| B17 | three friendlies watching | exactly one, the nearest, shouts |
| B17 | the dead unit itself | never the witness (it has B16) |
| unseen cue | an unseen enemy shouts | one cue for the nearest listener; it carries no trigger, set, kind or position |
| unseen cue | the same source shouts 5 times in one bucket | one cue |

## Verification
- `crates/sim/tests/shouts.rs`: the table above.
- **Digests unchanged:** the same gate as slice 11.

## Delegated to you
How the evidence is captured internally.

## Must stay green
Everything listed in slice 11.

## Feedback that would change this slice
The user hears "Taking fire!" too often from tanks under rifle fire: raise the tier floor for hulls. That is a data value in `ShoutRules`, not a named case.
