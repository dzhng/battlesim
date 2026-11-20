# Operation Ferryman: the story

The tutorial's content design, approved by the user on 2026-11-19. Each chapter
below is data: director beats in the scenario document, coach steps, solution
beats and forces. Its spoken lines are the `narration` section of
`fixtures/voices.json` (line ids match the `line` column). This file says what
was meant to happen; the scenario's files say what does, and where a chapter
was retuned to play (the [choices](choices.md) record each change), the files
win.

Rules for every chapter:

- **Roster units only**: `us` (blue, human) against `eastern` (red, a `DirectedAi`
  seat). The exact card for each role is picked in probe P4 and recorded in
  [decisions.md](decisions.md). Never a `test_*` or `capture_*` unit.
- **Steps advance on the player's action or on a game fact, never on winning a
  fight.** Every scripted fight is lopsided through authored `UnitCondition`
  (worn hulls, casualties, spent ammunition) and numbers, so a retune doesn't
  flip it.
- **Spoken lines carry the story and the intent.** They never name a key, a
  button, a price, a stat, a range or a unit's model. The coach card under the
  portrait shows keys from the command bindings, and amounts from their owners.
- **No soft locks.** A chapter that needs a unit the player could lose has a
  recovery beat that re-grants credits covering it (data, not code).
- The story talks about places by their **zone names**. Zones are data in the
  scenario document, and the chapter text, the director and the coach all read
  them from there.

## The map: Ferryman valley

A small generated valley, saved once as the `scenario` map `ferryman-valley`
(probe P1 finds the seed). These are the zones the story needs. Their shapes
come from the map, and a missing one changes the story (a ford instead of a
bridge is fine), never the rules.

| Zone | What it is | Used in |
|---|---|---|
| `entry` | Blue's road in at the south edge (the skirmish entry) | 1, 5, 6 |
| `ridge` | The wooded ridge above the village, with a treeline facing north | 2, 4 |
| `farm` | The farm, with a garrisonable farmhouse and a stone wall on the village side | 3, 5 |
| `crossroads` | The village crossroads: **objective 1** | 3, 7 |
| `river_woods` | The woods across the river from the village, where the unseen gunfire comes from | 2 |
| `north_road` | The road from the bridge down to the village, where the tank comes in | 4 |
| `ambush_woods` | Woods beside `north_road`, where the AT team waits | 4 |
| `far_treeline` | The treeline across the fields, where the mortar fires | 5 |
| `bridge` | The river bridge: **objective 2** | 7 |
| `hill` | The enemy hill with the SAM radar: **objective 3** | 6, 7 |
| `red_entry` | Red's road in at the north edge | 6, 7 |

## Chapter 1: Briefing (preparation phase)

Nothing moves. The scenario's match rules make preparation long enough that
the countdown never cuts the briefing short, and the red seat is ready from the
start, so **Start battle** is the player's own choice.

| Step | Gate | Line | Notes |
|---|---|---|---|
| Opening card | continue | `c1.open` | The camera starts high over the valley. |
| Pan | ui: camera pan | `c1.camera_pan` | |
| Turn and tilt | ui: camera rotate + tilt | `c1.camera_turn` | |
| Zoom | ui: camera zoom (out, then in) | `c1.camera_zoom` | |
| The flags | continue | `c1.objectives` | Camera tour over objectives 1–3 (a reel). |
| Our road | continue | `c1.entry` | Tour ends on `entry`. |
| Credits | continue | `c1.credits` | Highlight the wallet. |
| Buy scouts | ui: shop opened | `c1.buy_recon` | The shop offers only this chapter's cards. Highlight the shop button, then the card. |
| Place | beat: recon purchased | `c1.place` | A placement is the purchase: the unit drives there once the battle starts. |
| Buy the squad in its carrier | beat: carrier purchase made | `c1.buy_carrier` | One purchase, two units. |
| Orders, not units | continue | `c1.orders_not_units` | Highlight the greyed cards. |
| Start | beat: battle started | `c1.start` | Highlight Start battle. |

Opening credits cover exactly the chapter's purchases at current prices.

## Chapter 2: Eyes forward

| Step | Gate | Line | Notes |
|---|---|---|---|
| Arrival | ui: scouts selected (click or card) | `c2.arrive` | |
| To the ridge | beat: recon in `ridge` | `c2.move` | Mark `ridge` on the ground. |
| Facing | ui: move with facing | `c2.facing` | |
| Outpost seen | beat: blue identifies the outpost | `c2.spotted` | Red's `outpost` squad stands in the village, worn. |
| Gunfire | beat: blue holds a contact from `river_woods` | `c2.gunfire` | The director orders red's `river_guns` to fire on a patch of ground near the village. They are heard, never seen. |
| Jump to it | ui: contact focus | `c2.jump` | |
| Stay hidden | beat: recon on return fire only | `c2.hidden` | The panel already shows Hidden. |

## Chapter 3: The crossroads

