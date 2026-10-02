//! SCRATCH PROBE (not committed): move admission over distance on a saved map.
use contract::command::MovePreviewRequest;
use contract::ids::{Side, UnitId};
use contract::scenario::{EncounterDefinition, Rules};
use sim::battle::Battle;
use std::time::Instant;

fn main() {
    let folder = std::env::args().nth(1).expect("map folder");
    let folder = std::path::Path::new(&folder);
    let map = sim::maps::load_folder(folder)
        .unwrap_or_else(|e| panic!("{e}"))
        .definition;
    let size = map.size;
    let encounter: EncounterDefinition = serde_json::from_str(
        &std::fs::read_to_string(folder.join("encounters/assault.json")).unwrap(),
    )
    .unwrap();
    let rules: Rules = serde_json::from_value(sim::fixtures::game()).unwrap();
    let blue: Vec<[f64; 2]> = encounter
        .units
        .iter()
        .filter(|u| u.side == Side::Blue)
        .map(|u| u.position)
        .collect();
    let far = encounter
        .units
        .iter()
        .rev()
        .find(|u| u.side == Side::Red)
        .unwrap()
        .position;
    let c = [
        blue.iter().map(|p| p[0]).sum::<f64>() / blue.len() as f64,
        blue.iter().map(|p| p[1]).sum::<f64>() / blue.len() as f64,
    ];
    let t = Instant::now();
    let mut battle = Battle::new(&encounter.on(map, rules), 1);
    println!(
        "map {:?} size {:?} build {:.1}s blue centre {:?} far {:?}",
        folder,
        size,
        t.elapsed().as_secs_f64(),
        c,
        far
    );
    let d = [far[0] - c[0], far[1] - c[1]];
    let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
    let groups: [(&str, &[u32]); 6] = [
        ("jeep", &[0]),
        ("tank", &[2]),
        ("rifle squad", &[4]),
        ("vehicles x4", &[0, 2, 3, 8]),
        ("infantry x4", &[4, 5, 6, 7]),
        ("mixed x9", &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    ];
    for dist in [500.0, 1500.0, 3000.0, len - 150.0] {
        let goal = [c[0] + d[0] / len * dist, c[1] + d[1] / len * dist];
        for (label, ids) in groups {
            let request = MovePreviewRequest {
                units: ids.iter().copied().map(UnitId).collect(),
                goal,
                ..Default::default()
            };
            println!("{label} | {dist:.0} m");
            let t = Instant::now();
            let out = battle.preview_move(Side::Blue, &request).unwrap();
            let ms = t.elapsed().as_secs_f64() * 1e3;
            let slots = out
                .iter()
                .filter(|m| {
                    let u = battle.unit(m.unit).unwrap().position;
                    (m.goal[0] - u.x).abs() + (m.goal[1] - u.y).abs() > 1.0
                })
                .count();
            let placed = out.iter().filter(|m| m.placed).count();
            println!(
                "  placed {placed}/{} (standing places {slots}) {ms:.0} ms",
                ids.len()
            );
        }
    }
}
