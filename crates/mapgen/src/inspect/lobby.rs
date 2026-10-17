//! The player's picture of an admitted skirmish map, for the lobby (D39):
//! the compiled map's ground, water, forest, roads and buildings, simplified
//! to what reads at lobby size, under the admitted sites' objectives and each
//! side's entries. Turned a quarter so Blue's edge (the map's north) faces
//! left (D23): a map point `[x, y]` is drawn at `[height - y, width - x]`.
//!
//! Terrain has this picture's own dark palette. The marks a player reads
//! (side accents, text) are the HUD's, by its `--hud-*` custom properties
//! (`web/src/battle/present/hudTheme.ts`), so the page that shows the
//! picture supplies them and the picture holds no second copy.
use super::xml;
use crate::layout::geometry::{direction, Point};
use contract::encounter::SkirmishSites;
use contract::ground::GroundShape;
use contract::ids::Side;
use contract::map::{MapDefinition, SurfaceKind};
use std::fmt::Write;

const GROUND: &str = "#223528";
const FOREST: &str = "#0c2414";
const FOREST_EDGE: &str = "#33683f";
const WATER: &str = "#1f5a7c";
/// The ground's faint grid of dots: a map, not a void.
const GRAIN: &str = "#2c4434";
const ROAD: &str = "#c9bf9f";
const ROAD_CASING: &str = "#4d4a3f";
const STREET: &str = "#4a5048";
const TRACK: &str = "#968a6a";
/// Dimmer than every mark, so a town never competes with an objective in it.
const BUILDING: &str = "#7f878b";
/// The dark edge every mark carries so it reads over any ground.
const HALO: &str = "#05090d";

/// Line weights and mark sizes, in hundredths of the map's longer side: the
/// lobby shows every size of map in about the same frame, so a mark keeps
/// its size on screen whatever the map's.
struct Weights {
    country_road: f64,
    casing: f64,
    street: f64,
    track: f64,
    /// A wood narrower than this is a speck at lobby size, and is left out.
    least_wood: f64,
    /// Water is drawn this much wider than its banks, so a stream reads.
    water_edge: f64,
    /// The ground grid's spacing.
    grain: f64,
    /// Each building is drawn this much larger, so a house is a mark at all.
    building_edge: f64,
    objective_ring: f64,
    objective_stroke: f64,
    numeral: f64,
    entry_length: f64,
    entry_half_width: f64,
    halo: f64,
}
const WEIGHTS: Weights = Weights {
    country_road: 0.7,
    casing: 1.1,
    street: 0.2,
    track: 0.3,
    least_wood: 1.5,
    water_edge: 0.8,
    grain: 4.0,
    building_edge: 0.16,
    objective_ring: 2.0,
    objective_stroke: 0.4,
    numeral: 2.6,
    entry_length: 3.6,
    entry_half_width: 2.0,
    halo: 0.55,
};
/// Points closer than this share of the longer side to the last one kept are
/// dropped: below a pixel at lobby size.
const SIMPLIFY: f64 = 1.0 / 800.0;

