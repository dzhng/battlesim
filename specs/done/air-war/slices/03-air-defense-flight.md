# 03 — Authored AA, seekers, and projectile interception

**Contract:** AA and aircraft weapons use authored target layers, FOV, turn
rate, motor lifetime, guidance support, interception eligibility, decoyability,
and splash. Ground AA requires physical line of sight through buildings and
trees; forest placement can conceal an AA unit but cannot make it fire through
foliage. Short ballistic AA can engage Ground (reduced accuracy), LowAir,
HighAir, and incoming missiles. Medium missile AA targets both air layers and
intercepts only when its row explicitly allows it. Long AA has 4–8 rounds,
one-at-a-time slow reload, HighAir-only targets, and authored anti-missile rows.
Stinger/Igla-style short missile rows cannot intercept.

**Seam:** `contract::weapons`, `sim::weapons`, `sim::flight`, `sim::damage`.
Projectile-versus-projectile resolution is deterministic, uses the shared
flight store, and records ordering, splash, ownership, and ammo in the digest.

**Runnable proof:** `/lab/air-defense` includes short ballistic, medium SAM,
long SAM, a helicopter, jet, incoming SEAD missile, LOS blockers, flare and
near-splash cases.

**Checks:** ballistic spread/distance/perpendicular penalties, LOS gates,
missile turn/lifetime misses, interception and splash, decoyability, FOV,
reload/ammo, and native/WASM replay parity.

**Visual gate:** launch/intercept/readout crop; use `game-ui`, comparison when
available, last-pass `screenshot-critique`, and `preview-shots`.

**Scope firewall:** no radar contacts, SEAD target policy, dogfight policy,
roster promotion, or final balance tuning.
