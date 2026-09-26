//! Spike 02 (throwaway): dump the simulation's fog inputs and sweep results
//! for the GPU fog prototype. Writes ../data/<scenario>.json.
use contract::ids::Side;
use contract::scenario::{Rules, ScenarioDefinition, SensorRules, UnitKind, UnitSetup};
use serde_json::json;
use sim::battle::Battle;
use sim::math::{v2, V3};
use sim::world::WorldGeometry;

/// The spike's hand-written sight shape: (front, side, rear) multipliers.
fn shape(kind: UnitKind) -> [f64; 3] {
    match kind {
        UnitKind::Tank | UnitKind::Supply => [1.0, 0.5, 0.3],
        _ => [1.0, 1.0, 1.0],
    }
}

/// Range multiplier at `off` radians from forward: linear in angle,
/// front→side over [0, π/2], side→rear over [π/2, π].
pub fn shape_mult(s: [f64; 3], off: f64) -> f64 {
    let a = off.abs() % std::f64::consts::TAU;
    let a = if a > std::f64::consts::PI { std::f64::consts::TAU - a } else { a };
    let h = std::f64::consts::FRAC_PI_2;
    if a <= h {
        s[0] + (s[1] - s[0]) * (a / h)
    } else {
        s[1] + (s[2] - s[1]) * ((a - h) / h)
    }
}

struct Grid {
    cell: f64,
    nx: usize,
    ny: usize,
    top: Vec<f64>,
}

fn occlusion(world: &WorldGeometry, cell: f64) -> Grid {
    let nx = (world.width() / cell).ceil() as usize;
    let ny = (world.depth() / cell).ceil() as usize;
    let mut top = vec![f64::NEG_INFINITY; nx * ny];
    for prop in world.props().filter(|p| p.kind.occludes()) {
        let r = prop.footprint_radius();
        let i0 = ((prop.center.x - r) / cell).floor().max(0.0) as usize;
        let j0 = ((prop.center.y - r) / cell).floor().max(0.0) as usize;
        let i1 = (((prop.center.x + r) / cell).floor() as usize).min(nx - 1);
        let j1 = (((prop.center.y + r) / cell).floor() as usize).min(ny - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let c = v2((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell);
                if prop.footprint().contains(c, 0.0) {
                    let t = &mut top[j * nx + i];
                    *t = t.max(prop.top_z());
                }
            }
        }
    }
    Grid { cell, nx, ny, top }
}

/// `visibility::sweep` with the range shrunk per ray by the sight shape
/// (what slice 04 will do), as the CPU reference for directional fog.
fn sweep_dir(
    world: &WorldGeometry,
    g: &Grid,
    s: &SensorRules,
    eye: V3,
    range: f64,
    fwd: f64,
    sh: [f64; 3],
    bits: &mut [u32],
) {
    let cell = g.cell;
    let (ei, ej) = ((eye.x / cell).floor(), (eye.y / cell).floor());
    if ei >= 0.0 && ej >= 0.0 && (ei as usize) < g.nx && (ej as usize) < g.ny {
        let idx = ej as usize * g.nx + ei as usize;
        bits[idx / 32] |= 1 << (idx % 32);
    }
    let rays = ((std::f64::consts::TAU * range / cell).ceil() as usize).max(64);
    for r in 0..rays {
        let angle = r as f64 / rays as f64 * std::f64::consts::TAU;
        let ray_range = range * shape_mult(sh, angle - fwd);
        let steps = (ray_range / cell).ceil() as usize;
        let dir = v2(angle.cos(), angle.sin());
        let mut horizon = f64::NEG_INFINITY;
        let mut foliage = 0.0;
        for k in 1..=steps {
            let dist = k as f64 * cell;
            let p = eye.xy() + dir * dist;
            let Some(ground) = world.height_at(p.x, p.y) else { break };
            let i = (p.x / cell) as usize;
            let j = (p.y / cell) as usize;
            if i >= g.nx || j >= g.ny {
                break;
            }
            let slope = (ground + s.fog_target_height_m - eye.z) / dist;
            match sim::sensing::foliage_reach(ray_range, foliage, s) {
                Some(reach) if dist <= reach => {}
                _ => break,
            }
            if slope >= horizon {
                let idx = j * g.nx + i;
                bits[idx / 32] |= 1 << (idx % 32);
            }
            let sight_z = eye.z + horizon.max(slope) * dist;
            if world.forest_depth(p.x, p.y).is_some() {
                let canopy = world
                    .forests()
                    .iter()
                    .filter(|f| sim::world::in_forest(f, p.x, p.y))
                    .map(|f| f.canopy_height_m)
                    .fold(0.0, f64::max);
                if sight_z < ground + canopy {
                    foliage += cell;
                }
            }
            horizon = horizon.max((ground.max(g.top[j * g.nx + i]) - eye.z) / dist);
        }
    }
}

