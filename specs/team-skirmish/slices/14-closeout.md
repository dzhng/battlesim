# Slice 14: 5v5 AI-versus-AI closeout

**Decisions:** D27, D28, L13. **Needs:** every other slice.

## What happens

1. **Behaviour.** Run 5v5 tournament matches across the skirmish corpus. Check that:
   - matches finish;
   - each AI fights as one group in its lane;
   - several objectives change hands at the same time;
   - requests for help and pushes appear;
   - `ai_receipts()` shows zero refused orders.
2. **Frame time.** Run a 5v5 all-AI benchmark battle in the browser at the unit cap, up to 300 units. Pick a loose frame-time tripwire, never a target. Watch and listen for the quiet limits (L13: 24 muzzle lights, 1024 pending sounds) and record what you see and hear.
3. **Native.** Confirm that item 10 of [native gaps](../../native-gaps/README.md) still describes what native lacks.
4. **Full run.** Run the full Rust and browser suite once.
5. **Close.** Run `review`, then archive the spec with `close-spec`.

## Done when

Both 5v5 runs are recorded in `throwaway/`, with their summary in the archived spec. The full suite passes, and the spec is closed.
