use crate::common::jet::{JetField, Track};

#[test]
fn a_placed_jet_records_one_sample_per_tick() {
    let mut battle = JetField::new(3000.0)
        .jet("test_heli", [300.0, 1500.0], 0.0)
        .battle();
    let samples = Track::record(&mut battle, 60);
    assert_eq!(samples.len(), 60);
    assert_eq!(samples.last().unwrap().tick, 60);
}
