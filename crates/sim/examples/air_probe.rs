//! SCRATCH probe (delete before handing off): plays an air-map encounter as
//! its lab does and prints each side's observation summary.
//! Usage: cargo run -p sim --example air_probe -- <spec.json>
use contract::command::CommandEnvelope;
use contract::ids::Side;
use contract::scenario::{EncounterDefinition, Rules};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::fixtures::{self, CatalogSet};

fn main() {
    let path = std::env::args().nth(1).expect("spec path");
    let spec: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let map_id = spec["map"].as_str().unwrap_or("air");
    let map = sim::maps::load(map_id).unwrap().definition;
    let encounter: EncounterDefinition = match &spec["encounter"] {
        Value::String(name) => sim::maps::encounter(map_id, name).unwrap(),
        inline => serde_json::from_value(inline.clone()).unwrap(),
    };
    let mut game = fixtures::game();
    game["catalog"] = Value::Array(fixtures::catalog_documents(CatalogSet::Test));
    let rules: Rules = serde_json::from_value(game).unwrap();
    let mut setup = encounter.on(map, rules);
    if !spec["skirmish"].is_null() {
        setup.skirmish = Some(serde_json::from_value(spec["skirmish"].clone()).unwrap());
    }
    let seed = spec["seed"].as_u64().unwrap_or(6);
    let mut b = Battle::new(&setup, seed);
    let ticks = spec["ticks"].as_u64().unwrap_or(600);
    let every = spec["every"].as_u64().unwrap_or(30);
    let mut seq = [0u64; 2];
    let empty = vec![];
    let commands = spec["commands"].as_array().unwrap_or(&empty);
    for _ in 0..ticks {
        for c in commands {
            if c["tick"].as_u64() == Some(b.tick()) {
                let side: Side = serde_json::from_value(c["side"].clone()).unwrap();
                seq[side.index()] += 1;
                let ack = b.accept(CommandEnvelope {
                    side,
                    slot: None,
                    seq: seq[side.index()],
                    order: serde_json::from_value(c["order"].clone()).unwrap(),
                    queued: false,
                });
                println!("ACK t{} {:?}", b.tick(), serde_json::to_string(&ack).unwrap());
            }
        }
        b.step();
        let t = b.tick();
        let print = t % every == 0;
        for side in [Side::Blue, Side::Red] {
            let o = serde_json::to_value(b.observe(side)).unwrap();
            let mut summary = vec![];
            for u in o["own"].as_array().unwrap() {
                let p = &u["position"];
                summary.push(json!([
                    u["id"], u["kind"].to_string().chars().take(14).collect::<String>(),
                    [p[0].as_f64().map(|v| v.round()), p[1].as_f64().map(|v| v.round()), p[2].as_f64().map(|v| v.round())],
                    u["hp"], u["fuel_s"].as_f64().map(|f| f.round()), u["afterburner"], u["traversing"],
                    u["in_dogfight"], u["flares"], u["withdrawing"], u["state"],
                    u["mounts"].as_array().unwrap().iter().map(|m| json!([m["ammo"], m["reason"], m["target"]])).collect::<Vec<_>>(),
                    u["weapon_poses"].as_array().unwrap().iter().map(|m| m["shots"].clone()).collect::<Vec<_>>()
                ]));
            }
            let contacts: Vec<Value> = o["contacts"].as_array().unwrap().iter().map(|c| json!([c["id"], c["source"], c["center"], c["radius"], c["bearing_deg"], c["range_m"], c["kind"]])).collect();
            let ident: Vec<Value> = o["identified"].as_array().unwrap().iter().map(|u| json!([u["id"], u["kind"]])).collect();
            let guided = o["guided"].as_array().map_or(0, |g| g.len());
            let proj: Vec<Value> = o["projectiles"].as_array().map(|ps| ps.iter().map(|p| json!([p["own"], p["kind"]])).collect()).unwrap_or_default();
            let blasts = o["blasts"].as_array().map_or(0, |g| g.len());
            let sk = &o["skirmish"];
            if print || (!proj.is_empty() && spec["shots"].as_bool().unwrap_or(false)) {
                println!(
                    "t{t} {:?} own={} ident={} contacts={} guided={guided} proj={} blasts={blasts} sk={}",
                    side,
                    serde_json::to_string(&summary).unwrap(),
                    serde_json::to_string(&ident).unwrap(),
                    serde_json::to_string(&contacts).unwrap(),
                    serde_json::to_string(&proj).unwrap(),
                    if sk.is_null() { "-".into() } else { serde_json::to_string(&json!([sk["phase"], sk["credits"], sk["pending"], sk["report"]["events"]])).unwrap() }
                );
            }
        }
        if spec["fog"].as_bool().unwrap_or(false) && print {
            let f = &b.observe(Side::Blue).ground_visibility;
            let n: u32 = f.bits.iter().map(|w| w.count_ones()).sum();
            println!("t{t} blue fog cells visible={n}");
        }
    }
}
