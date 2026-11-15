# Decisions ledger

What was decided for [control groups, range-select and the fire blink](README.md),
by whom, and why. The planning walk's full record is
[`assets/walk-map.html`](assets/walk-map.html).

## Settled in the planning walk, 2026-11-16

The player answered most of these. The agent answered the rest on the player's
behalf, under the preferences below, and showed each one. **The player's
confirmed preferences:** follow established RTS and file-list conventions; take
the agent's recommendation when the code grounds it; keep it simple. Delegation
covered behaviour, never looks.

| # | Decision | Closed by |
| --- | --- | --- |
| D1 | Ctrl+N makes group N exactly the selection, taking those units out of any other group; Shift+N adds the selection to group N. | player |
| D2 | On cards, Shift-click selects the run from the anchor (the last card clicked without Shift) and replaces the selection; Cmd-click (Mac) or Ctrl-click adds or removes one card. | player |
| D3 | N twice within the double-click window centres the camera on the member nearest the group's centre; one press only selects. | player |
| D4 | Ctrl+0–9 on every platform, its browser default prevented while the battle takes input. Ten groups. | player |
| D5 | A grouped squad that boards stays in its group; N selects only members on the field. | player |
| D6–D8 | A group is a 1 px white bracket on the cards' outer edge, so grouping moves no card; its × shows only on hover. | player |
| D9–D11 | The tutorial teaches groups and range-select; selecting a group plays the units' answers; the command bar's hints don't name the group keys. | player |
| D12 | The group number is a white tab with a deep-blue digit, flush inside the group's first card. | player |
| D13, D14 | With two or more rows the deck fills top to bottom, then across; the bracket hugs the cards (an L where a column is part-filled). | player |
| D15 | No gaps: the deck is one sequence and a group never forces a new column; its spare slot takes the next card. | agent, shown |
| S1–S4 | Groups are client-only; a unit is in one group at most; the dead leave their group; keys typed into a field stay text. | shown, unchallenged |
| L2–L10 | Brackets drawn per card; groups owned by `useUnitControl`; restart clears them; ids kept and filtered, never pruned; Shift-click in the world unchanged; team battles group only the player's units; an armed command stays armed; number row only; tutorial text without a new picture. | agent, shown |
| B1–B6 | A rising glow from the card's foot spilling softly below: yellow when an aimed round passed, red when the unit was hurt; two eased pulses; a running blink finishes, but red cuts in; a colour waits before it repeats; colours mixed from the panel glass toward the HUD's warning and damage colours. | player |
| B7 | The simulation publishes, per own unit per tick and outside the digest, whether an enemy round aimed at it passed near; rounds carry their target. | player |
| B8, B9 | A strike without damage, by a round aimed at the unit, blinks yellow; no new pass sound, and red fires on its hit's existing impact tick. | agent, shown |

## Decided during the build

Each was made where the plan was silent, and checked against the shipped code
at close.

### Needs the player

**The glow sits behind the card's face, not over it as the approved mock drew it.**
In the mock the glow lay on top of the card, tinting its icons. Built that
way, an unprimed reviewer saw a hurt tank's cyan health bar turn pink-grey at
the very moment the player needs to read it, and the group's white bracket
turn yellow. Behind the face, the light shows through the card's translucent
fill and the bar, symbols and bracket stay clean. It reads fainter only
because the face covers it; its values are the mock's.
*Verdict:* needs-user (taste). Provisional call: behind. To reverse it, move
the blink element after the card in `ArmyDeck`'s slot and raise its z-index.
*Confidence:* medium.

**An anti-air missile that narrowly misses never blinks yellow.**
"Near" uses the same test as the "under fire" bark: the round's near-miss
suppression is above zero at that distance, so inside its suppression radius
with a near-miss suppression above zero. Anti-air missiles have neither, so a
helicopter that a missile just misses shows no yellow, though a hit still
blinks red. Making them count would mean giving them near-miss suppression,
which is a balance change, not a HUD one. *Verdict:* needs-user (tuning).
Provisional call: leave them as they are. *Confidence:* medium.

### Sound

**A range's anchor lapses when the selection changes any other way.**
Click a tank's card, drag a box over some infantry, then Shift-click a truck's
card: you get the truck alone, not a run from the tank. The deck remembers
the selection it last asked for, and a different selection means someone else
changed it. That avoids wiring box and world selection into the deck.
*Confidence:* medium-high.

