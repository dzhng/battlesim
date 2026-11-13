# Slice 07 unprimed critiques

Two unprimed critiques ran: one on the first look, and one on the final look, as the last check.

**Verdict: accepted.** The cloud's own rendering is clean in the final critique:
- no seams, hard sprite edges or lines where puffs meet the ground;
- overlapping clouds composite cleanly;
- the bloom reads as a discharge;
- it is clearly unlike the wreck's smoke, which reads as a tan column lit by its fire rather than grey-black;
- it is still a recognisable white puff at far zoom.

Over seen ground it is the dense white-grey billow the mock and D11 ask for. The receipts check passed on both critiques: they state no unattributed quantities and name no invented files.

## What changed after the first critique

| First critique's finding | Fix |
|---|---|
| The fade looked almost like the full cloud | A puff's opacity now follows the stepped density optically: it passes `(1 − opacity)^density` of the light behind it, so the billow's optical depth scales with density the way sight's does. |
| A warm cream tint | Albedo cooled to 0.82 / 0.84 / 0.87. |
| A ring of equal lobes at the edge | Rim puffs taper to three quarters of a middle puff's size. |

An intermediate pass (fewer, smaller and fainter puffs, with a short soft distance) broke the billow into separate cotton balls. It was rejected on screen, and the final look is denser: 64 puffs at opacity 0.6.

## Triage of the final critique

**1–3, 6, 7: the cloud takes the fog veil, which is the spec's contract.**
- **The findings:** the cloud splits into a white half and a hatched lavender half at the fog rim. The clear patch round a unit inside reads as a flat cream plate with a white rim. The sight shadow dominates the far view.
- **Why it happens:** the slice requires screen puffs not to lower the fog mask, so a cloud over ground the side doesn't see is veiled like that ground. A unit in smoke is nearly blind, so most of a side's own cloud lies on unseen ground and draws hatched. The plate is the fog's own seen patch, about 5 m, round the unit inside.
- **Status:** the fog is frozen in this slice, so this is left as seen and raised as an open question in `choices.md`.

**4, 5: a faint rectangle of thinner smoke round the hull inside the cloud, and the barrel drawn crisply over the smoke.**
- **Why it happens:** depth occlusion of camera-facing puffs. The hull's top stands in front of a low puff's plane and hides that part of it.
- **Status:** a billboard puff can't wrap round a hull. A shorter soft distance didn't remove it; the denser final look mutes it. It also reads as "the tank is in there", which answers the slice's silhouette question: no x-ray is needed.

**Others:**
- **8, lobed outline:** softened by the rim taper. What remains is the dust flipbook's cumulus.
- **9, the half-way fade still dense:** honest. At half density, sight through the cloud is still blocked within 10 m.
- **10, the canopy hides the squad's cloud in the wood:** realism. The cloud is 5 m tall, under the crowns.
- **11, red's last-contact markers cover the clouds:** the existing marker.
- **12, label contrast:** the HUD is frozen in this slice.
