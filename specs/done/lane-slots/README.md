# Lane slots

**Status:** shipped 2026-11-14. Rationale record. The user's decisions are
below; the calls the build made where the plan was silent are in
[choices.md](choices.md). The build plan (slices, handoffs) is in git history.

A skirmish player's slot number on its side is its **lane**: slot k holds its
side's k-th road in. The number is painted on the lobby picture's entry tags and
on the ground behind the chevrons of each of your side's roads in battle, the
lobby marks which lanes are open and which is yours, and every AI that holds a
lane opens on it. Teammates call
lanes by number ("push 2"); human-vs-human multiplayer is the reason.

A lane is a label and an AI preference, never an arrival rule. Entries stay
shared by the side, and a bought unit still arrives at the side's quickest road
([D8](../team-skirmish/choices.md)).

The same pass cleaned the lobby's strip: the status says only what the preview
is doing (no seed), sizes too small for the match are greyed out with no reason
line, and Region is a row of tabs.

## The contract

- **One owner.** `sim::entries` (`crates/sim/src/entries.rs`) numbers lanes
  (`LaneNumber`, `lane_number`, `lane_entry`), maps slots to lanes (`homes`) and
  picks where a seated player's view opens (`opening`). Every consumer calls it:
  the lobby picture (`crates/mapgen/src/inspect/lobby.rs`), the planner and help
  requests (`crates/sim/src/skirmish_ai/team/`), and the page through one Wasm
  call.
- **Numbering.** A side's lanes count 1..n from the map's high-x end, for both
  sides; equal x falls back to list order. The lobby picture draws world (x, y)
  at (height − y, width − x), so high x is its top and lanes read top to bottom
  like the player banners. n is the side's road count (1, 2 or 3,
  `fixtures/skirmish-sizes.json`).
- **Slots.** A player's slot is its rank among its own side's players in the
  battle's player list. Slot k holds lane k when k ≤ n; slots past the side's
  roads hold none. The `Player` contract carries no lane.
- **The entry list is unchanged.** Entries stay listed centre first
  ([I18](../team-skirmish/choices.md)); jets by the centre road, the centre
  tie-break and quickest-road arrivals read that list. Lanes are a second
  ordering of the same entries, never a re-sort.
- **Across the boundary.** `skirmish_lanes(sites, players)`
  (`crates/game-wasm/src/skirmish.rs`) answers, per entry in list order,
  `{ side, lane, holder }` and the `opening` entry. TypeScript reads it only
  through `lanesOf` (`web/src/battle/prepare/lanes.ts`) and never numbers a lane.
  `EntrySite` carries no lane: it is `deny_unknown_fields` and part of scenario
  identity, so a lane field would make a UI change move digests.
- **The opening camera** is the human's home entry; a human with no lane opens
  on its side's centre road, and with no human seated the view opens on Blue's. Preparation takes `report.start` from
  the Wasm answer (`web/src/battle/prepare/prepare.ts`).
- **The lobby picture stays player-free.** It is drawn once per map input (the
  lobby starts a new preview only when one changes) and holds no player data, so
  other views of the map, such as a minimap, can reuse it. Each entry is a pointed side-coloured tag with its
  digit, its group carrying `data-entry`, `data-lane`, `--lane-*` styling hooks
  and a hidden own-lane band. The lobby (`apps/battle-lab/src/lobby/Lobby.tsx`)
  marks each tag `data-held` (you, player, none) by its side and lane, and
  `web/src/menu.css` draws an open lane on a dark tint of its side colour, with a
  heavier side-coloured edge and digit, and yours with a white band. A
  seat change re-marks the tags without a new preview.
- **In battle** each road in of the viewer's side paints its lane digit behind
  its chevrons, nearest the edge (`packages/battle-renderer/src/deploymentMarker.ts`,
  placed by `battleOverlayScenario` in `apps/battle-lab/src/battleOverlay.ts`).
  Digits are stroke glyphs (a lane is 1, 2 or 3) drawn with the chevrons' outline
  colour and stroke weights. The whole marker lies under every mark of the units:
  it draws first in the marching paint layer, and the units' still marks follow
  it there, unpulsed (`unpulsed`, `packages/battle-renderer/src/mesh.ts`), because
  the renderer draws all still paint before all marching paint. Native replays
  the browser's recorded overlays and draws nothing itself.
- **The AI's home pull** (`TeamTuning::home_lane_bonus`, `home_lane_fade_s` in
  `fixtures/skirmish-ai.json`) scores an AI's own lane better, starting at the
  match's first tick and fading to nothing, preparation included. Every AI on
  both sides feels it; there is no rule about human-held lanes. A human with no
  lane asks for help from its side's road nearest its fight.

## Why it is shaped this way

- **A label, not ownership.** Spawning was already shared and quickest-road
  (D8). Numbering the roads gives a team words to coordinate with and leaves the
  arrival rules, and their digests, alone.
- **Top to bottom, not centre first.** The camera used to number from the centre
  road. The banners read 1, 2, 3 down the lobby, so the tags do too. The old
  `(slot − 1) mod count` rule lived twice, in TypeScript for the camera and in
  the help requests; both now read `sim::entries`.
- **The pull fades.** A probe across five admitted maps a size and every human
  seat found no constant pull that both starts every AI on its lane and lets need
  move it later: the planner's lane-dependent terms are straight-line distance and
  a small bonus (80) for keeping the last plan's road (`team/mod.rs`), so a pull large enough to beat the opening's distance gaps
  (about 800 on Large) beats them for the rest of the battle too. The user chose a
  fade: literally "start on your lane, then go where you're needed". Shipped 1000
  over 600 s, still 800 two minutes in. In the tournament corpus the one-lane rows
  are byte-identical and only the 3v3 and 5v5 rows moved; no pinned battle digest
  moved.
