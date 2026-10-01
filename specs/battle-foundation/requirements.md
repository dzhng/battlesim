# User requirements

These are the canonical user decisions. They preserve all 80 entries from the interview map; owners name the slice that implements or verifies the rule. A slice numbered 17–22 is **deferred until after the village checkpoint**, not deleted scope. See `README.md` for the graph and `contracts.md` for spec-authored implementation resolutions. A proposal in the spec is not retroactively a user answer.

The original brief is retained in [assets/original-brief.txt](assets/original-brief.txt); it contains typos and unfinished phrases. Later decisions below take precedence. The archived [interview map](assets/interview-map.html) is evidence, not a second live specification.

## S01

**Decision:** Local prototype first; eventual multiplayer constrains architecture, but networking is deferred.

**Why:** Prove combat without making future authority and information separation expensive.

**Owner:** slices 03. **Authority:** user brief/interview.

## S02

**Decision:** Reuse the stack and monorepo pattern of ../game: Rust/WASM simulation, TypeGPU battle rendering, TypeScript browser shell.

**Why:** Reuse established foundations rather than inventing a second stack.

**Owner:** slices 01. **Authority:** user brief/interview.

## S03

**Decision:** Establish the 3D foundation first, using simple assets; defer polished visuals, extensive rosters, and deck building.

**Why:** Battle mechanics and readability are the priority.

**Owner:** slices 01. **Authority:** user brief/interview.

## S04

**Decision:** Design for about 60–100 controllable units per side on roughly 4 × 4 km; one infantry squad is one unit.

**Why:** Provide room for combined arms, reconnaissance, and flanking.

**Owner:** slices 16,22. **Authority:** user brief/interview.

## S05

**Decision:** Target 45–60 minutes for an evenly matched full battle; no hard time limit has been selected.

**Why:** Allow sustained maneuver, recovery, and repeated assaults.

**Owner:** slices 22. **Authority:** user brief/interview.

## S06

**Decision:** Holding a majority of marked objectives earns victory progress.

**Why:** Force movement and contested territory.

**Owner:** slices 21. **Authority:** user brief/interview.

## S07

**Decision:** Initial forces plus equal periodic purchasing income; reinforcements enter from friendly map edges.

**Why:** Permit recovery without territory also compounding purchasing power.

**Owner:** slices 21. **Authority:** user brief/interview.

## S08

**Decision:** Infantry and armed ground vehicles capture; aircraft and supply units do not.

**Why:** Support different ground force compositions.

**Owner:** slices 21. **Authority:** user brief/interview.

## S09

**Decision:** All eligible units contest, including concealed units. Contested status reveals presence somewhere in the zone, not identity or location.

**Why:** Physical occupation matters, with a deliberate information clue.

**Owner:** slices 21. **Authority:** user brief/interview.

## S10

**Decision:** Capture takes an uncontested timer; ownership persists after departure. Contesting pauses capture and scoring from that objective; enemy takeover requires a new capture timer.

**Why:** Avoid permanent caretaker units while supporting counterattacks.

**Owner:** slices 21. **Authority:** user brief/interview.

## V01

**Decision:** Use ground, helicopter, and jet detection layers, with per-unit detection ranges for each.

**Why:** Recon and specialized sensors have distinct roles.

**Owner:** slices 05,18,19. **Authority:** user brief/interview.

## V02

**Decision:** Ground fog represents line of sight; airborne contacts can appear over ground fog. Terrain, buildings, and sufficient forest depth obstruct ground vision.

**Why:** Separate map visibility from detection of airborne targets.

**Owner:** slices 05,06. **Authority:** user brief/interview.

## V03

**Decision:** Concealment reduces detection distance; cover reduces hit probability rather than successful-hit damage. Both vary continuously with their source.

**Why:** Preserve meaningful differences between forests and buildings.

**Owner:** slices 05,09. **Authority:** user brief/interview.

## V04

**Decision:** Infantry can conceal near forest edges; vehicles need more depth. Thin forest may permit ground observers to see vehicles beyond it; sufficiently thick forest blocks vision.

**Why:** Unit size and forest depth shape reconnaissance.

**Owner:** slices 05. **Authority:** user brief/interview.

## V05

**Decision:** Air observers ignore forest foliage when detecting vehicles inside or beyond forests. Range, hills, and buildings still matter. Infantry retains forest protection against air.

