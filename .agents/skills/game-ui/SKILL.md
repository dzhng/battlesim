---
name: game-ui
description: Design or judge the player-facing UI of the battle game so it feels like a holo-tactical RTS (WARNO, Broken Arrow) and never like B2B SaaS or a web dashboard. Use when adding or changing anything the player sees or reads: ground markers, order lines, selection, rings and ranges, callouts and info panels, the HUD and command bar, menus, contact and fog glyphs, measuring tools, or colour choices; and when reviewing a UI screenshot.
---

# Game UI

**It must never feel like B2B SaaS.** That is the test every other principle serves: when a choice is unclear, pick the one a game would make, not the one a web dashboard would. SaaS feel creeps in through panels and boxes, tables and forms, browser-default controls, flat fills, admin-style labels and settings-page density. Once it appears, the fantasy is gone.

The player's UI is a **holo-tactical** layer over a war film: marks that belong to the battlefield, and information that appears when the player needs it. The references (WARNO, Broken Arrow, ARMAPHRACT) are a floor to improve on, not a template to copy. The `renderer` skill owns *how* things are drawn. This skill is the judgement about *whether* something is shown, *where*, and *how it reads*.

## Workflow

1. **Name the player's question.** Every element answers one, such as "is it in range?", "where is it going?" or "is it resupplying?". If you can't name the question, don't add the element.
2. **Sweep the concept, not just your change.** Find every place the player already meets the concept you're touching, and bring them all into line. A new rule applied to one instance leaves the old look on the others, and the player is left with two looks for one concept.
3. **Weigh it against the principles below.** Most of the decision is whether to show it at all, and which existing surface it belongs on.
4. **Iterate on a component in isolation.** The battle is a slow, noisy place to judge a component. Render the real component on a plain background, in every permutation its data can produce, so no case surprises you later. When the choice is a matter of taste, let the user pick between variants, then keep only the winner.
5. **Shoot it and judge it** at default and far zoom, over grass, road, fog and smoke. Run `screenshot-critique` unprimed. When the choice is a matter of taste, show labelled variants side by side with `preview-shots`, rather than describing them.

## Principles

- **Show information only when it really helps.** An element earns its place by changing what the player decides at that moment. Two tests:
  - *Is it related to what this element is for?* The ruler measures range, so the name of the unit it measures from doesn't belong on it; the player already knows, because they selected it.
  - *Is the state worth knowing?* "Being resupplied" is, and "not being resupplied" isn't. Show the state that matters, and let its absence carry the rest.
- **Every unit has an info panel, and every unit state lives there.** Deployment, resupply, suppression, ammo and last-seen time go in the floating panel as an icon plus a short word. Ground markers are only for selection and movement: where a unit is, what is selected, where it is going, and how well each soldier is covered where he stands. A state drawn as a ring on the ground is a smell, however well it reads.
- **A panel reads the same way every time.** Its name, then what it fights with, then what it is doing, with steady facts above passing ones. Within it, each form means one thing: a ring round an icon is a timer running, and pips are an amount left. Draw a form only while it has something to say, so an idle row is just its icon and word.
- **Reuse the surface that already fits before creating a new one.** A unit's state goes on that unit's existing floating panel, not on a new ground marker. A new ring, glyph or panel is a last resort; each one adds to what the player must learn and read.
- **One concept, one component, everywhere.** A concept the player learns once should look the same wherever it appears, and be drawn by the same code. Cases differ in the data they feed it, never by a parallel version. This wins twice:
  - *less for the player to remember,* since every appearance teaches the same shapes and slots, and the eye learns where to look;
  - *less code and a simpler architecture,* with one thing to fix and restyle.

  For example, the confirmation after an order is the same view as holding Space, and an enemy's panel is the same panel as your own with different data. Before adding a variant, ask whether the existing component can carry it. If it can't, extend that component rather than fork it.
- **Contextual, not permanent.** Most information matters during one action: while a unit is selected, while a key is held, while the cursor is over something. Show it then and hide it otherwise. A supply truck's reach matters while you are placing that truck, not all battle.
- **Marks belong to the world; information floats.** Anything describing a position, extent or path lies on the ground like paint. It sits under units and shines through smoke, and it never lies across a model. Readouts and state float beside their subject in the holo style. A model is never recoloured to show selection.
- **Each colour means one thing.** A colour that shows up in a second role muddies both. When a new element needs colour, reuse the role it shares or ask the user. Never add a hue silently. The current roles live in the fixture's presentation blocks (`presentation.hud`, `presentation.overlay`); read them there.
- **One subject, one mark.** Marks belong to what the player commands: the unit, not each soldier. A tool that could apply to every selected unit shows one (the most relevant) to keep the view quiet.
- **Clean geometry reads as a game.** Keep strokes thin, sized on screen, and thinner as the camera pulls out, by one rule for every mark, so a far view's borders never outweigh the scene. A line meeting a circle stops at its border instead of piercing it. The same concept looks the same on every kind of unit.
- **Form before colour.** States differ by shape (full or broken, filled or hollow), so they read at a glance, in fog and for colour-blind players.
- **Motion and effects exist for readability.** Animate to make direction or change obvious, and make projectiles slow and bright enough to follow (Hollywood realism). Never animate for decoration.
- **The UI never disagrees with the rules.** A readout measures the way the simulation does, and the fog shows what the side actually knows.

## Smells

- Anything that would look at home in a B2B SaaS app: boxed panels, tables, forms, browser controls, flat uniform fills, emoji, admin-style labels.
- A font glyph or emoji standing in for an icon: every icon is the generated set's (`assets/icons/`), drawn to read at its size.
- An element whose question you can't name, or a state shown only because it exists.
- A new marker or ring for something the unit's panel could say.
- The same fact in two places, or a colour used in a second role.
- Two code paths or two looks for one concept: a near-copy of an existing view or panel, or rows of one kind that don't share a format.
- A mark per soldier, or per selected unit, where one would do.
- Something visible all battle that matters only during one action.
- A mark over a model, or a line cutting through a unit's ring.
