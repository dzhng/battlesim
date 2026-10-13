# 08 — Air survivability, aspect damage, and flares

**Contract:** aircraft and helicopters receive aspect/directness damage from
the physical hit direction and tracking/evasion, with front highest, rear next,
and side lowest. Tune toward 1–3 long, 3–4 medium, and 4–6 short missile hits;
ballistic AA uses its spread rather than a hidden hit roll. Missiles have finite
turn/lifetime; afterburner can outrun a missile. `decoyable` is authored per
missile. Equipped aircraft have finite flare charges, automatic deployment on
seeker lock, and long reload. Flares divert decoyable seekers; nearby splash
can still damage the aircraft.

**Seam:** `sim::damage`, `sim::flight`, `sim::air_combat` flare state, and
publication counters/reasons. Directional damage is one function shared by
LowAir and HighAir.

**Runnable proof:** `/lab/air-survivability` covers front/rear/side hits,
flare diversion, no-flare aircraft, lifetime outrun, and near-field splash.

**Checks:** aspect monotonicity, flare ammo/reload/no-loop, decoyable versus
non-decoyable, splash, afterburner fuel/accuracy/evasion, and replay parity.

**Scope firewall:** no new roster models, dogfight policy, or audio polish.
