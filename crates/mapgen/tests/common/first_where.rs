/// The first seed of `candidates` whose generated map `wanted` accepts, with
/// that map: for a claim about a kind of map not every seed makes. A seed
/// refused on the way must be refused by name, as `admitted` holds it.
pub fn first_where<P>(
    candidates: impl IntoIterator<Item = u64>,
    mut generate: impl FnMut(u64) -> Result<P, Vec<mapgen::Diagnostic>>,
    wanted: impl Fn(&P) -> bool,
) -> Option<(u64, P)> {
    for seed in candidates {
        match generate(seed) {
            Ok(made) if wanted(&made) => return Some((seed, made)),
            Ok(_) => {}
            Err(errors) => assert!(
                errors
                    .iter()
                    .all(|error| error.code == mapgen::DiagnosticCode::GenerationFailed),
                "seed {seed}: {errors:?}"
            ),
        }
    }
    None
}
