# HUD, chrome and app flow (draft)

**Status:** a draft plan, not started. It begins after battle-look (archived in [`specs/done/battle-look/`](../done/battle-look/README.md)), with a [write-spec](../../.agents/skills/write-spec/SKILL.md) interview before any slice is cut. The items below are what the battle-look critiques and the user have raised; the interview turns them into a ladder.

## Goal

The game's chrome reads as a **game UI, not a B2B SaaS dashboard**, and the app moves between screens like a game: menu, (later) lobby, battle, results. It shouldn't reload the whole page between them.

## 1. App flow and routing (user, 2026-09-28)

Today (see `apps/battle-lab/src/router.tsx`):
- a tiny table-driven router: `apps/battle-lab/src/fixtures.json` lists each page `{id, route, describe}`, `ROUTES` maps each id to a lazily loaded page, and `LabRouter` matches `window.location.pathname` once at load;
- links are plain `<a href>`, so every screen change is a **full page reload**, which rebuilds the WebGPU device, the WebAssembly module and the appearance library;
- query parameters are parsed by hand, page by page (`?script=`, `?bundle=`, `?village`, `?inspect`);
- unknown paths fall back to the main menu, with no not-found page;
- Vercel serves `index.html` for client routes (`vercel.json` rewrite).

Wanted:
- **A history-based router with typed routes and typed parameters,** with in-app navigation and no page reloads.
- **Long-lived resources outlive screen changes:** the GPU device, the WebAssembly module, the appearance library and the audio context are owned by the app shell, not by a page.
- **A screen-flow model** (menu, lobby, battle, results) that the labs still fit into. The labs keep their one registry and their headless scene checks.
- A proper not-found page.
- Open question for the interview: adopt a small library (e.g. TanStack Router) or write a minimal typed router. The choice rests on bundle size, typed params and lazy loading.

## 2. Chrome design pass

The critiques in battle-look's 27e and 27f passes (see the ledger, `specs/done/battle-look/choices.md`) found that the **in-world layer reads as a game** but the **chrome reads as a dev console with a sci-fi tint**:
- **Top bar:**
  - the scenario picker and replay controls read as tooling;
  - three button styles sit in one row;
  - the sound checkbox and volume slider are browser-style controls;
  - the status is a run-on sentence.
- **Bottom command bar:**
  - mostly empty space;
  - small command glyphs and key chips;
  - very faint disabled tiles;
  - the `1/2` reach badge is explained only by its tooltip;
  - DEPLOY and PACK share the T key;
  - MOVE stays lit with nothing selected;
  - a full-colour ✋ emoji.
- **Unit card:**
  - weapon lines wrap mid-phrase;
  - the ⊕ and ⊘ state glyphs look alike;
  - icons and silhouettes are too small to set the tone.
- **Logs:** coordinates and tick numbers in the command log; placeholder text in the "Heard" log.
- **Type:** one monospace font everywhere; flat, outlined panels.
- **Main menu:** each whole entry card is clickable (fixed after battle-look); the rest of the menu needs the same pass.

References to aim at (and improve on): WARNO, Broken Arrow, ARMAPHRACT and sci-fi RTS HUDs. The holo-tactical language shipped in battle-look 27e is the starting point.

## Next Agent Prompt

This spec is a draft. Don't implement. When the user starts it, run the write-spec interview from the items above. Open with the routing seam (what owns the GPU device and assets across screens), then the chrome, one visual variable per slice, each with an unprimed screenshot-critique and compare-screenshots against the references.