**Why:** Vehicles cannot rely on woods to hide from aircraft.

**Owner:** slices 18. **Authority:** user brief/interview.

## V06

**Decision:** Camo, anti-radar properties, and low helicopter flight can reduce detection; precise modifiers remain open.

**Why:** Support different ways to evade sensors.

**Owner:** slices 18,19. **Authority:** user brief/interview.

## V07

**Decision:** Identified targets are shared across friendlies; shooters still need range and a valid projectile path.

**Why:** Recon extends supporting weapons’ effective reach.

**Owner:** slices 05,08. **Authority:** user brief/interview.

## V08

**Decision:** Approximate contacts disclose an uncertain area without exact identity or location, including concealed targets outside ground fog and firing sources behind obstacles.

**Why:** Support uncertain return fire without omniscient targeting.

**Owner:** slices 06. **Authority:** user brief/interview.

## V09

**Decision:** Losing identification creates a fading approximate contact around the last observed location, using the spotted-contact visual language. It never follows unseen movement.

**Why:** Preserve useful memory without tracking hidden enemies.

**Owner:** slices 06. **Authority:** user brief/interview.

## V10

**Decision:** Every shot, including fire at empty ground, creates or refreshes an approximate firing-source contact for the opposing side, regardless of line of sight. Direct identification takes precedence.

**Why:** Make firing a deliberate information cost.

**Owner:** slices 06,08. **Authority:** user brief/interview.

## V11

**Decision:** Only friendly units within hearing range reveal unseen-enemy sounds to the player. Audio conveys approximate direction and broad unit character, without creating a visual contact by itself.

**Why:** Sound prompts reconnaissance without camera scouting.

**Owner:** slices 06. **Authority:** user brief/interview.

## V12

**Decision:** Retain acquisition for 1.5 seconds after identification is lost. Unfinished aim continues toward the last observed position; returning identification preserves progress. Expiry clears acquisition.

**Why:** Prevent forest-edge flicker from repeatedly restarting aim.

**Owner:** slices 08. **Authority:** user brief/interview.

## V13

**Decision:** The visibility grace never tracks hidden movement or grants accurate hidden-target fire. Approximate-contact attacks follow their own accuracy rules.

**Why:** Keep observation memory separate from current knowledge.

**Owner:** slices 06,08. **Authority:** user brief/interview.

## W01

**Decision:** Each weapon independently aims, reloads, and selects a target.

**Why:** Allow mixed-weapon units to engage several enemies concurrently.

**Owner:** slices 08. **Authority:** user brief/interview.

## W02

**Decision:** Aim and reload overlap. A weapon aims once per target acquisition; repeated shots at an uninterrupted target require reload only.

**Why:** Separate acquisition from readiness.

**Owner:** slices 08. **Authority:** user brief/interview.

## W03

**Decision:** One stationary property covers aiming, firing, and reloading. Movement clears all aim, including completed acquisition, and resets unfinished reload for those weapons. Loaded ammunition stays loaded.

**Why:** Keep stationary requirements simple and legible.

**Owner:** slices 08. **Authority:** user brief/interview.

## W04

**Decision:** Movement-capable weapons can aim, reload, and fire while moving, with the brief’s 50% accuracy reduction as an initial tuning target.

**Why:** Enable mobile combat at a cost.

**Owner:** slices 08. **Authority:** user brief/interview.

## W05

**Decision:** Interrupted reload restarts from zero; switching targets starts fresh aim.

**Why:** Make interruption costs predictable.

**Owner:** slices 08. **Authority:** user brief/interview.

## W06

**Decision:** Prefer the highest-cost identified enemy a weapon can damage and legally engage; if none qualifies, choose the nearest eligible approximate contact.

**Why:** Preserve cost priority without reading hidden identities.

**Owner:** slices 08. **Authority:** user brief/interview.

## W07

**Decision:** An automatic target remains locked through aiming and the shot while valid. Reconsider priorities during reload. Explicit orders override immediately.

**Why:** Prevent target churn from indefinitely delaying fire.

**Owner:** slices 08. **Authority:** user brief/interview.

## W08

**Decision:** Explicit target orders focus weapons that can damage that target; other weapons may engage suitable alternatives.

**Why:** Permit deliberate focus without wasting incompatible weapons.

