# Lane slots

**Status:** in progress. Slices 01, 03, 04, 05 and 06 shipped; 02 in flight.
**Updated:** 2026-11-14.

A player's slot number on its side is its **lane**: slot k owns its side's k-th
entry road. The number is painted on the lobby picture's entry tags and on the
ground behind each entry's chevrons in battle, and every AI leans toward its own
lane. The purpose is team coordination. Human-vs-human multiplayer is planned,
and teammates will call lanes by number ("push 2").

Spawning does not change. Entries stay shared by the side, and a bought unit
still arrives at the side's quickest entry
([D8](../done/team-skirmish/choices.md)). A lane is a label plus an AI
preference, not an ownership rule for arrivals.

The plan also carries three lobby cleanups the user asked for in the same pass:
no seed in the status line, no "too small" reason line, and Region as tabs.

## Next Agent Prompt

You are resuming the lane-slots feature. Slices 01, 03, 04 and 05 are
committed, and slice 06 too (taken over in this checkout when its delegate ran
out of disk). Slice 02 (AI home lane) was handed to a delegate in its own
checkout, branched from the slice 01 commit; integrate it by cherry-pick, rerun
its focused checks on this branch, then do slice 07. The `allies` scene's
failure is fixed (it bought on the road's edge, which blocked the ally that
now shares Blue 1's lane). If 02 reported that no
single home-lane value meets both behaviours, stop and bring the user the
probe table.

Warnings:

- Slice 02 is the only slice that may change battle outcomes. It ships as one
  named re-baseline commit. Every other commit must leave digests and replays
  unchanged; an unexplained digest move means lane data leaked into
  simulation identity, so find it before going on.
- Do not add a `lane` field to `EntrySite`. It is `deny_unknown_fields` and
  part of scenario identity (`crates/contract/src/encounter.rs:82`).
- Inside the AI planner, "lane" already means an entry road
  (`PlayerPlan.lane: EntryIndex`). That is the same road a lane number labels,
  so it keeps its name; the new number is its own type, `LaneNumber`, and is
  never compared with an `EntryIndex`.
- Visual slices go through `game-ui` (and `renderer` for slice 06),
  `compare-screenshots`, `screenshot-critique` and `preview-shots`. The human
  checkpoint is non-blocking (see [Verification](#verification)).
- Pull LFS before the closing full run (slice 07).

TODO:

- [x] 01 · Lane rule in `sim::entries`, served through Wasm; opening camera on it → [slice 01](slices/01-lane-rule.md)
- [ ] 02 · AI home-lane pull: wire at zero, probe, fade, named re-baseline; help rule onto lanes → [slice 02](slices/02-ai-home-lane.md)
- [x] 03 · Lobby cleanups: status, size reason, Region tabs → [slice 03](slices/03-lobby-cleanups.md)
- [x] 04 · Lobby picture: numbered pointed tags, player-free → [slice 04](slices/04-picture-tags.md)
- [x] 05 · Lobby occupancy: open lanes hollow, your lane banded white → [slice 05](slices/05-lobby-occupancy.md)
- [x] 06 · Battle: lane digit painted behind the chevrons → [slice 06](slices/06-battle-digits.md)
- [ ] 07 · Docs and the one full run → [slice 07](slices/07-closeout.md)

Update this section (status, date, ticks, next pickup) before you end each pass.

## Slice graph

```
01 lane rule ──┬── 02 AI home lane (only digest change)
               ├── 04 picture tags ── 05 lobby occupancy
               └── 06 battle digits
03 lobby cleanups (independent)
all ── 07 closeout
```

## Contracts

**The lane rule** (owner: `sim::entries`).

- A side's lanes are numbered 1..n from the map's **high-x end**, for both
  sides, with ties broken by list order. The lobby picture draws world (x, y)
  at (height − y, width − x) (`crates/mapgen/src/inspect/lobby.rs:82`), so
  high x is the picture's top and lanes read top to bottom, like the banners.
- n is the side's entry count: 1, 2 or 3 (`fixtures/skirmish-sizes.json`).
- A player's **slot** is its rank among its own side's players in the battle's
  player list, from 1. The lobby names players `side-k` in that order
  (`skirmishPlayers`, `apps/battle-lab/src/skirmishSetup.ts:31`).
- Slot k holds lane k when k ≤ n. Slots 4 and 5 hold no lane. A lane whose slot
  is empty (Blue's third road in a 2v3) is unheld.
- The centre-first entry **list** is unchanged
  ([I18](../done/team-skirmish/choices.md)). Jets by the centre road (D15), the
  centre tie-break and quickest-entry arrivals (D8) keep reading it. Lanes are
  a second ordering of the same entries, never a re-sort.
- The opening camera is the human's home entry. A human with no lane, or no
  human, opens on the centre entry of the human's side (Blue when no human).
  The rule is side-general even though the lobby seats the human on Blue.

**Across the Rust/TypeScript boundary.** Rust serves lanes through one Wasm
function (working name `skirmish_lanes(sites, players)`), next to
`skirmish_picture` in `crates/game-wasm/src/skirmish.rs`. It answers, per entry
in list order, `{ side, lane, holder: player id | null }`, plus the opening
entry. TypeScript has one reader for it and never computes a lane itself. The
`Player` contract gains no field, and `EntrySite` gains no field.

**The lobby picture's markup.** `skirmish_picture(map, sites)` keeps its
signature and stays player-free (it is cached per map input and will be reused
by a future minimap). Each entry becomes a pointed tag carrying its digit, with
`data-entry="<side>"` and `data-lane="N"` on its group. The page styles a tag
by who holds it through custom properties on that group (`--lane-body`,
`--lane-edge`, `--lane-edge-scale`, `--lane-digit`; `--side` is the tag's
accent), and reveals the hidden own-lane mark, `[data-own]`.

**AI tuning.** `fixtures/skirmish-ai.json` gains one weight, the home-lane
bonus. Existing weights do not change.

## Single owners (end state)

The finished code reads as if lanes had always been designed in, not bolted on:

- `sim::entries` is the only owner of lane numbering, slot-to-lane and the
  opening entry. The picture, the Wasm export, the planner, help requests and
  the opening camera all call it.
- Delete the TypeScript opening-lane maths (`web/src/battle/prepare/prepare.ts:188-195`)
  and the help-request modulo rule (`crates/sim/src/skirmish_ai/team/help.rs:60`).
  No `indexOf(human)` or `% entries` lane maths may remain anywhere.
- One Rust picture owner, one browser deployment-marker owner, no native
  indicator code (native replays the browser's recorded overlays,
  `native/client/README.md:43-50`).
- Occupancy (which lanes are held, which is yours) lives only in the lobby
  overlay and the battle overlay, both fed by the one Wasm answer.
- "Lane" names an entry road everywhere, the planner included; its number is
  the separate `LaneNumber` type from `sim::entries`.

## Decisions

From the explore-unknowns walk on 2026-11-14. The full map, including the
landmine cards, is [visualizations/unknowns-map.html](visualizations/unknowns-map.html).
"You" means the user; "agent" means answered on the user's behalf under the
delegation they opted into (scope: small questions that follow from their
preferences; preferences: one general rule over special cases, what the player
sees beats legacy consistency, bold readable marks, AI gets soft nudges and need
wins, no implementation detail on screen).

| # | Decision | Why | Who |
|---|---|---|---|
| D1 | Every AI on both sides gets a soft pull toward its slot's lane. A lane in much greater need wins, and the AI then takes its whole force there. | Lanes are for coordination; need still governs. | you |
| D2 | ~~AI leans away from human-held lanes.~~ Withdrawn. | D1's relative pull already does it; a special case would repeat it. | you |
| D3 | Lanes are numbered top to bottom in the picture (high x first), matching the banners. I17's camera rule moves onto it. | The banners and the tags read in one order. Rejected: centre = 1 (today's camera order), because it reads 2/1/3 down the picture. | you |
| D4 | Slots 4–5 hold no lane: no number, no pull, they plan freely. | Three roads, three holders. Rejected: wrapping 4→1, 5→2, and stacking extras on the centre. | you |
| D5 | Lobby look: option D, "number is the arrow", a pointed side-coloured tag with a dark digit. Your own lane wears a white band round its tag. (A "YOU" label was dropped on review: the band already says it.) | Bold and distinct from the objectives' white circles. Mock: [lane-tag-looks.html](visualizations/lane-tag-looks.html). | you |
| D6 | Numbers only, never player names. | Human multiplayer is planned; names won't fit. | you |
| D7 | Lane numbers show in the lobby, in battle, and in the future minimap (which reuses the lobby picture). | | you |
| D8 | The home-lane pull is specified by behaviour, not a value: (a) at match start every AI takes its home lane on admitted maps of every size, including when a human holds the centre road; (b) later, a lane with clearly more enemy pressure or open objectives pulls an AI off home. **Revised after slice 02's probe: the pull fades over the match** (full at its first tick, gone after `home_lane_fade_s` of match time, preparation included). Shipped: 1000 fading over 600 s, so it is still 800 (what Large needed) two minutes in. | The probe showed no constant value meets both: the planner's only lane-dependent cost is straight-line distance (`team/mod.rs:537`), so a pull that beats the opening's distance gaps also beats them later. A fade is literally "start on your lane, then go where you're needed". | you |
| D9 | In battle, paint the lane digit as ground paint **behind** the existing entry chevrons: between the map edge and the first chevron, with the stack slid inward. | The chevrons are already ground paint; the digit joins them. Rejected: a HUD label (my earlier A2), digits under or past the chevrons. | you |
| D10 | Remove the seed from the status line ("Ready"). The seed stays in the Deploy link. | Implementation detail. | you |
| D11 | Remove the "S · M too small for N a side" line; closed sizes stay disabled. | Greyed-out already says it. | you |
| D12 | Region becomes tabs, using the existing `Choice` control. | | you |
| D13 | No backward compatibility, no migrations. Hard cutover; old replays re-baseline. | | you |
| A1 | In battle, only your own side's lanes are marked. | Already true of the chevrons; coordination is within a team. | agent |
| A3 | Battle digits are always on, like the chevrons. | "Push 2" needs the number mid-fight. | agent |
| A4 | ~~An unheld lane shows no digit.~~ Revised in slice 04: every lane keeps its number; an unheld lane's lobby tag goes hollow (an opaque dark tint of the side colour, a heavier side-coloured edge, a side-coloured digit). In battle every lane paints its digit. | Two unprimed critiques: a blank tag read as broken, and the open lane is the one teammates most need to name. A black hollow tag then read as an objective disc and vanished over forest; the tint fixed both. | agent |
| A5 | Red's lanes are numbered too, and Red's AI gets the same pull. | One general rule; humans may sit on Red later. | agent |
| A6 | A human in slot 4–5 opens on the centre entry. | Follows D4. | agent |
| A7 | The picture stays player-free; the lobby overlays occupancy by `data-lane`. | Seat changes don't restart the preview (`isMapInputChange`, `apps/battle-lab/src/lobby/model.ts:204`), and 2v3 → 3v3 keeps the entry count. | agent |
| A8 | Help requests' human lane (`help.rs:60`, the old I17 modulo rule) moves onto the lane rule, inside slice 02's re-baseline. | One slot-to-lane owner; otherwise help treats your lane as a different road from the one painted with your number. | agent |
| A9 | Lanes are served by a Wasm function, not a new `EntrySite` field. | `EntrySite` is `deny_unknown_fields` and scenario identity; a field would make a UI change move digests. | agent |
| A10 | The planner's `PlayerPlan.lane: EntryIndex` keeps its name; the number is a separate `LaneNumber` type. | The planner's lane is the same road the number labels, so there is one concept; a distinct type stops a number being compared with an index. Rejected: renaming the planner's field (churn with no second meaning to remove). | agent |

## Facts the territory settled

- The planner's distance is straight-line (`libm::hypot`, `crates/sim/src/skirmish_ai/team/mod.rs:537`), weight 1.0. Influence, open-objective and incumbency bonuses depend on the objective, not the lane. With ≤3 AI players every permutation gives each AI a distinct lane (`mod.rs:329`); larger teams use a subset search that picks each player's best lane.
- Small/Medium/Large/XL have 1/2/3/3 entries. A two-road map is the centre road plus one flank (`crates/mapgen/src/skirmish.rs:316-334`), so lane 1 is the centre on some Medium maps and the flank on others. Flank arms are mirrored between the sides and chosen on the same side of both centre exits (`is_left`/`is_right`, `skirmish.rs:301`), but the centre exits are not mirrored: lane k is the same road on both sides, though the two centres' x differ (`generated_lanes_number_the_same_road_on_both_sides`).
- No pinned battle digest runs an active AI: `fixtures/parity/skirmish-{objectives,purchases}.json` seat a human plus passive AI. Slice 02 will likely move no pinned value; its evidence is the AI behaviour tests and the tournament corpus. It is still the named outcome change.
- The battle overlay reads entries from the scenario JSON (`battleOverlayScenario`, `apps/battle-lab/src/battleOverlay.ts:73-93`); the observation carries no entries. The battle session already calls Wasm on the main thread (`apps/battle-lab/src/useBattleSession.ts:123`), and so does the lobby (`Lobby.tsx`).
- The lobby preview keeps the admitted `sites` (`web/src/battle/prepare/protocol.ts:97-105`).
- Native replays browser-recorded overlays; it needs no source change, only regenerated captures where parity fixtures hold overlays.
- `packages/battle-renderer/src/orderOverlay.ts` is over the file-size budget and may not grow.

## Firewalls

- No change to where units arrive, jets' entry, refunds' exit, or admission.
- No change to existing AI weights, help request triggers, or party logic beyond A8.
- No player names on maps. No minimap work (it doesn't exist yet).
- No new map-generation behaviour. Entry geometry is taken as generated.

## Verification

- Climb the [check ladder](../../development.md#check-ladder) only as far as each slice needs. Run everything once, in slice 07.
- Tests check mechanics, not configuration or strings: lane order by position, holders by slot, AI placement by behaviour. Never pin the home-lane value, a class name or a label text.
- **Every visual shot** (slices 03, 04, 05, 06): use `game-ui` before the change; judge before against after with `compare-screenshots` (against the [before shots](assets/) and, for slice 04, the [mock](visualizations/lane-tag-looks.html)); run an unprimed `screenshot-critique` as the **last** check before accepting the shot.
- **Human checkpoint, non-blocking:** open the shots with `preview-shots`, give the user about five minutes, and if they stay silent, decide on the evidence, record the decision and its reason in the slice's notes below, close the shots, and go on. Never stall on sign-off.

## Notes from implementation

(Record decisions made during the build here, one line each, with the slice.)

- 03: four Region tabs overflowed 1280 px, so every option tab's side padding went from 10 to 8 px; the status word lost its fixed width and then takes the strip's free room from a fixed point after Region (an unprimed critique caught it sliding left as its text grew).
- 04: the tag sits its halo and band in from the frame; its digit is placed by the tag's turn but never turned, so Red's tags read upright. Weights are the picture's (body 3.6, tip 2.2, half 2.1 hundredths of the longer side).
- 04/05: the own-lane "YOU" label was dropped at the user's call; the white band alone marks your lane.
- 05: the map frame keeps one markup object per picture (React 19 resets inner HTML on a new object, which re-parsed the picture every render and wiped the marks).
- 06: the digit is a stroke glyph about 7 m tall, upright as seen from the edge looking in; the chevrons sit 3 m past it. Pulled far out it grows only to 3.6 route widths, strokes scaled with it: the user cut it by 60% there, since it need not read at that zoom. It is still paint (`painted`); the chevrons stay marching paint, because the marching pass reads normals as pulse data. Your lane's digit is outlined in the HUD's white, wider.
- 02: corpus before/after (the pull at 0 against the shipped 1000/600 s): the seven one-lane rows are byte-identical; `team_3v3` and `team_5v5` digests moved, their peak lanes unchanged (3/3 and 4/5); every row replays identically. No pinned battle digest moved.
- 02 review: a human in a slot past the side's roads asks help from the side's road nearest its fight (the first cut gave it no help lane at all).
- 06 critique, not acted on: your outline is subtle at far zoom; the "1" reads a little like a font glyph. Reversible styling calls.
