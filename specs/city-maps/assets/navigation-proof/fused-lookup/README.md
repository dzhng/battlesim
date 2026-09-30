# One scratch lookup per relaxation

**Verdict: retain the lookup correction; full SA2 work gate remains red.**
The existing scratch owner checks a tentative cost, runs the existing optional bound
only for a fresh/improving label, and stores its parent through one retained hash-table
entry. It preserves the original strict comparison, including first-visit NaN/Infinity
behavior, and keeps queue priority, neighbor order, heuristic arithmetic and scheduling.
The three scratch arrays and their generation lifetime are unchanged; no derived cache
or public seam was added.

The frozen dense oracle matches 12,076 complete cases. All 1,800 original village tick
digests match, ending at `3c2f8cd1ccdd54e2`; the replay and 15 narrow navigation tests pass.
Root independently audited the whole source diff and scratch-generation context without
a finding. The CLI review was rejected before analysis by its configured model endpoint;
it is recorded as unavailable, with no global setting change.

## Matched resource proof

Actual production resource binaries were built serially at the same source paths with
the same dependency lock, workspace release profile and thin LTO. The baseline source
was restored only for its build, then the candidate restored and built. Source/compiled
identities and a zero-context reconstruction patch are retained; apply it with
`git apply --unidiff-zero`. Twelve alternating serial processes per arm preserve all
192 original route outputs. Root, C01 and Ground released their work before timing.

Median eight-route instructions fall from 7.6936 to 7.0301 billion, an 8.63% reduction.
Per-input median route time improves about 6–8%. Requested peak heap and reported retained
capacities remain unchanged apart from the candidate executable name's one extra argument
byte. Scratch retains 16 bytes per sample (16,384 bytes per 1,024-sample tile), with the
allocator reporting the actual high water. Any vacant-entry table reservation is included.

Candidate per-input empirical p95/max route times range from 28.06 to 43.22 ms. All eight
exceed the 16 ms target; seven exceed 33 ms. Twelve samples give a p95 equal to the sample
maximum, not an asserted population tail bound. Construction is reported separately;
initial Shortest scratch is cold and later searches retain capacity. Full 12/15/18 km
bridge arms remain unrun. This corrects repeated work without proving bounded scale work.

## Remaining work

The next owner needs a matched count/instruction attribution before another change.
Queue comparisons, exceptional-cell lookup and bound work are possibilities to measure,
not conclusions from this proof. The uniform scalar quotient and heuristic-cache rejection
remain separate evidence; no new route/tick policy or graph architecture is selected.
