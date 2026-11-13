# Armory: choices

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