- **An open lane keeps its number.** A blank tag read as broken, and the open
  lane is the one teammates most need to name; a black hollow tag read as an
  objective disc, so the open tag is a dark tint of its side colour.

## Invariants

- Nothing outside `sim::entries` numbers a lane or maps a slot to one.
- The lobby picture takes no players. Occupancy is the page's, from the one Wasm
  answer.
- Lanes never change where a unit arrives, which road jets use, or the entry
  list's order.
- A change that should not move outcomes leaves digests unchanged. Two outcomes
  moved, by name: the home pull, and a human teammate's help road, which follows
  its lane under the new numbering (or the nearest road to its fight when it
  holds none) and so matters only in matches that seat a human.

## Tests that pin it

- `crates/sim/tests/entries.rs`: numbering from high x, the same-x tie, slot k
  holds lane k, the opening.
- `crates/mapgen/tests/skirmish.rs`
  (`generated_lanes_number_the_same_road_on_both_sides`) and
  `crates/mapgen/tests/picture.rs` (`each_sides_lanes_count_down_the_picture`).
- `crates/mapgen/tests/home_lanes.rs`: with the shipped tuning every AI opens on
  its own lane on generated Medium, Large and XL maps, for every human seat.
- `crates/sim/src/skirmish_ai/team/tests/planning.rs` (the pull holds at the
  start and fades) and `tests/help.rs` (a laneless human's help road).
- `crates/game-wasm/tests/skirmish.rs`, `web/tests/prepareBattle.test.ts`,
  `web/tests/lobbyScreen.test.tsx`, `web/tests/entryChevrons.test.ts`; the
  `lobby` and `allies` scenes.

## Dead ends

- **A constant home pull.** No single value met both behaviours (above).
- **A lane field on `EntrySite`.** Rejected for scenario identity (above).
- **A HUD label for the battle digit.** The chevrons were already ground paint;
  the digit joined them.
- **Hiding an open lane's digit; a black hollow tag.** Both failed unprimed
  critiques (above).
- **A "YOU" label on your tag.** Dropped: the white band already says it.
- **Wrapping slots 4–5 onto lanes 1–2, or stacking them on the centre.** The user
  chose no lane for them.

## Visual provenance

- `assets/lobby-before-3v3.png`: the user's screenshot of the 3v3 lobby that
  started this. The other `lobby-before-*` files are the user's crops of it: the
  unnumbered entry arrow, the seed line, the size reason and the Region dropdown,
  each with the change asked for.
- `visualizations/lane-tag-looks.html` (backdrop `assets/lobby-map-crop.png`, cut
  from that screenshot): four mocked tag looks; the user picked D, "number is the
  arrow".
- `assets/slice-04-before.png`: the picture with plain arrows, the baseline the
  tag change was judged against. `assets/slice-05-2v3.png`: the `lobby` scene's
  `2v3` shot, the shipped lobby with you in Blue 2 and Blue's third lane open.
- `assets/slice-06-before.png`: an entry's chevrons before the digit.
  `slice-06-own-lane.png` and `slice-06-far.png`: the `allies` scene's
  `entry-lane` and `entry-lanes-far` shots, your lane's digit close up (from
  before the user asked for the chevrons' strokes, which replaced its white
  outline) and both team digits far out. An earlier far shot, with the digit grown large, drove the
  user's 60% cut.
- `visualizations/unknowns-map.html`: the explore-unknowns map the plan came
  from, banner-marked where the build superseded it.

## Decisions

"You" is the user; "agent" is answered on the user's behalf under the delegation
they opted into during the walk.

| # | Decision | Who |
|---|---|---|
| D1 | Every AI on both sides leans toward its slot's lane; a lane in much greater need wins. | you |
| D2 | ~~Lean away from human-held lanes~~: withdrawn, D1's relative pull already does it. | you |
| D3 | Lanes are numbered top to bottom in the picture (high x first); the opening camera follows. | you |
| D4 | Slots 4–5 hold no lane and feel no pull. | you |
| D5 | Look D, "number is the arrow"; your lane wears a white band (no "YOU" label). | you |
| D6 | Numbers only, never player names. | you |
| D7 | Lanes show in the lobby, in battle, and in the future minimap. | you |
| D8 | The pull is specified by behaviour; it fades over the match. | you |
| D9 | In battle the digit is ground paint behind the chevrons, in the chevrons' strokes and colours, under a unit's selection ring; far out it stays small. | you |
| D10–D12 | No seed in the status; no size reason line; Region as tabs. | you |
| D13 | No backward compatibility or migrations. | you |
| A1 | In battle only your own side's lanes are marked. | agent |
| A2 | ~~A HUD label for the battle digit~~: superseded by D9. | agent |
| A3 | Battle digits are always on. | agent |
| A4 | An open lane keeps its number (revised from "no digit" on evidence). | agent |
| A5 | Red's lanes are numbered too, and Red's AI gets the same pull. | agent |
| A6 | A human in slot 4–5 opens on the centre road. | agent |
| A7 | The picture stays player-free; the lobby overlays occupancy. | agent |
| A8 | Help requests' human lane moved onto the lane rule. | agent |
| A9 | Lanes cross to TypeScript through a Wasm call, not an `EntrySite` field. | agent |
| A10 | The planner's `lane` (an entry road) keeps its name; the number is `LaneNumber`. | agent |
