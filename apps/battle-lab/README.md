# Battle lab and player pages

The app composes the same simulation adapters, renderer, input and sound used by
player battles. Labs isolate a question with a declared fixture; a private visual
review setup must not change the game's shared rules.

## Routes and fixtures

[The router](src/router.tsx) owns player pages, developer tools and the lab index.
[The fixture registry](src/fixtures.json) declares focused routes and verification
identities; [route modules](src/routes/) implement them. The browser [scene runner](../../web/README.md#checks-and-evidence)
holds the registry and scene files to each other, so adding a lab needs its matching
check. The registry is the route inventory; docs do not maintain a parallel list.

A lab stands on a saved `test` map (its registry entry names it, and
[`fixtureMap`](src/fixtures.ts) refuses any other category) or on a generated
or synthetic world, and fields test units. The battle view itself is judged on
the street test map's battle (`/lab/street`), a test's own encounter, so no
lab plays on the game's content or the menu's. Player pages (`/battle`) only
ever prepare generated maps.

[Visit ownership](src/navigation.tsx) keeps client navigation separate from
battle identity publication. Leaving a page discards its battle; history
returning to its address prepares a fresh visit. Publishing the admitted exact
address updates the current entry without restarting that visit. Queued async
continuations stop when history leaves their visit, before React necessarily
finishes unmounting the previous page.

The main menu starts fresh generated skirmishes and opens replays. It is
one plate over its backdrop, with title-only entries over generated military art.
Opening a page replaces the plate's contents under a
Back button, and Escape is Back, so the film behind never changes. A menu address asking for a battle opens on the
skirmish page. A
seeded address names exact preparation inputs on the same build, both factions
included. A battle address without the player's faction, or with a parameter the
battle does not read, is refused by name; nothing is defaulted or ignored. Ordinary Play
may try bounded fresh candidates, then publishes the
actual admitted identity. A refused explicit seed cannot silently become another
battle. Exhausted generation returns a refusal; it never substitutes a fixed map.

[The menu backdrop](src/MenuBackdrop.tsx) is real battles, not recordings: scenes
played in order and then over again, each a saved encounter on its own `menu` map
(the reel refuses any other) run by
the ordinary session, silent and inert. Each is filmed by
[the reel](src/menuReel.ts) through the viewport's pilot and graded in CSS to the
HUD's `film` amber, which the plate's blue reads against. It shows blue's
observation like any battle, without the player's x-ray. Its preparation reports
to the same loading stages as a lab, and the menu's loading screen stands in the
plate's place until the battle first stands at its warm tick; the battle holds
there until the menu is shown, so the reel's first cut opens on the moment it was
cut for. Cuts dip through an opaque veil of the menu's own ground. When a reel
ends the next scene's battle (its saved map and encounter fetched while the last
one played) starts and warms behind the veil, without the loading screen. The plate measures where it stands, and the
pilot slides each framing so its subject plays in the open screen beside it. The
film's camera is the player's rig reaching in as close as the reels' closest
framing, so a shot in a narrow street may stand nearer than a player may zoom.

[The shared battle view](src/BattleView.tsx) composes page resources and observation
feeds. [Session ownership](src/useBattleSession.ts) keeps worker preparation and
cleanup tied to that page. A session also owns its unit catalog: [the router](src/router.tsx)
gives player battles and replays the game's set and labs the test set (the menu
backdrop takes the menu's), the session refuses a scenario fielding a unit its
catalog lacks, and views read the catalog from the session or its context, never
from a module-level import. [Lab loading](src/LabLoading.tsx) uses the player
loading screen across preparation, the first observation and the first drawn
frame; routes keep preparing beneath the cover so it never delays readiness. While
anything the page needs is downloading, the screen counts the megabytes
([download counting](../../web/src/downloads.ts)), so a first visit's long wait
visibly moves. The menu's build line names the app's commit and a fingerprint
of the simulation module the page actually loaded ([build identity](src/buildIdentity.ts)),
so a stale deploy or cache shows on screen.
Art uses [scene-assets](../../packages/scene-assets/README.md),
and sound uses [battle-audio](../../packages/battle-audio/README.md). [Model review](src/workbench/)
installs previews through the same appearance loader used by battles.

## Observation feeds

[Stable feeds](src/feed.ts) carry large, changing presentation data to the
viewport without making it a new React prop tree on every publication. React's
development instrumentation traverses changed typed-array props, so an apparently
ordinary prop update can retain enormous diagnostic payloads. Keep live geometry,
fog and effects behind the existing feed boundary rather than copying them into
component state.

[Page resources](src/appResources.ts) retain the admitted GPU across screen changes;
[the viewport](src/LabViewport.tsx) owns the canvas configuration and scene
allocations. A required resource failure ends play until manual Reload, while the
menu remains available. Document departure releases the page GPU; component
refresh cleanup only detaches listeners so development refresh cannot destroy a
still-live page's resources.

The viewport subscribes and updates [the renderer](../../packages/battle-renderer/README.md)
through that boundary. Published cause and identity remain intact; feed interpolation
cannot grant visibility or invent firing. [The scene runner](../../web/scene.mjs)
checks oversized development performance details as well as ordinary page errors.

## Presentation contracts

[Browser presentation](../../web/src/battle/present/) turns observed evidence into
panels, captions, contacts and cursors. Hidden contact reports retain uncertainty;
retiring visuals are remembered evidence, never live targets. Panels prioritize
legibility and camera depth while preserving hover access to detail. Sensing rules
own concealment; the UI cannot infer a hidden observer from private state. A new
piece of interface is drawn by the panel workbench, the [UI gallery](src/routes/ui.tsx)
or the cursor lab, whose scenes pin it with approved pictures
([browser checks](../../web/README.md#checks-and-evidence)).

[The army HUD and app-lifetime rationale](../../specs/done/hud-chrome/README.md)
records the player layout, menu audio and retained-resource contracts.

The shipped rationales explain the non-obvious decisions behind [readout backgrounds](../../specs/done/readout-backgrounds/README.md),
[group movement previews](../../specs/done/group-move-preview/README.md),
[physical move validity](../../specs/done/move-validity/README.md) and
[contextual cursors](../../specs/done/game-cursor/README.md). Weapon behavior and its
visible readiness follow [fire cadence](../../specs/done/fire-cadence/README.md) and
[infantry weapon use](../../specs/done/infantry-weapon-use/README.md).

[Projectile review](src/projectileReview.ts) uses gameplay flight and weapon cycles
with explicitly private nonlethal review settings. [Guided fire](../../specs/battle-foundation/contracts.md#guided-flight)
separates shared spotting from the launcher's physical sight. [Benchmarking](../../web/src/battle/benchmark/README.md)
retains exact workload identity so a convenient lab cannot replace a performance control.

The [template line-up scene](../../web/scenes/city-lineup.mjs) visits every generated source set separately and holds
its combined template identities to the complete generated catalogue. Each visit
retains the state, tier and transition-picture checks while keeping the shared
kit download within the same map budget. Use the line-up route's `?set=` selector
for development: its unrestricted all-set view currently exceeds that budget and
is refused; the scene's complete coverage does not imply that view is admitted.
