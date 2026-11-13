# Smoke screen: decisions

The user's decisions, and the ones the agent made on the user's behalf, from the
explore-unknowns walk on 2026-11-12. The [map](visualizations/map.html) holds the
walk, and the [mock](visualizations/mock.html) holds the moment the user reacted
to. Where an answer was given by the agent, the reason it rests on is
recorded alongside.

**Status (2026-11-13):** every decision below is built, checked against the
shipped code at closeout. D9 needed no new code: it is the existing
shared-spotting rule, and a simulation test pins it for a unit in smoke. D18 is
the roster switch-on, the command bar in every battle, and the tutorial's smoke
item. Choices the build made where these were silent are in
[`choices.md`](choices.md).

**Preferences the user confirmed, used for every delegated answer:**
1. Realism decides trade-offs.
2. The player decides; there are no reflexes for player units.
3. Share forest's logic unless doing so would make the screen lie.
4. The agent's suggested numbers are fine starting values.
5. Say whether each item is existing behaviour or new.

**Delegation scope:** data values and engineering calls. It excludes new
mechanics, changes to who gets smoke, and anything that adds work.

## Rules

- **D1: sight (user).**
  - Smoke reuses forest's reach formula: reach = range × concealment ×
    e^(−depth), with a full block at depth ≥ `foliage_full_block`.
  - Smoke adds **0.2 depth per metre** of sight line inside a cloud, below its
    height. That is a full block after 5 m.
  - A target standing in smoke gets **concealment 0.1**, through the existing
    "strongest concealment wins" rule in `sensing::ground`.
  - **Why:** the user first asked for a 1 m wall, then revised it, so that a
    unit right beside someone who pops smoke still sees them, while a distant
    observer needs to be within about 30 m to see someone at the edge.
- **D1a: the same rule from both ends (agent, explained to the user).**
  - Depth is counted along the line, so seeing in and seeing out are equal: a
    unit 2 m inside sees out well through the near side and is blind into the
    cloud.
  - Forest already works this way, but its density (0.011 per metre) is too low
    to notice.
  - Smoke adds no separate forest-ground-style concealment beyond D1's 0.1.
- **D2: who carries smoke (user; IFV pick by the agent).** Charges are authored
  on each unit row:
  - **Tanks, by cost:** ≤300 gets 1, 301–399 gets 2, ≥400 gets 3.
  - **High-end IFVs, 1 each:** T-15, Puma, CV90 Mk IV and M2A4 Bradley. The
    user said "you can still have high-end IFVs carry one charge". IFVs share
    a profile with SAM launchers, so the agent picked them vehicle by vehicle.
  - **Infantry, 1 each:** the assault squad (Europe and Eastern; the US has no
    assault squad), the US Marine squad, and US Force Recon. No unit in the
    roster is marked elite, so this is a one-time authoring pick.
- **D3: player order only (user).** Popping smoke is a button. Player units
  never pop it on their own.
  - **Why (the user):** "It's actually sometimes not good to automatically pop
    smoke. You only want to pop smoke when you want to escape. If you're
    attacking and you pop smoke, you lose sight of your target."
- **D4: the AI pops and reverses (user).** The skirmish AI pops smoke on a unit
  that is losing a fight, then orders it to reverse out.