**Owner:** slices 08. **Authority:** user brief/interview.

## W09

**Decision:** Every armed unit has an unlimited-ammo default gun, which still fires at an invulnerable identified enemy when no damageable target is available, before any approximate contact. Only that gun does: a weapon with finite ammunition fires automatically only at what it can damage. (Amended 2026-09-29, user.)

**Why:** Keep depleted units active; ineffective fire can still reveal them.

**Owner:** slices 08. **Authority:** user brief/interview.

## W10

**Decision:** Automatically engage approximate contacts with general-purpose weapons, but only while no identified enemy is within the weapon's reach: a unit fires at the enemy it can see before an unknown it cannot (amended 2026-09-29, user). Reserve dedicated anti-tank and anti-air missiles for identified compatible targets. Tanks prefer HE for uncertain contacts.

**Why:** Prevent hidden-state ammunition selection and limit specialist waste.

**Owner:** slices 08. **Authority:** user brief/interview.

## W11

**Decision:** Do not cap automatic speculative exchanges initially, even when they sustain themselves through firing reveals and unlimited guns.

**Why:** User chose simplicity over an extra burst-budget rule.

**Owner:** slices 08,16. **Authority:** user brief/interview.

## W12

**Decision:** Engagement policy belongs to the whole unit: Fire at will or Return fire only. “Hold fire” means Return fire only, not absolute silence.

**Why:** Keep policy at squad level.

**Owner:** slices 08. **Authority:** user brief/interview.

## W13

**Decision:** Return fire only allows retaliation against an enemy attacking that unit, including misses, not merely attacking a nearby ally. Permission lasts through identified or active approximate contact and ends when contact expires.

**Why:** Tie retaliation to a particular observed attacker.

**Owner:** slices 08. **Authority:** user brief/interview.

## W14

**Decision:** Explicit attack and attack-move switch the entire unit to Fire at will until the player changes it. Ordinary move and Stop preserve policy.

**Why:** Make offensive commands sufficient to start fighting.

**Owner:** slices 08. **Authority:** user brief/interview.

## W15

**Decision:** Stop clears queued orders and cancels current movement, aim, reload, and guidance once; autonomous combat may immediately restart aim or reload.

**Why:** Stop halts the current action rather than persistently disabling combat.

**Owner:** slices 08,12. **Authority:** user brief/interview.

## W16

**Decision:** Ordinary move keeps moving with eligible opportunistic fire. Attack-move stops for reachable targets and resumes when none remains; it does not chase off-route.

**Why:** Make repositioning predictable.

**Owner:** slices 04,08. **Authority:** user brief/interview.

## W17

**Decision:** Only direct attack orders authorize pursuit. Pursue identified targets to regain a firing position; after identification loss use the last reported location, never hidden movement.

**Why:** Separate intentional pursuit from automatic targeting.

**Owner:** slices 04,08. **Authority:** user brief/interview.

## W18

**Decision:** Automatically selected targets never initiate movement from a stationary position.

**Why:** Keep defensive units from being drawn out of cover.

**Owner:** slices 08. **Authority:** user brief/interview.

## P01

**Decision:** All weapons, including bullets, use physical projectiles. Shot deviation represents accuracy; travel and collision determine direct hits.

**Why:** Enable dodging and emergent interactions.

**Owner:** slices 07. **Authority:** user brief/interview.

## P02

**Decision:** Flight includes gravity; aiming compensates with elevation. Weapons lead using observed velocity and flight time, never future orders or hidden motion.

**Why:** Make predictable motion hittable and evasive changes useful.

**Owner:** slices 07. **Authority:** user brief/interview.

## P03

**Decision:** Direct-fire weapons use normal low ballistic arcs; only dedicated indirect-fire weapons choose high arcs. Check the actual curved firing path.

**Why:** Preserve artillery roles while honoring gravity.

**Owner:** slices 07,22. **Authority:** user brief/interview.

## P04

**Decision:** Unguided rockets require no post-launch guidance; changing target motion can evade them.

**Why:** Distinguish unguided flight from seeker guidance.

**Owner:** slices 07. **Authority:** user brief/interview.

## P05

**Decision:** ATGMs acquire targets identified by any friendly unit, while requiring the stationary launcher’s own clear physical line of sight for both acquisition and guidance. Spotting range does not limit this sight-line check.

