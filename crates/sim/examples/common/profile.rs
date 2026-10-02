//! Native report attribution over the production tick. Counter reads live in
//! the report, so ordinary simulation ticks have no clock or OS dependency.
use super::instructions::instructions;
use sim::battle::TickPhase;

const PHASES: [TickPhase; 10] = [
    TickPhase::Orders,
    TickPhase::Navigation,
    TickPhase::Movement,
    TickPhase::Flight,
    TickPhase::Sight,
    TickPhase::Fog,
    TickPhase::Learning,
    TickPhase::Weapons,
    TickPhase::Observation,
    TickPhase::Other,
];

#[derive(Default)]
pub struct Profile {
    totals: [u64; 10],
    maxima: [u64; 10],
    ticks: u64,
}

impl Profile {
    pub fn step(&mut self, battle: &mut sim::battle::Battle) {
        let mut previous = instructions();
        let mut tick = [0; 10];
        battle.step_profiled(|phase| {
            let now = instructions();
            let cost = now.zip(previous).map_or(0, |(a, b)| a.saturating_sub(b));
            tick[PHASES.iter().position(|p| *p == phase).unwrap()] += cost;
            previous = now;
        });
        self.ticks += 1;
        for (i, cost) in tick.into_iter().enumerate() {
            self.totals[i] += cost;
            self.maxima[i] = self.maxima[i].max(cost);
        }
    }

    pub fn print(&self, label: &str) {
        if self.ticks == 0 {
            return;
        }
        println!(
            "\n{label}: {} ticks; instructions include counter-read overhead",
            self.ticks
        );
        println!("| system | mean M instructions/tick | max M/tick | total G |");
        println!("|---|---:|---:|---:|");
        for (i, phase) in PHASES.iter().enumerate() {
            if instructions().is_some() {
                println!(
                    "| {phase:?} | {:.3} | {:.3} | {:.3} |",
                    self.totals[i] as f64 / self.ticks.max(1) as f64 / 1e6,
                    self.maxima[i] as f64 / 1e6,
                    self.totals[i] as f64 / 1e9
                );
            } else {
                println!("| {phase:?} | unavailable | unavailable | unavailable |");
            }
        }
    }
}

/// Wire costs read the serializer's published layout, including its headers.
pub struct Delivery {
    fog_index: usize,
    header_len: usize,
    group_count: usize,
    delivery_width: usize,
    payload_index: usize,
    prop_group: usize,
    corpse_group: usize,
    sum: [u64; 6],
    max: [u64; 6],
    records: u64,
}

impl Delivery {
    pub fn new(battle: &sim::battle::Battle) -> Self {
        let layout: serde_json::Value =
            serde_json::from_str(&sim::publication::layout_json(battle)).unwrap();
        let fields = layout["header"].as_array().unwrap();
        let group = |name: &str| {
            layout["groups"]
                .as_array()
                .unwrap()
                .iter()
                .position(|g| g["name"] == name)
                .unwrap()
        };
        Self {
            fog_index: fields.iter().position(|v| v == "fogFloats").unwrap(),
            header_len: fields.len(),
            group_count: layout["groups"].as_array().unwrap().len(),
            delivery_width: layout["groupDelivery"]["fields"].as_array().unwrap().len(),
            payload_index: layout["groupDelivery"]["fields"]
                .as_array()
                .unwrap()
                .iter()
                .position(|f| f == "floats")
                .unwrap(),
            prop_group: group("knownProps"),
            corpse_group: group("corpses"),
            sum: [0; 6],
            max: [0; 6],
            records: 0,
        }
    }

    pub fn publish(
        &mut self,
        publisher: &mut sim::publication::Publisher,
        battle: &sim::battle::Battle,
        side: contract::ids::Side,
    ) {
        let before = instructions();
        let record = publisher
            .publish(battle, side)
            .expect("publication admitted");
        let counted = instructions().zip(before).map_or(0, |(a, b)| a - b);
        let total = record.len() * 4;
        let fog = record[self.fog_index] as usize * 4;
        let mut at = self.header_len;
        let mut sizes = Vec::new();
        for _ in 0..self.group_count {
            let payload = record[at + self.payload_index] as usize;
            sizes.push((self.delivery_width + payload) * 4);
            at += self.delivery_width + payload;
        }
        let props = sizes[self.prop_group];
        let corpses = sizes[self.corpse_group];
        let ground = publisher.ground_patch_bytes();
        let values = [
            fog as u64,
            props as u64,
            corpses as u64,
            ground as u64,
            (total - fog - props - corpses - ground) as u64,
            counted,
        ];
        self.records += 1;
        for (i, value) in values.into_iter().enumerate() {
            self.sum[i] += value;
            self.max[i] = self.max[i].max(value);
        }
    }

    pub fn print(&self, label: &str) {
        println!(
            "\n{label}: {} publications; native packing excludes copy/decoder",
            self.records
        );
        println!("| payload | mean B/record | max B/record |");
        println!("|---|---:|---:|");
        for (i, name) in ["fog", "known props", "corpses", "ground", "other"]
            .iter()
            .enumerate()
        {
            println!(
                "| {name} | {} | {} |",
                self.sum[i] / self.records.max(1),
                self.max[i]
            );
        }
        if instructions().is_some() {
            println!(
                "packing mean {:.3} M instructions/record, maximum {:.3} M",
                self.sum[5] as f64 / self.records.max(1) as f64 / 1e6,
                self.max[5] as f64 / 1e6
            );
        } else {
            println!("packing instructions unavailable");
        }
    }
}

pub fn subscriptions(battle: &sim::battle::Battle) {
    let mut publisher = sim::publication::Publisher::new();
    for (label, side, reset) in [
        ("initial subscription", contract::ids::Side::Blue, false),
        ("unchanged same tick", contract::ids::Side::Blue, false),
        ("side switch", contract::ids::Side::Red, false),
        ("resubscription", contract::ids::Side::Red, true),
    ] {
        if reset {
            publisher.resync();
        }
        let mut sample = Delivery::new(battle);
        sample.publish(&mut publisher, battle, side);
        sample.print(label);
    }
}
