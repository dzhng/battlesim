# 01 · Menu routes

Status: done 2026-11-12, commit 223994a2a.

Map rows: Q4, L13, plus P1 (the hard cutover). No dependencies.

## Contract

Every menu page is a URL path (K1). The menu shell sits above `PageVisit` and
never remounts across menu paths. The shell owns:
- the backdrop;
- the music bed;
- warm-up and detection;
- the loading cover and `revealed`.

Each page is its own visit inside it. `menuHref` emits `/skirmish?…`, and
`/?type=…` opens the bare menu. The tutorial media moves so `/tutorial` is
only a route (K2).

This slice adds **no** Armory tile. Slice 08 adds the tile and the page.

## Seam

- **`apps/battle-lab/src/menu/pages.ts`, new, the one owner of menu pages.**
  - `MENU_PAGES: readonly MenuPageRow[]`, where
    `MenuPageRow = { id; path; title; tile: boolean; page: LazyExoticComponent<ComponentType> }`.
  - `menuPageOf(pathname): MenuPageId | "home" | null`. `null` means a lab or
    battle route; an unknown path is `"home"`.
  - `screenForPath` (`router.tsx:114`) answers `"menu"` from `menuPageOf`, so
    it has no case list.
- **`apps/battle-lab/src/menu/MenuShell.tsx`.**
  - It takes over the shell part of `MainMenu.tsx:196-327`: detection, warm-up,
    the cover, `MenuBackdrop`, `LoadingTasks`, `revealed` and `shown`.
  - It exposes `useMenuShell(): { revealed; detected; left: MenuPageId | null; back(): void }`.
  - `back()` goes back in history when the page was entered from home (tiles
    navigate with `{ state: { fromMenu: true } }`), and otherwise replaces with
    `/`.
  - Home focuses `[data-page=<left>]` on mount. `left` lives in shell state.
- **`LabRouter` (`router.tsx:83`).**
  - It renders `<MenuShell><PageVisit><MenuPage/></PageVisit></MenuShell>` when
    `menuPageOf(path) !== null`, and the existing branch otherwise.
  - `LabPage`'s fall-through to the menu (`router.tsx:136`) is deleted, and so
    is `PAGES` in `MainMenu.tsx:145`.
  - `MainMenu.tsx` shrinks to the home tile list. Skirmish, Tutorial, Settings
    and Developer become page modules.
- **Navigation details.**
  - Tiles stay `<button data-page>` and navigate.
  - The Escape listener stays bubbling on `window`, so the Lobby's dropdown
    still eats Escape first (`lobby/Lobby.tsx:333`).
- **Shell rules.**
  - Visit-scoped continuations (`usePageVisitActive`) stay in pages only.
  - The shell never calls it (`navigation.tsx:28` throws outside a visit).
- **Addresses.** `menuHref` (`battleLinks.ts:126`) returns `/skirmish?…`, and
  only the Skirmish page reads `askedChoice`.
- **Fixtures.** The `lobby` fixture's route `"/"` becomes `"/skirmish"`. It
  keeps no `ROUTES` entry, so it stays a menu path.
- **The tutorial media.**
  - `git mv assets/runtime/tutorial assets/runtime/tutorial-media`;
  - update `Tutorial.tsx:43,45`, `web/tutorial-shots.mjs`'s output path and
    `assets/README.md`.
  - The menu media isn't bake output, so L1 doesn't hold it.

## The human runs or sees

Run `bun run dev`, then:
- open `/settings` directly, and Back lands on `/`;
- browser Back and Forward between pages;
- refresh `/skirmish?type=mixed&size=small&seed=42`, which pre-fills the lobby;
- open `/?type=mixed&seed=42`, which shows the bare menu.

## Verification

**Write tests (1)–(5) first, against today's code where they can fail.** The
numbers are Q4's.

1. **The backdrop survives.**
   - Test: `web/tests/menuRoutes.test.tsx` `the backdrop mounts once across every menu page`.
   - It mocks `MenuBackdrop` to count mounts, then walks home → skirmish →
     back → settings → developer.
   - The cover never shows again after `revealed`.
2. **Back and Escape.**
   - `back and escape return home and focus the tile left`.
   - `a page opened by address goes back by replacing with home`
     (`history.length` unchanged).
3. **The cutover (P1).**
   - `web/tests/battleVisit.test.tsx:68` expects `/skirmish` plus the query.
   - `web/tests/navigationBrowser.test.ts:52` expects Cancel to land on
     `/skirmish`.
   - `lobbyScreen.test.tsx:32-35,66,170` and `generated.mjs:302,713` are
     rewritten.
   - Add `an old /?type= address opens the bare menu`.
4. **Unknown paths.** They show the menu (`navigation.test.tsx:121`, extended).
5. **Music and warm-up.** Every `MENU_PAGES` path is `"menu"`. The test reads
   `MENU_PAGES`; it lists no paths.
6. **Screenshots.**
   - `web/scenes/ui.mjs` (`:367-431`): the `menu` and `menu-<entry>` baselines
     are **byte-identical**.
   - The page loops navigate by path but keep their `[data-page]` clicks.

**Rewrite the other pins:**
- `router.test.ts` (tiles are buttons);
- `navigation.test.tsx:109-114` (history ×2 lands on `/skirmish`);
- `navigationBrowser.test.ts:52-54`;
- `_appJourney.mjs`;
- `unit-roster.mjs`.

`uiCoverage.test.ts` `PINNED_PAGES` gains the page modules, and `ui.mjs` draws
each one.

**Verify `/tutorial`** on Vite dev, `vite preview` (`scene.mjs:223`) and the
Vercel rewrite (`vercel.json`). It must serve the SPA on all three.

**Visual gate.**
- **The variable is "nothing changes".** Crop: the full frame of `menu` and of
  each `menu-<entry>`.
- `compare-screenshots` against the pre-slice baselines must report identity.
- An identical image needs no critique.
- Any difference is a regression to fix, not a new baseline.

## Delegated to the implementer

- Module and file names under `apps/battle-lab/src/menu/`.
- Whether `PageVisit` keeps keying on `location.key`.
- Whether `fromMenu` survives a refresh. Dropping it on refresh is acceptable;
  Back then replaces with `/`.

## Must stay green

- The battle loading screen's Cancel lands on `/skirmish?…`
  (`routes/battle.tsx:218`).
- `/battle` stays `"loading"` for music.
- `/labs/**` canonicalisation.
- `PauseMenu`'s modal Tutorial, Settings, Sound and Diagnostics stay
  path-agnostic (`PauseMenu.tsx:128-163`).
- `pauseMenu.test.tsx` and `graphicsSettingsControls.test.tsx`.
- `_appJourney.mjs`'s single device, zero leftover buffers and one backdrop
  worker.

## Feedback that would change it

"Keep `/?type=` working" reverses P1, which only the user can reopen.

## Reslice if too big

1. Extract the shell first, with URLs unchanged (tests 1, 2 and 6).
2. Then the paths, the cutover and the media rename (tests 3, 4 and 5).