**Why:** Breaking launcher sightlines has tactical value.

**Owner:** slices 10. **Authority:** user clarification, 2026-10-01.

## P06

**Decision:** Moving or canceling guidance releases the crew immediately. Lost guidance never reacquires; the missile continues toward its last target point. A target that stays there can still take normal damage.

**Why:** Loss of tracking creates a chance to evade, not automatic immunity.

**Owner:** slices 10. **Authority:** user brief/interview.

## P07

**Decision:** Radar AA missiles likewise require a stationary launcher and its own radar contact; lost contact ends guidance permanently and leaves flight toward the last target point.

**Why:** Reuse the support-guided interaction.

**Owner:** slices 19. **Authority:** user brief/interview.

## P08

**Decision:** Infantry anti-air missiles acquire before launch, then track with their own seeker. The launcher may move immediately; terrain breaking seeker sight ends tracking.

**Why:** Give mobile infantry AA a distinct role.

**Owner:** slices 19. **Authority:** user brief/interview.

## P09

**Decision:** Physical collisions and blast apply regardless of allegiance or intended target. A projectile is consumed at its first damaging hit; it does not overpenetrate into another unit.

**Why:** Support collateral damage without through-and-through simulation.

**Owner:** slices 07,09. **Authority:** user brief/interview.

## P10

**Decision:** Vehicles have front, side, rear, and roof armor. Penetration determines whether a hit defeats armor; penetrating hits damage vehicle health. No internal component simulation.

**Why:** Make flanking matter with manageable complexity.

**Owner:** slices 09. **Authority:** user brief/interview.

## P11

**Decision:** Weapons withhold visibly unsafe shots when friendly vehicles obstruct the predicted path or occupy the blast danger area. Friendly infantry does not trigger this prefire check, but can still be hit.

**Why:** Avoid knowingly firing through vehicles without infantry constantly blocking fire.

**Owner:** slices 08. **Authority:** user brief/interview.

## P12

**Decision:** Occupied-building protection is an incoming accuracy penalty, not an additional wall test or damage reduction against occupants. Buildings still block shots through them toward targets beyond.

**Why:** Treat misses as rounds stopped by walls, as requested.

**Owner:** slices 11. **Authority:** user brief/interview.

## P13

**Decision:** Blast exposure is resolved per soldier using radius and distance; cover lowers damaging-fragment hit probability. Successful hits retain normal damage for that distance.

**Why:** Extend probability-based cover to explosions.

**Owner:** slices 09,11. **Authority:** user brief/interview.

## P14

**Decision:** Near misses, impacts, and explosions add weapon-specific suppression even without damage. Suppression reduces infantry movement and fire rate, never forces retreat, and decays after a lull.

**Why:** Enable pinning and flanking while retaining player control.

**Owner:** slices 09. **Authority:** user brief/interview.

## M01

**Decision:** Ordinary movement uses the shortest traversable route; double-right-click fast travel minimizes travel time using roads and terrain speeds. Neither automatically avoids enemy fire.

**Why:** Give the two orders distinct, predictable purposes.

**Owner:** slices 04. **Authority:** user brief/interview.

## M02

**Decision:** Infantry and vehicles traverse forests, with a larger slowdown for vehicles. Roads through forests retain fast travel.

**Why:** Support roads without infantry-only forest routes.

**Owner:** slices 04. **Authority:** user brief/interview.

## M03

**Decision:** Slopes slow ground units; one shared steepness cutoff blocks all ground units. Do not use different infantry and vehicle slope-access limits.

**Why:** Keep terrain access understandable.

**Owner:** slices 02,04. **Authority:** user brief/interview.

## M04

**Decision:** Vehicles avoid one another or wait; infantry avoids vehicles and other squads with flexible spacing. Do not rigidly collide every soldier or allow driving through friendly formations.

**Why:** Preserve traffic without brittle body jams.

**Owner:** slices 04. **Authority:** user brief/interview.

## M05

**Decision:** Groups preserve relative destination positions where space permits, separate for navigation, and reform on arrival. Units travel at their own speeds.

**Why:** Preserve tactical arrangement without forced speed matching.

**Owner:** slices 04. **Authority:** user brief/interview.

## M06

**Decision:** Wrecks and corpses persist for the entire battle. Vehicle wrecks obstruct vehicles.

