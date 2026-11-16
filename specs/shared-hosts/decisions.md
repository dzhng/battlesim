# Shared hosts: decisions ledger

The final ledger, re-audited against the shipped code when the plan closed
(2026-11-16). Each entry is a decision the code now carries that the original
ask did not spell out, or that the user made in the walk. Calls a later pass
superseded or reverted are gone, and duplicates are merged. Entries are
grouped by verdict, least confident first within each group.

**Who decided:**

- **user**: the user's own call, in the 2026-11-16 unknowns walk
  ([map](unknowns-map.html)) or after it.
- **agent on the user's behalf**: decided while planning, before any code
  (the one walk question the user delegated, or the synthesis of four
  independent plan drafts), and not confirmed one by one by the user.
- **territory**: forced by the existing code or the repository's standing
  principles; the plan only had to notice it.
- **implementer**: decided during the build, where the plan was silent or
  delegated the choice.

**Confidence** is how sure the audit is that the user would have made the
same call.

**Terms used throughout.** A *host* is anything a squad can be inside: a
building or a carrier (a vehicle that carries infantry). A squad's *hold* is
its state on a host, in one of three phases: *stepping in* (its entry timer
runs), *inside*, and *stepping out* (its exit timer runs). A *seat* is room
for one living soldier. A building's seats are its *windows*, the exposed
facade bays its soldiers fight from, listed in the building's *window plan*.
`CapacityFull` is the refusal the player reads as "TOO MANY TO FIT".

**Review these first:** N1 (the Paris-corner menu reel lost its deaths), N2
(aiming at seen soldiers is limited to garrisons), U1 (the building order
carries a field nothing reads).

## Needs user

### N1. The Paris-corner menu reel was re-recorded with no one falling

- **Who, when:** implementer, at closeout, under the user's standing
  permission to bless a recording after inspecting it.
- **Scenario:** The main menu plays a scripted battle at the Paris corner.
  Blue's recon team comes under fire from a red rifle squad fighting from a
  house. Before this plan no gun could fire at that squad (see S3), so blue's
  recon shot at red's other squad, standing in the open, and killed one of
  its soldiers; two more of that squad died to rounds from red's own
  garrison passing through them. The reel's last shot framed the three
  bodies. Now blue's recon answers the squad shooting at it, as a soldier
  would: most of its rounds strike the house's walls, a few hit the
  garrison, and the squad in the open loses nobody. Every unit walks the
  same routes and the tank's scripted moves land the same; only whom blue
  shoots changed. The reel was re-recorded as it now plays, so the menu
  shows a firefight with no casualties, and its last shot frames eight
  standing soldiers where the bodies lay.
- **Gap:** The plan expected the menu reel not to move; the later
  engagement fix (S3) moved it.
- **Reach:** The reel is the first footage a player sees, and the
  re-recorded event log is now what its tests compare against.
- **Verdict:** needs-user. The mechanics read right; whether a reel with no
  casualties is good footage is taste. **Provisional call:** keep the
  re-recorded reel. **To reverse:** re-direct the reel's encounter
  (`fixtures/maps/paris-corner/encounters/corner.json`, for example the open
  squad's `soldier_hp` or the recon team's start) so deaths land on screen,
  look at it, and re-record (`BLESS_MENU_REEL=1`). Never narrow the
  engagement rule to get the old footage back.
- **Confidence:** low.

### N2. Guns aim at seen soldiers only when the target is a garrison

- **Who, when:** implementer, after slice 07 (the "garrisons are engageable"
  fix).
- **Scenario:** A gun aims at a squad's middle: the average position of the
  soldiers its side saw. S3 handles a garrison whose middle lies inside its
  own walls by trying each seen soldier in turn. The same failure happens
  away from buildings: seven soldiers crouched round a parked car have their
  middle behind the car, every solution at the middle meets the car, and the
  gunner holds fire although six of them stand in plain view. The general
  rule ("whenever the middle can't be fired at or seen, try the seen
  soldiers") fixes that too. It was tried and moved more than garrisons:
  squads in the open in the air, lean and collapse test cases, the
  market-town menu reel, a street defender squad bunched round its house
  before stepping in, and a Paris-corner squad round a low prop. The shipped
  rule is gated on the target being a squad fighting from a building
  (`garrisoned` in `crates/sim/src/weapons/assessment.rs`).
- **Gap:** The plan never expected engagement to change; the garrison lab
  exposed it.
- **Reach:** Every weapon's target assessment. While the gate stays, a squad
  bunched behind any body that isn't its building can't be shot at its
  middle and isn't shot at its visible soldiers either.
- **Verdict:** needs-user: product direction and the cost of re-recording.
  **Provisional call:** keep the garrison scope. **To reverse:** drop the
  `garrisoned` gate so the fallback serves any squad, then re-record and
  inspect what moves. The audit leans toward the general rule: the gate is a
  case the physical rule doesn't need, the behaviour it leaves looks wrong
  on screen, and part of the reason for the scope was that the general
  version moved recordings, which the user's standing rule says never to
  protect.
- **Confidence:** low.

## Unsound

### U1. The building order still carries a `gesture` nothing reads

- **Who, when:** agent on the user's behalf (the plan's contract table kept
  the field); slice 01 found that nothing reads it.
