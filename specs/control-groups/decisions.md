# Decisions ledger

What was decided for [control groups and the fire blink](README.md), by whom,
and why. Add to it whenever a slice settles something the plan left silent or
confirms a fact the plan only read from the code.

## From the planning walk, 2026-11-16

The player answered most of these. The agent answered the rest on the player's
behalf, under the preferences below, and showed each answer. The walk's full
record is [`assets/walk-map.html`](assets/walk-map.html).

**The player's confirmed preferences:** follow established RTS and file-list
conventions; take the agent's recommendation when the code grounds it; keep it
simple (one place per unit and group, nothing added on the player's behalf).
Delegation covered behaviour, never looks.

### Selection and groups

| # | Decision and why | Closed by |
| --- | --- | --- |
| D1 | Ctrl+N makes group N exactly the current selection; those units leave any other group. Shift+N adds the selection to group N. RTS convention, and one place per unit. | player |
| D2 | On cards, Shift-click selects from the anchor (the last card clicked without Shift) to the clicked card, in bar order, and replaces the selection. Cmd-click (Mac) or Ctrl-click (elsewhere) adds or removes one card. A Mac Ctrl-click is the system's right-click. | player |
| D3 | N twice within the double-click window centres the camera on the group's member closest to the group's centre; one press only selects. | player |
| D4 | Ctrl+0–9 on every platform, its browser default prevented while the battle takes input. Ten groups. If Windows Chrome cannot be stopped from switching tabs, that is a recorded gap; no other modifier replaces it. | player |
| D5 | A grouped squad that boards stays in its group, its dimmed card in place. N selects only members on the field; the carrier does not join. | player |
| S1 | Client only: no command, observation, digest or replay change for groups. | shown, unchallenged |
| S2 | A unit is in at most one group. | shown, unchallenged |
| S3 | Dead units leave their group; a group with no units left disappears. | shown, unchallenged |
| S4 | Keys typed into a field stay text. | shown, unchallenged |
| L2 | The bracket is drawn per card, one edge wherever a neighbour is not in the same group, so a wrapped or L-shaped group needs no layout case. | agent, shown |
| L3 | Group state lives in `useUnitControl` beside the selection; the deck gets the groups and a delete callback. Routes without the hook show no groups. One owner per concept. | agent, shown |
| L4 | Restart clears every group, as it clears the selection. | agent, shown |
| L5 | Groups keep unit ids and show and select only those still present; ids are never reused in a battle, so no pruning code. | agent, shown |
| L6 | Shift-click on a unit in the world still adds or removes it; only cards range-select. | agent, shown |
| L7 | In a team battle Ctrl+N takes only units the player commands (`controlled.ts`). | agent, shown |
| L8 | An armed command stays armed when N selects a group, as with any selection change. | agent, shown |
| L9 | The number row only, no numpad. | agent, shown |
| L10 | The tutorial gains text in "Selecting units" and its card line is corrected; no new picture. | agent, shown |

### How groups look

| # | Decision and why | Closed by |
| --- | --- | --- |
| D6 | A group is a 1 px white bracket (the HUD's text white). | player |
| D7 | Its × is a white tab, flush in the top right of the group's top row, shown only while the pointer is over the group. | player |
| D8 | Same margins everywhere: the bracket sits on the cards' outer edge, so grouping moves no card and the deck's 68 px pitch holds. | player |
| D9 | The tutorial teaches groups and range-select. | player |
| D10 | Selecting a group plays the units' spoken answers, as any selection does. | player |
| D11 | The command bar's hints do not name the group keys. | player |
| D12 | The group number is a white tab with a deep-blue digit, flush inside the top-left corner of the group's first card. | player |
| D13 | With two or more rows the whole deck fills top to bottom, then left to right. | player |
| D14 | The bracket hugs the cards: a part-filled column makes an L. | player |
| D15 | No gaps: the deck is one sequence (groups 1…9, 0, then ungrouped, then waiting cards). A group never forces a new column, so its spare slot takes the next card. The player's reference shows the same. | agent, shown |

### The fire blink

| # | Decision and why | Closed by |
| --- | --- | --- |
| B1 | A rising glow climbs from the card's foot, with a 2 px foot line and a soft glow spilling below the card. | player |
| B2 | Yellow: an enemy round aimed at the unit passed near it. Red: the unit's health dropped. | player |
| B3 | Two pulses, faded in and out with easing. | player |
| B4 | A blink in progress finishes and ignores new rounds; a hit cuts in as red. | player |
| B5 | A card does not show the same colour again within a wait; each colour keeps its own. The client applies it. | player |
| B6 | The colours are the HUD's warning yellow and damage red mixed toward the glass navy; no new colour. | player |
| B7 | The simulation publishes, per own unit per tick and outside the digest, whether an enemy round aimed at it passed near; every round carries what it was fired at. | player |
| B8 | A round that strikes without damage (a glance, a ricochet) blinks yellow. | agent, shown |
| B9 | No new pass sound. The red blink fires on the tick of its hit's existing impact sound. | agent, shown |

The blink's timing, wait, colour depth and glow size start from the player's
settings in [`assets/blink-reference.html`](assets/blink-reference.html) and are
tuning keys in `presentation.hud`, guesses the evidence may retune.

## During implementation

| # | Decision and why | Closed by |
| --- | --- | --- |
| I1 | A range's anchor lapses when the selection changed any other way: the deck compares the selection it last asked for with the current one, so a box or a world click needs no plumbing into the deck. | agent |
| I2 | Ctrl+N with nothing selected leaves group N as it was (RTS convention; an empty assignment never deletes a group — the × does). | agent |
| I3 | A group's cards follow army order, not the order units joined it. | agent |
| I4 | The × is a new generated HUD icon, `close`: a lighter cross than `rejected`, which means a refusal. | agent |
| I5 | The digit's ink is `presentation.hud.group_ink`; a snake_case HUD key becomes a kebab CSS variable (`--hud-group-ink`). | agent |
| I6 | A bracket piece reaches across the gap on its inner sides, so neighbouring pieces meet, an L's inner corner included; the white line overlays the card's outer border pixel. On a selected card the cyan border shows inside it as a second edge, accepted under D8. | agent |
| I7 | The units' voices moved out of `useBattleSession` into `useUnitVoices`: the session file is over its size budget and could not take the camera hook otherwise. | agent |
| I8 | The battle-deck workbench gained a "Control groups" case with its own pictures; every older deck picture was re-blessed for the lab's new nav button alone (the deck's pixels did not move). | agent |
| I9 | The unprimed critique's group-separation and hover-× findings are accepted: groups sit one card gap apart by D8, and the hover × covers a rider's seat mark as the walk showed. | agent |
| I10 | The flag is `OwnUnit.shot_at`, the layout's own-row column `shotAt`, `OwnUnitView.shotAt`: the card's question, was this unit shot at this tick? | agent (slice 5a) |
| I11 | `Round.fused_target` became `Round.aimed_at`, one field for both readers, the fuse and the flag. A flare that spoofs a round still clears it, so a decoyed missile no longer counts as aimed at the unit it left; the fuse behaves as before and the digest writes the same bytes. | agent (slice 5a) |
| I12 | "Near" is the bark's test (`shouts::NearRounds`): within the round's suppression radius, scored as a near miss; a strike by a round aimed at the unit counts whatever the radius (B8). A row with no suppression radius (today the anti-air missiles) never blinks yellow on a pass, though its hits blink red. A radius would also make them suppress, so that is a tuning call, not this plan's. | agent (slice 5a) |
| I13 | Only a round fired at a unit carries `aimed_at`: area fire at a contact or a ground point never blinks yellow, even where it lands. B2 says "aimed at the unit"; a blind barrage is not. | agent (slice 5a) |
| I14 | To stay under the file-size budget, slice 5a moved `Round` into `battle/fire.rs`, the body-to-unit lookup into `units.rs`, and a mount's publication row into its own writer and reader files, with no behaviour change. | agent (slice 5a) |
| I15 | Re-recording the publication captures also took in two older drifts: the market-town capture was already stale at the plan's base (its digests were recomputed at the window ticks, identical before and after this change), and an earlier rule change of vehicle eye height had never been re-recorded. Every battle digest in the re-recorded files is unchanged. | agent (slice 5a) |
| I16 | The blink reads every publication, not every render: `BattleView` hands each decoded publication to `FireBlinks` before the session's own consumer, so a pass published for one tick is never dropped. Its waits and blink length are counted in ticks. | agent |
| I17 | "Hurt" is any fall in the unit's health, a vehicle's plus every living soldier's, so a man killed counts. A hit inside red's wait still lets a simultaneous pass show yellow. | agent |
| I18 | The glow lies behind the card's face, not over it as in the mock: the unprimed critique found it muddied the strength meter just when a unit is hurt, and tinted the group's white bracket. The card's translucent face lets it show through. | agent |
| I19 | The tuning keys are `presentation.hud.fire_blink` (`on_s`, `off_s`, `wait_s`, `depth`, `glow`), started at the player's mock settings. | agent |
| I20 | The deck workbench gained a "Fire blink" button (first card shot at, third hurt); its pictures were re-blessed for the lab's nav row alone. | agent |
| I21 | A blink leaves the feed once it has run, and with its unit when the unit is gone, so a card that moves (a new group reorders the deck) or remounts never replays an old one. | agent (whole-spec review) |
| I22 | The blink is a Web Animation built from `on_s` and `off_s` (each pulse rises over half of `on_s`, eases off over the rest, fades over half of `off_s`), started by a stable ref as its element mounts. CSS keyframes could only honour their sum. | agent (whole-spec review) |
| I23 | A number key with no group leaves the selection alone (RTS convention); a group whose members all ride still selects nothing, as D5 says. | agent (whole-spec review) |
| I24 | Each card's grid cell is placed from its slot (`deckLayout.ts`), so one module owns where a card sits and how its bracket is drawn; CSS no longer flows them. | agent (whole-spec review) |
