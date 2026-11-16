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

**State.** Done. `OrderView.aimAloft` carries where a winged aircraft's current attack
aims; the lab's `attackAim` resolves it from what the side sees (an identified enemy's
observed position, a ground point as ordered, else the step's point on the ground), so
no publication layout changed. `orderOverlay`'s `attackAloft` draws dots in the attack
red from the airframe to it, spaced as a drop line's, in the overlay's air layer; nothing
is painted on the ground. `web/tests/attackLine.test.ts` failed before and passes.

Evidence: [the air-ground lab's jets, Space held](../assets/p1-jet-attack-lines.png),
[one line, 2x](../assets/p1-jet-attack-line-crop.png). Before, a jet's attack drew no mark
at all. An unprimed critique found each line followable from its jet to where it aims;
it noted the line reads nearly flat (a jet at 250 m attacking a point 2 km ahead is a
shallow slope: the geometry, not a defect), no mark where it meets the ground (decision
11: no ground reticle for jets), and small dots (the drop lines' size, one look for air
marks). Left to the user's eye.
