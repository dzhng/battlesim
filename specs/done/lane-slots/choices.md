# Lane slots: choices ledger

Every call made during the build where the spec was silent, checked against
the shipped code. The user's own decisions (D1–D13) and the walk's agent
answers (A1–A10) live in the [README](README.md#decisions); this file holds
only what the build decided. Grouped by verdict, least confident first.

**Review these first:** the fade clock includes preparation (C1), a parked
unit can block teammates on a shared road (C2), and the open-lane look (C3).

## Needs you (provisional call in place)

### C1 · The home pull's fade runs on match time, preparation included
- **When:** slice 02 (`ff7a7db27`), doc wording in `6961e07b6`.
- **The choice:** The AI's pull toward its own lane starts at 1000 at the
  match's first tick and fades to nothing 600 s later. Preparation (up to
  60 s, while players buy) counts toward those 600 s. So if both sides take
  the whole preparation, the battle goes live with the pull at 900; if
  everyone readies at once, it goes live at nearly 1000. The unbuilt
  alternative starts the clock when the battle goes live, so the pull is
  always full at the first shot.
- **The gap:** D8 said "at the start" without saying which start.
- **The reach:** How long AIs hold their lanes in the opening depends a
  little on how fast players ready. The probe chose 1000/600 s with this
  clock, so the values already cover it: no AI left its lane within the
  first 200 s on any size.
- **Verdict:** needs you. Provisional: keep match time (simpler; the planner
  already knows the tick, not when the phase turned). To reverse, measure
  `elapsed_s` from the tick the skirmish phase became active, in
  `crates/sim/src/skirmish_ai/team/mod.rs`.
- **Confidence:** medium.

### C2 · A unit parked on a road's edge blocks every later arrival there
- **When:** found in slice 06's integration; not changed.
- **The choice:** Teammates now share roads more often (your opening and
  your lane are the same road your AI ally may buy on). If you park a tank
  right where the road meets the map edge, every later purchase on that road,
  your ally's included, waits for the edge to clear. The `allies` scene did
  exactly this and its ally never took the field. The rule is physical and
  predates lanes; the build fixed the scene (buy 120 m in), not the rule.
- **The gap:** Lanes make shared roads common; the spec didn't consider
  arrivals on a crowded road.
- **The reach:** In human multiplayer a careless teammate can block a road.
  The game shows nothing about why.
- **Verdict:** needs you. Provisional: leave the rule. A later change could
  let an arrival take the side's next-quickest road while its own is blocked,
  or flag a blocked entry in the purchase preview (which already says
  "Entry blocked").
- **Confidence:** medium.

### C3 · An open lane keeps its number on a dark tint of its side colour
- **When:** slices 04 and 05 (`5b5ea7879`, `cf2fbee0e`); revises A4.
- **The choice:** In the lobby, a road no slot holds (Blue's third road in a
  2v3) keeps its digit, drawn in its side's colour on an opaque dark tint of
  that colour with a heavier edge. The first plan hid the digit; two
  unprimed critiques read the blank tag as broken, then read a black hollow
  tag as an objective disc. In battle every lane paints its digit, held or
  not.
- **The gap:** The walk's A4 (agent) said "no digit"; evidence overturned it.
- **The reach:** "Open" has its own look; a future minimap reusing the
  picture inherits the `--lane-*` styling hooks.
- **Verdict:** needs you (taste). Provisional: keep. You were shown the 2v3
  shot during the build and raised nothing about it. To reverse, change the `[data-held="none"]` rule
  in `web/src/menu.css`.
- **Confidence:** medium.

## Sound (the architecture you now own)

### C4 · The battle's lane digit is a stroke glyph, 7 m tall, upright from the edge
- **When:** slice 06 (`6f20c1df1`, `5aa426896`).
- **The choice:** Digits 1–3 are polylines with round joins, drawn with the
  chevrons' pen, about 7 m tall, reading upright to a player looking in from
  their own edge. You then asked for the chevrons' exact strokes: the digit
  shares their dark outline (2.6 route widths) and an outer chevron's fill
  (1.25), which dropped the white outline that marked your own lane in
  battle. Pulled far out the digit grows only to 3.6 route widths (you cut it
  60%), strokes thinning with it.
- **The gap:** D9 said where, not how big or how "yours" shows.
- **The reach:** A lane past 3 would need a new glyph (there are at most
  three roads a side).
- **Verdict:** sound. One critique note left open: the "1" reads a little
  like a font glyph. Your own lane no longer differs from a teammate's in
  battle (it still does in the lobby).
- **Confidence:** medium.

