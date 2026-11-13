# 09 · The Armory's own appearance library

Status: done 2026-11-12 (sha in the next spec commit, or the lane report).

Map row: L6, plus the library half of L5. No dependencies.

## Contract

The Armory owns a library that holds two things: the yard and the current unit.
Switching drops the previous unit, and a stale download never installs. A
failure rejects to the caller and never reaches `appResources`.

## Seam

`apps/battle-lab/src/armory/armoryAppearances.ts`:

```ts
export class ArmoryAppearances {
  constructor(fetcher?: Fetch);
  /** Resolves for the newest call only; earlier calls reject with AbortError and never install. */
  show(s: { kind: string | null; faction: Faction; scenery: ReadonlySet<string> }): Promise<InstalledAppearances>;
  dispose(): void;
}
```

- **One library per call.** Each `show` builds a fresh `AppearanceLibrary`
  (`packages/scene-assets/src/loader.ts:131`). It loads `wearing` set to the
  kind's appearances, `factions` set to `{faction}`, and `scenery` set to the
  yard's props.
- **The yard downloads once.** A fetch memo, shared across selections, keeps
  the yard's bytes and the catalog's bytes.
- **The shared library is left alone.** The Armory never touches
  `gameAppearances.ts`' module library or `useGameAppearances`. That hook
  refuses through `appResources` on error (`:241-248`), and its selection key
  would thrash the backdrop (`:156`).
- **Measurement.** Browsing all units through the shared library would hold
  about 2.1 GiB decoded, which is why this slice exists (M5).

## The human runs or sees

Nothing visible. Slice 11 measures switch time.

## Verification

Write these first, in `web/tests/armoryAppearances.test.ts`, with
`memoryFetch`:
- `switching installs only the newest unit and the yard`;
- `a stale download never installs`, with resolutions arriving out of order;
- `a failing unit rejects and app resources stay healthy`, meaning
  `appResources.error()` stays null;
- `the yard is fetched once across switches`.

## Delegated to the implementer

- The scope of the memo.
- Whether `AppearanceLibrary` gains a narrowing method instead of building a
  fresh instance each time. Choose the narrowing method only if it is simpler,
  keep one owner, and record the choice.

## Must stay green

- `sceneAppearances.test.ts`.
- `appResources.test.ts`.
- `transport.test.ts`.
- The download-budget tests.

## Feedback that would change it

None; this slice isn't visible.