- **Scenario:** Orders sent by a right-click carry a *gesture*: a token a
  quick second right-click (`UpgradeMove`) uses to switch that click's walk
  to the fastest route. `Board` passes its gesture into the squad's walk, so
  that walk can be upgraded. Before this plan the building order's entering
  squad also walked under a move carrying the gesture. Now
  `OccupyBuilding { units, building, gesture }` becomes
  `UnitOrder::Garrison { building, approach }` for each squad, which carries
  no gesture, and the client upgrades only a right-click on open ground. The
  field travels on the wire, the encounter defender fills it with 0, and
  nothing reads it.
- **Gap:** The plan deleted `facing` (only companion gathering read it) but
  kept `gesture` without asking whether anything still read it.
- **Reach:** A wire field that looks meaningful invites future code to rely
  on it; it is the leftover the repository's "when something replaces an old
  mechanism, delete the old one" rule forbids.
- **Verdict:** unsound: a contract field with no reader. **Corrected
  decision:** the building order carries only what the simulation reads, so
  `gesture` leaves `OccupyBuilding` (Rust contract, TypeScript protocol,
  defender). Whether a double right-click on a house should hurry the squad
  in is a separate decision; if wanted, it carries the gesture into the
  entry walk as `Board` does.
- **Confidence:** low.

## Sound

Ranked least confident first. Every one is architecture the user now owns.

### S1. A squad gets an entry if its side's grid reaches the approach; nothing proves the route

- **Who, when:** agent on the user's behalf (one plan draft, over the other
  three); slice 01.
- **Scenario:** The player selects a squad and right-clicks a house behind a
  row of other buildings. The order works out the squad's *approach*, the
  point outside the house where it will stand to step in
  (`garrison::approach`). The squad gets that entry if it already stands
  within entry reach of the building, or if its side's navigation grid,
  built from what the side knows, connects its start to the approach
  (`NavGrid::reaches`). Before, the order replayed the battle ahead to
  *prove* the squad would end up inside, which the relief manoeuvre (S36)
  fails: the house is still full when the proof runs. Now nothing is proved;
  like `Board`, the squad sets off and finds out on the way. The grid calls
  open ground reachable even from a boxed-in start, so a walled-in squad
  also gets an entry and meets the wall en route. The rejected alternative
  (three drafts) kept the bounded proof with "reached the building" as its
  success, about 25 KB of machinery built for companion gathering, and
  risked changing how the encounter defenders enter.
- **Gap:** The plan had to say how an order naming several squads decides
  which of them can get there.
- **Reach:** Anything that later wants guaranteed reachability at the
  command needs its own check. The building order and `Board` share the
  stance.
- **Verdict:** sound: one rule with `Board`, keeps the relief manoeuvre
  possible, and the building-behind-a-row case answers correctly.
- **Confidence:** medium.

### S2. Squads sent into one house together take approaches apart

