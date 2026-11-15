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