**Why:** Let battle history change routes and appearance.

**Owner:** slices 09,16. **Authority:** user brief/interview.

## M07

**Decision:** Wrecks and ruins follow normal prop geometry for sight and projectile obstruction. Lower remains can be seen or fired over.

**Why:** Avoid special cover logic detached from physical shape.

**Owner:** slices 02,09,11. **Authority:** user brief/interview.

## M08

**Decision:** New blockage triggers rerouting. If no route exists, stop, show route blocked, and retain the intended destination.

**Why:** Handle destruction without constant corrective commands.

**Owner:** slices 04,09. **Authority:** user brief/interview.

## M09

**Decision:** Water blocks every ground unit; bridges provide crossings. Bridges are indestructible initially, although wrecks can obstruct them.

**Why:** Provide chokepoints without bridge engineering mechanics.

**Owner:** slices 02,04. **Authority:** user brief/interview.

## L01

**Decision:** Deployment and undeployment have equal duration and reversible progress. A move order unwinds completed setup before movement; canceling movement reverses back toward deployed from current progress.

**Why:** Support consistent behavior and future reversed animation.

**Owner:** slices 12. **Authority:** user brief/interview.

## L02

**Decision:** Supply vehicles must remain stationary and complete a long deployment before service; movement requires long undeployment.

**Why:** Make supply placement a commitment.

**Owner:** slices 12,13. **Authority:** user brief/interview.

## L03

**Decision:** Deployed supply vehicles automatically replenish finite ammunition, repair surviving vehicles, and replace casualties in surviving infantry squads, consuming shared supply stock. Fuel is omitted initially.

**Why:** Favor sustainable gameplay, including deliberately nonrealistic infantry replacement.

**Owner:** slices 13. **Authority:** user brief/interview.

## L04

**Decision:** Recipients must be stationary and not firing. Incoming fire does not interrupt service.

**Why:** Encourage rotation without brittle combat cooldowns.

**Owner:** slices 13. **Authority:** user brief/interview.

## L05

**Decision:** Supply stock is finite and cannot be refilled. Purchase additional vehicles when it is exhausted.

**Why:** Make supply a spending and positioning choice.

**Owner:** slices 13. **Authority:** user brief/interview.

## L06

**Decision:** Loading and unloading take a short stationary timer; loading requires the squad and transport to stop near each other. Passengers cannot fire aboard.

**Why:** Reward dismounting before contact.

**Owner:** slices 17. **Authority:** user brief/interview.

## L07

**Decision:** Transport destruction resolves survival per passenger; survivors emerge near the wreck heavily suppressed.

**Why:** Make transport loss costly without guaranteed squad deletion.

**Owner:** slices 17. **Authority:** user brief/interview.

## L08

**Decision:** Whole squads occupy buildings after a short entry time; soldier capacity permits multiple friendly squads if they fit. No room-by-room navigation.

**Superseded (2026-09-28):** one squad per building, within the soldier capacity (`specs/done/battle-look/decisions.md`).

**Why:** Support varied squad and building sizes simply.

**Owner:** slices 11. **Authority:** user brief/interview.

## L09

**Decision:** Garrisoned soldiers use abstract perimeter firing positions. Their own building does not block outgoing shots; other obstacles do.

**Why:** Make prototype buildings functional without authored windows.

**Owner:** slices 11. **Authority:** user brief/interview.

## L10

**Decision:** Buildings are destructible and leave persistent impassable ruins. Collapse causes occupant casualties; survivors emerge outside heavily suppressed.

**Why:** Allow defensive positions to be destroyed while retaining obstruction.

**Owner:** slices 11. **Authority:** user brief/interview.

## A01

**Decision:** Helicopter low flight physically lowers altitude and speed, trading mobility for concealment and more restricted own sight/firing paths. It remains in the helicopter detection layer.

**Why:** Make low flight interact with terrain.

**Owner:** slices 18. **Authority:** user brief/interview.

## A02

**Decision:** Purchased jets perform off-map sorties: enter from the friendly map edge, strike, exit, and rearm before reuse. Destroyed jets need replacement.

**Why:** Deliver delayed powerful air strikes with limited management.

**Owner:** slices 20. **Authority:** user brief/interview.

## A03

