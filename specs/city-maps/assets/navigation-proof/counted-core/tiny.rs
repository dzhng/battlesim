#[path = "core.rs"]
mod core;
#[path = "intent.rs"]
mod intent;
#[path = "topology.rs"]
mod topology;
use core::{Physical, Request, Status};
use std::sync::Arc;
use topology::{fold, Point, Prepared, Segment};
#[derive(Clone, Copy)]
struct Toy {
    directed: bool,
    block: bool,
}
struct Check {
    a: Point,
    b: Point,
    road: bool,
    done: bool,
}
impl Physical for Toy {
    type Job = Check;
    fn identity(&self) -> u64 {
        fold(self.directed as u64, self.block as u64)
    }
    fn begin(&self, a: Point, b: Point, road: bool) -> Check {
        Check {
            a,
            b,
            road,
            done: false,
        }
    }
    fn poll(&self, j: &mut Check, budget: &mut usize) -> Option<Option<f64>> {
        if *budget == 0 {
            return None;
        }
        *budget -= 1;
        j.done = true;
        Some(
            if (self.directed && !j.road && j.a == Point(99.0, 1.0))
                || (self.block && j.road && j.a.0.min(j.b.0) < 50.0 && j.a.0.max(j.b.0) > 50.0)
            {
                None
            } else {
                Some(j.a.distance(j.b))
            },
        )
    }
    fn digest(&self, j: &Check) -> u64 {
        fold(
            topology::point_hash(j.a),
            fold(
                topology::point_hash(j.b),
                fold(j.road as u64, j.done as u64),
            ),
        )
    }
}
fn finish(job: &mut Request<Toy>, budget: usize) -> (Vec<Point>, Vec<u64>) {
    let mut trace = Vec::new();
    for _ in 0..100000 {
        let mut credit = budget;
        let before = job.work;
        let result = job.poll(&mut credit);
        assert_eq!(job.work - before, budget - credit);
        assert!(job.work - before <= budget);
        assert_eq!(
            job.digest(),
            job.reference_digest(),
            "incremental semantic digest"
        );
        trace.push(job.digest());
        if result != Status::Pending {
            return (job.route().collect(), trace);
        }
    }
    panic!("request must make finite progress")
}
#[test]
fn one_credit_does_not_complete_query_search_or_refinement() {
    let p = Prepared::prepare(&[Segment {
        a: Point(0.0, 0.0),
        b: Point(100.0, 0.0),
    }]);
    let mut job = Request::new(
        p,
        Toy {
            directed: true,
            block: false,
        },
        Point(1.0, 1.0),
        Point(99.0, 1.0),
        2.0,
        1.0,
    );
    let mut one = 1;
    assert_eq!(job.poll(&mut one), Status::Pending);
    assert_eq!(one, 0);
    let (route, _) = finish(&mut job, 1);
    assert_eq!(
        route,
        vec![
            Point(1.0, 1.0),
            Point(1.0, 0.0),
            Point(99.0, 0.0),
            Point(99.0, 1.0)
        ]
    );
    assert_eq!(job.actual_cost, 100.0);
}
#[test]
fn two_requests_share_topology_without_sharing_labels() {
    let p = Prepared::prepare(&[Segment {
        a: Point(0.0, 0.0),
        b: Point(100.0, 0.0),
    }]);
    let mut a = Request::new(
        p.clone(),
        Toy {
            directed: true,
            block: false,
        },
        Point(1.0, 1.0),
        Point(99.0, 1.0),
        2.0,
        1.0,
    );
    let mut b = Request::new(
        p.clone(),
        Toy {
            directed: false,
            block: true,
        },
        Point(1.0, 1.0),
        Point(99.0, 1.0),
        2.0,
        1.0,
    );
    assert!(Arc::ptr_eq(a.prepared(), b.prepared()));
    for _ in 0..100000 {
        for j in [&mut a, &mut b] {
            let mut credit = 3;
            j.poll(&mut credit);
            assert_eq!(j.digest(), j.reference_digest());
        }
        if a.status() != Status::Pending && b.status() != Status::Pending {
            break;
        }
    }
    assert_eq!(a.status(), Status::Ready);
    assert_eq!(b.status(), Status::CorridorDeclined);
    assert_eq!(a.actual_cost, 100.0);
    assert!(b.searches > 1);
    assert!(b.retained().3 > 0);
}
#[test]
fn complete_trace_replays_and_quanta_preserve_output() {
    let roads = [
        Segment {
            a: Point(0.0, 0.0),
            b: Point(100.0, 0.0),
        },
        Segment {
            a: Point(50.0, -50.0),
            b: Point(50.0, 50.0),
        },
    ];
    let p = Prepared::prepare(&roads);
    let make = || {
        Request::new(
            p.clone(),
            Toy {
                directed: false,
                block: false,
            },
            Point(1.0, 1.0),
            Point(51.0, 49.0),
            2.0,
            1.0,
        )
    };
    let mut a = make();
    let mut b = make();
    let mut c = make();
    let (ra, ta) = finish(&mut a, 7);
    let (rb, tb) = finish(&mut b, 7);
    let (rc, _) = finish(&mut c, 1);
    assert_eq!(ra, rb);
    assert_eq!(ta, tb);
    assert_eq!(ra, rc);
    assert_eq!(a.actual_cost.to_bits(), c.actual_cost.to_bits());
}
