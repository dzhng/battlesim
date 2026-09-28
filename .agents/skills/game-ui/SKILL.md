---
name: game-ui
description: Design or judge the player-facing UI of the battle game so it feels like a holo-tactical RTS (WARNO, Broken Arrow), never a web dashboard. Use when adding or changing anything the player sees or reads: ground markers, order lines, selection, rings and ranges, callouts and info panels, the HUD and command bar, menus, contact and fog glyphs, measuring tools, or colour choices; and when reviewing a UI screenshot.
---

# Game UI

The player's UI reads as a **holo-tactical** game layer: glowing marks painted on the battlefield and slim holo panels hanging off units, glanceable mid-battle. The references (WARNO, Broken Arrow, ARMAPHRACT) are a floor to improve on, not a target to copy. The renderer skill owns *how* marks are drawn; this skill owns *what* is shown, *where*, and *in which colour*.

## Workflow

1. **Place each element in its layer** (below). An element in the wrong layer is the most common defect.
2. **Give it one colour role** from the palette. If none fits, ask the user rather than inventing a hue.
3. **Decide when it shows.** Default to *contextual*: shown while it answers a question the player is asking (a unit is selected, Space is held, the cursor hovers), hidden otherwise.
4. **Check that nothing is redundant.** If the player can already read it elsewhere (the unit card, the panel, the command bar), drop it here.
5. **Shoot it and judge it** at default and far zoom, over grass, road, fog and smoke. Run `screenshot-critique` unprimed. When the user must choose between looks, show labelled variants side by side with `preview-shots`, rather than describing them.

   **Done when** every element has a layer, a role and a visibility rule, and the shots pass the smells below.

## Layers

- **Ground marks are paint on the ground,** under units. They are depth-tested, never drawn over a body, and glow a little so they shine through smoke and effects. Grass and rocks they cross are lit by the paint, as if the mark were a light shining up. Selection circles, soldier markers, order lines, rings, the ruler and zones all belong here. A mark around a unit is sized to peek out from under it.
- **Info is holo callouts:** thin cyan panels tied to their unit by a leader line, floating beside the unit, never boxed on black. Per-unit state (ammo, resupply, suppression) goes here as an icon plus a short word, not as another ring on the ground.
- **The HUD chrome is a game surface:** a full-width bottom command bar, game-styled controls (no browser checkboxes or sliders), and whole cards that are clickable.
- **Models are never tinted for selection.** The x-ray highlight only colours a unit's parts hidden behind something.

## Palette (colour roles)

- **Order ink:** true yellow.
- **Ground markers** (selection, soldiers): amber. **Amber appears nowhere else,** not in text and not on leader lines.
- **Info panels, their text, icons and leader lines:** cyan.
- **Secondary marks** (destinations, route lines): white.
- **Enemy, danger, contacts, the playable border, out of range:** red.
- **Cover:** a ramp from light yellow to strong green, drawn as a pip centred in the soldier's marker.

The fixture's presentation block owns the values. Read the current scheme there rather than hard-coding a hue.

## Rules

- **One unit, one mark.** Orders, lines and rulers belong to a unit, never to each soldier. When several units are selected, a measuring or detail tool picks one (the nearest to the cursor), so the view stays quiet.
- **Lines meet circles at their border.** A line from or to a unit's ring, or to an end marker, stops at the ring's edge and never pierces it. Infantry and vehicles use the same marker style.
- **Thin, clean strokes** with fixed on-screen widths. A thick line or a glowing blob reads as SaaS chart art.
- **Minimal text.** No labels on destination markers, and no restating the unit's name where the selection already says it. A number carries its unit ("600 m").
- **Shape before colour.** States differ by form (full against broken ring, filled against hollow dot), so they read at a glance and in fog.
- **Motion clarifies,** for example animated travel chevrons. Never animate for decoration.
- **Contextual rings.** A service or reach radius (a supply truck's reach) shows only while its owner is selected.
- **Fog is whole.** A structure is fogged all over or not at all. Unseen marks stay in their place and fade; they never track the hidden unit.
- **Help the player decide.** Prefer readouts that answer tactical questions, like "is it in range?" or "will cover hold?", over data dumps. Measure the way the sim does, so the UI never disagrees with the rules.
- **Hollywood readability:** projectiles and effects slow enough and bright enough to follow, and your own units always audible (a distance floor) even when zoomed out.

## Smells

- It looks like a dashboard: boxed panels, table layouts, browser controls, uniform flat fills, emoji.
- A mark sits on top of a model, or a line crosses a unit's body or ring.
- The same fact appears in two places, or a state gets its own ring when the unit's panel could say it.
- A colour appears outside its role, most often amber on text or panels.
- Many near-identical marks: one per soldier, or one per selected unit, where one would do.
- A new element is visible all the time when it matters only during one action.
