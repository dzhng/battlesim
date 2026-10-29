# 02 Spike: which round a hull fired

**Question:** can presentation tell which weapon a hull fired, without adding
a published field?

Today a hull launch takes its mount's `kind`, which is the mount's first
weapon row (`effects/launches.rs` `hull_launch`, `effects/publication.rs:52`).
So the BMP-3's Bastion flashes, sounds and kicks like its 100 mm gun, and tank
HE fires with AP's look. 21 hull mounts carry two weapons.

A vehicle gun's segments publish shooter -1
(`sim/src/publication/layout.rs:140`). A mount fires at most one weapon per
tick.

**Candidates.** Both use existing fields. Pick the one that gives soldiers and
hulls **one** matching rule:
- **a.** Fill the existing `shooterLo/Hi` of a vehicle gun's segment with the
  firing unit's key. Hull launches then match their rounds the way soldier
  launches match by member.
- **b.** Publish the round a mount last fired in the shooter row's existing
  mount `kind`, instead of its first weapon. Soldier matching
  (`s.kind != mount.kind`) gets the same benefit.

Rejected: matching by position near the muzzle. It's a guess, and it fails for
adjacent hulls and for rounds whose first stretch isn't visible.

**Run / see:** a scratch branch of the probe from 01 prints, for every hull
shot in the BMP-3 and tank traces, the round fired against the round
presented.

**Verify:** for each candidate, record:
- whether any battle digest, `exact_numbers` record or replay moves (a
  published value is not simulation state, so none should);
- which presentation vectors move;
- whether the flash keeps its tick, with no added one-tick delay.

**Moves:** nothing; the spike is scratch.

**Outcome:** write the choice and its evidence into
[12](12-firing-look.md) and delete the scratch code.

**Delegated:** everything inside the spike.

**Would change the plan:** if neither candidate works without a new field,
say so in 12 and ask the user. A new published field reopens a decision.