**Decision:** Radar jet detection requires range and sightlines unobstructed by terrain, buildings, or forests. Radar deployed inside forest is completely obstructed.

**Why:** Make radar placement important.

**Owner:** slices 19. **Authority:** user brief/interview.

## U01

**Decision:** Shift queues movement and actions such as attack, load/unload, and garrison. Ordinary orders replace the queue; Stop clears it.

**Why:** Reduce repetitive control.

**Owner:** slices 04,08,11,17. **Authority:** user brief/interview.

## U02

**Decision:** Each weapon has concentric aim and reload rings; deployment has a separate circle. Colors distinguish actions. Active guidance uses an icon; ammo is shown with the reload display.

**Why:** Explain concurrent weapon readiness.

**Owner:** slices 14. **Authority:** user brief/interview.

## U03

**Decision:** Use remaining-round numbers inside the weapon rings, with infinity for unlimited ammo; completed timers disappear.

**Why:** Accepted display proposal accompanying the queue decision.

**Owner:** slices 14. **Authority:** user brief/interview.

## N01 — Compatibility

User explicitly confirmed **neither backward compatibility nor data migrations**. Do not preserve sibling saves, gameplay formats, public APIs, campaigns, or obsolete interfaces. Preserve reusable code behavior only where it supports this game.

## N02 — Intended player

Strategy players who find WARNO-style games too demanding. Explain why a unit cannot fire, why a route fails, what revealed a unit, and what an order does. Tactical depth is retained; routine babysitting is reduced.

## N03 — Ambush taste

Prompt reaction gives a narrow chance to escape an ordinary ambush; a strongly prepared ambush can make losses unavoidable. Recon buys safety, but reaction can rescue some mistakes. Do not tune every ambush into either guaranteed death or guaranteed escape.

## N04 — First acceptance checkpoint

A small replayable village encounter: scout defenses, suppress infantry, maneuver tanks around an AT ambush, and rotate surviving forces through finite supplies. Simple readable 3D is enough. It precedes a complete long match; it is not merely a static mechanics sandbox.

## N05 — Hardware

The user's current machine first; broaden support after mechanics work. Planning measured Apple M5 Pro, arm64, Mac17,9, 51,539,607,552 bytes RAM (48 GiB). Browser adapter and real performance are unmeasured; the implementation session must verify its own host.

## N06 — Durable handoff

The user will hand this spec to another implementation session. No gameplay rule may rely on scrollback. Full-game rules must remain findable even if deferred from the village checkpoint. The user requested a specification, not implementation in this planning session.

## Original roles and provisional numerical intent

Infantry consists of squads: fragile exposed or suppressed, survivable through plentiful concealment/cover and disengagement. Tanks have large weapon range but weak optics; they outrange most ground weapons except artillery, resist normal guns, and need recon/infantry support against scarce anti-tank weapons. Specialist AT/AA squads are smaller and weaker against infantry, carry limited missiles, and have long aim/reload. Vehicles are fast and fragile; armored carriers trade speed against protection. Helicopters favor quick strikes and withdrawal; jets are deliberately powerful without radar AA opposition, while AA operates far behind the line.

The original approximate kill counts (tanks about 2–3 ATGMs or 4–5 rockets; armored carriers about 1 ATGM or 2–3 rockets; helicopters and jets about 2–3 relevant missiles), concealment of 50–90% reduced detection distance, ~100 m contact radius, ~10% uncertain-fire accuracy, and 50% moving-fire accuracy penalty are **tuning intent**, not historical realism or frozen final values. Initial fixture choices belong only in `fixtures/game.json`. No physical-simulation detail overrides the explicit cover/garrison/infantry-replacement abstractions.

## Superseded alternatives

Per-target shared aim lost to independent weapon aim. Separate stationary flags lost to one flag for aim/fire/reload. Immediate loss-of-aim reset lost to 1.5 s grace with continuing aim. Straight-heading lost-guidance flight lost to the last target point. Permanent infantry casualties lost to supply replacement of living squads. Absolute hold-fire lost to Return fire only. Stop-waits-for-a-new-order lost to one-shot cancellation and automatic restart. Strict soldier collisions, per-class slope access, individually authored windows, overpenetration, destructible bridges, and capped speculative bursts were rejected in favor of the simpler rules above. Do not resurrect these alternatives as silent implementation improvements.
