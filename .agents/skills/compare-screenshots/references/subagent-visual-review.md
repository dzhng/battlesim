# Subagent Visual Review

Use this when history or prior conclusions could bias the main agent's visual
judgment.

## Spawn Config

- `agent_type`: `default`
- `fork_context`: `false`
- Attach the two screenshots as `local_image` items.
- For an approved-design comparison, identify `Reference` and `Candidate` and
  supply the intended design requirements. For an open preference comparison,
  use `Image A` and `Image B` without declaring a target.
- Supply full frames and matched detail crops including surrounding space.
  Withhold previous verdicts, implementation details and the author's preferred
  answer; do not withhold the target needed to judge fidelity.

## Prompt

```text
You are doing an unbiased visual review of two screenshots for the same visual target. You have no prior context.

Compare Image A and Image B. Report:

1. Whether they appear to show the same viewport/state/content.
2. Major visible differences in camera/view, layout, content, missing details,
   labels/text, icons, color, lighting, depth/layering, clipping, artifacts,
   readability, or style.
3. Inspect every changed effect at all four boundaries, including its full fade
   beyond the component. Compare extent relative to visible anchors on each
   side. Check brightness, color, sharpness and continuity of every foreground
   feature it crosses. Report local discrepancies even if everything is legible.
4. Give a per-feature reference observation → candidate observation →
   pass/fix/uncertain record; request closer crops when evidence is insufficient.
5. A concise verdict on whether the images preserve the intended visual
   relationship or need another pass.

When an approved reference is identified, judge fidelity to it. Otherwise do not
assume either image is the target. Overall attractiveness does not excuse a
local mismatch. Judge only from visible pixels and supplied design requirements.
```

## How To Use The Result

- Treat the subagent result as independent evidence about which image is less
  wrong, not a replacement for metrics or your own inspection — and not a vote
  for whichever image is the baseline.
- If the subagent flags wrong camera, mismatched state, missing content, or
  visible artifacts, fix capture/rendering quality before judging the rest.
- Quote the subagent verdict in the working notes when it changes or confirms
  the next implementation target.
