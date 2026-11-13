//! Smoke in the publication (D12, D13): the clouds a side sees and its own
//! units' charges travel the packed record and read back as the battle
//! observed them, and a cloud's record never says who popped it.
use contract::smoke::SmokeCloud;
use sim::publication::reader::{self, Observation, Reader};
use sim::publication::{self, Publisher};

use super::sight::{battle_on, bloomed, facing, open_map, rules_with, tough};
use super::*;

/// One side's stream, read back as a client reads it.
struct Stream {
    side: Side,
    publisher: Publisher,
    reader: Reader,
}

impl Stream {
    fn new(b: &Battle, side: Side) -> Self {
        Stream {
            side,
            publisher: Publisher::new(),
            reader: Reader::new(&publication::layout_json(b)).unwrap(),
        }
    }

    fn read(&mut self, b: &Battle) -> Observation {
        let words: Vec<u32> = self
            .publisher
            .publish(b, self.side)
            .unwrap()
            .iter()
            .map(|w| w.to_bits())
            .collect();
        self.reader.decode(&words).unwrap().expect("one stream")
    }
}

/// `read` is `cloud` as the record carries it: every number a float32.
fn same_cloud(read: &reader::SmokeCloud, cloud: &SmokeCloud) -> bool {
    let f = |v: f64| v as f32 as f64;
    let (r, s) = (&read.spec, &cloud.spec);
    read.id == f64::from(cloud.id)
        && read.centre == cloud.centre.map(f)
        && read.popped == cloud.popped as f64
        && [
            r.radius_m,
            r.height_m,
            r.life_s,
            r.bloom_s,
            r.fade_s,
            r.depth_per_m,
            r.concealment,
        ] == [
            s.radius_m,
            s.height_m,
            s.life_s,
            s.bloom_s,
            s.fade_s,
            s.depth_per_m,
            s.concealment,
        ]
        .map(f)
        && [r.bloom_steps, r.fade_steps] == [f64::from(s.bloom_steps), f64::from(s.fade_steps)]
        && *r.look == *s.look
}

/// The read-back observation carries the side's clouds and its units'
/// charges exactly as the battle observed them.
fn assert_reads_back(b: &Battle, side: Side, read: &Observation) {
    let frame = b.observe(side);
    assert_eq!(read.smoke_clouds.len(), frame.smoke_clouds.len());
    for (r, c) in read.smoke_clouds.iter().zip(&frame.smoke_clouds) {
        assert!(same_cloud(r, c), "tick {}: {r:?} vs {c:?}", b.tick());
    }
    let charges: Vec<_> = frame.own.iter().map(|u| u.smoke.map(f64::from)).collect();
    let read_charges: Vec<_> = read.own.iter().map(|u| u.smoke).collect();
    assert_eq!(read_charges, charges, "tick {}", b.tick());
}

#[test]
fn clouds_and_charges_read_back_through_the_publication_as_they_come_and_go() {
    let mut b = Battle::new(
        &setup(json!([
            unit("blue", TANK, [100.0, 150.0]),
            unit("blue", SQUAD, [300.0, 150.0]),
            unit("blue", "test_tank", [500.0, 150.0]),
        ])),
        1,
    );
    let mut stream = Stream::new(&b, Side::Blue);
    let (tank, squad) = (smoke(TANK).cloud, smoke(SQUAD).cloud);
    let (mut most, mut read) = (0, None);
    send(&mut b, 1, pop(&[0]), false);
    for tick in 0..seconds(tank.life_s.max(squad.life_s + 5.0)) + 2 {
        if tick == seconds(5.0) {
            send(&mut b, 2, pop(&[1]), false);
        }
        b.step();
        let now = stream.read(&b);
        assert_reads_back(&b, Side::Blue, &now);
        most = most.max(now.smoke_clouds.len());
        read = Some(now);
    }
    assert_eq!(most, 2, "both clouds stood at once");
    let read = read.unwrap();
    assert!(read.smoke_clouds.is_empty(), "both clouds are gone");
    let charges: Vec<_> = read.own.iter().map(|u| u.smoke).collect();
    assert_eq!(
        charges,
        [
            Some(f64::from(smoke(TANK).capacity - 1)),
            Some(f64::from(smoke(SQUAD).capacity - 1)),
            None,
        ],
        "each carrier spent one charge; a unit without smoke has none"
    );
}

/// Red watches from the open while blue's tank pops smoke: what red reads.
fn red_watches(rules: Value, blue: Value) -> Observation {
    let mut units = blue.as_array().unwrap().clone();
    units.push(facing(
        "red",
        "test_tank",
        [380.0, 150.0],
        std::f64::consts::PI,
        "return_fire_only",
    ));
    let mut b = battle_on(rules, open_map(), json!(units), 1);
    let popper = units.len() as u32 - 2;
    send(&mut b, 1, pop(&[popper]), false);
    run(&mut b, bloomed(TANK));
    Stream::new(&b, Side::Red).read(&b)
}

#[test]
fn a_side_that_sees_only_a_clouds_rim_reads_the_cloud_but_not_who_popped_it() {
    let read = red_watches(
        rules_with(&tough()),
        json!([facing(
            "blue",
            TANK,
            [200.0, 150.0],
            0.0,
            "return_fire_only"
        )]),
    );
    assert_eq!(read.smoke_clouds.len(), 1, "red reads the cloud");
    assert!(
        read.identified.is_empty(),
        "but identifies no unit: the tank is inside it"
    );
    // Another unit of another kind pops the same cloud at the same place:
    // red reads the same record, so the record carries neither.
    let mut patches = tough();
    let carried = serde_json::to_value(smoke(TANK)).unwrap();
    patches.push((
        "units",
        "test_tank",
        json!({ "capabilities": { "smoke": carried } }),
    ));
    let other = red_watches(
        rules_with(&patches),
        json!([
            facing("blue", "test_rifle", [20.0, 20.0], 0.0, "return_fire_only"),
            facing("blue", "test_tank", [200.0, 150.0], 0.0, "return_fire_only"),
        ]),
    );
    assert_eq!(other.smoke_clouds.len(), 1);
    let record = |o: &Observation| serde_json::to_value(&o.smoke_clouds).unwrap();
    assert_eq!(record(&other), record(&read));
}
