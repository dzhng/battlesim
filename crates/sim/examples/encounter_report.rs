//! Encounter placement report: plans one recipe on each compiled map it is
//! given, prints what the placement is and what it cost, and draws it.
//!
//!     cargo run -p sim --release --example encounter_report -- \
//!         [--recipe assault] [--seed 1] [--out <directory>] <map-directory>...
//!
//! A map directory is what `mapgen generate-map <request> <presets>
//! <catalogue> <directory>` writes: `map.json` and `sites.json`. The rules
//! are the village's, the recipe a row of `fixtures/encounters.json`.
//!
//! One table row per map: each column's drive to the objective for the
//! recipe's pace unit and their difference, how far a column was moved up
//! its road, the squads garrisoned, the candidates tried, and instructions
//! retired (macOS only) by the world build and by planning. A refused
//! placement prints its diagnostics in the row.
//!
//! With `--out`, each map gets overlay pictures there: the whole map, the
//! objective and each column, with deployments, the capture zone, garrisoned
//! buildings, overwatch posts with their sight lines, the open approaches
//! and the two drives.
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use contract::encounter::{EncounterDefinition, EncounterRecipes, EncounterSites};
use contract::ground::GroundShape;
use contract::identity::Seed;
use contract::ids::Side;
use contract::map::{MapDefinition, SurfaceKind};
use contract::scenario::Rules;
use sim::encounter::{plan_encounter, PreparedMap};

fn main() {
    let mut recipe_id = "assault".to_string();
    let mut seed = 1u64;
    let mut out: Option<PathBuf> = None;
    let mut maps = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .unwrap_or_else(|| panic!("{name} takes a value"))
        };
        match arg.as_str() {
            "--recipe" => recipe_id = value("--recipe"),
            "--seed" => seed = value("--seed").parse().expect("--seed takes a u64"),
            "--out" => out = Some(value("--out").into()),
            _ => maps.push(PathBuf::from(arg)),
        }
    }
    assert!(
        !maps.is_empty(),
        "usage: encounter_report [--recipe id] [--seed n] [--out dir] <map-directory>..."
    );
    let rules: Rules = serde_json::from_value(sim::fixtures::village()).expect("the village rules");
    let recipes = EncounterRecipes::from_json(
        &std::fs::read_to_string(sim::fixtures::dir().join("encounters.json"))
            .expect("fixtures/encounters.json"),
    )
    .expect("valid recipes");
    let recipe = recipes
        .recipes
        .get(&recipe_id)
        .unwrap_or_else(|| panic!("no recipe {recipe_id}"));
    if let Some(out) = &out {
        std::fs::create_dir_all(out).expect("the output directory");
    }

    println!(
        "recipe {recipe_id} ({}), encounter seed {seed}, pace {}",
        recipes.revision, recipe.deployment.pace
    );
    println!("| map | objective | attacker drive s | defender drive s | difference s | moved up m (att/def) | garrisoned squads | overwatch | attempts | world build G | planning G | result hash |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|");
    for directory in &maps {
        let name = directory
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let read = |file: &str| {
            std::fs::read_to_string(directory.join(file))
                .unwrap_or_else(|e| panic!("{}/{file}: {e}", directory.display()))
        };
        let map: MapDefinition = serde_json::from_str(&read("map.json")).expect("a compiled map");
        let sites: EncounterSites = serde_json::from_str(&read("sites.json")).expect("map sites");
        let before = instructions();
        let prepared = PreparedMap::new(&map, &rules);
        let built = instructions();
        let outcome = plan_encounter(
            &prepared.queries(&map, &sites),
            &rules,
            recipe,
            Seed::from(seed),
        );
        let planned = instructions();
        let giga = |a: Option<u64>, b: Option<u64>| {
            a.zip(b).map_or("n/a".to_string(), |(a, b)| {
                format!("{:.2}", (b - a) as f64 / 1e9)
            })
        };
        match &outcome {
            Ok(encounter) => {
                let p = &encounter.placement;
                let drive = |side: Side| p.deployments.iter().find(|d| d.side == side);
                let (a, d) = (drive(recipe.attacker), drive(recipe.defender()));
                let seconds = |x: Option<&contract::encounter::Deployment>| {
                    x.map_or("-".to_string(), |x| format!("{:.1}", x.route_s))
                };
                let moved = |x: Option<&contract::encounter::Deployment>| {
                    x.map_or("-".to_string(), |x| format!("{:.0}", x.advance_m))
                };
                println!(
                    "| {name} | {} | {} | {} | {} | {}/{} | {} | {} | {} | {} | {} | {:.12} |",
                    p.objective.settlement,
                    seconds(a),
                    seconds(d),
                    a.zip(d).map_or("-".to_string(), |(a, d)| format!(
                        "{:.1}",
                        (a.route_s - d.route_s).abs()
                    )),
                    moved(a),
                    moved(d),
                    p.garrisons.len(),
                    p.overwatch.len(),
                    p.attempts,
                    giga(before, built),
                    giga(built, planned),
                    contract::identity::json_hash(encounter).expect("an encounter serializes"),
                );
            }
            Err(diagnostics) => {
                let why: Vec<String> = diagnostics
                    .iter()
                    .map(|d| format!("{:?}: {}", d.code, d.message))
                    .collect();
                println!(
                    "| {name} | refused | - | - | - | - | - | - | - | {} | {} | {} |",
                    giga(before, built),
                    giga(built, planned),
                    why.join("; ")
                );
            }
        }
        if let (Some(out), Ok(encounter)) = (&out, &outcome) {
            draw(out, &name, &map, &sites, encounter, &rules);
        }
    }
}