- **Who, when:** implementer (slice 01a, added after an unprimed critique of
  slice 01's preview; the sampling was delegated).
- **Scenario:** Three squads on one side of a house are sent in by one
  order. With only the nearest approach, all three would stand on the same
  point: their destination rings stacked half under the house and their
  order lines chained through each other. Now squads are taken in unit order.
  The first takes the nearest approach exactly as a lone squad would. Each
  later squad takes the nearest point it can reach whose ground (half its
  soldiers' spread, `arrangement::spread`) stays
  `infantry_movement.spacing_m` clear of every earlier entry's ground, so
  their soldiers don't mingle; with no such point it takes the nearest
  anyway. Candidates are the old nearest-point ones plus points sampled at a
  fixed step along each exposed facade the side knows, dropped if inside any
  part. Only squads actually given an entry reserve ground. The preview
  draws each squad's destination ring at its approach, and the accepted
  order draws one line per squad. The rejected spacing was the formation's
  (the drawn ring's size), which around a lab-sized house left no two points
  far enough apart; so the drawn rings still overlap each other and the
  house, but no longer coincide.
- **Gap:** The plan gave each squad the old single approach; the shot showed
  it stacking.
- **Reach:** Any later entry feature (a waiting area, a group entry
  formation) starts from this spacing rule.
- **Verdict:** sound: spacing by the soldiers' real ground is the physical
  quantity, and a single squad's path is untouched.
- **Confidence:** medium.

### S3. A gun fires at a garrison's soldiers at the windows its side sees

- **Who, when:** implementer, after slice 07.
- **Scenario:** Two squads share a house and stand at windows on every
  facade. A tank sees them and its side records where each soldier was. Its
  gun aims at the squad's middle, the average of those positions, which
  lies inside the house: every firing solution met the house's own walls,
  and a tank at fire at will never fired at a full house. Now, when a
  garrison's middle can't be reached or seen (`BlockedTrajectory`,
  `NoOwnSight`), the assessment tries each soldier the side saw at the last
  sighting, in the order its rounds walk them, and fires from the first one
  it can hit. Every weapon reads it, guided ones included. Fog is kept (only
  soldiers in the side's track count), and the building's shell still stops
  every round that meets it.
- **Gap:** Shared houses were the first time a garrison's middle sat inside
  its walls; nothing planned for it.
- **Reach:** Garrisons are now targets in every scene; this is what moved
  the Paris-corner reel (N1). Its scope is N2.
- **Verdict:** sound: a soldier in a window is visible and shootable, and
  this is what a viewer expects.
- **Confidence:** medium.

### S4. Window swaps run over every squad in a house, and a leaving squad keeps its windows

- **Who, when:** implementer (slice 04), within the user's decision that
  swaps span the building (S33).
- **Scenario:** A house holds squad A (riflemen) and squad B (an AT
  gunner). A tank appears east. The gunner wants the east window, which A's
  rifleman holds. Each tick the building's soldiers request windows facing
  what their weapons aim at: squads in unit order, and within a squad its
  weapons in turn, each weapon's shooters in member order. A soldier facing
  already stays; otherwise he takes a free facing window, or trades with the
  soldier at the nearest facing window who needs it less. *Needs it less* is
  judged by that soldier's own squad's aims: the lone operator of a weapon
  (the AT gunner, a grenadier) outranks one of a squad's many riflemen,
  whichever squad either is in, never the reverse. A squad stepping out is
  never displaced and its windows are not free until it has left. The
  rejected order, strictly by (unit, member), would have reordered a lone
  squad's own requests and changed single-squad window fights.
- **Gap:** The plan said "stable unit and member order" without seeing it
  would reorder a lone squad.
- **Reach:** Every garrison fight. No test pins the leaver rule.
- **Verdict:** sound: one allocation per building, single squads unchanged.
- **Confidence:** medium.

### S5. A squad stepping into a building can be reinforced while seats remain

- **Who, when:** implementer (slice 04).
- **Scenario:** A squad of seven is walking into a house with ten seats
  free when a supply truck delivers a replacement soldier. He is added to
  the squad while it is still stepping in: he stands on the ground beside it
  and is seated with the squad when its entry timer ends. If the house had
  no free seat, he would not be delivered (S18).
- **Gap:** The plan named reinforcement inside and at carriers, not during
  entry.
- **Reach:** Supply and entry timing interact here; the end-of-timer check
  (S35) counts him.
- **Verdict:** sound: the seat count already includes squads stepping in,
  so he can never make his own squad fail.
- **Confidence:** medium.

### S6. A squad ordered back into the building it is leaving finishes leaving, then is admitted again

- **Who, when:** territory (existing hold behaviour, confirmed in slice 03).
- **Scenario:** Squad A is stepping out of a house. The player orders it
  back in. An entry order is not "stay where you are", so A finishes
  stepping out, walks to the approach, and goes through admission like
  anyone else; if another squad took the seats meanwhile, A is refused.
  Only an order that keeps A in place (a Stop) cancels its exit and keeps
  its seat.
- **Gap:** The plan asked whether a re-entry order cancels an exit.
- **Reach:** The relief manoeuvre's timing, and any future "turn back"
  control.
- **Verdict:** sound: one admission path; no seat is held by intent.
- **Confidence:** medium.

### S7. No new selection interaction for a shared house

- **Who, when:** agent on the user's behalf (one plan draft, kept as a scope
  firewall).
- **Scenario:** Two of the player's squads share a house and draw one ring
  (S24). Clicking it selects one squad, the one whose centre is nearest the
  click, as overlapping rings always did; clicking a soldier or a deck card
  selects that squad. The rejected alternative (three drafts) made a click
  on a held house select every own squad inside.
- **Gap:** One ring per house raised what clicking it selects.
- **Reach:** A player can't grab a whole shared garrison in one click.
- **Verdict:** sound: no new interaction without the user asking.
  Revisit only if the user wants it; it would be its own selection change.
- **Confidence:** medium.

### S8. A right-click on a building with no squad selected moves to the ground point

- **Who, when:** implementer (slice 01).
- **Scenario:** The player selects a tank (or a helicopter) and right-clicks
  a house. The building order goes only to infantry squads
  (`commandReach`), so with no squad selected the click is an ordinary move
  to that point. A helicopter used to hover by the entrance as a
  "companion" of an entering squad; companion gathering is gone.
- **Gap:** Deleting companion gathering left vehicles' building clicks
  undefined.
- **Reach:** Mixed selections: the squads get the entry, vehicles move.
- **Verdict:** sound.
- **Confidence:** medium.

### S9. The IN BUILDING row reads `IN BUILDING · SEATS used/total`

- **Who, when:** user (seats used of total on the row, like the carrier's
  Seats row); implementer for the wording (slice 05).
- **Scenario:** A player selects one of two squads sharing a house. Its
  panel's IN BUILDING row shows the house's seats taken of its total, the
  word SEATS as on a carrier's row. *Used* counts the living soldiers of
  every squad on the player's side whose hold names that house, in any
  phase, allies included, as admission counts. The first take,
  `IN BUILDING used/total`, was read beside the strength pips as men missing
  from the squad; naming the count SEATS fixed that. Only the inside row
  changes: stepping in and out keep their timer rows, and the compact card
  shows only the house icon.
- **Gap:** The user named the content, not the words.
- **Reach:** One word for one concept on both hosts' panels.
- **Verdict:** sound.
- **Confidence:** medium.

### S10. The garrison scene's walls-and-roofs check counts added light only

- **Who, when:** implementer (slice 07).
- **Scenario:** A browser scene checks that occupant overlays never paint
  through walls: it renders overlays on black and compares pixels. Once a
  rifle squad held the near facade's ground floor, an occupant's body hid
  his own pad, which takes light away, and the symmetric pixel difference
  read that as paint. Paint through a wall adds light, so the check now
  counts only added light. It was never tested against a body that really
  paints through, because no such state can be reached without changing the
  renderer.
- **Gap:** The check assumed one occupant per facade.
- **Reach:** Whoever touches overlay depth testing relies on this check.
- **Verdict:** sound: it measures what the rule is about. It is not
  falsified against a real failure.
- **Confidence:** medium.

### S11. A building's window plan closes once a tick, after every hold has advanced

- **Who, when:** implementer (slices 02 and 03).
- **Scenario:** The last squad in a house dies to a shell. Its plan isn't
  dropped at the death; plans whose building no living squad holds are
  closed once per tick at the end of `hold::advance`
  (`Windows::close_unheld`). Nothing reads the plan meanwhile, because every
  reader goes through a squad inside or leaving. The first squad seated in a
  house no living squad holds opens a fresh plan, replacing any leftover, so
  it starts with fresh window history.
- **Gap:** The plan said "dropped when no living squad holds it", not when.
- **Reach:** Any new place that ends a hold need not remember to close the
  plan.
- **Verdict:** sound: one closing point instead of one per way a hold ends.
- **Confidence:** medium.

### S12. Units that project to one screen spot each show their card

- **Who, when:** implementer, after slice 07.
- **Scenario:** Two squads share a house and the camera pulls back far.
  Their middles, metres apart, project inside each other's anchor square.
  The card layout refused every card whose leader started inside another
  unit's anchor, so both cards hid. Now units on one spot (`oneSpot` in
  `web/src/battle/present/readoutLayout.ts`) are spared the two rules their
  shared start breaks (a leader keeping off the other's anchor, and two
  leaders' rises meeting); every other rule (no overlap, no leader across a
  card, no foot crossed) still holds. It is the general card layout, not a
  garrison case. Up close, a hovered card still opens in place over its
  neighbour, the standing rule that hovering moves nothing; the far view
  still overprints.
- **Gap:** Sharing created the first case of two units on one spot.
- **Reach:** Any stacked units (a convoy, a crowd) now show every card.
- **Verdict:** sound.
- **Confidence:** medium.

### S13. What the plan left alone

- **Who, when:** agent on the user's behalf (the plan's scope firewall; the
  posting choice from the walk's map).
- **Scenario:** Sharing changed admission, not the hosts themselves. A
  building's seat count (its window bays, bounded by
  `buildings.capacity_soldiers`) is computed as before, as are carrier
  sizes, unloading, bail-out, exit spots and collapse, and the skirmish AI's
  strategy. A helicopter whose squad has a pending `Board` it won't fit
  still holds its low hover until the squad arrives and is refused. An
  encounter still posts one defender squad per building, as a posting
  choice (`Post::Garrison`), not a rule.
- **Gap:** The plan had to bound itself.
- **Reach:** Each is free to change later on its own.
- **Verdict:** sound.
- **Confidence:** medium.

### S14. The labs show the rule with their own staging

- **Who, when:** user (the labs show two squads sharing and a third finding
  no room); implementer for the staging (slice 07).
- **Scenario:** In the garrison lab the house seats exactly the two rifle
  squads, set by the encounter's own override of
  `buildings.capacity_soldiers` (shipped rules untouched). The cast stands
  on one line so arrival order tells the story: the first rifle squad goes
  in, the second follows, and the scouts arrive to "TOO MANY TO FIT" and
  wait at the door, where the house hides them. Red's tank is set to fire at
  will and shells the house's occupants it sees (S3); the house comes down
  and the scouts cross the ruin. In the transport controls lab an AT team
  and a recon team share the APC, and a rifle squad, small enough for the
  APC alone so its order is accepted, arrives to no room. Both labs show
  per-unit refusals on screen. Labs run without sound, so the refusal's
  voice line is covered by its own test, not the lab.
- **Gap:** The plan named the story, not the cast.
- **Reach:** The labs are developer arenas; their scenes are the browser
  proof of the rule.
- **Verdict:** sound.
- **Confidence:** medium.

### S15. A building's seat total is its window plan's while a squad holds it

- **Who, when:** implementer (slice 03).
- **Scenario:** A house's seats are its exposed window bays, which shelling
  can take away. While a living squad holds the house, its total is the
  length of the window plan opened when the first squad was seated; with
  nobody inside, it is the bays the standing building offers now
  (`seats_for_state`). "Is this the first squad seated?" reads the same
  condition (`garrison::held`), so a plan left by squads that died earlier
  in the tick never sets the total. The command-time size check uses the
  standing building as the side knows it.
- **Gap:** The plan said "plan total, or `seats_for_state` before a plan
  exists".
- **Reach:** Admission, reinforcement and the published total (S25) all
  read this one number (`hold::Seats`).
- **Verdict:** sound.
- **Confidence:** high.

### S16. A squad entering a shared house takes the windows nobody holds

- **Who, when:** agent on the user's behalf (plan); implementer for the
  detail (slice 03).
- **Scenario:** Squad A holds a house. Squad B finishes stepping in. B's
  soldiers are spread round the facades as a lone squad would be, the next
  free window of each facade in turn, skipping every window a living soldier
  inside or stepping out holds. Nobody already inside moves on entry. A
  first squad sees none taken, so a lone squad is seated exactly as before.
- **Gap:** The plan said "windows nobody holds"; whether leavers' windows
  count was open.
- **Reach:** Entry never reshuffles a fight in progress; swaps (S4) do that.
- **Verdict:** sound.
- **Confidence:** high.

### S17. One window plan per building, owned by the battle; window-change time stays per window

- **Who, when:** agent on the user's behalf (three of four drafts, reversing
  the walk map's card 3, which put the time on soldiers); implementer for
  the threading (slice 02).
- **Scenario:** Before, each squad in a house kept its own copy of the
  window plan; with several squads the copies would drift apart. Now one
  plan per building lives on `Battle` (`garrison::Windows`), keyed by the
  building's structure owner (the part that stands for the whole building).
  A squad's hold keeps only which window each of its soldiers stands at.
  Each window remembers when a soldier last came to it, which is what the
  hold time after a swap reads; moving that onto soldiers would have changed
  the digest of every scene with a window swap. The battle digest folds the
  plan inside each occupant's hold, where the old copy was, and a squad
  still stepping in folds none. Code reaches the plan through methods on
  `Windows`, and fire and damage carry it in their contexts. The rejected
  alternative (one draft) shared a plan between holds by reference, which
  can't record the per-window time.
- **Gap:** The plan had to give shared windows one owner.
- **Reach:** Every reader of a garrison's windows (sight, fire, damage,
  supply, digest).
