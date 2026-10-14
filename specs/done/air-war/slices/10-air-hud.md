# 10 — Aircraft cards, ghosts, radar glyphs, and controls

**Contract:** presentation decodes only published state. Aircraft have no
permanent ground circles. Traversing aircraft use a compact info card attached
to their ingress ghost while Space is held; selected aircraft cards show fuel,
flares, afterburner, dogfight, radar, and AA/SEAD state. Radar contacts use an
area-circle visual related to fog contacts but carry explicit “RADAR CONTACT”
text and bearing/range; they never imply exact Ground visibility.

**Seam:** presentation observation decoder, `web/src/battle/present`,
battle-renderer overlays, selection/picking, and existing typed command input.

**Runnable proof:** `/lab/air-hud` covers ingress ghost/card, HighAir/LowAir
contacts, radar area contact, dogfight state, and no-circle regression.

**Checks:** publication decode, card transitions, ghost picking, Space preview,
radar text/layer separation, explicit order controls, and no browser-owned
physics or hidden target state.

**Visual gate:** invoke `game-ui`; judge ghost/card, radar contact, and dogfight
state as separate crops, compare where targets exist, run critique last, and
offer Preview non-blockingly.

**Scope firewall:** no simulation or target-selection decisions in TypeScript.