| Step | Gate | Line | Notes |
|---|---|---|---|
| Drive | beat: carrier at `farm` | `c3.drive` | |
| Unload | beat: squad unloaded | `c3.unload` | |
| Garrison | beat: squad inside the farmhouse | `c3.garrison` | |
| Suppress | beat: outpost suppressed | `c3.suppress` | The carrier's gun on the outpost. |
| Pinned | beat: outpost pinned | `c3.pinned` | |
| Attack-move | beat: squad attack-moving out of `farm` | `c3.attack_move` | |
| Show orders | ui: show-orders held | `c3.show_orders` | |
| Capture | beat: objective 1 capturing | `c3.capture` | |
| Captured | beat: objective 1 blue | `c3.captured` | |
| Contested | beat: objective 1 contested, then not contested | `c3.contested` | The director sends red's worn `counter` squad into the ring. |

## Chapter 4: Steel on the ridge

| Step | Gate | Line | Notes |
|---|---|---|---|
| Tank! | beat: blue identifies `tank` | `c4.tank` | The director sends red's worn `tank` down `north_road`. It fires at will. |
| Groups | ui: two control groups set | `c4.groups` | |
| Smoke and reverse | beat: carrier popped smoke, then reversed | `c4.smoke` | |
| The AT team | continue | `c4.at_team` | Chapter 4 opens by offering and granting blue an AT team, and asks the player to buy it and put it in `ambush_woods` on return fire only. |
| Ambush | beat: AT team ordered to attack `tank` | `c4.ambush` | The step advances on the order, not on the kill. |
| Tank down | beat: `tank` destroyed | `c4.killed` | Recovery: if the AT team dies first, a beat re-grants a replacement. |

## Chapter 5: Keep it running

| Step | Gate | Line | Notes |
|---|---|---|---|
| Spend | continue | `c5.credits` | The wallet holds income by now. The grant tops it up only to cover this chapter's purchases. |
| Supply truck | beat: supply purchased | `c5.supply` | |
| Deploy | beat: supply deployed near `farm` | `c5.deploy` | The deploy key comes from the bindings (it is T, not D). |
| Resupply | beat: a blue unit serviced (ammunition, repair or a soldier replaced) | `c5.resupply` | The chapter 3–4 units are worn by then. |
| Mortar | beat: mortar ordered to attack ground in `far_treeline` | `c5.mortar` | Red's `treeline` team hides there. |
| Done | beat: `treeline` destroyed or withdrawn | `c5.mortar_done` | |

## Chapter 6: Eyes in the sky

| Step | Gate | Line | Notes |
|---|---|---|---|
| Rotors | beat: blue identifies red's helicopter | `c6.helicopter` | Red buys it with granted credits at `red_entry` (a normal purchase, no spawn). It is worn and short of ammunition. |
| Air defence | beat: blue air defence purchased | `c6.air_defence` | |
| Splash | beat: red helicopter destroyed | `c6.helicopter_down` | Recovery re-grants air defence if it is lost. |
| Our helicopter | beat: blue helicopter airborne | `c6.own_helicopter` | |
| Jet | beat: jet purchase pending, then arrived | `c6.jet` | Highlight the arrival countdown on its card. |
| Passes | beat: jet ordered to attack | `c6.jet_attack` | |
| The radar | beat: SEAD jet purchased | `c6.radar` | Red's `sam` emits radar on `hill`. Blue can't see it. |
| Radar contact | beat: blue holds a radar contact | `c6.radar_found` | |
| Radar down | beat: `sam` destroyed | `c6.radar_down` | |

## Chapter 7: The push

| Step | Gate | Line | Notes |
|---|---|---|---|
| Take them | continue | `c7.push` | The shop offers the faction's whole list. Red's defenders move onto `bridge` and `hill` as blue approaches. |
| Contested | beat: objective 2 or 3 contested | `c7.contested` | |
| Score | continue | `c7.score` | Highlight the score bar. |
| Victory | beat: match won by blue | `c7.victory` | Total control is the existing rule. The scenario's `majority_s` patch keeps red's score from ending the battle earlier. |
| The guide | continue | `c7.guide` | Points to Esc → Guide. |

General lines: `any.retry` plays when a recovery beat fires, `any.unit_lost`
when a coach-marked unit dies, and `any.take_your_time` when a preparation step
has been idle for a while (the timing is coach data).

## Mechanics coverage

Every Guide section is taught somewhere:

| Guide section | Chapter |
|---|---|
| The aim, Starting a battle, Preparation, Credits, Buying units | 1 |
| Selecting units | 1, 2, 4 (control groups) |
| Moving (move, facing, fast move, queue, reverse, garrison, transport) | 2, 3, 4 |
| Fighting (auto fire, attack, attack-move, attack ground, fire policy, deploy, show orders) | 2–5 |
| The camera (pan, rotate, tilt, zoom, contact focus) | 1, 2 |
| Seeing the enemy (fog, contacts, Hidden, panels) | 2 |
| Staying alive (cover, suppression, supply, facing, smoke) | 3, 4, 5 |
| Aircraft, Air defence (helicopters, jets, fuel, SAMs, radar, SEAD) | 6 |
| How to win (capture, contest, score, total control) | 3, 7 |
| Pausing | 7 (closing line) |

Fast move (double right-click) and queued waypoints (Shift) are not gated
steps. The coach mentions them as a tip in chapter 2 (card text only).
