# 02 — Design spike: the airborne lost-contact sign

**Status:** done (D picked; see Result). **Depends on:** nothing (runs alongside 01). **Owns:** D11 (look).

## Contract

The user picks the look of the red, perfectly circular sign that floats in the air where an enemy helicopter was last seen. The spike decides the look only; slice 13 builds it.

## API seam

None in code. The output is a written choice in this slice: shape, colour, halo or outline, size at map zoom and close zoom, and how it relates to the existing ground contact glyph (`packages/battle-renderer/src/contactGlyph.ts`, `presentation.contacts` in `fixtures/game.json`).

## What you can run or see

A sheet of 3–5 variants made with [design-with-images](../../../.agents/skills/design-with-images/SKILL.md), judged with [game-ui](../../../.agents/skills/game-ui/SKILL.md). Each variant is shown over ground fog, over a village and beside a metro tower, at map zoom and close zoom. Show the D18 drop line faintly as context, so the sign and the line read as one family. Evidence goes in `specs/helicopters/assets/contact-sign/`.

## Verification

The sheet must show every variant on the same three backgrounds and at both zooms.

**Visual variable:** legibility of the floating sign
**Crop or mask:** the sign plus 3× its size around it, at map and close zoom
**Out of scope (later slices):** the drop line's final design (12), the ground glyph, fog styling

1. Human checkpoint (**non-blocking**): open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and leave about 5 minutes for a reply, carrying on with independent work meanwhile. If the user is silent, decide on the evidence, record the decision and why in this slice and in [choices](../choices.md), close the opened shots and go on.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

How many variants and how the sheet is laid out.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

The user's pick. Any 'it should look like X in WARNO' reference becomes a reference image here.

## Result

**Recommended pick: D · Keyed disc. Pending the user's choice (non-blocking checkpoint).**

**Evidence:** everything is in [`assets/contact-sign/`](../assets/contact-sign/).
- `sheet.jpg` has five variants in rows and six backgrounds in columns: ground fog, a village and beside a metro tower, each at map and close zoom. Each cell is a 1:1 crop of a real game capture, with the D18 drop line and ground ring drawn faintly.
- `crops-3x.jpg` shows the sign and about 3× its size around it, enlarged 3×.
- `full-<variant>-fog-close.jpg` and `full-<variant>-tower-map.jpg` are full 1920×1080 frames. The fog-close frame also contains the real ground contact glyph, for comparison.
- `compose.mjs` is the provenance: the Canvas2D compositor and the source captures it reads, from the main checkout's `throwaway/`. These are mockups composited onto screenshots, not renderer output.

### Variants

Every variant uses the enemy red `presentation.hud.enemy` (1.0, 0.38, 0.32) and the pale outline `presentation.contacts.outline_color`. No new hue is added.

| Variant | Description | Image row |
|---|---|---|
| **A · Lifted twin** | The ground glyph's recipe on a disc that faces the camera: 0.32 red fill, 45° red hatch, pale rim and a soft red glow. Its size is fixed on screen: Ø26 px at map zoom, Ø44 px at close zoom. | `sheet.jpg` row A |
| **B · Red cloud** | A dense red core with a radial falloff to a soft edge, and a thin crisp pale rim. No hatch. Ø30 px at map zoom, Ø52 px at close zoom. This is the most literal reading of "almost like a red cloud". | row B |
| **C · Hollow ring** | A pale rim over a red band, with a nearly clear centre and a red pip in the middle. | row C |
| **D · Keyed disc** | A's recipe with a stronger fill (0.5) and hatch (0.75), a stronger glow, and a thin dark keyline (about 1.5 px, 55% black) just outside the pale rim. | row D |
| **E · Flat at altitude** | The ground glyph itself, lifted to flight height and lying flat, so it is an ellipse on screen. Included to show the "same mesh, just raised" option. | row E |

### Unprimed critique

From [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). The reviewer saw only the images and a neutral brief.

**Its ranking:** D, A, B, C, E. My own inspection agrees.

**What it found:**
- **D:** the keyline gives the cleanest edge on every background: grass, cream walls, dark fog and the terracotta roof. The fill and hatch still read at map zoom. It is in the same family as the ground glyph.
- **A:** the hatch runs at about the same diagonal as the fog's own hatching. Over fog and busy streets at map zoom, the inside turns to mud and only the rim carries the shape.
- **B:** reads as the most visible, but the soft halo looks out of focus rather than designed. Over a red roof the fill nearly merges with the roof. It looks like a UI status dot or notification badge, which is a SaaS smell, and without the hatch it matches the ground glyph poorly.
- **C:** has crisp edges, but busy ground shows through the hollow centre, and the 2 px pip disappears at map zoom. It reads as a reticle or radio button, which suggests live targeting rather than "last seen", and it is the weakest match to the ground glyph.
- **E:** is an ellipse, so it fails the "perfectly circular" requirement. It reads as lying on a roof or on the ground, which loses the airborne cue.

**Findings for later slices:**
- **The drop line and ring are nearly lost at map zoom.** The 1 px line and the 7 px ring disappear into the fog hatch and the street clutter, so the sign reads as an unanchored dot. This goes to slice 12, which owns the line's final design. At map zoom the line needs more weight, or the ring does.
- **The airborne sign is much smaller than a nearby ground glyph.** At close zoom it is about 5× smaller, so beside one it reads as an accessory. The ground glyph is sized in the world by its uncertainty, and the airborne sign is sized on screen. Slice 13 must decide whether the sign scales with the contact's uncertainty radius, with a minimum size on screen.
- **The airborne sign has no label.** Ground contacts show a type label such as "TANK". This one should go through the same last-seen panel. Per game-ui ("one concept, one component"), it should not get a parallel label.
- **The circle on a stalk can read as a map pin or lollipop.** This is medium confidence. It is mostly carried by the drop line, so it also goes to slice 12.

### Why D

These reasons are judged against [game-ui](../../../.agents/skills/game-ui/SKILL.md).
- **One concept, one component.** D is the ground glyph's own recipe: fill, 45° hatch, pale rim and glow. Only its orientation changes (it faces the camera) and its edge is keyed. The player reads it as "a contact, in the air" with nothing new to learn. Slice 13 can build it as a billboard mode of `contactGlyph.ts`, not a second glyph.
- **It is a perfect circle and reads as floating.** That meets D11. E fails here.
- **It is legible over fog and roofs.** The keyline is the only treatment that held on all six backgrounds. That includes the worst case, red on a terracotta roof, where B merged into the roof and A's inside went muddy.
- **It reads as holo-tactical, not SaaS.** The hatch and rim keep the tactical-glyph look. B's soft blob reads as a badge, and C reads as a form control or reticle.
- **Form before colour.** A filled, hatched disc keeps the same form as a ground contact. Its height comes from the drop line and its billboard facing, not from a new colour.

**Open choice for the user:**
- **The dark keyline is a contrast edge, not a new hue.** If the user counts it as a new colour role, A is the fallback, but it needs A's hatch decoupled from the fog's hatch angle.
- **B is the closest to the words "red cloud".** If the user wants that feel, take B's soft outer glow onto D's crisp hatched disc, not B as it stands.

### Checkpoint resolution (2026-10-10)

The sheet was opened for the user in Preview for about five minutes with no reply. **Picked: D · Keyed disc**, on the evidence above. This is reversible: if the user picks another variant later, slice 13 builds that one instead. Slice 12 carries the drop-line weight and pin findings; slice 13 carries sizing (scales with the contact's uncertainty, with a minimum on screen) and the label through the existing last-seen panel.