- **D5: exact circles (agent, delegated by the user: "A or C is fine with me,
  you decide").**
  - Clouds are exact circles, never forest's 8 m foliage grid.
  - **Why:** the grid would make the drawn cloud and the area that blocks sight
    disagree by metres. A finer grid would be a second store to maintain.
- **D6: size and timing (user).** The cloud is centred on the unit.

  | Carrier | Radius | Height | Lifetime |
  |---|---|---|---|
  | Tank or IFV | 15 m | 6 m | 40 s |
  | Infantry | 12 m | 5 m | 30 s |

  - It blooms to full size within 1 s.
  - Its density and concealment fade to nothing over its last 10 s.
  - Every number is data on the unit row.
  - The user rejected a forward-thrown wall.
- **D7: resupply (user).** Resupply refills one charge per delivery, the way
  flares are refilled.
- **D8: no drift (agent).**
  - The simulation has no wind, so the cloud never moves. The drawn smoke
    swirls inside its circle.
  - **Why:** preference 3. Drawn drift would make the screen lie, and wind
    would be a new mechanic.
- **D9: firing from inside smoke (agent default; the user may overturn).**
  - A unit in smoke may fire at targets its side has spotted. This is the
    existing shared-spotting rule: a weapon aims at its side's shared track
    of the target (`weapons.rs`, `resolve`).
  - Each shot refreshes that unit's firing contact, so firing from smoke keeps
    drawing return fire.
- **D10: contacts (user, given at the start).**
  - The last-contact marker appearing the moment smoke is popped is a feature.
  - A unit that stays in its smoke keeps getting hit, because that is still the
    last contact.
  - A unit that reverses out is lost to the enemy, because smoke blocks sight
    like a building or a forest.

## Look, sound and UI

- **D11: look (user).** A dense white-grey billow, distinct from the grey-black
  smoke of wrecks.
- **D12: cloud visibility (user).** A side sees the cloud when any of its units
  can see where the cloud stands, even if it can't see the unit inside.
- **D13: charges readout (user).** A "SMOKE n" row in the floating panel, the
  same row component flares use, in the warning colour at 0.
- **D14: HIDDEN (user).** The existing HIDDEN row lights up while the unit is
  in smoke.
- **D15: button (user; key chosen by the agent).**
  - POP SMOKE goes next to REVERSE.
  - With several units selected, every selected unit with a charge pops one.
  - The key is **H**. X, the mock's key, is the camera pitch key
    (`packages/renderer-core/src/cameraController.ts`), and
    `web/tests/commandBindings.test.ts` asserts that X is no command.
- **D16: sound (user; how it is heard and where it comes from, by the agent).**
  - **Tanks:** a discharger thump-thump, then a hiss. **Infantry:** a grenade
    pop and a hiss.
  - A side hears the pop at the cloud when it sees the cloud within the bloom
    time of the pop tick. A pop never creates a contact.
  - **Why the hearing rule:** unseen enemies are heard only through hearing
    cues (`specs/done/battle-look/decisions.md`, the user's 2026-09-26 sound
    entry). A new cue category would
    change the closed publication layout a second time.
  - **Where the sound comes from:**
    - It is generated with ElevenLabs through Bento, as the flare burst was.
    - There is no synthesized fallback; none is required
      (`crates/presentation/src/sound/catalog/check.rs`).
    - Downloaded YouTube clips are for audition only, because the catalog
      keeps All-Rights-Reserved sources unassigned.
- **D17: the AI's smoke (user).** It looks and sounds exactly like a player's.
- **D18: done (user).** Smoke is in the game and playable wherever units fight.

## Engineering (agent, from the trap sweep)

- **D19: the cloud is not a prop.**
  - **Why:** a prop is a box, and sides remember props forever, so an expired
    cloud would linger as a ghost. A prop also has no birth tick, forces
    navigation and air replans, and makes helicopters climb over it.
  - Clouds get their own collection.
  - The Q28 transient-prop machinery is **deleted**: `PropBody.lifetime_s`,
    expiries, the Q28 tests in `crates/sim/tests/bodies.rs`, and the
    `PropBody` doc sentence. It is not kept alongside.
- **D20: smoke has its own depth term.** `foliage_depth` returns 0 without a
  forest, and `launch_clear` refuses fire through any foliage. So smoke depth
  is a separate term that only the sight paths read.
- **D21: popping is an instant state change** outside the replacing-order path,
  following the `UpgradeMove` precedent.
  - It never cancels the current order.
  - A unit without a charge ignores it silently.
  - The AI orders smoke first and the reverse second, because of the order rate
    limit.
- **D22: the AI's "losing" signal.**
  - The doctrine keeps a per-unit hp baseline. Friendly fire counts, and that
    is accepted.
  - It skips units that are reversing toward a goal.
  - It makes no extra random draws.
- **D23: digests.**
  - Clouds fold in the old expiry slot.
  - Smoke state folds only for units that carry smoke.
  - Test units get no smoke.
  - The shift in roster battle digests is a named decision.
- **D24: resupply order and refund.**
  - Smoke is served after replacement soldiers.
  - Price and service time are authored on the capability.
  - Refund value ignores smoke.
- **D25: no backward compatibility (user).** No compatibility shims and no
  migrations; hard cutovers only.
