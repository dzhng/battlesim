---
name: game-ui
description: Design or judge the player-facing UI of the battle game so it feels like a holo-tactical RTS (WARNO, Broken Arrow), never a web dashboard. Use when adding or changing anything the player sees or reads: ground markers, order lines, selection, rings and ranges, callouts and info panels, the HUD and command bar, menus, contact and fog glyphs, measuring tools, or colour choices; and when reviewing a UI screenshot.
---

# Game UI

The player's UI is a **holo-tactical** layer over a war film: marks that belong to the battlefield, and information that appears when the player needs it. The references (WARNO, Broken Arrow, ARMAPHRACT) are a floor to improve on, not a template to copy. The `renderer` skill owns *how* things are drawn. This skill is the judgement about *whether* something is shown, *where*, and *how it reads*.

## Workflow

1. **Name the player's question.** Every element answers one, such as "is it in range?", "where is it going?" or "is it resupplying?". If you can't name the question, don't add the element.
2. **Weigh it against the principles below.** Most of the decision is whether to show it at all, and which existing surface it belongs on.
3. **Shoot it and judge it** at default and far zoom, over grass, road, fog and smoke. Run `screenshot-critique` unprimed. When the choice is a matter of taste, show labelled variants side by side with `preview-shots`, rather than describing them.

## Principles

- **Show information only when it really helps.** An element earns its place by changing what the player decides at that moment. Two tests:
  - *Is it related to what this element is for?* The ruler measures range, so the name of the unit it measures from doesn't belong on it; the player already knows, because they selected it.
  - *Is the state worth knowing?* "Being resupplied" is, and "not being resupplied" isn't. Show the state that matters, and let its absence carry the rest.
- **Reuse the surface that already fits before creating a new one.** A unit's state goes on that unit's existing floating panel, not on a new ground marker. A new ring, glyph or panel is a last resort; each one adds to what the player must learn and read.
- **Contextual, not permanent.** Most information matters during one action: while a unit is selected, while a key is held, while the cursor is over something. Show it then and hide it otherwise. A supply truck's reach matters while you are placing that truck, not all battle.
- **Marks belong to the world; information floats.** Anything describing a position, extent or path lies on the ground like paint. It sits under units and shines through smoke, and it never lies across a model. Readouts and state float beside their subject in the holo style. A model is never recoloured to show selection.
- **Each colour means one thing.** A colour that shows up in a second role muddies both. When a new element needs colour, reuse the role it shares or ask the user. Never add a hue silently. The current roles live in the fixture's presentation scheme; read them there.
- **One subject, one mark.** Marks belong to what the player commands: the unit, not each soldier. A tool that could apply to every selected unit shows one (the most relevant) to keep the view quiet.
- **Clean geometry reads as a game.** Keep strokes thin at a fixed on-screen width. A line meeting a circle stops at its border instead of piercing it. The same concept looks the same on every kind of unit.
- **Form before colour.** States differ by shape (full or broken, filled or hollow), so they read at a glance, in fog and for colour-blind players.
- **Motion and effects exist for readability.** Animate to make direction or change obvious, and make projectiles slow and bright enough to follow (Hollywood realism). Never animate for decoration.
- **The UI never disagrees with the rules.** A readout measures the way the simulation does, and the fog shows what the side actually knows.

## Smells

- It looks like a dashboard: boxed panels, tables, browser controls, flat uniform fills, emoji.
- An element whose question you can't name, or a state shown only because it exists.
- A new marker or ring for something the unit's panel could say.
- The same fact in two places, or a colour used in a second role.
- A mark per soldier, or per selected unit, where one would do.
- Something visible all battle that matters only during one action.
- A mark over a model, or a line cutting through a unit's ring.