**A number key with no group does nothing.**
Pressing 7 when no group 7 exists keeps your selection. Clearing it would
punish a mis-tap, and RTS games ignore the key. A group whose members are all
riding still selects nothing, as D5 says. *Confidence:* high.

**Ctrl+N with nothing selected keeps the group.**
Hitting Ctrl+2 with nothing selected doesn't wipe group 2. A group goes only
by its ×, by its units moving to other groups, by their deaths, or on restart. *Confidence:* high.

**A group's cards follow army order.**
Within a bracket, cards sit in the order the army lists them, not the order
they joined, so a group's look doesn't depend on how it was assembled.
*Confidence:* high.

**Each card is placed from its slot.**
One function, `deckSlots`, says where every card sits and which sides of it
the bracket runs along. Earlier the CSS grid also flowed the cards, which made
two owners of one fact. *Confidence:* high.

**Bracket pieces reach across the gaps.**
Each card draws its part of the bracket on its outer edge, extended across
the 4 px gap on its inner sides. Neighbouring pieces then meet, an L's inner
corner included, and no card moves. On a selected card the cyan border shows
inside the white line as a second edge, which D8 accepts. *Confidence:* high.

**The × is a new generated icon, `close`.**
It's a lighter cross than `rejected`, which means a refused order, so a
dismissal never looks like a refusal. *Confidence:* high.

**New HUD keys: `presentation.hud.group_ink` and `presentation.hud.fire_blink`.**
The fire blink's keys are `on_s`, `off_s`, `wait_s`, `depth` and `glow`,
starting from the player's mock settings. A snake_case colour key becomes a
kebab-case CSS variable (`--hud-group-ink`). The blink's `depth` and `glow`
become `--hud-blink-depth` and `--hud-blink-glow`, and its timings stay in
the script. *Confidence:* high.

**The blink reads every publication, not every render.**
React draws at most one observation a frame. A pass that is true for one tick
could therefore fall between frames, so `BattleView` hands every decoded
publication to `FireBlinks` first. Waits and blink lengths count ticks.
*Confidence:* high.

**"Hurt" is any fall in health.**
A vehicle's health plus every living soldier's. A rifleman killed counts as
hurt. A hit inside red's wait still lets a simultaneous pass show yellow.
*Confidence:* high.

**A finished blink leaves the feed.**
Making a group reorders the deck, and the browser restarts any animation on
an element it moves. So a blink is dropped once it has run, and with its unit
when the unit is gone. A card shot at a minute ago never flashes again.
*Confidence:* high.

**The blink honours `on_s` and `off_s` separately.**
It's a Web Animation built from both values, started once as the element
mounts. CSS keyframes could only honour their sum. At the shipped values it
matches the mock's curve exactly. *Confidence:* high.

**The flag is `OwnUnit.shot_at`, published as `shotAt`.**
Every round records the unit it was aimed at in `Round.aimed_at`, which
replaced `fused_target`, one field for the fuse and the flag. A flare that
decoys a missile clears it, so a decoyed missile no longer counts as aimed at
the unit it left. Fuse behaviour and digest bytes are unchanged. Area fire at
a contact or a ground point carries no target, so it never blinks yellow.
*Confidence:* high.

**Re-recording the captures took in two older drifts.**
The market-town capture was already stale at the plan's base, and an earlier
eye-height rule change had never been re-recorded. This change moved no battle
digest. The market-town capture's seven digests did change in the
re-recording, because the capture was already stale; they compute the same
before and after this change. *Confidence:* high.

**Files moved to stay under the size budget, with no behaviour change.**
The units' voices left `useBattleSession` for `useUnitVoices`. `Round` moved
to `battle/fire.rs`, the body-to-unit lookup to `units.rs`, and a mount's
publication row to its own writer and reader. *Confidence:* high.

**The deck workbench gained a "Control groups" case and a "Fire blink" button.**
The case has pictures of its own. Older deck pictures changed only by the
lab's nav row. *Confidence:* high.

**The group keys need a live client.**
A replay or scripted battle has no client, so Ctrl+digit stays the browser's
there and nothing is stored. Before, an enabled replay page still took the
key. *Confidence:* high.

## Open

Whether Windows Chrome lets the page stop Ctrl+1–9 from switching tabs (D4)
needs a Windows machine. Until someone checks, it's a recorded gap.
