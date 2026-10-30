# Battle lab and player pages

The app composes the same simulation adapters, renderer, input and sound used by
player battles. Labs isolate a question with a declared fixture; a private visual
review setup must not change the game's shared rules.

## Routes and fixtures

[The router](src/router.tsx) owns player pages, developer tools and the lab index.
[The fixture registry](src/fixtures.json) declares focused routes and verification
identities; browser scene and tool policy lives in [web/README.md#checks-and-evidence](../../web/README.md#checks-and-evidence); [route modules](src/routes/) implement them. The browser [scene runner](../../web/README.md#checks-and-evidence)
holds the registry and scene files to each other, so adding a lab needs its matching
check. The registry is the route inventory; docs do not maintain a parallel list.

A lab stands on a saved `test` map (its registry entry names it, and
[`fixtureMap`](src/fixtures.ts) refuses any other category) or on a generated
or synthetic world, and fields test units. The battle view itself is judged on
the street test map's battle (`/labs/street`), a test's own encounter, so no
lab plays on the game's content or a capture's. Player pages (`/battle`) only
ever prepare generated maps.

[Visit ownership](src/navigation.tsx) keeps client navigation separate from
battle identity publication. Leaving a page discards its battle; history
returning to its address prepares a fresh visit. Publishing the admitted exact
address updates the current entry without restarting that visit. Queued async
continuations stop when history leaves their visit, before React necessarily
finishes unmounting the previous page.

The main menu starts fresh generated skirmishes and teaches the
game: [the tutorial](src/Tutorial.tsx) reads its economy and command keys from
the skirmish rules and command bindings, and its prose must follow the simulation's
rules when they change. Its pictures are not hand-shot: [the tutorial shots
tool](../../web/tutorial-shots.mjs) plays one seeded skirmish with the faction roster
through the ordinary picker, cursor and orders, finds each moment by what the
observation shows, and writes stills and gesture loops to the served
`assets/runtime/tutorial/`. They show the game as it was drawn, so a change to
the look, the HUD, the generator or the rules is followed by rerunning it and
reviewing the pictures; the tool's header says how. A moment the battle no
longer reaches fails by name rather than shooting something else. It is
one plate over its backdrop, with title-only entries over generated military art.
The same tutorial is available from a live battle's pause menu; it opens as a
modal over the paused battle and closes back to that menu.
Opening a page replaces the plate's contents under a
Back button, and Escape is Back, so the film behind never changes. A menu address asking for a battle opens on the
skirmish page. That page is [the lobby](src/lobby/Lobby.tsx), an order of
battle: [the lobby model](src/lobby/model.ts) owns its rules, and the page only
shows them. Its map is a real admitted preparation, started again only when a
map input changes, and Deploy opens only on a ready map and names the seed that
was admitted, so the battle is the map the player saw. The preview may try
bounded fresh seeds; when generation is exhausted the lobby shows no map and
Deploy stays shut, and no fixed map is ever substituted. The lobby's players are
always the battle's players, numbered by place on their side, and its preview
and its Deploy link build one request ([skirmish setup](src/skirmishSetup.ts)),
so the battle is the one previewed. A battle address names exact preparation
inputs on the same build. It must name both sides' players and factions; the
map type, size, map seed and battle seed default when left out, the size to the
smallest map that fits the players. A link with humans has exactly one, on
Blue. An old-shape link (its `faction` or `enemy`, or no players), a wrong
value, or a parameter the battle does not read is refused by name, never
ignored, and a refused seed never silently becomes another battle.

[The menu backdrop](src/MenuBackdrop.tsx) is real battles, not recordings: scenes
played in order and then over again, each a saved encounter on its own `capture` map
(a capture refuses any other) run by
the ordinary session, silent and inert. Each is played by
[the scene player](src/capture/ScenePlayer.tsx), the [capture tool](src/capture/README.md)'s too, which
flies [its reel](src/capture/capture.ts) through the viewport's pilot; the menu grades it in CSS to the
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
backdrop and the capture route take the capture set), the session refuses a scenario fielding a unit its
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
viewport and army HUD without making it a new React prop tree on every publication. React's
development instrumentation traverses changed typed-array props, so an apparently
ordinary prop update can retain enormous diagnostic payloads. Keep live geometry,
fog, effects and complete unit observations behind the feed boundary. Consumers
that render React views subscribe to the same feed, publishing before paint so
cards and commands agree with the committed selection. This keeps full observations
out of development profiling details.

[Page resources](src/appResources.ts) retain the admitted GPU across screen changes;
[the viewport](src/LabViewport.tsx) owns the canvas configuration and scene
allocations. [Session telemetry](src/useViewportGpu.ts) borrows that live renderer;
the viewport withdraws the borrow before freeing its scene, so telemetry has no
renderer between generations. The battle draws one pixel per CSS pixel on any display, and a
high-density screen scales it up: drawing at the display's density multiplies the
frame's cost for detail a moving battlefield hardly shows, while the HUD, drawn by
the page, stays at full density. A required resource failure ends play until manual Reload, while the
menu remains available. Document departure releases the page GPU; component
refresh cleanup only detaches listeners so development refresh cannot destroy a
still-live page's resources.

Source loading retains the last complete topology and art generation for the
same battle and observer. Downloads cannot withdraw the running battlefield;
changing battle or observer retires its old generation. Audio continues receiving
publications during visual admission. The viewport reconciles sources after
canonical framing so a topology change takes one rebuild path. Picking,
projection and drawn observation advance with successful submission.

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
with explicitly private nonlethal review settings. [Guided fire](../../specs/done/battle-foundation/contracts.md#guided-flight)
separates shared spotting from the launcher's physical sight. [Benchmarking](../../web/src/battle/benchmark/README.md)
retains exact workload identity so a convenient lab cannot replace a performance control.

The [template line-up scene](../../web/scenes/city-lineup.mjs) visits every source set separately and holds
its combined template identities to the complete catalogue. Each visit
retains the state, tier and transition-picture checks while keeping the shared
kit download within the same map budget. Use the line-up route's `?set=` selector
for development: its unrestricted all-set view currently exceeds that budget and
is refused; the scene's complete coverage does not imply that view is admitted.

The [geometry scaling control](src/benchmark/geometryScaling.ts) appends distant
terrain to one admitted export without recompiling the local patch. It uses the
production viewport and measurement pilot; material domains and static identities
stay local while the full mesh reaches receiver fitting. Its registered scene is
an explicit measurement tool. The [performance rationale](../../specs/done/performance/README.md)
records its scope and measurement limits.
