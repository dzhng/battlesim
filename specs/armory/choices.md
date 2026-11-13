# Armory: choices

## Slice 07

- **Fade: 0.5 s, in and out.** A dropped loop fades over 0.5 s, and a new one
  ramps in over the same time (its `attack`). Why: a unit switch reads as a
  quick crossfade, well under the menu's 1.2 s bed fade, which is a
  screen-change pace. Alternative: reuse `MENU_FADE_S`, which drags on a list
  sweep. *Provisional*: slice 11 hears it.
- **Id range: open-ended, from 16 up.** The sink keys voices in a map, so the
  range needs no cap; each new show voice takes the next id. The bed (1) and
  the cue (3) sit below. Alternative: a fixed block of N ids, which adds a
  "too many loops" case that no unit has. *Sound.*
- **The bed restart clears by id.** `WebAudioSink.clear` takes an optional
  id filter, and the bed restart clears everything but show voices. Why: the
  restart has to cut the previous bed while it is still fading, and a stopped
  voice has already left the id map, so stopping slot 1 cannot reach it.
  Alternative: a second sink for show loops, which would need its own master,
  limiter and volume wiring. *Sound.*
- **Leaving the menu screen forgets the asked-for loops; mute keeps them.**
  `loading` fades them out and `other` cuts them with the rest of the menu
  mix. Returning to the menu plays nothing until a page asks again. Why: the
  loops belong to the page that asked, and that page is gone. Mute is a
  setting, so unmuting resumes what was asked for. *Sound.*
- **Readiness has one owner.** `SoundBank.prepared(name)` reports whether
  `get` can return a sound. `AppAudio` starts a loop only once its sound is
  prepared, and asks the bank for the missing ones. The bank already joins
  duplicate preparations. Alternative: a ready set in `AppAudio`, which
  duplicates the bank's own state. *Sound.*
- **Show loops ride the `units` bus with no low-pass, no reverb and no
  position.** The spec names the bus. The rest matches the bed: the sound is
  heard at the camera, not placed in a world. *Sound.*
- **Loops are keyed by sound name.** If a call names the same sound twice,
  the last entry wins. *Sound.*

## Slice 09

- **The constructor takes the unit catalog: `new ArmoryAppearances(units, fetcher?)`.**
  The spec's seam was `constructor(fetcher?)`, but a kind's art comes from
  `UnitCatalog.appearancesFor`. The catalog is the session's
  (`useSessionCatalog`), and taking it once keeps `show` to the spec's
  selection. Alternative: pass the worn appearance set into `show`, which
  pushes the kind-to-art mapping onto every caller. *Sound.*
- **`show` resolves `{ appearances, runtime }`, not the bare
  `InstalledAppearances`.** `runtime` is the selection's library as
  `RuntimeFiles` (its `catalog` and `contents`). `fieldedBattle` reads a
  unit's bundle contents through these, and they must come from the library
  that installed the art: the shared `gameRuntime` would answer from the
  wrong library. Alternative: a `runtime` getter for the newest library,
  which races a pending switch. *Sound*; slice 11 consumes it.
- **A fresh `AppearanceLibrary` per call, no narrowing method.** The loader
  is untouched, so it stays the one owner of loading, and an old selection's
  decoded art goes when its library is dropped. A narrowing method would add
  a second way to shrink a generation that only the Armory uses. *Sound.*
- **The memo keeps the newest selection's files.** It holds every file that
  the last selection to resolve read, as bytes, keyed by URL. After each
  answered call it is replaced by that call's files. The yard, the catalog
  and the template library are in every selection, so they download once,
  while the previous unit's bytes leave on the next switch. A failed or
  superseded call never replaces it, and no failed response is kept.
  Alternative: memoize only the yard's own URLs, which means asking the
  catalog which files belong to the yard. That duplicates
  `catalogLoadNames`. *Sound.*
- **A superseded call stops fetching.** Its next fetch rejects with
  AbortError, so a quick list sweep doesn't keep downloading units nobody
  will see. The call itself answers AbortError however its load ended.
  Fetches already in flight are not cancelled, because `Fetch` takes no
  signal. *Sound.*
