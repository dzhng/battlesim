# 03 — Voices tab

**Contract this unlocks:** the user can review, judge and regenerate voice takes in the sound workbench, with verdicts that bind to the bytes they judged.

## Seam

**`apps/sound-workbench/voiceStore.ts` → `VoiceWorkbenchStore`**
- Uses `FixturePublication(root, "voices")`. Add `voices` to the editor allowlist (`apps/fixture-publication/publication.ts:15`, `:107-111`).
- Snapshots hash **only** `voices.json` and `voices.lock.json`. `SoundWorkbenchStore` hashes every clip (`server.ts:86-101`); don't copy that.

**Endpoints**, under the existing local-only guard. Move that guard out of `soundWorkbenchMiddleware` into a shared `localOnly()` (`server.ts:225-244`).

| Method | Path | What it does |
|---|---|---|
| `GET` | `/__sound-workbench/voices/snapshot` | catalog, lock, and the counts per set |
| `POST` | `/__sound-workbench/voices/preview` | edit text or descriptions through FixturePublication's draft step |
| `POST` | `/__sound-workbench/voices/save` | save through FixturePublication |
| `POST` | `/__sound-workbench/voices/verdict` | `{take, verdict}`: the server runs `generate verdict` so the lock keeps one writer |
| `POST` | `/__sound-workbench/voices/generate` | `{scope}`: spawns `generate voices` as a child process |
| `GET` | `/__sound-workbench/voices/jobs` | progress lines from running jobs |

Hot-update suppression should cover `voices.json` and `voices.lock.json`, the same way it covers `sounds.json` (`server.ts:303`).

**`apps/sound-workbench/src/voices/VoicesTab.tsx`**, added next to the existing tabs (`SoundWorkbench.tsx:199-206`):
- **Left:** sets, with counts of unreviewed, flagged and bad takes.
- **Right:** one set's grid of voices × 29 triggers, with 4 chips per cell (steady/stressed × 2 takes). Flagged takes are marked.
- **Review queue, keyboard-driven.** Flagged takes play first, then one take per voice and trigger.

  | Key | Action |
  |---|---|
  | Space | replay |
  | J / K | next / previous |
  | A | accept |
  | X | mark bad |
  | R | mark bad and regenerate (bumps `retakes`, then calls `generate --take`) |
  | E | edit the line's text |

- **"Play row"** plays a voice's 29 lines back to back.
- **"Accept unflagged in set"** accepts in bulk. See `choices.md` → Open.
- **A take changes when its sha changes.** Its verdict then falls back to unreviewed.
- Audio plays from the served URL through the existing audition context (`audition.ts`). Don't route voices through `Library.tsx`.

## What the human sees
The Voices tab, with the T1 profile takes from slice 02.

## Verification
- `web/tests/soundWorkbenchServer.test.ts` is extended:
  - a verdict survives a reload;
  - a changed sha invalidates the verdict;
  - `generate` spawns with the right scope (stub the child process).
- A DOM test for the queue's keys.
- **Last check:** an unprimed [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md) of the tab, showing a set with flagged and accepted takes. This is a developer tool, so judge legibility and density, not game styling.

## Delegated to you
- layout details;
- colours;
- the progress display;
- paging.

## Must stay green
The existing workbench tabs and their server tests.

## Feedback that would change this slice
The user wants a different review flow, for example by trigger across all sets. That is a tab change only.

## Checkpoint (non-blocking)
Open the tab for the user, wait about 5 minutes, then proceed.
