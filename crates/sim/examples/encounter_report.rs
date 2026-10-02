//! Encounter placement report: plans one recipe on each compiled map it is
//! given, prints what the placement is and what it cost, and draws it.
//!
//!     cargo run -p sim --release --example encounter_report -- \
//!         [--recipe assault] [--seed 1] [--out <directory>] [--save] <map-directory>...
//!
//! A map directory is what `mapgen generate-map <request> <presets>
//! <catalogue> <catalog> <directory>` writes: a saved map (`map.json`, `SOURCES.json`)
//! and its `sites.json`. The rules are the village's, the recipe a row of
//! `fixtures/encounters.json`.
//!
//! With `--save`, each map's planned encounter is written as its saved
//! encounter, `<map-directory>/encounters/<recipe>.json`.
//!
//! One table row per map: each column's drive to the objective for the
//! recipe's pace unit and their difference, how far a column was moved up
//! its road, the squads garrisoned, the candidates tried, and instructions
//! retired (macOS only) by the world build and by planning. A refused
//! placement prints its diagnostics in the row.
//!
//! With `--out`, each map gets overlay pictures there (SVG): the whole map,
//! the objective, each overwatch post and each column, with deployments, the
//! capture zone, garrisoned buildings, the posts' sight lines, the open
//! approaches and the two drives.
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
    let mut save = false;
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
            "--save" => save = true,
            _ => maps.push(PathBuf::from(arg)),
        }
    }
    assert!(
        !maps.is_empty(),
        "usage: encounter_report [--recipe id] [--seed n] [--out dir] [--save] <map-directory>..."
    );
    let rules: Rules = serde_json::from_value(sim::fixtures::game()).expect("the village rules");
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
    println!("| map | buildings | river | objective | attacker drive s | defender drive s | difference s | moved up m (att/def) | garrisoned squads | overwatch | attempts | world build G | planning G | result hash |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for directory in &maps {
        let name = directory
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let read = |file: &str| {
            std::fs::read_to_string(directory.join(file))
                .unwrap_or_else(|e| panic!("{}/{file}: {e}", directory.display()))
        };
        let map: MapDefinition = sim::maps::load_folder(directory)
            .unwrap_or_else(|e| panic!("{e}"))
            .definition;
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
                    "| {name} | {} | {} | {} | {} | {} | {} | {}/{} | {} | {} | {} | {} | {} | {:.12} |",
                    map.buildings.len(),
                    if map.rivers.is_empty() { "no" } else { "yes" },
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
                    "| {name} | {} | {} | refused | - | - | - | - | - | - | - | {} | {} | {} |",
                    map.buildings.len(),
                    if map.rivers.is_empty() { "no" } else { "yes" },
                    giga(before, built),
                    giga(built, planned),
                    why.join("; ")
                );
            }
        }
        if let (true, Ok(encounter)) = (save, &outcome) {
            let folder = directory.join("encounters");
            std::fs::create_dir_all(&folder).expect("the encounters directory");
            std::fs::write(
                folder.join(format!("{recipe_id}.json")),
                serde_json::to_vec(&encounter.setup).expect("an encounter serializes"),
            )
            .expect("the saved encounter");
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

impl View {
    /// The square of `side` metres about `centre`.
    fn about(centre: [f64; 2], side: f64) -> View {
        View {
            x: centre[0] - side / 2.0,
            y: centre[1] - side / 2.0,
            side,
        }
    }
}

/// The middle of `points` and the side of the square that holds them.
fn extent(points: &[[f64; 2]]) -> ([f64; 2], f64) {
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for q in points {
        for k in 0..2 {
            lo[k] = lo[k].min(q[k]);
            hi[k] = hi[k].max(q[k]);
        }
    }
    (
        [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0],
        (hi[0] - lo[0]).max(hi[1] - lo[1]),
    )
}

fn side_name(side: Side) -> &'static str {
    match side {
        Side::Blue => "blue",
        Side::Red => "red",
    }
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
    let units = &encounter.setup.units;
    let mut views = vec![(
        "map".to_string(),
        View {
            x: 0.0,
            y: 0.0,
            side: map.size[0].max(map.size[1]),
        },
    )];
    // The objective: the zone and the squads round it, the zone in the middle.
    let reach = p
        .garrisons
        .iter()
        .map(|g| {
            let at = units[g.unit as usize].position;
            (at[0] - p.objective.center[0]).hypot(at[1] - p.objective.center[1])
        })
        .fold(p.objective.radius_m, f64::max);
    views.push((
        "objective".to_string(),
        View::about(p.objective.center, 2.0 * (reach + 60.0)),
    ));
    for (n, post) in p.overwatch.iter().enumerate() {
        views.push((format!("overwatch-{n}"), View::about(post.at, 320.0)));
    }
    // The columns, at one scale.
    let columns: Vec<(Side, [f64; 2], f64)> = p
        .deployments
        .iter()
        .map(|d| {
            let at: Vec<[f64; 2]> = d
                .units
                .iter()
                .map(|&u| units[u as usize].position)
                .collect();
            let (middle, side) = extent(&at);
            (d.side, middle, side)
        })
        .collect();
    let widest = columns.iter().map(|c| c.2).fold(100.0, f64::max) + 80.0;
    for (side, middle, _) in columns {
        views.push((
            format!("{}-column", side_name(side)),
            View::about(middle, widest),
        ));
    }
    for (label, view) in views {
        let title = format!("{name} · {label}");
        let picture = svg(&title, view, map, sites, encounter, rules);
        std::fs::write(out.join(format!("{name}-{label}.svg")), picture).expect("write a picture");
    }
}