- **The default fetcher is `countedFetch`,** so the Armory's downloads move
  the loading screen's counter as the battle's do. *Sound.*
- **The yard is the scenery appearance set, the loader's own `scenery`
  filter.** Slice 10 maps prop kinds to appearances (`propsDrawnBy`) before
  calling. *Sound.*
Decisions made during implementation where the spec was silent or where the
code argued for a different seam. Each gives the choice, why, the alternative,
and a verdict.

## Slice 01

- **`MENU_PAGES` rows are plain data (`id`, `path`, `title`, `tile`). Each
  page's contents are in `menu/MenuVisit.tsx` `CONTENTS: Record<MenuPageId,
  ComponentType>`, and they are eager, not `lazy`.**
  - Why:
    - `menuHref` reads the Skirmish path from the rows. A component in the
      row would make an import cycle: `battleLinks` → `pages` →
      `SkirmishPage` → `battleLinks`.
    - The browser scenes (`.mjs` under node) import the same paths.
    - A `lazy` page leaves the plate blank the first time it opens, because
      React throttles a Suspense reveal by about 300 ms.
    - The four pages were already in the entry bundle.
    - The `Record` is exhaustive, so a new row without contents fails to
      type-check.
  - Alternative: the slice's `page: LazyExoticComponent` in the row.
  - Verdict: provisional. Slice 08 adds `armory` to `CONTENTS`, as a `lazy`
    import under a `Suspense` if its bundle earns it.
- **`menuPageOf` lives in `router.tsx`, beside `resolvePage`, not in
  `pages.ts`.**
  - Why: its `null` (a lab or battle route) needs the lab route table
    (`ROUTES`, the dev editors, `/labs`). Putting it in `pages.ts` would need
    an import cycle or moving the lab table.
  - Alternative: `pages.ts` as the slice says.
  - Verdict: sound.
- **`left` is derived in the shell from the page changing.** When the address
  goes from a page to home, `left` is that page. Home's tile takes focus with
  `autoFocus`.
  - Why: Back, Escape and the browser's Back all focus the tile left, with no
    flag set at each call site.
  - Alternative: set `left` inside `back()` only.
  - Verdict: sound.
- **`fromMenu` is router location state, so it survives a refresh** (it lives
  in `history.state`). After a refresh, Back still steps back to the home entry
  it came from.
  - Alternative: drop it on refresh, which the slice allowed.
  - Verdict: sound.
- **`PageVisit` keeps keying on `location.key`.**
  - Why: slice 08's raw `history.replaceState` is invisible to the router, so
    a unit pick never remounts the page.
  - Verdict: sound.
- **`detected` means the graphics tier is settled:** detection wasn't needed,
  or it has finished. It is the same condition that gates the runtime
  prefetch.
  - Verdict: sound.
- **The shell's `LoadingTasks` still wraps only the backdrop.**
  - Why: no page reports loading yet.
  - Slice 11 must move the page inside it before `ArmoryStage` can report
    `useLabLoading("renderer", …)` to the shell.
  - Verdict: provisional.
- **`MainMenu.tsx` became `menu/Home.tsx`** (`git mv`), and the page modules
  sit beside it: `SkirmishPage`, `SettingsPage` and `DeveloperPage`. The
  Tutorial page's contents are the existing `Tutorial`, which the pause menu
  also opens.
  - Verdict: sound.
- **The developer list lost its disabled-entry branch** (`href: null`). No
  entry used it.
  - Verdict: sound.
- **`menuHref(null)` is `/`.** A replay's Cancel lands on home, because a
  replay names no map to set up.
  - Verdict: sound.
- **The `lobby` scene opens `/skirmish` directly**, without clicking the tile.
  The `ui` scene, the journey, `generated` and the jsdom tests still click it.
  - Verdict: sound.
- **`ui.mjs` gains one check per page loop:** the tile opens the page at its
  own address.
  - Verdict: sound.
- **`unit-roster.mjs` and `_appJourney.mjs` are unchanged.** The slice listed
  them for rewriting, but neither pins a menu address.
  - The journey opens Skirmish by its tile.
  - Its history steps land on home (the pause menu's Main menu link) or on the
    battle, never on a page that Back would leave.
  - Verdict: sound.
