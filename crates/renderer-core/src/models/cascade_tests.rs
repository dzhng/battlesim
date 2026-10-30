use super::*;

#[test]
fn caster_routing_preserves_dynamic_and_filters_static_runs() {
    let run = |mask, caster| Run {
        row: 0,
        tier: 0,
        pass: CAST_PASS,
        caster,
        cascade_mask: mask,
        fog: Layer::of(0),
        held: None,
        first: 0,
        count: 1,
    };
    let runs = [
        run(15, Some(1)),
        run(1, Some(2)),
        run(6, Some(3)),
        run(0, None),
    ];
    let routed = |c| {
        runs.iter()
            .filter_map(|r| r.caster_for(c))
            .collect::<Vec<_>>()
    };
    assert_eq!(routed(0), [1, 2]);
    assert_eq!(routed(1), [1, 3]);
    assert_eq!(routed(2), [1, 3]);
    assert_eq!(routed(3), [1]);
}
