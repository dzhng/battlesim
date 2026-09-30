#[allow(dead_code)]
mod road_graph;
use road_graph::{Point, Route, Segment};
fn word(out: &mut Vec<u8>, v: u64) {
    out.extend(v.to_le_bytes());
}
fn record(out: &mut Vec<u8>, r: Option<Route>) {
    match r {
        None => out.push(0),
        Some(r) => {
            out.push(1);
            word(out, r.points.len() as u64);
            for p in r.points {
                word(out, p.0.to_bits());
                word(out, p.1.to_bits());
            }
            word(out, r.cost.to_bits());
            word(out, r.nodes as u64);
            word(out, r.relaxed as u64);
        }
    }
}
fn exercise() -> Vec<u8> {
    let mut out = Vec::new();
    for distance in [4999.999, 5000.0, 5000.001] {
        out.push(road_graph::auto_road_leg(Point(0.0, 0.0), Point(distance, 0.0), 5000.0) as u8);
    }
    for scale in [1.0, 100.0] {
        let roads = [
            Segment {
                a: Point(0.0, 0.0),
                b: Point(100.0 * scale, 0.0),
            },
            Segment {
                a: Point(50.0 * scale, -50.0 * scale),
                b: Point(50.0 * scale, 50.0 * scale),
            },
        ];
        for reverse in [false, true] {
            for blocked in [false, true] {
                let (a, b) = (Point(scale, 0.0), Point(50.0 * scale, 49.0 * scale));
                let (a, b) = if reverse { (b, a) } else { (a, b) };
                record(
                    &mut out,
                    road_graph::plan(&roads, a, b, 2.0 * scale, |a, b, road| {
                        if blocked && a.0.min(b.0) < 51.0 * scale && a.0.max(b.0) > 49.0 * scale {
                            None
                        } else {
                            Some(a.distance(b) / if road { 12.0 } else { 6.0 })
                        }
                    }),
                );
            }
        }
    }
    let roads = [Segment {
        a: Point(0.0, 0.0),
        b: Point(20000.0, 0.0),
    }];
    record(
        &mut out,
        road_graph::plan(
            &roads,
            Point(500.0, 100.0),
            Point(19500.0, 100.0),
            256.0,
            |a, b, road| Some(a.distance(b) / if road { 12.0 } else { 6.0 }),
        ),
    );
    out
}
static RESULT: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
#[no_mangle]
pub extern "C" fn run() -> usize {
    let data = exercise();
    let len = data.len();
    *RESULT.lock().unwrap() = data;
    len
}
#[no_mangle]
pub extern "C" fn result_ptr() -> *const u8 {
    RESULT.lock().unwrap().as_ptr()
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    for b in exercise() {
        print!("{b:02x}");
    }
    println!();
}