/// A square window of the map, in metres: its south-west corner and side.
#[derive(Clone, Copy)]
struct View {
    x: f64,
    y: f64,
    side: f64,
}

fn draw(
    out: &Path,
    name: &str,
    map: &MapDefinition,
    sites: &EncounterSites,
    encounter: &EncounterDefinition,
    rules: &Rules,
) {
    let p = &encounter.placement;
    let whole = View {
        x: 0.0,
        y: 0.0,
        side: map.size[0].max(map.size[1]),
    };
    let about = |points: &[[f64; 2]], margin: f64, least: f64| {
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for q in points {
            for k in 0..2 {
                lo[k] = lo[k].min(q[k]);
                hi[k] = hi[k].max(q[k]);
            }
        }
        let side = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(least) + 2.0 * margin;
        View {
            x: (lo[0] + hi[0]) / 2.0 - side / 2.0,
            y: (lo[1] + hi[1]) / 2.0 - side / 2.0,
            side,
        }
    };
    let mut views = vec![("map".to_string(), whole)];
    let reach = p
        .overwatch
        .iter()
        .map(|o| o.at)
        .chain(
            p.garrisons
                .iter()
                .map(|g| encounter.setup.units[g.unit as usize].position),
        )
        .chain([p.objective.center])
        .collect::<Vec<_>>();
    views.push((
        "objective".to_string(),
        about(&reach, 80.0, 2.0 * p.objective.radius_m),
    ));
    for d in &p.deployments {
        let at: Vec<[f64; 2]> = d
            .units
            .iter()
            .map(|&u| encounter.setup.units[u as usize].position)
            .collect();
        let side = if d.side == Side::Blue { "blue" } else { "red" };
        views.push((format!("{side}-column"), about(&at, 40.0, 100.0)));
    }
    for (label, view) in views {
        let title = format!("{name} · {label}");
        let picture = svg(&title, view, map, sites, encounter, rules);
        std::fs::write(out.join(format!("{name}-{label}.svg")), picture).expect("write a picture");
    }
}

