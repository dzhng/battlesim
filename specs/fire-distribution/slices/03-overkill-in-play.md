# 03 Overkill in play

## Contract

The README's "Overkill coordination" rules run in battle for any row that
sets `avoids_overkill`. Rows still unflagged in `rules.json`; tests flag their
own rows. Decisions D1, D6, D10–D17, A1–A4, A6.

## Seam

1. **Expected arrival on `Round`** (`battle/fire.rs`). Set at launch to the
   launch tick plus `Ready.time_of_flight_s` in ticks; carry the time of
   flight on `Shot` so `battle/fire.rs` can set it. Digest it in
   `battle/state_digest.rs` with the other `Round` fields.
2. **Build the ledger** once per fire step in `Battle::fire`, before the
   `context!` macro: every `Round` whose row is flagged, `aimed_at` is a unit,
   expected arrival is not past, and that is still steering if guided (seeker
   row: `seeker_target.is_some()`; launcher-guided row: the projectile's
   `guidance` is supported; decide guidedness from the row, never the
   projectile alone, since seeker rounds launch unsupported). Damage via
   `overkill::hit_damage` from the round's current position.
3. **`FireContext.committed: &Committed`.** In `tick::advance`, clone the
   ledger into a local tally; for each unit, shadow
   `let ctx = &FireContext { committed: &tally, ..*ctx };` and after each
   flagged launch add it to the tally (from the muzzle, arrival from its
   `Ready`), including launches made under orders.
4. **The gate** in the firing reason chain in `weapons/tick.rs`, just before
   `fire(...)` and after `Guiding`: if `lock.target != ordered`, the kind
   `ready.kind` is flagged, the target is a unit, and
   `overkill::doomed(.., now + ready.time_of_flight_s)`, report
   `HoldingFire` and do not fire, leaving `engaging = true` (A2).
5. **Reconsider** in `weapons/choice.rs`: `automatic_lock`'s reconsider
   check uses the cached `assessed.shot` and also reconsiders a doomed lock,
   unless the mount is guiding (`mount.support.is_some()`, A3). `select`
   switches to `assessed.shot` (cached; no extra solve), skips doomed
   candidates for a flagged kind, remembers them, and returns one only when
   nothing else qualifies, preferring the current lock (A4).

## Verify (tests first, in `crates/sim/tests/fire_distribution.rs`)

A few bigger functional tests over test units (`test_at`, `test_tank`,
`test_jeep`, `test_gun_jeep`, `test_rifle`), flags set in the test's rules,
hp thresholds derived from row damage, Trophy patched on with `patch_catalog`
(as `tests/protection.rs` does):

- **Teams spread (S1).** Three ATGM teams on one tank whose health two
  missiles cover: two launch, the third holds; with a second tank present,
  the third switches to it.
- **Same-tick (S2).** Three tanks finishing their aim on the same tick at a
  damaged tank: launches stop once the tally covers its health.
- **Follow-up (S3, S8).** Two teams on a light target: the second holds with
  its aim full; release the first missile's guidance by moving its team, and
  the second fires sooner than its aim time.
- **Missed shell.** A held crew fires once a counted shell passes its
  expected arrival without killing.
- **Trophy (S4).** One charge: two missiles launch before a third holds.
- **Unflagged untouched (S5–S7, D17).** With a flagged missile in flight, an
  autocannon, an HMG, HE and rifles keep firing, and a default gun keeps
  plinking at a tank it can't hurt.
- **No ammo swap (D13).** A tank held on AP never launches HE at the doomed
  tank.
- **Orders (D1).** A flagged mount under an explicit attack order fires at a
  doomed target.
- **Attack-move (D15).** A held unit on attack-move stops.
- **Held squad keeps engaging (A2).** A held RPG soldier's squad keeps its
  engaged posture and its RPG mount.

## Stays green

All sim tests, with `target_priority_tests`' hand-built `FireContext`s given
an empty ledger. Digests change only through the `Round` arrival field;
name it. Outcomes otherwise unchanged, because no fixture row is flagged yet.

## Delegated

- Whether the gate sits before or after the garrison window check.
- How the time of flight reaches `Round` (on `Shot`, or on each `Launch`).
- Helper names and module-internal structure.

## Feedback that would change it

If holding plays badly on screen, the fallback is ranking doomed targets last
without the hold (README "Rejected" lists why it lost). That is a user call.
