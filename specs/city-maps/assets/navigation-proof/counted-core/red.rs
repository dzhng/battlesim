#[path = "reference_graph.rs"]
mod reference;
fn main() {
    let roads = [reference::Segment {
        a: reference::Point(0.0, 0.0),
        b: reference::Point(100.0, 0.0),
    }];
    let mut budget = 1;
    // Rejected wrapper: one debit hides endpoint discovery and complete graph search.
    let result = if budget > 0 {
        budget -= 1;
        reference::plan(
            &roads,
            reference::Point(1.0, 1.0),
            reference::Point(99.0, 1.0),
            2.0,
            |a, b, _| Some(a.distance(b)),
        )
    } else {
        None
    };
    assert!(
        result.is_none(),
        "one credit cannot discover both endpoints and complete a nontrivial route"
    );
    assert_eq!(budget, 0);
}
