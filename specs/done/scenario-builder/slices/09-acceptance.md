# 09 — Mixed-player, replay, and closeout

## Contract unlocked

The feature proves the complete user journey and the final chosen side topology in one fixture-free browser scenario.

## Acceptance run

1. Open the tactical scenario builder.
2. Load or create a resolved map snapshot.
3. Create multiple player slots with factions, sides, and controller requests.
4. Place units owned by those slots.
5. Add one bundled user audio file and one advanced script.
6. Download the self-contained package.
7. Start a fresh browser context and Upload it.
8. Start normal play with one active human, one active AI, one empty human/passive slot, and one passive neutral slot (or the accepted arbitrary-side equivalent).
9. Switch the human slot to active AI at a deterministic tick.
10. Verify observers/publications never expose hidden enemy truth and commands cannot cross slot ownership.
11. Finish through scripted victory or the selected economy result.
12. Export/import the replay explicitly and compare every tick digest/accepted command outcome.

## Verification

Add one registered browser scene and focused native/WASM paired records. Run the narrowest checks while implementing, then run `bun run check` and `bun run verify` once at closeout. Run `review` before declaring the feature complete; the visual slices retain `game-ui`, `compare-screenshots`, `screenshot-critique`, and `preview-shots` gates.

## Failure meaning

Any fixture lookup, hidden persistence, non-deterministic controller/script result, cross-slot command acceptance, hidden-state leak, or replay mismatch fails the feature. Do not loosen the requirement to make the scene pass; reslice the owner that moved.

## Human review

Open the final builder and in-battle shots with `preview-shots` for a non-blocking review window. If no response arrives, record the evidence-based decision and close the shots before closeout.