fn kind_tag(k: UnitKind) -> u32 {
    match k {
        UnitKind::Rifle => 0,
        UnitKind::At => 1,
        UnitKind::Recon => 2,
        UnitKind::Tank => 3,
        UnitKind::Supply => 4,
    }
}

fn dump(name: &str, setup: &ScenarioDefinition, battle: &Battle, sides: &[Side]) {
    let rules: &Rules = &setup.rules;
    let s = &rules.sensors;
    let world = battle.world();
    let (verts, _) = world.terrain_mesh();
    let spacing = setup.map.height_grid_m;
    let nx = (setup.map.size[0] / spacing).round() as usize + 1;
    let ny = (setup.map.size[1] / spacing).round() as usize + 1;
    assert_eq!(verts.len(), nx * ny);
    // Vertex order check: row-major j*nx+i.
    assert!((verts[1].x - spacing).abs() < 1e-9 && verts[nx].y > 0.0);
    let heights: Vec<f32> = verts.iter().map(|v| v.z as f32).collect();
    let props: Vec<Vec<f64>> = world
        .props()
        .map(|p| {
            vec![
                if p.kind.occludes() { 1.0 } else { 0.0 },
                p.center.x, p.center.y, p.yaw, p.half.x, p.half.y, p.half.z, p.base_z,
                match p.kind {
                    contract::map::PropKind::Building => 0.0,
                    contract::map::PropKind::Trunk => 1.0,
                    contract::map::PropKind::Wreck => 2.0,
                    _ => 3.0,
                },
            ]
        })
        .collect();
    let forests: Vec<Vec<f64>> = world
        .forests()
        .iter()
        .map(|f| vec![f.rect[0], f.rect[1], f.rect[2], f.rect[3], f.canopy_height_m])
        .collect();
    let grid = occlusion(world, s.fog_cell_m);
    let mut eyes = Vec::new();
    let mut dir_bits = vec![0u32; (grid.nx * grid.ny).div_ceil(32)];
    let mut iso_bits = vec![0u32; dir_bits.len()];
    let mut sim_bits = vec![0u32; dir_bits.len()];
    let mut veh_bits = vec![0u32; dir_bits.len()];
    for &side in sides {
        let frame = battle.observe(side);
        for f in frame.ground_visibility.bits.iter().enumerate() {
            sim_bits[f.0] |= f.1;
        }
        for own in &frame.own {
            let unit = battle.unit(own.id).unwrap();
            let range = sim::sensing::ground_range(unit.kind, s);
            let fwd = match unit.kind {
                UnitKind::Tank => unit.mounts.first().map_or(unit.yaw, |m| m.bearing),
                _ => unit.yaw,
            };
            let sh = shape(unit.kind);
            for eye in sim::sensing::eyes(unit, rules) {
                sweep_dir(world, &grid, s, eye, range, fwd, sh, &mut dir_bits);
                sweep_dir(world, &grid, s, eye, range, 0.0, [1.0; 3], &mut iso_bits);
                if matches!(unit.kind, UnitKind::Tank | UnitKind::Supply) {
                    sweep_dir(world, &grid, s, eye, range, fwd, sh, &mut veh_bits);
                }
                eyes.push(vec![
                    eye.x, eye.y, eye.z, range, fwd, sh[0], sh[1], sh[2],
                    kind_tag(unit.kind) as f64,
                ]);
            }
        }
    }
    // Every living unit of both sides, for drawing proxies.
    let mut units = Vec::new();
    for side in [Side::Blue, Side::Red] {
        for own in &battle.observe(side).own {
            let unit = battle.unit(own.id).unwrap();
            let fwd = match unit.kind {
                UnitKind::Tank => unit.mounts.first().map_or(unit.yaw, |m| m.bearing),
                _ => unit.yaw,
            };
            if unit.hull.is_some() {
                let h = unit.hull.unwrap();
                units.push(json!([side as u32, kind_tag(unit.kind), unit.position.x, unit.position.y, unit.position.z, unit.yaw, h.x, h.y, h.z, fwd]));
            } else {
                for p in unit.member_positions() {
                    units.push(json!([side as u32, kind_tag(unit.kind), p.x, p.y, p.z, unit.yaw, 0.17, 0.22, 0.8, unit.yaw]));
                }
            }
        }
    }
    let out = json!({
        "name": name,
        "size": setup.map.size,
        "spacing": spacing, "nx": nx, "ny": ny, "heights": heights,
        "props": props, "forests": forests,
        "roads": serde_json::to_value(&setup.map.roads).unwrap(),
        "sensors": { "cell": s.fog_cell_m, "targetH": s.fog_target_height_m,
            "forestAtt": s.forest_attenuation_m, "forestFull": s.forest_full_block_m },
        "eyes": eyes,
        "units": units,
        "fog": { "nx": grid.nx, "ny": grid.ny, "iso": iso_bits, "dir": dir_bits, "sim": sim_bits, "dirVeh": veh_bits },
    });
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(format!("{dir}/{name}.json"), out.to_string()).unwrap();
    let same = iso_bits == sim_bits;
    let count = |b: &[u32]| b.iter().map(|w| w.count_ones()).sum::<u32>();
    println!("{name}: tick {} {} eyes, {} props, fog {}x{}; iso ref == sim field: {same} (sim {} ref {} dir {})",
        battle.tick(), eyes.len(), props.len(), grid.nx, grid.ny, count(&sim_bits), count(&iso_bits), count(&dir_bits));
}

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/village.json");
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

    // The village street: blue's eight units moved into and around the village.
    let mut street = sim::village::scenario(&fixture, "ordinary").unwrap();
    let blue: [(UnitKind, f64, f64, f64); 8] = [
        (UnitKind::Recon, 1004.0, 788.0, 0.0),
        (UnitKind::Rifle, 1012.0, 842.0, 0.0),
        (UnitKind::Rifle, 930.0, 800.0, 0.0),
        (UnitKind::Rifle, 905.0, 745.0, 0.0),
        (UnitKind::Tank, 942.0, 826.0, 0.15),
        (UnitKind::Tank, 870.0, 790.0, 0.3),
        (UnitKind::At, 950.0, 715.0, 0.0),
        (UnitKind::Supply, 820.0, 790.0, 0.0),
    ];
    let mut units: Vec<UnitSetup> = blue
        .iter()
        .map(|&(kind, x, y, yaw)| UnitSetup {
            side: Side::Blue, kind, position: [x, y], yaw,
            engagement: None, condition: None, stock: None,
        })
        .collect();
    units.extend(street.units.iter().filter(|u| u.side == Side::Red).cloned());
    // Red ids shift by nothing: blue count is unchanged (8), so garrisons hold.
    assert_eq!(street.units.iter().filter(|u| u.side == Side::Blue).count(), 8);
    street.units = units;
    let battle = Battle::new(&street, 1);
    dump("street", &street, &battle, &[Side::Blue]);

    // The village as authored at its start.
    let village = sim::village::scenario(&fixture, "ordinary").unwrap();
    let battle = Battle::new(&village, 1);
    dump("village", &village, &battle, &[Side::Blue]);

    // Endurance, 100 a side, three simulated minutes in.
    let endurance = sim::endurance::scenario(&fixture, 1, false).unwrap();
    let mut battle = Battle::new(&endurance, 1);
    let ticks = 3 * 60 * endurance.rules.tick_hz as u64;
    for _ in 0..ticks {
        battle.step();
    }
    // Sweeps run every few ticks per side; step until blue just swept.
    while battle.tick() % 6 != 0 {
        battle.step();
    }
    dump("endurance", &endurance, &battle, &[Side::Blue]);
    dump("stress", &endurance, &battle, &[Side::Blue, Side::Red]);
}
