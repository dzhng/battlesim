# P1 (part 2, its own push) — a jet's 3D attack line

**Seam.** The attack drawer's fixed-wing branch draws only the attack line, in the
overlay's opaque mesh layer for marks in the air (where drop lines draw), from the jet
model's position to the target's 3D point or the ground point. No ground chain, ground
line or reticle (decision 11). It draws the current attack only.

**Tests.** `web/tests/attackLine.test.ts`: a fixed-wing attack yields one segment in the
air layer from the airframe to the target point, and nothing in paint.

**Visual.** A jet doing attack-ground → queued move in a real battle, before and after
in the Space view (decision 12). `screenshot-critique` unprimed, `compare-screenshots`,
show the user inline.

**Commit and push.** `feat(overlay): a jet draws its attack line in the air`, `/review`,
push `staging`, verify the ancestor check, then `close-spec`.
