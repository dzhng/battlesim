# Tick budget: decisions

Decisions taken where a faster rule changed what the battle does.

## Out of range is judged before the line of fire

- **The choice:** a mount judges a target's reach before it solves or flies
  the arc. A target whose every possible meeting point lies beyond the
  weapon's reach is out of range, whatever stands between.
- **What changed:** such a target used to report a blocked trajectory when a
  body also stood in the way; it now reports out of range. Two retries that
  a blocked line triggers no longer run for it: aiming at each seen soldier
  of a squad whose middle is blocked, and firing from each soldier's own
  muzzle when the squad's is blocked. A squad whose middle is past reach while
  one of its soldiers stands inside it is therefore out of range, as it
  already was when nothing stood in the way.
- **Why:** 98% of the firing solutions a late battle asked for were for
  targets beyond reach. Counted on the recorded battle, the two retries
  decided 0.3% of the shots a mount could take, all at the edge of reach.
- **Cost:** battle digests move; the recorded battle plays out differently
  from its fifth minute. No pinned digest or replay in the sim crate moved.
- **Skirmish AI:** it relocates a unit whose idle weapon reports a blocked
  trajectory. A unit whose blocked target is also beyond reach no longer
  sidesteps, which could not have brought the target into reach.