fn svg(
    title: &str,
    view: View,
    map: &MapDefinition,
    sites: &EncounterSites,
    encounter: &EncounterDefinition,
    rules: &Rules,
) -> String {
    let p = &encounter.placement;
    let depth = map.size[1];
    // Map metres to picture units: +Y up on the map, down in the picture.
    let px = |q: [f64; 2]| format!("{:.2},{:.2}", q[0], depth - q[1]);
    let line =
        |points: &mut dyn Iterator<Item = [f64; 2]>| points.map(px).collect::<Vec<_>>().join(" ");
    // One pixel of a 1600 px picture, in metres: nothing is drawn thinner.
    let hair = view.side / 1600.0;
    let close = view.side < 2000.0;
    let mut s = String::new();
    let _ = write!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="1600" viewBox="{:.2} {:.2} {:.2} {:.2}" font-family="Helvetica, Arial, sans-serif">"##,
        view.x,
        depth - view.y - view.side,
        view.side,
        view.side
    );
    let _ = write!(
        s,
        r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="#d9d2bd"/><rect x="0" y="0" width="{}" height="{}" fill="#efe8d4"/>"##,
        view.x,
        depth - view.y - view.side,
        view.side,
        view.side,
        map.size[0],
        depth
    );
    for forest in &map.forests {
        match &forest.shape {
            GroundShape::Polygon { ring } => {
                let _ = write!(
                    s,
                    r##"<polygon points="{}" fill="#6f9f66"/>"##,
                    line(&mut ring.iter().copied())
                );
            }
            GroundShape::Stroke {
                centerline,
                width_m,
            } => {
                let _ = write!(
                    s,
                    r##"<polyline points="{}" fill="none" stroke="#6f9f66" stroke-width="{width_m}" stroke-linecap="round"/>"##,
                    line(&mut centerline.samples().iter().copied())
                );
            }
        }
    }
    for river in &map.rivers {
        for pair in river.samples().windows(2) {
            let _ = write!(
                s,
                r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#4a86c5" stroke-width="{:.2}" stroke-linecap="round"/>"##,
                pair[0].xy[0],
                depth - pair[0].xy[1],
                pair[1].xy[0],
                depth - pair[1].xy[1],
                pair[0].half_width_m + pair[1].half_width_m
            );
        }
    }
    for surface in &map.surfaces {
        let (colour, dash) = match surface.kind {
            SurfaceKind::CountryRoad => ("#1f1f1f", ""),
            SurfaceKind::Road => ("#77726a", ""),
            SurfaceKind::DirtTrack => ("#8a6a3a", "stroke-dasharray=\"12 8\""),
            SurfaceKind::Sidewalk => continue,
        };
        match &surface.shape {
            GroundShape::Polygon { ring } => {
                let _ = write!(
                    s,
                    r##"<polygon points="{}" fill="#c9c4b8"/>"##,
                    line(&mut ring.iter().copied())
                );
            }
            GroundShape::Stroke {
                centerline,
                width_m,
            } => {
                let _ = write!(
                    s,
                    r##"<polyline points="{}" fill="none" stroke="{colour}" stroke-width="{:.2}" {dash}/>"##,
                    line(&mut centerline.samples().iter().copied()),
                    width_m.max(if surface.kind == SurfaceKind::CountryRoad {
                        2.5 * hair
                    } else {
                        hair
                    })
                );
            }
        }
    }
    for bridge in &map.bridges {
        let _ = write!(
            s,
            r##"<polygon points="{}" fill="#b9b2a2" stroke="#3a3a3a" stroke-width="{:.2}"/>"##,
            line(&mut corners(bridge.center, bridge.yaw, bridge.half_extents).into_iter()),
            hair
        );
    }
    let held: Vec<u32> = p.garrisons.iter().map(|g| g.building).collect();
    for building in &map.buildings {
        let fill = if held.contains(&building.owner) {
            "#e02020"
        } else {
            "#8c7b6b"
        };
        for part in &building.geometry.parts {
            let _ = write!(
                s,
                r##"<polygon points="{}" fill="{fill}"/>"##,
                line(
                    &mut corners(
                        part.center,
                        part.yaw,
                        [part.half_extents[0], part.half_extents[1]]
                    )
                    .into_iter()
                )
            );
        }
    }
    // The objective's settlement, its open approaches and the capture zone.
    if let Some((index, site)) = sites
        .settlements
        .iter()
        .enumerate()
        .find(|(_, site)| site.id == p.objective.settlement)
    {
        let _ = write!(
            s,
            r##"<polygon points="{}" fill="none" stroke="#7a3fb0" stroke-width="{:.2}" stroke-dasharray="{:.2} {:.2}"/>"##,
            line(&mut site.outline.iter().copied()),
            2.0 * hair,
            10.0 * hair,
            6.0 * hair
        );
        for (a, approach) in sites.approaches.iter().enumerate() {
            if approach.settlement != index {
                continue;
            }
            let watched = p.overwatch.iter().any(|o| o.approach == Some(a));
            let to = if approach.to_rad < approach.from_rad {
                approach.to_rad + std::f64::consts::TAU
            } else {
                approach.to_rad
            };
            let mid = (approach.from_rad + to) / 2.0;
            let (dy, dx) = mid.sin_cos();
            let c = site.center;
            // From the centre to the approach's far end: the corridor's
            // near end is the settlement's own ground, drawn over it.
            let far = approach.depth_m + site_radius(&site.outline, c);
            let half = approach.front_m / 2.0;
            let corner = |along: f64, across: f64| {
                [
                    c[0] + dx * along - dy * across,
                    c[1] + dy * along + dx * across,
                ]
            };
            let _ = write!(
                s,
                r##"<polygon points="{}" fill="{}" stroke="#3f6fb0" stroke-width="{:.2}" stroke-dasharray="{:.2} {:.2}"/>"##,
                line(
                    &mut [
                        corner(0.0, -half),
                        corner(far, -half),
                        corner(far, half),
                        corner(0.0, half)
                    ]
                    .into_iter()
                ),
                if watched {
                    "rgba(63,111,176,0.16)"
                } else {
                    "rgba(63,111,176,0.05)"
                },
                hair,
                8.0 * hair,
                8.0 * hair
            );
        }
    }
    let c = p.objective.center;
    let _ = write!(
        s,
        r##"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="rgba(255,196,0,0.22)" stroke="#c89000" stroke-width="{:.2}"/>"##,
        c[0],
        depth - c[1],
        p.objective.radius_m,
        2.5 * hair
    );
    let colour = |side: Side| {
        if side == Side::Blue {
            "#1f5fff"
        } else {
            "#e02020"
        }
    };
    for d in &p.deployments {
        let _ = write!(
            s,
            r##"<polyline points="{}" fill="none" stroke="{}" stroke-width="{:.2}" stroke-opacity="0.55" stroke-linejoin="round"/>"##,
            line(&mut d.route.iter().copied()),
            colour(d.side),
            3.0 * hair
        );
    }
    for o in &p.overwatch {
        let (dy, dx) = o.yaw.sin_cos();
        let far = [o.at[0] + dx * 1000.0, o.at[1] + dy * 1000.0];
        let _ = write!(
            s,
            r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#e02020" stroke-width="{:.2}" stroke-dasharray="{:.2} {:.2}"/>"##,
            o.at[0],
            depth - o.at[1],
            far[0],
            depth - far[1],
            1.5 * hair,
            6.0 * hair,
            6.0 * hair
        );
    }
    for g in &p.garrisons {
        let unit = &encounter.setup.units[g.unit as usize];
        if let Some(building) = map.buildings.iter().find(|b| b.owner == g.building) {
            let [x, y, _] = building.geometry.frame.translation;
            let _ = write!(
                s,
                r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#e02020" stroke-width="{:.2}"/>"##,
                unit.position[0],
                depth - unit.position[1],
                x,
                depth - y,
                1.5 * hair
            );
        }
    }
    for (id, unit) in encounter.setup.units.iter().enumerate() {
        let t = rules.catalog.by_id(&unit.kind);
        let fill = colour(unit.side);
        let [x, y] = unit.position;
        if close {
            // The footprint at its true size.
            match t.hull() {
                Some(hull) => {
                    let _ = write!(
                        s,
                        r##"<polygon points="{}" fill="{fill}" stroke="#ffffff" stroke-width="{:.2}"/>"##,
                        line(
                            &mut corners(
                                unit.position,
                                unit.yaw,
                                [hull.half_extents_m[0], hull.half_extents_m[1]]
                            )
                            .into_iter()
                        ),
                        hair
                    );
                }
                None => {
                    let radius =
                        sim::arrangement::spread(&rules.infantry_movement, t.squad_size()) / 2.0;
                    let _ = write!(
                        s,
                        r##"<circle cx="{x:.2}" cy="{:.2}" r="{radius:.2}" fill="{fill}" fill-opacity="0.45" stroke="{fill}" stroke-width="{:.2}"/>"##,
                        depth - y,
                        hair
                    );
                }
            }
            let _ = write!(
                s,
                r##"<text x="{:.2}" y="{:.2}" font-size="{:.2}" fill="#111" stroke="#fff" stroke-width="{:.2}" paint-order="stroke">{} {}</text>"##,
                x + 8.0 * hair,
                depth - y - 8.0 * hair,
                14.0 * hair,
                3.0 * hair,
                id,
                unit.kind
            );
        } else {
            let _ = write!(
                s,
                r##"<circle cx="{x:.2}" cy="{:.2}" r="{:.2}" fill="{fill}" stroke="#ffffff" stroke-width="{:.2}"/>"##,
                depth - y,
                6.0 * hair,
                1.5 * hair
            );
        }
    }
    let drive = |side: Side| {
        p.deployments
            .iter()
            .find(|d| d.side == side)
            .map_or("no column".to_string(), |d| {
                format!(
                    "{:.0} s over {:.0} m, moved up {:.0} m",
                    d.route_s, d.route_m, d.advance_m
                )
            })
    };
    let lines = [
        title.to_string(),
        format!(
            "objective {} (gold, {:.0} m) · blue drive {} · red drive {}",
            p.objective.settlement,
            p.objective.radius_m,
            drive(Side::Blue),
            drive(Side::Red)
        ),
        format!(
            "{} garrisoned squads (red buildings) · {} overwatch posts (dashed sight lines) · open approaches shaded · {} candidates tried · view {:.0} m",
            p.garrisons.len(),
            p.overwatch.len(),
            p.attempts,
            view.side
        ),
    ];
    for (k, text) in lines.iter().enumerate() {
        let _ = write!(
            s,
            r##"<text x="{:.2}" y="{:.2}" font-size="{:.2}" font-weight="{}" fill="#111" stroke="#fff" stroke-width="{:.2}" paint-order="stroke">{}</text>"##,
            view.x + 12.0 * hair,
            depth - view.y - view.side + (28.0 + 26.0 * k as f64) * hair,
            if k == 0 { 24.0 } else { 17.0 } * hair,
            if k == 0 { "bold" } else { "normal" },
            4.0 * hair,
            text
        );
    }
    s.push_str("</svg>");
    s
}

/// The corners of a box of `half` extents at `centre`, heading `yaw`.
fn corners(centre: [f64; 2], yaw: f64, half: [f64; 2]) -> [[f64; 2]; 4] {
    let (sin, cos) = yaw.sin_cos();
    [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)].map(|(a, b)| {
        let (x, y) = (half[0] * a, half[1] * b);
        [centre[0] + x * cos - y * sin, centre[1] + x * sin + y * cos]
    })
}

/// The farthest an outline reaches from `centre`.
fn site_radius(outline: &[[f64; 2]], centre: [f64; 2]) -> f64 {
    outline
        .iter()
        .map(|q| (q[0] - centre[0]).hypot(q[1] - centre[1]))
        .fold(0.0, f64::max)
}

#[path = "common/instructions.rs"]
mod instructions;
use instructions::instructions;