- **Verdict:** sound: a lone squad's digest is byte-identical.
- **Confidence:** high.

### S18. A reinforcement needs a free seat in its host

- **Who, when:** user (shown on the walk map's card 2); implementer for the
  check (slice 04); territory for riders.
- **Scenario:** A full house's squad loses a soldier and a supply truck is
  near. A replacement is delivered only while the host has a free seat:
  total less everyone stepping in, inside and stepping out
  (`hold::Seats::free`), the same count the end-of-timer check uses, so a
  replacement can never make a squad fail its own entry. Supply picks the
  next need on the tick it delivers it, so one check is enough. With no
  room the need falls through to the next (smoke, or nothing): no stock and
  no soldier id are spent, and the replacement clock pauses. Squads riding a
  carrier are never reinforced (`Unit::on_field` already excluded them).
- **Gap:** The plan said "check when choosing and when delivering"; they
  are the same moment.
- **Reach:** Supply and host seats share one count.
- **Verdict:** sound.
- **Confidence:** high.

### S19. What the command refuses, and what it leaves to arrival

- **Who, when:** agent on the user's behalf (drafts); implementer for the
  detail (slices 01 and 03).
- **Scenario:** The player sends four squads into a house; one is bigger
  than the whole house. That one is refused at once with `CapacityFull` and
  keeps its orders; the others get entries. An order naming something that
  isn't a building is refused whole (`NotABuilding`), as is one naming a
  vehicle (`NotInfantry`), though the client only sends squads. A squad with
  no entry (S1) is left as it was. If no squad got an entry and none was
  refused, the order is acknowledged `NoValidDestination` and does nothing,
  like a move nobody can make; if every squad was refused, there is no
  whole-order error, as with `Board`. For carriers, `Board` refuses a squad
  bigger than its carrier and `Collect` one bigger than the largest carrier
  it lists. A squad listed twice is taken once, in the order given; the old
  closest-first trimming is gone. Who finds room is decided on arrival. This
  also fixed a `Collect` bug: before, an overflowing `Collect` refused
  whichever squad finished its timer first, because later squads'
  reservations already counted.
- **Gap:** The plan said "only a squad bigger than the whole host" without
  the edge cases.
- **Reach:** The command-time answers for all three host orders come from
  `hold::refuse_oversized`.
- **Verdict:** sound.
- **Confidence:** high.

### S20. How the building order plans each squad's start

- **Who, when:** implementer (slice 01).
- **Scenario:** The player Shift-queues a building order after a move.
  Squads are taken in unit order, each once, so the preview doesn't depend
  on selection order. A queued squad sets off from where its queue ends (a
  move's destination or an earlier entry's approach), counting orders its
  side had accepted earlier in the same tick, as move placement does; a
  replacing order sets off from where it stands, exactly the old lowering,
  so the encounter defenders enter unchanged. The clicked building resolves
  as before: the clicked part as the side knows it, else another remembered
  part of the same building, filed under its structure owner.
- **Gap:** Delegated to the implementer.
- **Reach:** Previews and queued orders read the same plan.
- **Verdict:** sound.
- **Confidence:** high.

### S21. Another side's squad in a building, in any phase, turns an entrant back silently

- **Who, when:** user (one side in a building at a time, kept from before);
  implementer for "any phase" (slice 03).
- **Scenario:** A blue squad reaches a house where a red squad is stepping
  out. Any hold on the building by a living squad of another side
  (stepping in, inside or stepping out) makes blue give up its order
  silently when it would start stepping in, as before. Same-side squads,
  allies included, share.
- **Gap:** "Enemy inside" didn't say which phases.
- **Reach:** Close-quarters fights over houses stay out of this rule.
- **Verdict:** sound.
- **Confidence:** high.

### S22. A squad found too many at the end of its entry timer hears `CapacityFull`

- **Who, when:** agent on the user's behalf (drafts); slice 03.
- **Scenario:** Squad B starts stepping in beside A, which is leaving. A
  turns back (S6's Stop). When B's timer ends, everyone stepping in, inside
  or stepping out no longer fits, so B drops its hold, ends its order and
  publishes `CapacityFull`. Before, a building's end-of-timer failure was
  silent, and so was the old drop of soldiers beyond the window count; both
  are this one check now.
- **Gap:** The plan had to say how the end check tells the player.
- **Reach:** Every refusal for lack of room reads the same.
- **Verdict:** sound.
- **Confidence:** high.

### S23. A refusal on arrival reads exactly like the one at the command

- **Who, when:** implementer (slice 07).
- **Scenario:** A squad refused at the command (too big) and one refused on
  arrival (no room) both show the callout "TOO MANY TO FIT" and get the
  same voice line, which answers any refusal the side observes. One reason,
  one text.
- **Gap:** The plan didn't say whether arrival needed its own words.
- **Reach:** `CapacityFull` stays one concept to the player.
- **Verdict:** sound.
- **Confidence:** high.

### S24. One ring and one outline per house

- **Who, when:** user (one ring whichever occupants are selected; it means
  "in this building", not a count); implementer for the drawing (slice 06).
- **Scenario:** Two own squads hold a house; one is selected and the other
  revealed. The overlay groups occupants by their hold's host, which is the
  structure owner. The brightest occupant paints the ring (selected, then
  hovered, then own over ally, then moving, then reveal strength); the
  others keep their circle, so routes still leave it and picking never sees
  the grouping, but skip the paint. The house's outline is drawn once, in
  the first holder's colour, own before enemy, resolved through the map's
  building index so an identified enemy naming any part or its ruin draws
  the same outline. The rejected alternative, painting the first occupant's
  ring, would show a selected squad in its housemate's reveal colour.
- **Gap:** The user named the result, not the drawing rule.
- **Reach:** Any later marker for shared positions builds on this grouping.
- **Verdict:** sound.
- **Confidence:** high.

### S25. A held building publishes its seat total; used is summed by the reader

- **Who, when:** agent on the user's behalf (drafts); implementer for the
  wire (slice 05).
- **Scenario:** The browser must show S9's row. The simulation publishes
  the building's seat total (S15) on the player's own unit rows as
  `hostSeats` (−1 for a carrier or no hold), and the observation's
  `HostRef::Building` carries it as `seats`. Seats used is not published; the
  client sums its side's holds on that building, with the carrier row's
  phase rule. Enemy panels gain no capacity.
- **Gap:** Building seats weren't published at all.
- **Reach:** A publication layout field; captures and parity vectors carry
  it.
- **Verdict:** sound.
- **Confidence:** high.

### S26. The entry proof's planning-snapshot machinery is gone, and the principle reworded

- **Who, when:** implementer (slice 01, then slice 07 for the skill text).
- **Scenario:** The deleted entry proof (S1) was the only reader of a
  planning snapshot: a side's private replay of the world (a prop overlay,
  its believed geometry, per-request route budgets). It all went, along with
  the two navigation rules only it read (`navigation.move_validation_work`
  and `navigation.move_rehearsal_m`); the fixtures that embed the rules were
  re-recorded, with no battle digest moving. The `tweak-mechanics` skill's
  principle now reads "a side plans from what it knows: all of it, and
  nothing more", without naming the deleted machinery.
- **Gap:** Deleting the proof left machinery and rules with no reader.
- **Reach:** A future planner that needs foresight builds it fresh.
- **Verdict:** sound: when a mechanism is replaced, the old one goes.
- **Confidence:** high.

### S27. The building cursor badge reads the pointer intent alone

- **Who, when:** implementer (whole-spec review).
- **Scenario:** Hovering a house with squads selected shows the enter
  badge. The intent only ever names squads and becomes a ground move with
  none (S8), so the separate "someone can enter" flag the cursor took was
  always true for a building intent; it went.
- **Gap:** A leftover of the old preview.
- **Reach:** None beyond the cursor.
- **Verdict:** sound.
- **Confidence:** high.

### S28. Same-tick arrivals resolve in unit order, and the timers keep their cadence

- **Who, when:** agent on the user's behalf (the walk map's card 6, and the
  plan's invariants).
- **Scenario:** Two squads reach a house with room for one on the same
  tick. The lower unit index is admitted first; no test relies on which. A
  carrier hold starts and advances in the same tick, and a building hold
  starts one step into its entry timer, as before; flattening that would
  have moved every single-host digest. Before admission the squad's
  movement stops whatever the outcome, so a refused squad stands where it
  reached the building.
- **Gap:** The plan had to fix an order for simultaneous arrivals.
- **Reach:** Determinism: a single-squad host behaves byte for byte as
  before.
- **Verdict:** sound.
- **Confidence:** high.

### S29. One building order: `OccupyBuilding`

- **Who, when:** territory (one owner per concept), in the walk (Q4).
- **Scenario:** There were two ways in: the player's `OccupyBuilding`
  (one entrant plus companions gathered nearby) and `Order::Garrison`, sent
  by the encounter defender, labs and tests. `Order::Garrison`, companion
  gathering, `OneSquadPerBuilding` and `BuildingOccupied` are deleted.
  `OccupyBuilding { units, building, gesture }` gives each listed squad its
  own entry, applied as the internal `UnitOrder::Garrison { building,
  approach }` exactly as the old order lowered (no move in front), so the
  defenders' entries are byte-identical. The building preview became
  `BuildingPreviewRequest { units, building, queued }` answered by
  `BuildingPlacement { building, entries }`.
- **Gap:** Arrival admission made gathering spots and one-squad refusals
  meaningless.
- **Reach:** Every script, lab and fixture sends `occupy_building`.
- **Verdict:** sound.
- **Confidence:** high.

### S30. No backward compatibility and no migrations

- **Who, when:** user.
- **Scenario:** Old `garrison` orders in scripts, labs and fixtures were
  rewritten as `occupy_building`, never aliased; fixtures that changed were
  re-recorded.
- **Gap:** A contract cutover always asks this.
- **Reach:** No shim to maintain.
- **Verdict:** sound.
- **Confidence:** high.

### S31. No full-host warning at the cursor

- **Who, when:** user (U3 in the walk).
- **Scenario:** Hovering a house your squads fill shows the ordinary enter
  cursor. A warning would discourage sending a squad to a house you are
  about to empty (S36).
- **Gap:** A full host invites a warning.
- **Reach:** The player learns of a full host only on arrival.
- **Verdict:** sound.
- **Confidence:** high.

### S32. Squads enter whole; one side per building

- **Who, when:** user (assumptions kept from before the walk).
- **Scenario:** A squad of eight never splits to fill five free seats: it
  fits whole or is refused. One seat is one living soldier. A building holds
  one side at a time (S21).
- **Gap:** Sharing could have reopened either.
- **Reach:** Seat arithmetic counts squads' living soldiers
  (`hold::living`).
- **Verdict:** sound.
- **Confidence:** high.

### S33. Window swaps span every squad in the building

- **Who, when:** user (Q5).
- **Scenario:** The AT gunner of one squad takes the window facing the
  tank, whoever's rifleman held it. The rejected alternative split windows
  per squad at entry. S4 is how.
- **Gap:** Sharing raised whose windows a soldier may take.
- **Reach:** Squads sharing a house fight as one garrison.
- **Verdict:** sound.
- **Confidence:** high.

### S34. A squad that arrives to no room ends its order and never waits

- **Who, when:** user (Q3).
- **Scenario:** The scouts reach a full house. They hear "TOO MANY TO FIT",
  their order ends, and they stand where they reached it. If a seat frees a
  minute later, they stay outside until ordered in again. A waiting phase
  existed once and was deleted in September; it stays deleted.
- **Gap:** Overflow had to go somewhere.
- **Reach:** No queue of waiting squads exists anywhere.
- **Verdict:** sound.
- **Confidence:** high.

### S35. The check when stepping in starts ignores leavers; the check when it ends counts everyone

- **Who, when:** user (L1).
- **Scenario:** A house holds squad A, and A is stepping out when squad B
  arrives. B's start check counts squads stepping in or inside, not A, so B
  starts stepping in. A host uses one timer for both directions, so A is
  gone before B finishes. B's end check counts everyone, so if A turned back
  (S6), B is refused then (S22). The rejected alternative, leavers keeping
  their seats, would have made the relief manoeuvre a timing trick.
- **Gap:** Leavers' seats were undefined.
- **Reach:** `hold::admit` (start) and `seated` (end) read this rule.
- **Verdict:** sound.
- **Confidence:** high.

### S36. Seats are taken when stepping in starts, never at the command; the relief manoeuvre is a feature

- **Who, when:** agent on the user's behalf (Q2, the one question the user
  delegated), then locked by the user's own example (U3).
