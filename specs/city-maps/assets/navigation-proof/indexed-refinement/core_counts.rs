#[allow(dead_code)]
mod road_graph;
use road_graph::{Point, Segment};
fn main() {
    for n in [64, 256, 1024] {
        let roads: Vec<_> = (0..n)
            .map(|i| Segment {
                a: Point(0.0, i as f64 * 18.0),
                b: Point(20000.0, i as f64 * 18.0),
            })
            .collect();
        let r = road_graph::plan(
            &roads,
            Point(1.0, 0.0),
            Point(19999.0, 0.0),
            1.0,
            |a, b, _| Some(a.distance(b)),
        )
        .unwrap();
        println!(
            "roads={n} pair_tests={} query_nodes={} primitive_bounds={} graph_nodes={} relaxed={}",
            r.index.pairs, r.index.nodes, r.index.primitives, r.nodes, r.relaxed
        );
        assert_eq!(r.index.pairs, 0);
    }
    let mut roads = Vec::new();
    for i in 0..=40 {
        let p = i as f64 * 500.0;
        roads.push(Segment {
            a: Point(0.0, p),
            b: Point(20000.0, p),
        });
        roads.push(Segment {
            a: Point(p, 0.0),
            b: Point(p, 20000.0),
        });
    }
    let r = road_graph::plan(
        &roads,
        Point(1.0, 10000.0),
        Point(19999.0, 10000.0),
        1.0,
        |a, b, _| Some(a.distance(b) / 12.0),
    )
    .unwrap();
    println!(
        "grid_roads={} pair_tests={} query_nodes={} primitive_bounds={} graph_nodes={} relaxed={}",
        roads.len(),
        r.index.pairs,
        r.index.nodes,
        r.index.primitives,
        r.nodes,
        r.relaxed
    );
    assert_eq!(r.index.pairs, 41 * 41);
}
