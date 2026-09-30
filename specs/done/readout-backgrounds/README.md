# Readout backgrounds

Floating unit information needs contrast without becoming a wall of framed cards. The user selected [concept A](assets/concept-a.png): soft dark shading, generous breathing room, sharp text and unchanged unit anchoring. The concept was generated from this game's screenshot with the built-in image-generation tool; its [prompt](assets/prompt.txt) records that provenance. It is a design reference, never runtime art. Generated text and scenery distortions are not implementation requirements.

The shared callout style in [hud.css](../../../web/src/hud.css) owns the backing for friendly units, enemies and contacts. The backing paints separately from text, so its softness does not blur glyphs and its extra visual space does not displace labels or their leaders. Selection keeps the existing foreground priority. Dense placement remains best effort.

Implementation is judged against the selected concept with [design-with-images](../../../.agents/skills/design-with-images/SKILL.md), not solely against an earlier implementation. A narrow shadow around a tight rectangle did not reproduce the concept's diffuse shape; direct reference comparison exposed the difference despite passing behavior checks. The [readout scene](../../../web/scenes/readouts.mjs) owns placement and command integration, while screenshots judge the visual design.
