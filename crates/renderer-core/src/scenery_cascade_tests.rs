use super::*;

#[test]
fn cascade_routing_keeps_felled_all_cascade_and_camera_pass_separate() {
    let run = |key, pass, mask| Run {
        row: 0,
        tier: 0,
        pass,
        cascade_mask: mask,
        key,
        count: 1,
    };
    let runs = [
        run(1, CAST_PASS, 1),
        run(2, CAST_PASS, 2),
        run(3, CAST_PASS, 15),
        run(4, VIEW_PASS, 0),
    ];
    let routed = |pass, c| {
        runs.iter()
            .filter(|r| r.draws_in(pass, c))
            .map(|r| r.key)
            .collect::<Vec<_>>()
    };
    assert_eq!(routed(CAST_PASS, Some(0)), [1, 3]);
    assert_eq!(routed(CAST_PASS, Some(1)), [2, 3]);
    assert_eq!(routed(CAST_PASS, Some(3)), [3]);
    assert_eq!(routed(VIEW_PASS, None), [4]);
}