### C5 · The pull's values: 1000 fading linearly over 600 s
- **When:** slice 02 (`ff7a7db27`).
- **The choice:** Large maps needed a pull of about 800 for every AI to open
  on its lane. Starting at 1000 keeps it at 800 two minutes in; fading
  linearly to zero at ten minutes hands the battle to need. Corpus: the
  seven one-lane rows are byte-identical, the 3v3 and 5v5 rows moved, every
  row replays identically.
- **The gap:** D8 asked for behaviour and left the values to a probe.
- **The reach:** Two tuning numbers in `fixtures/skirmish-ai.json`. Tests
  pin the behaviour (every AI opens home on generated Medium, Large and XL
  maps; one AI leaves home once the pull fades), never the values.
- **Verdict:** sound; picks, not targets.
- **Confidence:** medium.

### C6 · A human with no lane asks help from the road nearest its fight
- **When:** `6961e07b6`, after the whole-feature review.
- **The choice:** Help requests stage from the asking player's lane. A human
  in slot 4 or 5 holds none, and the first cut then gave it no help lane, so
  an AI teammate never answered it. Now it asks from the side's road nearest
  the objective it is fighting for.
- **The gap:** A8 moved help onto the lane rule without saying what a
  laneless human gets.
- **The reach:** One fallback in `help.rs`, tested.
- **Verdict:** sound.
- **Confidence:** medium.

### C7 · The lobby's strip: tighter tabs and a status word that holds still
- **When:** slice 03 (`89ce8096e`, `1ea8a60e2`).
- **The choice:** Four Region tabs pushed the strip past a 1280-wide window,
  so every option tab's side padding went from 10 to 8 px. The status word
  lost its fixed width, then slid left as its text grew; it now takes the
  strip's free room starting a fixed gap after Region.
- **The gap:** D12 asked for tabs; the room for them was unplanned.
- **Verdict:** sound.
- **Confidence:** high.

### C8 · The lobby marks tags in the page, found by side and lane
- **When:** slice 05 (`cf2fbee0e`), review fix `3bf352791`.
- **The choice:** The picture stays player-free, so the lobby sets
  `data-held` on each tag after the picture is drawn, finding the tag by its
  `data-entry` side and `data-lane` number. The map frame keeps one markup
  object per picture: React 19 re-sets raw HTML whenever that object is new,
  which re-parsed the whole picture on every render and wiped the marks.
- **The gap:** A7 said "overlay occupancy"; the mechanism was open.
- **The reach:** Seat changes re-mark tags without a new preview; the lobby
  no longer re-parses its picture each render.
- **Verdict:** sound.
- **Confidence:** high.

### C9 · The entry marker draws under the units' marks
- **When:** slice 06, reordered after you saw a selection ring under the digit.
- **The choice:** The renderer draws all still paint, then all marching paint.
  The chevrons pulse, so they are marching paint, and a selection ring is
  still paint, so the chevrons covered the ring. Now the whole marker (digit
  included) draws first in the marching layer, and the units' still marks
  (supply, orders, aircraft) follow it there with their pulse data zeroed,
  which the shared fragment shader draws exactly as still paint.
- **The reach:** Units' marks now order after other marching paint in the
  same list, not in a separate pass. Objectives, the zone and the border stay
  in the still layer, under everything. A renderer change (a second marching
  pass) was the alternative, with a capture-format cost.
- **Verdict:** sound.
- **Confidence:** high.

### C10 · The chevron stack moves about 6.5 m inward
- **When:** slice 06.
- **The choice:** The digit takes the spot nearest the edge, and the three
  chevrons start 3 m past its top.
- **Verdict:** sound.
- **Confidence:** high.

### C11 · Blue 1 now opens on the top road, not the centre
- **When:** slice 01 (`f45687f73`).
- **The choice:** A consequence of D3: on a map with several roads, the human
  in Blue 1 holds lane 1, the top road, and the camera opens there. The
  `allies` scene's purchase spot moved with it (C2).
- **Verdict:** sound.
- **Confidence:** high.

### C12 · Equal-x entries number in list order
- **When:** slice 01.
- **The choice:** If two of a side's roads sat at the same x, the
  earlier-listed one gets the lower lane number. Generated maps are not
  expected to; the rule just makes numbering total.
- **Verdict:** sound.
- **Confidence:** high.

### C13 · Tests and scenes
- **When:** slices 01–06.
- **The choice:**
  - the faked camera-lane test went; a real-Wasm test checks that preparation
    opens on the human's lane;
  - the party tests turn the home pull off (lanes aren't their subject);
  - the lobby scene shoots a 2v3 and checks your lane and the open one;
  - the `allies` scene shoots your entry close and far, and buys 120 m inward.
- **Verdict:** sound.
- **Confidence:** high.

Trivial discretion, compressed: picture tag weights (body 3.6, tip 2.2, half
2.1 hundredths of the map's longer side); the overlay parses the scenario
once (launcher follow-ups item 4); the viewer's own lane comes from the
session's viewer id.