/// The picture's header band, in picture pixels of a 1600 px wide picture.
const HEADER_PX: f64 = 132.0;

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
    // One pixel of the 1600 px picture, in metres: nothing is drawn thinner.
    let hair = view.side / 1600.0;
    let close = view.side < 2000.0;
    let top = depth - view.y - view.side;
    let header = HEADER_PX * hair;
    let mut s = String::new();
    // A square picture (some viewers crop a tall one): the header across the
    // top, the map below it with a margin either side.
    let canvas = view.side + header;
    let left = view.x - header / 2.0;
    let _ = write!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{0:.0}" height="{0:.0}" viewBox="{left:.2} {1:.2} {canvas:.2} {canvas:.2}" font-family="Helvetica, Arial, sans-serif">"##,
        1600.0 + HEADER_PX,
        top - header,
    );
    let _ = write!(
        s,
        r##"<defs><clipPath id="map"><rect x="{:.2}" y="{top:.2}" width="{:.2}" height="{:.2}"/></clipPath></defs>"##,
        view.x, view.side, view.side
    );
    let _ = write!(
        s,
        r##"<rect x="{left:.2}" y="{:.2}" width="{canvas:.2}" height="{canvas:.2}" fill="#ffffff"/><g clip-path="url(#map)"><rect x="{:.2}" y="{top:.2}" width="{:.2}" height="{:.2}" fill="#d9d2bd"/><rect x="0" y="0" width="{}" height="{}" fill="#efe8d4"/>"##,
        top - header,
        view.x,
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
        let (colour, dash, least) = match surface.kind {
            SurfaceKind::CountryRoad => ("#1f1f1f", String::new(), 2.5 * hair),
            SurfaceKind::Road => ("#77726a", String::new(), hair),
            SurfaceKind::DirtTrack => (
                "#7a5a2a",
                format!(r#"stroke-dasharray="{:.2} {:.2}""#, 9.0 * hair, 5.0 * hair),
                1.6 * hair,
            ),
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
                    width_m.max(least)
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
        let garrisoned = held.contains(&building.owner);
        for part in &building.geometry.parts {
            let _ = write!(
                s,
                r##"<polygon points="{}" fill="{}"/>"##,
                line(
                    &mut corners(
                        part.center,
                        part.yaw,
                        [part.half_extents[0], part.half_extents[1]]
                    )
                    .into_iter()
                ),
                if garrisoned { "#e02020" } else { "#8c7b6b" }
            );
        }
    }
    // The objective's settlement and, on the wide views, its open approaches.
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
            if approach.settlement != index || close {
                continue;
            }
            let watched = p.overwatch.iter().any(|o| o.approach == Some(a));
            let to = if approach.to_rad < approach.from_rad {
                approach.to_rad + std::f64::consts::TAU
            } else {
                approach.to_rad
            };
            let (dy, dx) = ((approach.from_rad + to) / 2.0).sin_cos();
            let c = site.center;
            // From the centre to the corridor's far end: its near end is the
            // settlement's own ground, drawn over it.
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
                    "rgba(63,111,176,0.30)"
                } else {
                    "rgba(63,111,176,0.10)"
                },
                1.5 * hair,
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
        let points = line(&mut d.route.iter().copied());
        let _ = write!(
            s,
            r##"<polyline points="{points}" fill="none" stroke="#ffffff" stroke-width="{:.2}" stroke-opacity="0.8" stroke-linejoin="round"/><polyline points="{points}" fill="none" stroke="{}" stroke-width="{:.2}" stroke-linejoin="round"/>"##,
            6.0 * hair,
            colour(d.side),
            2.5 * hair
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
            2.0 * hair,
            7.0 * hair,
            5.0 * hair
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
                2.0 * hair
            );
        }
    }
    for (id, unit) in encounter.setup.units.iter().enumerate() {
        let t = rules.catalog.by_id(&unit.kind);
        let fill = colour(unit.side);
        let [x, y] = unit.position;
        if !close {
            // Too small to see at this scale: a marker, hollow over a
            // garrisoned building so the building shows.
            let hollow = p.garrisons.iter().any(|g| g.unit == id as u32);
            let _ = write!(
                s,
                r##"<circle cx="{x:.2}" cy="{:.2}" r="{:.2}" fill="{}" stroke="{}" stroke-width="{:.2}"/>"##,
                depth - y,
                if hollow { 7.0 } else { 4.5 } * hair,
                if hollow { "none" } else { fill },
                if hollow { fill } else { "#ffffff" },
                if hollow { 2.0 } else { 1.2 } * hair
            );
            continue;
        }
        // The footprint at its true size, and its label clear of it.
        let reach = match t.hull() {
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
                hull.half_extents_m[0]
            }
            None => {
                let radius =
                    sim::arrangement::spread(&rules.infantry_movement, t.squad_size()) / 2.0;
                let _ = write!(
                    s,
                    r##"<circle cx="{x:.2}" cy="{:.2}" r="{radius:.2}" fill="{fill}" fill-opacity="0.75" stroke="#ffffff" stroke-width="{:.2}"/>"##,
                    depth - y,
                    hair
                );
                radius
            }
        };
        let _ = write!(
            s,
            r##"<text x="{:.2}" y="{:.2}" font-size="{:.2}" fill="#111" stroke="#fff" stroke-width="{:.2}" paint-order="stroke">{} {}</text>"##,
            x + reach + 6.0 * hair,
            depth - y + 5.0 * hair,
            15.0 * hair,
            3.5 * hair,
            id,
            unit.kind
        );
    }
    // A scale bar of a round length, bottom left, and north.
    let bar = [5000.0, 2000.0, 1000.0, 500.0, 200.0, 100.0, 50.0, 20.0]
        .into_iter()
        .find(|b| *b <= view.side / 5.0)
        .unwrap_or(10.0);
    let (bx, by) = (view.x + 24.0 * hair, top + view.side - 24.0 * hair);
    let _ = write!(
        s,
        r##"<line x1="{bx:.2}" y1="{by:.2}" x2="{:.2}" y2="{by:.2}" stroke="#111" stroke-width="{:.2}"/><text x="{:.2}" y="{:.2}" font-size="{:.2}" fill="#111" stroke="#fff" stroke-width="{:.2}" paint-order="stroke">{bar:.0} m</text><text x="{:.2}" y="{:.2}" font-size="{:.2}" fill="#111" stroke="#fff" stroke-width="{:.2}" paint-order="stroke" text-anchor="end">N ↑</text>"##,
        bx + bar,
        5.0 * hair,
        bx + bar + 8.0 * hair,
        by + 6.0 * hair,
        17.0 * hair,
        3.5 * hair,
        view.x + view.side - 20.0 * hair,
        top + 30.0 * hair,
        20.0 * hair,
        3.5 * hair
    );
    s.push_str("</g>");
    let drive = |side: Side| {
        p.deployments
            .iter()
            .find(|d| d.side == side)
            .map_or("no column".to_string(), |d| {
                format!(
                    "{:.0} s over {:.0} m (column started {:.0} m up its road)",
                    d.route_s, d.route_m, d.advance_m
                )
            })
    };
    let count =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let lines = [
        title.to_string(),
        format!(
            "objective {} ({:.0} m zone) · jeep's drive to it: blue {} · red {}",
            p.objective.settlement,
            p.objective.radius_m,
            drive(Side::Blue),
            drive(Side::Red)
        ),
        format!(
            "{} · {} · {} · this view is {:.0} m across",
            count(p.garrisons.len(), "garrisoned squad", "garrisoned squads"),
            count(p.overwatch.len(), "overwatch post", "overwatch posts"),
            count(p.attempts as usize, "candidate tried", "candidates tried"),
            view.side
        ),
        "blue / red: units (true size in close views) · gold circle: capture zone · red building: a squad is ordered into it · red dashes: where a post looks".to_string(),
        "thick blue / red line: each column's drive · purple dashes: the objective settlement's edge · shaded strips: its measured open approaches (darker when watched)".to_string(),
    ];
    for (k, text) in lines.iter().enumerate() {
        let _ = write!(
            s,
            r##"<text x="{:.2}" y="{:.2}" font-size="{:.2}" font-weight="{}" fill="#111">{}</text>"##,
            left + 12.0 * hair,
            top - header + (30.0 + 24.0 * k as f64) * hair,
            if k == 0 { 24.0 } else { 16.0 } * hair,
            if k == 0 { "bold" } else { "normal" },
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
