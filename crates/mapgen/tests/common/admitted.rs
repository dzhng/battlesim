/// The first `count` seeds of `candidates` that generate, with what they
/// generated. Generation may refuse a seed by name, and the game then draws
/// another, so a claim about generated maps is a claim about admitted ones:
/// a fixed list of seeds would pin whichever happened to be admitted. A
/// refusal must be that named exhaustion, and rare: `count` are admitted
/// within twice as many candidates.
pub fn admitted<P>(
    candidates: impl IntoIterator<Item = u64>,
    count: usize,
    mut generate: impl FnMut(u64) -> Result<P, Vec<mapgen::Diagnostic>>,
) -> Vec<(u64, P)> {
    let mut found = Vec::new();
    let mut refused = Vec::new();
    for seed in candidates.into_iter().take(2 * count) {
        match generate(seed) {
            Ok(plan) => found.push((seed, plan)),
            Err(errors) => {
                assert!(
                    errors
                        .iter()
                        .all(|error| error.code == mapgen::DiagnosticCode::GenerationFailed),
                    "seed {seed}: {errors:?}"
                );
                refused.push((seed, errors));
            }
        }
        if found.len() == count {
            return found;
        }
    }
    panic!(
        "{} of {} candidates admitted: {refused:?}",
        found.len(),
        2 * count
    );
}
