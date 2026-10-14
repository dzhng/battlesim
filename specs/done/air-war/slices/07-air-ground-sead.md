# 07 — Air-to-ground missions and SEAD ordnance

**Contract:** aircraft ground orders never depend on aircraft Ground vision.
Right-click Ground and Attack Ground use a bombing run when a bomb/guided
rocket is available, otherwise a strafing run. Bomb rows author guided/cluster
spread. SEAD aircraft are EA-18G, Tornado ECR, and an Eastern Su-34 variant;
their anti-radiation missiles have the longest aircraft range and may fire only
at visible radar emitters or radar contacts. SEAD auto-fire prioritises
long-range radar AA. No hidden Ground retargeting is permitted.

**Seam:** `sim::air_combat` mission policy, `sim::weapons` target filters,
command order validation, flight release, and fixtures weapon rows.

**Runnable proof:** `/lab/air-ground` shows strafing, guided/cluster bombing,
attack-ground, SEAD launch at a radar contact, and a non-radar/hidden rejection.

**Checks:** bomb-vs-strafe availability, spread determinism, ground order
semantics, SEAD-only radar targets, longest-range ordering, no Ground vision,
and replay parity.

**Scope firewall:** no new radar model, dogfight transitions, or final audio.
