# 06 — Ingress, fuel, afterburner, and withdrawal

**Contract:** a purchased aircraft appears immediately as paid and greyed with
a Traversing card. Its intended destination comes from the ordinary ghost; the
simulation selects the nearest friendly map-edge point and uses a seeded 15–30
second off-map delay. Fuel starts at physical map entry and lasts about three
minutes. Afterburner is an explicit persistent toggle: +50% speed, 2x fuel
burn, and reduced ground-attack accuracy. Near-empty aircraft retreat; refund
weights health, remaining ammunition, and remaining fuel. Destruction pays no
refund.

**Seam:** skirmish reservations/dispatch, `units::AirState`, observation own
unit rows, withdrawal settlement, and ArmyDeck/ghost publication. No old
fixed-center spawn or sortie adapter survives the cutover.

**Runnable proof:** `/lab/air-ingress` covers both edges, seeded delay, ghost,
card state, map entry, fuel/afterburner, emergency retreat, and refund.

**Checks:** payment/card transitions, nearest-edge placement, no teleport,
fuel monotonicity and burn ratio, retreat/refund arithmetic, destroyed/no
refund, deterministic replay and WASM publication.

**Visual gate:** compact traversal ghost/card and fuel state; use `game-ui`,
comparison if a target exists, critique last, and Preview non-blockingly.

**Scope firewall:** no rearm loop, airfield, dogfight target policy, or final
aircraft art/audio.