- **Scenario:** Squad A holds a house. The player sends squad B to it, then
  orders A out; by the time B arrives the house is empty and B walks in.
  The user: "This is GOOD, it's a feature." First to arrive gets in; at the
  command only a squad bigger than the whole host is refused. This
  manoeuvre is the required end-to-end test. The rejected alternative,
  promising seats at the command, would refuse B.
- **Gap:** When seats are claimed decides everything else.
- **Reach:** Nothing about who is inside is read at the command; the order
  planner never reads occupants.
- **Verdict:** sound.
- **Confidence:** high.

### S37. Several squads share a host, buildings and carriers alike

- **Who, when:** user (Q1), who widened the question from buildings to both
  hosts. It reverses the 2026-09-28 "a building takes one squad" decision.
- **Scenario:** Two squads walk into one house, or board one APC, while
  seats remain, one seat per living soldier. Those that don't fit end up
  outside. Seats and admission are one rule with one owner (`crate::hold`)
  for both hosts.
- **Gap:** The ask itself.
- **Reach:** Everything above.
- **Verdict:** sound.
- **Confidence:** high.

## Trivial discretion

About a dozen implementer calls with no behaviour of their own, not listed:
which test files hold the new tests and the file splits the size ratchet
forced, internal type and method names, how scenes wait for an arrival
refusal or a carrier's rider card, lab legend wording, and leaving
`PropStore`'s copy-on-write bodies and the movement code's own living counts
as they were.
