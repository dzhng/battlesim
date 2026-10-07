# 15 Tracked families

Runs in parallel with the other lanes (slices 15–18) once the pilot (14)
sets the bar; workers take one family at a time, export and validate only,
and the coordinator bakes and commits runtime output in batches.

**Unlocks:** every runtime tank and tracked IFV/recon vehicle at the pilot's bar.

Families, one worker each at most, shared platforms once:
Leopard 2 (2A6, 2A7V, 2A8), Challenger 2 TES, Leclerc XLR, KF51 Panther,
T-72B3 2016, T-80BVM, T-90M, Type 99A, Bradley (M2A4, M3A3), CV90 (CV9040C,
Mk IV), Puma, Ajax, BMP-2M and BMP-3.

## Per family

1. Collect its references (README [References](../README.md#references)), then read them and its frames (fixture catalog).
2. Rebuild in its own `roster/<family>.py` from `vehicle_parts.py`, variants
   as branches. Correct road-wheel, return-roller and sprocket counts and
   positions from the photos (for example: Leopard 2 seven road wheels and four
   return rollers; T-72/T-90 six large road wheels, three return rollers;
   BMP-3 six road wheels, its own hull; Bradley six road wheels).
   Russian ERA (Kontakt-5, Relikt) as tile blocks where the vehicle wears them;
   T-90M's slat cage, T-72B3's side screens as the reference shows.
3. Scheme from slice 13, chosen from the photos.
4. Within the slice 14 budget.
5. Tank commanders in open hatches where the references show them, through
   `vehicle_crew.py` as fixed in [slice 09](09-pipeline.md).
6. Delete the family's branch from its legacy helper; delete a helper when empty.

**Own wreck.** Each variant exports its own wreck (slice 11): the burnt hull
at the unit's size, plus a turret piece where it has a turret, replacing its
interim wreck.

## Verify (per family)

validate, bake, check; `asset sheet --references`; compare-screenshots
against before and references; screenshot-critique unprimed, last. Batch the
user preview by group (Western MBTs, Eastern MBTs, IFVs), non-blocking: about
five minutes, then decide on the evidence, record, close the shots, continue.

## Delegated

Order of families; worker split by family.
