# C61: map catalogue listings

**Depends on:** C60. **Kind:** slice.

## Question
Do saved-map listings come from the catalogue while runtime acquisition uses the same map resolver?

## Contract it unlocks
- `MainMenu.tsx:6-13` builds Play and Watch from `category: playable, status: released`, and Benchmark from `benchmarks[]`. `/labs` lists `category: lab`, and drafts show only there.
- **Routes stay separate from maps:** `apps/battle-lab/src/fixtures.json` stays the owner of routes and scenes. There are 25 routes over 12 maps, and tool pages (workbench, sound) have no map. Every route that shows a map names its map id.
- One generic map-view route and scene covers any catalogued map without a bespoke route. C55 adds the primary generated-map type/size controls and preparation status; transient seeds do not join a hand-kept saved list.
- A cross-check test: every map that isn't retired has a route, and every route's map exists.
- No router is built here (that's hud-chrome's firewall); hud-chrome consumes `listMaps` later.

## API seam
`MainMenu.tsx`, `apps/battle-lab` listing, `web/scene.mjs:56-67`.

## What the human can run or see
The menu and `/labs` look the same, now generated.

## Verification
- Cross-check test.
- A temp draft map appears only where its status allows.
- `scene -- --list` ids unchanged.

## Delegated to the implementer
Menu ordering; the generic route's path. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Scene ids; the menu scene.

## Feedback that would change this slice
A listing that makes maps hard to find changes metadata presentation while keeping the catalogue as the sole source.
