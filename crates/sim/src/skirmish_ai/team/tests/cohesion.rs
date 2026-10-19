use super::*;

#[test]
fn cohesion_counts_a_gathered_group_and_rejects_a_spread_group() {
    let gathered = [[95.0, 100.0], [100.0, 105.0], [105.0, 95.0]];
    let spread = [[95.0, 100.0], [100.0, 105.0], [500.0, 500.0]];
    assert_eq!(cohesion_sample(&gathered, [100.0, 100.0], 20.0), [3, 3]);
    assert_eq!(cohesion_sample(&spread, [100.0, 100.0], 20.0), [2, 3]);
    assert!(!group_ready(&spread, [100.0, 100.0], 20.0));
}
