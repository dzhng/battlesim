# Reuse the implicit material costs for one plan

**Retain the repeated-work correction; full SA2 work remains red.**
The existing uniform-stencil certificate already selects the same implicit flat Cell.
Its cardinal and diagonal material costs depend only on that immutable cell, the fixed
Mobility and policy, and the two fixed step lengths. Compute these exact two f64 results
once on the plan stack. Exceptional stencils still use the existing cost owner.
No reciprocal multiplication or arithmetic reassociation is introduced: the edge half-sum,
g accumulation, Euclidean queue, tie/parent order, smoothing and immediate tick semantics
remain unchanged. There is no retained cache, invalidation contract or public schema change.

## Cause and parity

The separately instrumented preceding source counts about 2.3–3.2 million heap comparisons
and 418–568 thousand search material-cost calls per safe bridge route. Nearly all expanded
stencils are certified uniform. These are counts, not inferred percentages of runtime.
The disposable red/green cause assertion bounds cost calls by two per plan plus ordinary
exceptional expansion work. It fails on the preceding source and passes on this candidate.
The two startup calls appear in the prepare phase, before the probe switches to search;
the search assertion conservatively allows them as well. The actual material-call count
can be read in the raw phase records. Counter overhead and scratch-clone instructions are
not the production acceptance measurement. The original generator scripts and both derived source snapshots are proof-local. Their
input hashes pin the baseline/candidate variants; their original staging location is
checkout `throwaway/navigation-remaining-attribution`. Reproduction supplies the matching
source variant in an isolated checkout, without changing an integrated production tree.
They do not add production counters or hooks.

The untouched dense oracle matches 12,076 complete cases. All 1,800 frozen village tick
digests match, ending at `3c2f8cd1ccdd54e2`; replay and 15 narrow navigation tests pass.
Every one of 192 matched process route outputs also equals the preceding frozen corpus.
Root independently reviewed the complete source change without a finding. CLI review
failed before analysis because its configured model is unsupported at that endpoint;
there is no independent CLI verdict or global configuration change.

## Matched production evidence

Actual production resource binaries use the same source paths, dependency lock and
workspace release profile with thin LTO. The baseline navigation source is restored only
for its build and the candidate restored before its build. Protocol, source/binary hashes,
raw process records and the zero-context reconstruction patch are retained. Apply the
patch with `git apply --unidiff-zero`. Twelve alternating serial pairs ran after Root,
C01 and Ground explicitly released their measurement lanes; compiler work was held.

Median eight-route instructions fall from 6.9361 to 6.8524 billion, a 1.21% reduction.
Requested peak heap and retained capacities are unchanged apart from the candidate
executable argument's one additional byte. Two stack f64 values add no retained payload.
Median route times are essentially flat (changes from −0.22% to +1.10%). This earns a
small repeated-work correction, with no claimed latency improvement.

Candidate per-input empirical p95/max ranges from 28.97 to 56.60 ms. All eight exceed
16 ms and seven exceed 33 ms. Twelve samples give p95 equal to the sample maximum, not
a population tail assertion. Route timing excludes world/grid construction; the first
blue/red Infantry Shortest searches have cold scratch and later inputs retain capacity.
No larger bridge arm is admitted. The smaller current world residency is an integrated
terrain change; only this matched baseline/candidate comparison attributes navigation.

The remaining heap comparisons and sparse lookups are hypotheses for another separately
proved correction. These counts select no graph quotient, queue replacement or scheduling
change, and another constant factor alone does not prove full-size bounded work.
