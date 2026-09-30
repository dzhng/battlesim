# Map acquisition core evidence

This checkpoint proves the pure contract resolver and the compiler's saved-source
record. It does not complete C09/C60: no fixture moved and no native filesystem,
browser HTTP/WASM, asset-tool adapter or catalogue metadata was implemented.
Production scenarios still use their existing factories.

`receipt.json` pins the original source revision, evidence hashes and claim bounds.
The red logs precede their corresponding corrections; the final resolver and CLI
logs each contain the focused complete test result. The CLI test writes real files
and resolves them through the common contract, while its existing frozen stdout
corpus remains unchanged. The single-template mutant deliberately ignores the
selection; its refusal proves why one shared physical library needs explicit IDs.
Its SHA receipt records exact restoration of that intermediate source, not the
later final source hash. All output lines and original paths are retained. Repeated
terminal blank newlines are removed where necessary for the whitespace gate;
`receipt.json` records both original and stored hashes for each such log.

The immutable producer inventory is a migration baseline. Recover exact source
bytes with `git show <baseline_revision>:<path>` and check their recorded SHA256.
`probe-inventory.py` performs that check and compares the original route registry
with original scene modules. The physical-source summary and discovery logs are
observations at that revision, not a second live registry or schema.

The resolver proves typed content identity, catalogue selection/materialization,
authored IDs, header validity and the caller's narrow part/bay allowance. Nonempty
shape admission must consume C72's shared owner. C33 still owns complete
rules/body/tree/raster/resource preparation. Receipt syntax does not authenticate
an unavailable historical revision, recipe or art source.

Reproduce the narrow gates with the worktree's isolated `CARGO_TARGET_DIR`:

```sh
cargo test -p contract --test maps
cargo test -p mapgen --test compiler
cargo clippy -p contract -p mapgen --all-targets -- -D warnings
python3 specs/city-maps/assets/map-acquisition/probe-inventory.py
```

No new WASM/browser parity claim is made by this checkpoint. The configured Codex
CLI review was unavailable; the coordinating agent's independent source review
is recorded separately. Original native/WASM/compiler evidence stays unchanged.