/// The picture's SVG: one SVG unit is one metre.
pub fn skirmish_picture(map: &MapDefinition, sites: &SkirmishSites) -> String {
    let [width, height] = map.size;
    let unit = width.max(height) / 100.0;
    let step = width.max(height) * SIMPLIFY;
    let at = |p: Point| [height - p[1], width - p[0]];
    let path = |points: &[Point], close: bool| {
        let mut d = String::new();
        let mut last: Option<Point> = None;
        for (index, &p) in points.iter().enumerate() {
            let end = index + 1 == points.len();
            if let Some(q) = last {
                if !end && libm::hypot(p[0] - q[0], p[1] - q[1]) < step {
                    continue;
                }
            }
            let [x, y] = at(p);
            let _ = write!(d, "{}{x:.0} {y:.0}", if last.is_none() { "M" } else { "L" });
            last = Some(p);
        }
        if close {
            d.push('Z');
        }
        d
    };
    // The pattern is in metres, so pictures of different sizes on one page
    // each need their own.
    let grain_id = format!("lobby-map-grain-{:.0}", unit * 100.0);
    let mut out = String::new();
    let _ = write!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {height} {width}" data-map-picture=""><defs><radialGradient id="lobby-map-vignette"><stop offset="0.6" stop-color="#000" stop-opacity="0"/><stop offset="1" stop-color="#000" stop-opacity="0.35"/></radialGradient><pattern id="{grain_id}" width="{grain:.1}" height="{grain:.1}" patternUnits="userSpaceOnUse"><rect width="{grain:.1}" height="{grain:.1}" fill="{GROUND}"/><circle cx="{dot:.1}" cy="{dot:.1}" r="{dot:.1}" fill="{GRAIN}"/></pattern></defs><rect width="{height}" height="{width}" fill="url(#{grain_id})"/>"##,
        grain = unit * WEIGHTS.grain,
        dot = unit * 0.12,
    );

    // Woods as one shape. Tree lines, copses and lone trees are specks at
    // this size, and are left out.
    let mut woods = String::new();
    for forest in &map.forests {
        if let GroundShape::Polygon { ring } = &forest.shape {
            let [x0, y0, x1, y1] = contract::ground::limits(ring, 0.0);
            if (x1 - x0).max(y1 - y0) >= unit * WEIGHTS.least_wood {
                woods.push_str(&path(ring, true));
            }
        }
    }
    let _ = write!(
        out,
        r##"<path data-layer="forest" d="{woods}" fill="{FOREST}" stroke="{FOREST_EDGE}" stroke-width="{:.1}" stroke-linejoin="round"/>"##,
        unit * 0.2
    );

    // Water as one shape, edged in its own colour: that closes the seams
    // between its rings and keeps a narrow stream visible.
    let water: String = crate::layout::water::rings(&map.rivers)
        .iter()
        .map(|ring| path(ring, true))
        .collect();
    let _ = write!(
        out,
        r##"<path data-layer="water" d="{water}" fill="{WATER}" stroke="{WATER}" stroke-width="{:.1}" stroke-linejoin="round"/>"##,
        unit * WEIGHTS.water_edge
    );

    // Every building part as its footprint, in one shape.
    let mut buildings = String::new();
    for building in &map.buildings {
        for part in &building.geometry.parts {
            let along = direction(part.yaw);
            let across = [-along[1], along[0]];
            let [a, b, _] = part.half_extents;
            let corner = |u: f64, v: f64| {
                [
                    part.center[0] + along[0] * u * a + across[0] * v * b,
                    part.center[1] + along[1] * u * a + across[1] * v * b,
                ]
            };
            let corners = [
                corner(-1.0, -1.0),
                corner(1.0, -1.0),
                corner(1.0, 1.0),
                corner(-1.0, 1.0),
            ];
            let mut d = String::new();
            for (index, p) in corners.iter().enumerate() {
                let [x, y] = at(*p);
                let _ = write!(d, "{}{x:.0} {y:.0}", if index == 0 { "M" } else { "L" });
            }
            buildings.push_str(&d);
            buildings.push('Z');
        }
    }
    let _ = write!(
        out,
        r##"<path data-layer="buildings" d="{buildings}" fill="{BUILDING}" stroke="{BUILDING}" stroke-width="{:.1}" stroke-linejoin="round"/>"##,
        unit * WEIGHTS.building_edge
    );
    // Roads over the buildings, whose drawn edge would otherwise eat into
    // them: tracks, then streets, then the country roads over a casing that
    // keeps them apart from the ground. One path per kind.
    let strokes = |kind: SurfaceKind| -> String {
        map.surfaces
            .iter()
            .filter(|area| area.kind == kind)
            .filter_map(|area| match &area.shape {
                GroundShape::Stroke { centerline, .. } => Some(path(centerline.samples(), false)),
                GroundShape::Polygon { .. } => None,
            })
            .collect()
    };
    let country = strokes(SurfaceKind::CountryRoad);
    for (d, colour, weight, dash) in [
        (strokes(SurfaceKind::DirtTrack), TRACK, WEIGHTS.track, true),
        (strokes(SurfaceKind::Road), STREET, WEIGHTS.street, false),
        (country.clone(), ROAD_CASING, WEIGHTS.casing, false),
        (country, ROAD, WEIGHTS.country_road, false),
    ] {
        let dash = if dash {
            format!(r#" stroke-dasharray="{:.1} {:.1}""#, unit * 0.8, unit * 0.5)
        } else {
            String::new()
        };
        let _ = write!(
            out,
            r##"<path data-layer="roads" d="{d}" fill="none" stroke="{colour}" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round"{dash}/>"##,
            unit * weight
        );
    }
    // A bridge is its deck, in the road's colour, across the water.
    let decks: String = map
        .bridges
        .iter()
        .map(|bridge| {
            let ends = bridge.ends();
            path(&[ends[0][0], ends[0][1], ends[1][1], ends[1][0]], true)
        })
        .collect();
    let _ = write!(
        out,
        r##"<path data-layer="roads" d="{decks}" fill="{ROAD}"/>"##
    );

    let _ = write!(
        out,
        r##"<rect width="{height}" height="{width}" fill="url(#lobby-map-vignette)"/>"##
    );

    // Objectives: a ring the size of the capture area (never smaller than a
    // mark reads) over a dark disc that lifts it off a town or a junction, numbered as the
    // battle's score strip numbers them.
    for (index, objective) in sites.objectives.iter().enumerate() {
        let [x, y] = at(objective.center);
        let radius = objective.radius_m.max(unit * WEIGHTS.objective_ring);
        let _ = write!(
            out,
            r##"<g data-objective="{id}" transform="translate({x:.1} {y:.1})"><circle r="{radius:.1}" fill="none" stroke="{HALO}" stroke-opacity="0.8" stroke-width="{halo:.1}"/><circle r="{radius:.1}" fill="{HALO}" fill-opacity="0.75" style="stroke:rgb(var(--hud-text))" stroke-width="{ring:.1}"/><text dy="0.35em" text-anchor="middle" font-size="{size:.1}" font-weight="700" font-family="ui-monospace, Menlo, monospace" stroke="{HALO}" stroke-width="{text_halo:.1}" stroke-linejoin="round" paint-order="stroke" style="fill:rgb(var(--hud-text))">{n}</text></g>"##,
            id = xml(&objective.id),
            halo = unit * (WEIGHTS.objective_stroke + WEIGHTS.halo),
            ring = unit * WEIGHTS.objective_stroke,
            size = unit * WEIGHTS.numeral,
            text_halo = unit * WEIGHTS.halo,
            n = index + 1,
        );
    }

    // Entries: an arrowhead in its side's accent, from the edge the entry
    // stands at straight into the map, wholly inside the frame.
    for entry in &sites.entries {
        let [x, y] = at(entry.center);
        // The picture's edges, nearest first: the distance to each and the
        // way into the map from it, in SVG degrees (clockwise, Y down).
        let edges = [(x, 0.0), (height - x, 180.0), (y, 90.0), (width - y, 270.0)];
        let (edge, degrees) = edges
            .into_iter()
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .expect("four edges");
        let (side, accent) = match entry.side {
            Side::Blue => ("blue", "--hud-accent"),
            Side::Red | Side::Neutral => ("red", "--hud-enemy"),
        };
        // Its base stands its halo's width in from the frame, so the whole
        // arrowhead shows.
        let base = unit * WEIGHTS.halo - edge;
        let half = unit * WEIGHTS.entry_half_width;
        let _ = write!(
            out,
            r##"<g data-entry="{side}" transform="translate({x:.1} {y:.1}) rotate({degrees})"><path d="M{base:.1} {top:.1}L{tip:.1} 0L{base:.1} {half:.1}Z" stroke="{HALO}" stroke-width="{halo:.1}" stroke-linejoin="round" paint-order="stroke" style="fill:rgb(var({accent}))"/></g>"##,
            top = -half,
            tip = base + unit * WEIGHTS.entry_length,
            halo = unit * WEIGHTS.halo,
        );
    }
    out.push_str("</svg>");
    out
}
