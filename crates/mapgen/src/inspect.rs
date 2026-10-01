//! A picture of a plan's layers for a person to review: one SVG unit is one
//! metre, north is up. Tooling only; nothing here reaches the map.
use crate::layout::geometry::{add, direction, ray_crossings, scale, Point};
use crate::layout::LayoutMetrics;
use crate::MapPlan;
use contract::ground::GroundShape;
use contract::map::SurfaceKind;
use std::fmt::Write;

/// District colours, by the ids the shipped presets use; an id this table
/// does not know draws grey rather than failing.
const DISTRICTS: [(&str, &str); 8] = [
    ("farm", "#c9b27c"),
    ("village", "#dcc06a"),
    ("garden_suburb", "#f0a85e"),
    ("small_centre", "#c96a8c"),
    ("centre", "#d2452c"),
    ("apartments", "#8c2f3f"),
    ("core", "#4b2a6b"),
    ("industrial", "#6f8196"),
];
const PLAIN: &str = "#efe9d3";
const FIELD: &str = "#dfe3b4";
const FOREST: &str = "#6f9e63";
const APPROACH: &str = "#3f8fd0";
const ROAD: &str = "#1c1c1c";
const TRACK: &str = "#7a5a2e";

pub fn svg(plan: &MapPlan, title: &str, metrics: &LayoutMetrics) -> String {
    let [width, height] = plan.size;
    // Text and line weights follow the map's size so every size reads alike.
    let unit = width / 100.0;
    let header = 9.0 * unit;
    let footer = 8.0 * unit;
    let mut out = String::new();
    let path = |points: &[Point], close: bool| {
        let mut d = String::new();
        for (index, p) in points.iter().enumerate() {
            let _ = write!(
                d,
                "{}{:.1} {:.1}",
                if index == 0 { "M" } else { "L" },
                p[0],
                height - p[1]
            );
        }
        if close {
            d.push('Z');
        }
        d
    };
    let _ = write!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{x} {y} {w} {h}" font-family="Helvetica, Arial, sans-serif"><rect x="{x}" y="{y}" width="{w}" height="{h}" fill="#ffffff"/><rect width="{width}" height="{height}" fill="{PLAIN}"/>"##,
        x = -unit,
        y = -header,
        w = width + 2.0 * unit,
        h = height + header + footer
    );
    // What a settlement's outline holds beyond its districts is field, unless
    // a wood stands there.
    for settlement in &plan.settlements {
        let _ = write!(
            out,
            r##"<path d="{}" fill="{FIELD}"/>"##,
            path(&settlement.outline, true)
        );
    }
    // Open approaches under everything they lead to: the main settlement's
    // filled, the others' in outline so that they do not bury the map.
    for approach in &plan.approaches {
        let settlement = &plan.settlements[approach.settlement];
        let steps = (((approach.to_rad - approach.from_rad) / 0.03) as usize).max(1);
        let rays: Vec<(Point, f64)> = (0..=steps)
            .map(|step| {
                let angle = approach.from_rad
                    + (approach.to_rad - approach.from_rad) * step as f64 / steps as f64;
                let toward = direction(angle);
                let edge = ray_crossings(settlement.center, toward, &settlement.outline)
                    .fold(0.0, f64::max);
                (toward, edge)
            })
            .collect();
        let near = rays
            .iter()
            .map(|(toward, edge)| add(settlement.center, scale(*toward, *edge)));
        let far = rays
            .iter()
            .rev()
            .map(|(toward, edge)| add(settlement.center, scale(*toward, edge + approach.depth_m)));
        let wedge: Vec<Point> = near.chain(far).collect();
        let main = approach.settlement == 0;
        let _ = write!(
            out,
            r##"<path d="{}" fill="{APPROACH}" fill-opacity="{}" stroke="{APPROACH}" stroke-opacity="{}" stroke-width="{}"/>"##,
            path(&wedge, true),
            if main { 0.22 } else { 0.0 },
            if main { 0.9 } else { 0.3 },
            unit * if main { 0.12 } else { 0.06 }
        );
    }
    for forest in &plan.forests {
        if let GroundShape::Polygon { ring } = &forest.shape {
            let _ = write!(out, r##"<path d="{}" fill="{FOREST}"/>"##, path(ring, true));
        }
    }
    let colour = |kind: &str| {
        DISTRICTS
            .iter()
            .find(|(known, _)| *known == kind)
            .map_or("#999999", |(_, fill)| fill)
    };
    for settlement in &plan.settlements {
        for district in &settlement.districts {
            let _ = write!(
                out,
                r##"<path d="{}" fill="{}" stroke="#3a2f2a" stroke-width="{}"/>"##,
                path(&district.ring, true),
                colour(&district.kind),
                unit * 0.06
            );
        }
        let _ = write!(
            out,
            r##"<path d="{}" fill="none" stroke="#6b5d3a" stroke-width="{}" stroke-dasharray="{dash} {dash}"/>"##,
            path(&settlement.outline, true),
            unit * 0.05,
            dash = unit * 0.3
        );
    }
    // Tracks under roads; both drawn wider than life so they read at this scale.
    let track_dash = format!(r#" stroke-dasharray="{} {}""#, unit * 0.6, unit * 0.4);
    for kind in [SurfaceKind::DirtTrack, SurfaceKind::CountryRoad] {
        for area in plan.surfaces.iter().filter(|area| area.kind == kind) {
            if let GroundShape::Stroke { centerline, .. } = &area.shape {
                let (stroke, weight, dash) = if kind == SurfaceKind::DirtTrack {
                    (TRACK, 0.16, track_dash.as_str())
                } else {
                    (ROAD, 0.3, "")
                };
                let _ = write!(
                    out,
                    r##"<path d="{}" fill="none" stroke="{stroke}" stroke-width="{}" stroke-linejoin="round"{dash}/>"##,
                    path(centerline.samples(), false),
                    unit * weight
                );
            }
        }
    }
    let _ = write!(
        out,
        r##"<rect width="{width}" height="{height}" fill="none" stroke="#111111" stroke-width="{}"/><path d="M0 {mid}H{width}" stroke="#111111" stroke-opacity="0.35" stroke-width="{}" stroke-dasharray="{unit} {unit}"/><circle cx="{}" cy="{mid}" r="{}" fill="#d0021b"/>"##,
        unit * 0.3,
        unit * 0.08,
        width / 2.0,
        unit * 0.45,
        mid = height / 2.0
    );

    // Below the map: what each mark means, then a one-kilometre scale bar
    // and the district kinds this plan uses.
    let row = |line: f64| height + unit * (2.6 + 2.8 * line);
    let _ = write!(
        out,
        r##"<path d="M0 {y}h1000" stroke="#111111" stroke-width="{}"/><text x="{}" y="{}" font-size="{}">1 km</text>"##,
        unit * 0.4,
        1000.0 + unit,
        row(1.0) + 0.6 * unit,
        unit * 1.8,
        y = row(1.0)
    );
    let label = |out: &mut String, x: &mut f64, line: f64, text: &str| {
        let _ = write!(
            out,
            r##"<text x="{}" y="{}" font-size="{}">{text}</text>"##,
            *x + unit * 2.4,
            row(line) + 0.55 * unit,
            unit * 1.5
        );
        *x += unit * (4.2 + 0.8 * text.len() as f64);
    };
    let block = |out: &mut String, x: f64, line: f64, fill: &str, opacity: f64| {
        let _ = write!(
            out,
            r##"<rect x="{x}" y="{}" width="{}" height="{}" fill="{fill}" fill-opacity="{opacity}" stroke="#3a2f2a" stroke-width="{}"/>"##,
            row(line) - 0.8 * unit,
            unit * 1.8,
            unit * 1.6,
            unit * 0.05
        );
    };
    let mut x = 0.0;
    for (stroke, weight, dash, text) in [
        (ROAD, 0.3, "", "country road"),
        (TRACK, 0.16, track_dash.as_str(), "dirt track"),
    ] {
        let _ = write!(
            out,
            r##"<path d="M{x} {}h{}" stroke="{stroke}" stroke-width="{}"{dash}/>"##,
            row(0.0),
            unit * 1.8,
            unit * weight
        );
        label(&mut out, &mut x, 0.0, text);
    }
    for (fill, opacity, text) in [
        (FOREST, 1.0, "forest"),
        (FIELD, 1.0, "settlement field"),
        (
            APPROACH,
            0.22,
            "open approach (filled: to the main settlement)",
        ),
    ] {
        block(&mut out, x, 0.0, fill, opacity);
        label(&mut out, &mut x, 0.0, text);
    }
    let mut x = 1000.0 + 7.0 * unit;
    for (kind, fill) in DISTRICTS {
        let used = plan
            .settlements
            .iter()
            .flat_map(|settlement| &settlement.districts)
            .any(|district| district.kind == kind);
        if used {
            block(&mut out, x, 1.0, fill, 1.0);
            label(&mut out, &mut x, 1.0, &kind.replace('_', " "));
        }
    }

    let km2 = |m2: f64| m2 / 1e6;
    let classes: Vec<String> = metrics
        .settlements
        .iter()
        .map(|(class, count)| format!("{count} {}", class.replace('_', " ")))
        .collect();
    let transit = metrics
        .transit
        .iter()
        .map(|edge| edge.elapsed_s)
        .fold(0.0, f64::max);
    let fair = |fair: bool| if fair { "fair" } else { "UNFAIR" };
    let lines = [
        title.to_string(),
        format!(
            "built {:.1}%  forest {:.1}%  open {:.1}%  |  {}  |  main settlement {:.0}% of built ground",
            metrics.urban_share * 100.0,
            metrics.forest_share * 100.0,
            metrics.plain_share * 100.0,
            classes.join(", "),
            metrics.main_settlement_share * 100.0
        ),
        format!(
            "top/bottom: built {:.2}/{:.2} km² ({}), forest {:.2}/{:.2} km² ({}), approaches {}/{}  |  roads {:.0} km, tracks {:.0} km, loops {}, edge exits {}  |  slowest edge to centre {:.0} s",
            km2(metrics.town.top_m2),
            km2(metrics.town.bottom_m2),
            fair(metrics.town.fair),
            km2(metrics.forest.top_m2),
            km2(metrics.forest.bottom_m2),
            fair(metrics.forest.fair),
            metrics.approaches_top,
            metrics.approaches_bottom,
            metrics.roads.country_road_km,
            metrics.roads.dirt_track_km,
            metrics.roads.loops,
            metrics.roads.edge_exits,
            transit
        ),
    ];
    for (index, line) in lines.iter().enumerate() {
        let _ = write!(
            out,
            r##"<text x="0" y="{}" font-size="{}"{}>{}</text>"##,
            -header + unit * (3.2 + 2.3 * index as f64),
            unit * if index == 0 { 3.2 } else { 1.4 },
            if index == 0 {
                r#" font-weight="bold""#
            } else {
                ""
            },
            line.replace('&', "&amp;").replace('<', "&lt;")
        );
    }
    out.push_str("</svg>\n");
    out
}
