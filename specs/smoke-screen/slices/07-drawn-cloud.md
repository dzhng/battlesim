# 07: the drawn cloud

## Contract

A smoke cloud is drawn as a dense white-grey billow (D11) that:
- blooms and fades in step with the simulation's cloud;
- swirls within its circle and never drifts out of it (D8);
- reads clearly differently from the grey-black smoke of wrecks;
- costs nothing from the wreck-smoke budget.

## Seam

- **Look rows.** Add `fixtures/presentation.json` rows
  `smoke_looks.{discharger, grenade}`:
  - albedo, opacity, puff sizes, swirl rate and height profile;
  - a `sound` slot, used by slice 10.

  The capability's `cloud.look` names its row, following the weapon-look
  pattern. An unknown look fails when the catalog is fielded.
- **Budget.** Add `effects.smoke_screen_budget`, kept apart from the 4096-puff
  `smoke_budget` (`effects/frame.rs:655`, `presentation.json:552`).
- **Law.** A new `crates/presentation/src/effects/frame/smoke_screen.rs`:
  - puff slots seeded by the cloud id;
  - each puff orbits within the current stepped radius and stays below the
    cloud's height;
  - opacity follows the stepped density;
  - no wind term.

  Feed it from `ObservationFrame.smoke_clouds` through the effects publication.
  It is not a wreck-style smoke source, because those always drift
  (`smoke.rs:13`).
- **Fog interaction.** Smoke-screen puffs must **not** lower the fog mask the way
  other effects do (`passes_effects.wgsl:113`). Otherwise a cloud on unseen
  ground would draw as seen.
- **Known traps to judge on screen:**
  - **A clear bubble round the unit.** The soft-particle fade works by depth
    over `radius * 0.8` (`smoke.rs:93`).
  - **Popping and flicker in overlapping puffs.** They are unsorted, blended
    with premultiplied alpha.
  - **Silhouettes.** An own unit inside the cloud has no x-ray silhouette.
    Decide whether that reads well; the panel's HIDDEN row is the
    authoritative signal.

## What you can see

The smoke lab, now with a drawn cloud.

## Tests and checks

- Every puff stays inside the circle and below the cloud height for the
  cloud's whole life.
- The wreck-smoke instance count is unchanged when screens are present.
- An unknown look is refused.
- **Visual checks:** load `renderer` and `game-ui` first. Take shots at default
  and far zoom, over grass, road and forest, at bloom, at full strength, mid-fade,
  with two overlapping clouds, and next to a burning wreck.
  - Judge before against after, and the cloud against the mock's billow, with
    `compare-screenshots`.
  - Run an unprimed `screenshot-critique` as the **last** check.
  - Show the user with `preview-shots`. This is non-blocking: allow about 5
    minutes, then decide on the evidence and record it.

**This slice's one visual variable:** the cloud's look (density, colour, motion).
The fog (06) and UI (08) are frozen.

## Delegated

You may choose the puff count, the sizes and the swirl law's internal shape, as
long as no puff leaves the circle. The grenade and discharger rows may share
everything except size.

## Must stay green

- The effects vectors, re-blessed only for the new rows.
- The wreck-smoke look.
- Frame cost.

## Feedback that would change this slice

The user may react to colour or density. Retune the look row only; never touch
D1's sight values to make the cloud look thicker.
