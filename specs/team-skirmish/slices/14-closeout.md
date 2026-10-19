# Slice 14: 5v5 AI-versus-AI closeout

**Decisions:** D27, D28, L13. **Needs:** every other slice.

## What happens

1. **Behaviour.** Run 5v5 tournament matches across the skirmish corpus. Check that:
   - matches finish;
   - each AI fights as one group in its lane;
   - several objectives change hands at the same time;
   - requests for help and pushes appear;
   - `ai_receipts()` shows zero refused orders.
2. **Frame time: deferred.** The 300-unit browser benchmark and the quiet limits (L13) move to a future performance spec (user, 2026-10-19). This spec closes without them.
3. **Native.** Confirm that item 10 of [native gaps](../../native-gaps/README.md) still describes what native lacks.
4. **Full run.** Run the full Rust and browser suite once.
5. **Close.** Run `review`, then archive the spec with `close-spec`.

## Done when

The 5v5 behaviour run is recorded in `throwaway/`, with its summary in the archived spec. The full suite passes, and the spec is closed.
