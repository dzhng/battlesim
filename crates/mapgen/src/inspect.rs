//! A picture of a plan's layers for a person to review: one SVG unit is one
//! metre, north is up. The whole map, or one settlement or district of it.
//! Tooling only; nothing here reaches the map.
use crate::layout::corridor_start;
use crate::layout::geometry::{add, direction, scale, sub, Point};
use crate::layout::LayoutMetrics;
use crate::MapPlan;
use contract::ground::GroundShape;
use contract::map::SurfaceKind;
use contract::templates::{BuildingCategory, TemplateGeometryCatalog};
use std::collections::BTreeMap;
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
/// Building colours by category, dark enough to read on a district's tint.
const CATEGORIES: [(BuildingCategory, &str, &str); 6] = [
    (BuildingCategory::Farmstead, "farmstead", "#6b4423"),
    (BuildingCategory::DetachedHome, "detached home", "#c2410c"),
    (BuildingCategory::AttachedHome, "attached home", "#9d174d"),
    (BuildingCategory::UrbanApartment, "apartment", "#5b21b6"),
    (BuildingCategory::Highrise, "highrise", "#0f172a"),
    (BuildingCategory::Industry, "industry", "#1d4ed8"),
];
const PLAIN: &str = "#efe9d3";
const FIELD: &str = "#dfe3b4";
const FOREST: &str = "#6f9e63";
const APPROACH: &str = "#3f8fd0";
const WATER: &str = "#2b6cb0";
const DECK: &str = "#f59e0b";
const ROAD: &str = "#1c1c1c";
const TRACK: &str = "#7a5a2e";
const STREET: &str = "#4a4a4a";
const PAVING: &str = "#a9a9a9";
const YARD: &str = "#c2410c";
const BODY: &str = "#57534e";
const ENTRANCE: &str = "#ffd43b";
/// Street furniture, by what a reviewer looks for: (label, colour, the prop
/// kinds drawn in it). A kind this table does not know draws as the last.
const FURNITURE: [(&str, &str, &[&str]); 6] = [
    ("parked car", "#e11d48", &["parked_car"]),
    ("street tree", "#15803d", &["street_tree"]),
    ("lamp", "#facc15", &["lamp"]),
    (
        "site cabin and fence",
        "#f97316",
        &["site_cabin", "heras_fence"],
    ),
    ("skip and pallets", "#a16207", &["skip_bin", "pallet_stack"]),
    ("other furniture", "#0ea5e9", &[]),
];

/// The row of `FURNITURE` a prop kind is drawn by.
fn furniture(kind: &str) -> usize {
    FURNITURE
        .iter()
        .position(|(_, _, kinds)| kinds.contains(&kind))
        .unwrap_or(FURNITURE.len() - 1)
}
/// Parcels and entrances are drawn when the view is at most this wide: on a
/// whole map they are below a pixel.
const DETAIL_VIEW_M: f64 = 2_600.0;

/// The part of the map a picture shows, `[min_x, min_y, width, height]`: the
/// ground about the settlement, district or bridge `crop` names (`bridge-2`
/// is the plan's third), the rectangle it spells (`x,y,width,height`), or
/// the whole map.
fn view(plan: &MapPlan, crop: Option<&str>) -> Result<[f64; 4], String> {
    let Some(crop) = crop else {
        return Ok([0.0, 0.0, plan.size[0], plan.size[1]]);
    };
    let bridge = crop
        .strip_prefix("bridge-")
        .and_then(|index| plan.bridges.get(index.parse::<usize>().ok()?));
    if let Some(bridge) = bridge {
        // Ten deck lengths square: the deck, the roads onto it and both banks.
        let side = 20.0 * bridge.half_extents[0];
        let [x, y] = bridge.center;
        return Ok([x - side / 2.0, y - side / 2.0, side, side]);
    }
    let numbers: Vec<f64> = crop.split(',').filter_map(|v| v.parse().ok()).collect();
    if let [x, y, width, height] = numbers[..] {
        if width > 0.0 && height > 0.0 {
            return Ok([x, y, width, height]);
        }
    }
    let ring = plan
        .settlements
        .iter()
        .find_map(|settlement| {
            if settlement.id == crop {
                return Some(&settlement.outline);
            }
            let district = settlement.districts.iter().find(|d| d.id == crop)?;
            Some(&district.ring)
        })
        .ok_or_else(|| {
            format!("{crop:?} is no settlement, district, bridge or x,y,width,height")
        })?;
    let [x0, y0, x1, y1] = contract::ground::limits(ring, 0.0);
    // A square view with a margin, so the neighbours show.
    let side = 1.25 * (x1 - x0).max(y1 - y0);
    Ok([(x0 + x1 - side) / 2.0, (y0 + y1 - side) / 2.0, side, side])
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct LegendEntry {
    pub label: String,
    pub color: String,
}
/// Geometry and its explanations have one formatting and palette owner.
#[derive(Debug, serde::Serialize)]
pub struct Picture {
    pub svg: String,
    pub summary: Vec<String>,
    pub legend: Vec<LegendEntry>,
}

pub fn svg(
    plan: &MapPlan,
    catalogue: &TemplateGeometryCatalog,
    title: &str,
    metrics: &LayoutMetrics,
    crop: Option<&str>,
) -> Result<String, String> {
    Ok(render(plan, catalogue, title, metrics, crop, true)?.svg)
}

/// The workbench places these notes in responsive HTML beside a geometry-only picture.
pub fn picture(
    plan: &MapPlan,
    catalogue: &TemplateGeometryCatalog,
    title: &str,
    metrics: &LayoutMetrics,
    crop: Option<&str>,
) -> Result<Picture, String> {
    render(plan, catalogue, title, metrics, crop, false)
}

fn render(
    plan: &MapPlan,
    catalogue: &TemplateGeometryCatalog,
    title: &str,
    metrics: &LayoutMetrics,
    crop: Option<&str>,
    standalone: bool,
) -> Result<Picture, String> {
    let [left, bottom, width, height] = view(plan, crop)?;
    // North is up: the picture's Y runs down from the view's top edge.
    let top = bottom + height;
    let detail = width <= DETAIL_VIEW_M;
    // Text and line weights follow the view's size so every picture reads alike.
    let unit = width / 100.0;
    let header = if standalone { 13.0 * unit } else { unit };
    let footer = if standalone { 11.0 * unit } else { unit };
    let mut legend = Vec::new();
    let mut out = String::new();
    let path = |points: &[Point], close: bool| {
        let mut d = String::new();
        for (index, p) in points.iter().enumerate() {
            let _ = write!(
                d,
                "{}{:.1} {:.1}",
                if index == 0 { "M" } else { "L" },
                p[0] - left,
                top - p[1]
            );
        }
        if close {
            d.push('Z');
        }
        d
    };
    let shown = |p: Point, margin: f64| {
        p[0] >= left - margin
            && p[0] <= left + width + margin
            && p[1] >= bottom - margin
            && p[1] <= top + margin
    };
    let _ = write!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{x} {y} {w} {h}" font-family="Helvetica, Arial, sans-serif"><rect x="{x}" y="{y}" width="{w}" height="{h}" fill="#ffffff"/><clipPath id="view"><rect width="{width}" height="{height}"/></clipPath><g clip-path="url(#view)"><rect width="{width}" height="{height}" fill="{PLAIN}"/>"##,
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
            r##"<path data-layer="districts" data-rule-group="classes.{class}" data-feature-id="{id}" d="{}" fill="{FIELD}"/>"##,
            path(&settlement.outline, true),
            class = xml(&settlement.class),
            id = xml(&settlement.id)
        );
    }
    // The widest open approach in each half to the first settlement (the
    // main one of a generated plan), under everything it leads to: the
    // corridor along its middle bearing, from where `measure` starts it.
    // The rest are counted in the header and not drawn: they would bury the
    // map.
    let widest = |half: crate::Half| {
        plan.approaches
            .iter()
            .filter(|a| a.settlement == 0 && a.half == half)
            .max_by(|a, b| (a.to_rad - a.from_rad).total_cmp(&(b.to_rad - b.from_rad)))
    };
    for approach in [crate::Half::Top, crate::Half::Bottom]
        .into_iter()
        .filter_map(widest)
    {
        let settlement = &plan.settlements[approach.settlement];
        let toward = direction((approach.from_rad + approach.to_rad) / 2.0);
        let aside = scale([-toward[1], toward[0]], approach.front_m / 2.0);
        let edge = corridor_start(
            &settlement.outline,
            settlement.center,
            toward,
            approach.front_m,
        );
        let near = add(settlement.center, scale(toward, edge));
        let far = add(near, scale(toward, approach.depth_m));
        let corridor = [
            sub(near, aside),
            add(near, aside),
            add(far, aside),
            sub(far, aside),
        ];
        let _ = write!(
            out,
            r##"<path data-layer="approaches" data-rule-group="approach" data-feature-id="approach" d="{}" fill="{APPROACH}" fill-opacity="0.05" stroke="{APPROACH}" stroke-opacity="0.45" stroke-width="{}" stroke-dasharray="{dash} {dash}"/>"##,
            path(&corridor, true),
            unit * 0.08,
            dash = unit * 0.5
        );
    }
    // A wood is its ring; a tree line its stroke, and a copse or a single
    // tree a ring drawn with an edge wide enough to find on a whole map.
    for forest in &plan.forests {
        match &forest.shape {
            GroundShape::Polygon { ring } => {
                let _ = write!(
                    out,
                    r##"<path data-layer="forests" data-rule-group="forests" data-feature-id="forests" d="{}" fill="{FOREST}" stroke="{FOREST}" stroke-width="{}"/>"##,
                    path(ring, true),
                    unit * 0.1
                );
            }
            GroundShape::Stroke {
                centerline,
                width_m,
            } => {
                let _ = write!(
                    out,
                    r##"<path data-layer="forests" data-rule-group="forests" data-feature-id="forests" d="{}" fill="none" stroke="{FOREST}" stroke-width="{}"/>"##,
                    path(centerline.samples(), false),
                    width_m.max(unit * 0.25)
                );
            }
        }
    }
    for ring in crate::layout::water::rings(&plan.rivers) {
        // A hairline of its own colour closes the seams between its rings.
        let _ = write!(
            out,
            r##"<path data-layer="water" data-rule-group="rivers" data-feature-id="rivers" d="{}" fill="{WATER}" stroke="{WATER}" stroke-width="{}"/>"##,
            path(&ring, true),
            unit * 0.02
        );
    }
    let colour = |kind: &str| {
        DISTRICTS
            .iter()
            .find(|(known, _)| *known == kind)
            .map_or("#999999", |(_, fill)| fill)
    };
    // A district is a tint once it is built on, so its buildings read.
    let tint = if plan.buildings.is_empty() { 1.0 } else { 0.3 };
    for settlement in &plan.settlements {
        for district in &settlement.districts {
            let _ = write!(
                out,
                r##"<path data-layer="districts" data-rule-group="districts.{kind}" data-feature-id="{id}" d="{}" fill="{}" fill-opacity="{tint}" stroke="#3a2f2a" stroke-width="{}"/>"##,
                path(&district.ring, true),
                colour(&district.kind),
                unit * 0.06,
                kind = xml(&district.kind),
                id = xml(&district.id)
            );
        }
        let _ = write!(
            out,
            r##"<path data-layer="districts" data-rule-group="classes.{class}" data-feature-id="{id}" d="{}" fill="none" stroke="#6b5d3a" stroke-width="{}" stroke-dasharray="{dash} {dash}"/>"##,
            path(&settlement.outline, true),
            unit * 0.05,
            dash = unit * 0.3,
            class = xml(&settlement.class),
            id = xml(&settlement.id)
        );
    }
    if detail {
        for lot in plan.lots.iter().filter(|lot| shown(lot.ring[0], 200.0)) {
            let _ = write!(
                out,
                r##"<path data-layer="parcels" data-rule-group="parcels" data-feature-id="parcels" d="{}" fill="#ffffff" fill-opacity="0.35" stroke="#5c5346" stroke-width="{}"/>"##,
                path(&lot.ring, true),
                unit * 0.04
            );
        }
    }
    // The yards of homes out in the country, at any scale: on a whole map
    // they are the only sign of a house.
    for lot in crate::open_country::country_lots(plan) {
        let _ = write!(
            out,
            r##"<path data-layer="parcels" data-rule-group="open_country" data-feature-id="open_country" d="{}" fill="{YARD}" fill-opacity="0.45" stroke="{YARD}" stroke-width="{}"/>"##,
            path(&lot.ring, true),
            unit * 0.12
        );
    }
    // Paving: aprons and courts, hard ground that is no way through.
    for area in plan
        .surfaces
        .iter()
        .filter(|a| a.kind == SurfaceKind::Paving)
    {
        if let GroundShape::Polygon { ring } = &area.shape {
            let _ = write!(
                out,
                r##"<path data-layer="roads" data-rule-group="parcels" data-feature-id="parcels" d="{}" fill="{PAVING}"/>"##,
                path(ring, true)
            );
        }
    }
    // Loose bodies: low cover in the fields, a car in a yard.
    for prop in &plan.props {
        let _ = write!(
            out,
            r##"<circle data-layer="furniture" data-rule-group="open_country" data-feature-id="open_country" cx="{}" cy="{}" r="{}" fill="{BODY}"/>"##,
            prop.center[0] - left,
            top - prop.center[1],
            prop.half_extents[0].max(unit * 0.12)
        );
    }
    // Tracks under roads under streets. Each is drawn at its own width, or
    // wider when that would be too thin to read at this scale.
    let track_dash = format!(r#" stroke-dasharray="{} {}""#, unit * 0.6, unit * 0.4);
    let marks = [
        (
            SurfaceKind::DirtTrack,
            TRACK,
            0.16,
            track_dash.as_str(),
            "dirt track",
        ),
        (SurfaceKind::CountryRoad, ROAD, 0.3, "", "country road"),
        (SurfaceKind::Road, STREET, 0.1, "", "street"),
    ];
    for (kind, stroke, weight, dash, _) in marks {
        for area in plan.surfaces.iter().filter(|area| area.kind == kind) {
            if let GroundShape::Stroke {
                centerline,
                width_m,
            } = &area.shape
            {
                let _ = write!(
                    out,
                    r##"<path data-layer="roads" data-rule-group="roads" data-feature-id="roads" d="{}" fill="none" stroke="{stroke}" stroke-width="{}" stroke-linejoin="round"{dash}/>"##,
                    path(centerline.samples(), false),
                    width_m.max(unit * weight)
                );
            }
        }
    }
    // A deck over its road; on a view too wide to show one, a ring round it.
    for (bridge_index, bridge) in plan.bridges.iter().enumerate() {
        let ends = bridge.ends();
        let _ = write!(
            out,
            r##"<path data-layer="water" data-rule-group="crossings" data-feature-id="bridge-{bridge_index}" d="{}" fill="{DECK}" stroke="#111111" stroke-width="{}"/>"##,
            path(&[ends[0][0], ends[0][1], ends[1][1], ends[1][0]], true),
            unit * 0.05
        );
        if !detail {
            let _ = write!(
                out,
                r##"<circle data-layer="water" data-rule-group="crossings" data-feature-id="bridge-{bridge_index}" cx="{}" cy="{}" r="{}" fill="none" stroke="{DECK}" stroke-width="{}"/>"##,
                bridge.center[0] - left,
                top - bridge.center[1],
                unit * 0.9,
                unit * 0.25
            );
        }
    }
    let mut built: BTreeMap<&str, usize> = BTreeMap::new();
    for building in &plan.buildings {
        let template = catalogue
            .templates()
            .iter()
            .find(|template| template.id == building.template_id)
            .ok_or_else(|| format!("the catalogue has no template {}", building.template_id))?;
        let (_, name, fill) = CATEGORIES
            .iter()
            .find(|(category, ..)| *category == template.category)
            .ok_or("a category without a colour")?;
        *built.entry(name).or_default() += 1;
        let [x, y, _] = building.frame.translation;
        if !shown([x, y], 200.0) {
            continue;
        }
        let placed = template.materialize(building.frame)?;
        for part in &placed.parts {
            let along = direction(part.yaw);
            let across = [-along[1], along[0]];
            let corners = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]].map(|[u, v]| {
                add(
                    part.center,
                    add(
                        scale(along, u * part.half_extents[0]),
                        scale(across, v * part.half_extents[1]),
                    ),
                )
            });
            let _ = write!(
                out,
                r##"<path data-layer="buildings" data-rule-group="parcels" data-feature-id="parcels" d="{}" fill="{fill}"/>"##,
                path(&corners, true)
            );
        }
        if detail {
            // An entrance is a tick out from its door.
            for entrance in placed.entrances.iter().flatten() {
                let door = [entrance.position[0], entrance.position[1]];
                let _ = write!(
                    out,
                    r##"<path data-layer="buildings" data-rule-group="parcels" data-feature-id="parcels" d="{}" stroke="{ENTRANCE}" stroke-width="{}"/>"##,
                    path(&[door, add(door, scale(entrance.normal, 3.0))], false),
                    unit * 0.12
                );
            }
        }
    }
    // Street furniture and any other authored body, each as its own box: on
    // a whole map they are below a pixel.
    let mut placed = [0usize; FURNITURE.len()];
    for prop in &plan.props {
        placed[furniture(&prop.kind)] += 1;
        if !detail || !shown(prop.center, 10.0) {
            continue;
        }
        let along = direction(prop.yaw);
        let across = [-along[1], along[0]];
        // A post is drawn no thinner than this, so it shows at all.
        let half = [prop.half_extents[0], prop.half_extents[1]].map(|half| half.max(0.25));
        let corners = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]].map(|[u, v]| {
            add(
                prop.center,
                add(scale(along, u * half[0]), scale(across, v * half[1])),
            )
        });
        let _ = write!(
            out,
            r##"<path data-layer="furniture" data-rule-group="street_props" data-feature-id="street_props" d="{}" fill="{}" stroke="#111111" stroke-width="0.06"/>"##,
            path(&corners, true),
            FURNITURE[furniture(&prop.kind)].1
        );
    }
    let _ = write!(
        out,
        r##"</g><rect width="{width}" height="{height}" fill="none" stroke="#111111" stroke-width="{}"/>"##,
        unit * 0.3,
    );
    if crop.is_none() {
        let _ = write!(
            out,
            r##"<path d="M0 {mid}H{width}" stroke="#111111" stroke-opacity="0.35" stroke-width="{}" stroke-dasharray="{unit} {unit}"/><circle cx="{}" cy="{mid}" r="{}" fill="#d0021b"/>"##,
            unit * 0.08,
            width / 2.0,
            unit * 0.45,
            mid = height / 2.0
        );
    }

    // Below the map: what each mark means, then a scale bar and the district
    // kinds and building categories this plan uses.
    let row = |line: f64| height + unit * (2.6 + 2.6 * line);
    let bar = [50.0, 100.0, 200.0, 500.0, 1000.0]
        .into_iter()
        .rfind(|metres| *metres <= width / 6.0)
        .unwrap_or(50.0);
    if standalone {
        let _ = write!(
            out,
            r##"<path d="M0 {y}h{bar}" stroke="#111111" stroke-width="{}"/><text x="{}" y="{}" font-size="{}">{}</text>"##,
            unit * 0.4,
            bar + unit,
            row(1.0) + 0.6 * unit,
            unit * 1.8,
            if bar >= 1000.0 {
                "1 km".to_string()
            } else {
                format!("{bar} m")
            },
            y = row(1.0)
        );
    }
    let mut label = |out: &mut String, x: &mut f64, line: f64, text: &str, color: &str| {
        legend.push(LegendEntry {
            label: text.into(),
            color: color.into(),
        });
        if standalone {
            let _ = write!(
                out,
                r##"<text x="{}" y="{}" font-size="{}">{text}</text>"##,
                *x + unit * 2.4,
                row(line) + 0.55 * unit,
                unit * 1.5
            );
        }
        *x += unit * (4.2 + 0.8 * text.len() as f64);
    };
    let block = |out: &mut String, x: f64, line: f64, fill: &str, opacity: f64| {
        if standalone {
            let _ = write!(
                out,
                r##"<rect x="{x}" y="{}" width="{}" height="{}" fill="{fill}" fill-opacity="{opacity}" stroke="#3a2f2a" stroke-width="{}"/>"##,
                row(line) - 0.8 * unit,
                unit * 1.8,
                unit * 1.6,
                unit * 0.05
            );
        }
    };
    let mut x = 0.0;
    for (_, stroke, weight, dash, text) in marks.iter().rev() {
        if standalone {
            let _ = write!(
                out,
                r##"<path d="M{x} {}h{}" stroke="{stroke}" stroke-width="{}"{dash}/>"##,
                row(0.0),
                unit * 1.8,
                unit * weight.max(0.2)
            );
        }
        label(&mut out, &mut x, 0.0, text, stroke);
    }
    let water = [(WATER, 1.0, "river"), (DECK, 1.0, "bridge")];
    for (fill, opacity, text) in [(FOREST, 1.0, "forest"), (APPROACH, 0.22, "open approach")]
        .into_iter()
        .chain(detail.then_some((PAVING, 1.0, "paving")))
        .chain(water.into_iter().filter(|_| !plan.rivers.is_empty()))
    {
        block(&mut out, x, 0.0, fill, opacity);
        label(&mut out, &mut x, 0.0, text, fill);
    }
    if detail {
        if standalone {
            let _ = write!(
                out,
                r##"<path d="M{x} {}h{}" stroke="{ENTRANCE}" stroke-width="{}"/>"##,
                row(0.0),
                unit * 1.8,
                unit * 0.3
            );
        }
        label(&mut out, &mut x, 0.0, "entrance", ENTRANCE);
    }
    let mut x = bar + 9.0 * unit;
    for (kind, fill) in DISTRICTS {
        let used = plan
            .settlements
            .iter()
            .flat_map(|settlement| &settlement.districts)
            .any(|district| district.kind == kind);
        if used {
            // The tint as it is drawn: over a settlement's field.
            block(&mut out, x, 1.0, FIELD, 1.0);
            block(&mut out, x, 1.0, fill, tint);
            label(&mut out, &mut x, 1.0, &kind.replace('_', " "), fill);
        }
    }
    let mut x = 0.0;
    for (_, name, fill) in CATEGORIES {
        if let Some(count) = built.get(name) {
            block(&mut out, x, 2.0, fill, 1.0);
            label(&mut out, &mut x, 2.0, &format!("{count} {name}"), fill);
        }
    }
    if detail {
        for ((name, fill, _), count) in FURNITURE.iter().zip(placed) {
            if count > 0 {
                block(&mut out, x, 2.0, fill, 1.0);
                label(&mut out, &mut x, 2.0, &format!("{count} {name}"), fill);
            }
        }
    }

    let km2 = |m2: f64| m2 / 1e6;
    let classes: Vec<String> = metrics
        .settlements
        .iter()
        .map(|(class, count)| format!("{count} {}", class.replace('_', " ")))
        .collect();
    let seconds = |journey: &Option<crate::layout::Journey>| {
        journey.as_ref().map_or("no road".into(), |journey| {
            format!("{:.0} s", journey.elapsed_s)
        })
    };
    let transit = &metrics.transit;
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
            "top/bottom: built {:.2}/{:.2} km² ({}), forest {:.2}/{:.2} km² ({}), approaches {}/{}  |  roads {:.0} km, tracks {:.0} km, loops {}, exits {}",
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
            metrics.roads.edge_exits
        ),
        format!(
            "by road to the centre: from the top {}, from the bottom {}  |  bottom to top {}  |  side to side {}",
            seconds(&transit.top),
            seconds(&transit.bottom),
            seconds(&transit.top_bottom),
            seconds(&transit.east_west)
        ),
        format!(
            "whole map: {} buildings on {} parcels, {} street props  |  streets {:.0} km  |  river {:.1} km, {} bridges  |  this view is {:.0} m wide",
            plan.buildings.len(),
            plan.lots.len(),
            plan.props.len(),
            metrics.roads.street_km,
            metrics.river.top_km + metrics.river.bottom_km,
            plan.bridges.len(),
            width
        ),
    ];
    if standalone {
        for (index, line) in lines.iter().enumerate() {
            let _ = write!(
                out,
                r##"<text x="0" y="{}" font-size="{}"{}>{}</text>"##,
                -header + unit * (3.2 + 2.1 * index as f64),
                unit * if index == 0 { 3.2 } else { 1.4 },
                if index == 0 {
                    r#" font-weight="bold""#
                } else {
                    ""
                },
                line.replace('&', "&amp;").replace('<', "&lt;")
            );
        }
    }
    out.push_str("</svg>\n");
    let mut summary = lines.to_vec();
    if let Some(crop) = crop {
        summary.push(format!("Inspection view: {crop}"));
    }
    Ok(Picture {
        svg: out,
        summary,
        legend,
    })
}
