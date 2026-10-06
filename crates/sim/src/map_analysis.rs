//! The sight circle (M25), measured with the simulation's own sight: from a
//! place on the map, the share of evenly spaced bearings on which an
//! infantry eye sees all the way to its sight range. A bearing is open when
//! the line from the eye to the ground target at full range (or at the
//! map's edge, where that is nearer) is clear by the simulation's two sight
//! queries: no body or ground that occludes (`sight_clear`) and no foliage
//! (`foliage_depth`), which shortens sight however little of it there is.
//! Those are what `sim::sensing::sees_point` and the fog's sweep are made
//! of. Nothing here says what blocks sight.
use crate::math::{v2, v3, V2};
use crate::sight::Sight;
use crate::world::WorldGeometry;
use contract::encounter::EncounterSites;
use contract::scenario::Rules;

/// The unit type whose sight is the standard infantry sight.
pub const INFANTRY: &str = "rifle";

/// The standard infantry sight: an even circle of the rifle squad's range.
pub fn infantry_sight(rules: &Rules) -> Sight {
    let sensors = &rules.catalog.by_id(INFANTRY).sensors;
    Sight {
        forward: 0.0,
        shape: sensors.sight_shape,
        range: sensors.ground_m,
    }
}

/// How many bearings a circle is judged on: one a fog cell apart at full
/// range, as the fog's own sweep casts them.
pub fn bearings(world_fog_cell_m: f64, sight: &Sight) -> usize {
    ((std::f64::consts::TAU * sight.max_range() / world_fog_cell_m).ceil() as usize).max(64)
}

/// The share of `bearings` on which an infantry eye at `at` sees to full
/// range; `None` where no eye can stand (off the map).
pub fn open_share(
    world: &WorldGeometry,
    rules: &Rules,
    sight: &Sight,
    at: V2,
    bearings: usize,
) -> Option<f64> {
    let ground = world.height_at(at.x, at.y)?;
    let eye = v3(at.x, at.y, ground + rules.physics.infantry_eye_m);
    let (width, depth) = (world.width(), world.depth());
    let mut open = 0;
    for index in 0..bearings {
        let angle = index as f64 / bearings as f64 * std::f64::consts::TAU;
        let (dx, dy) = (angle.cos(), angle.sin());
        // To full range, or to just inside the map's edge.
        let mut reach = sight.range_at(angle) - 0.01;
        for (from, along, limit) in [(at.x, dx, width), (at.y, dy, depth)] {
            if along > 0.0 {
                reach = reach.min((limit - 0.5 - from) / along);
            } else if along < 0.0 {
                reach = reach.min((from - 0.5) / -along);
            }
        }
        if reach <= 0.0 {
            open += 1;
            continue;
        }
        let end = v2(at.x + dx * reach, at.y + dy * reach);
        let Some(far) = world.height_at(end.x, end.y) else {
            continue;
        };
        let target = v3(end.x, end.y, far + rules.sensors.fog_target_height_m);
        if world.foliage_depth(eye, target) == 0.0 && world.sight_clear(eye, target) {
            open += 1;
        }
    }
    Some(open as f64 / bearings as f64)
}

/// Whether `p` is open ground: dry, outside every settlement's outline and
/// off forest ground (a wood, a copse or a tree line), with no body on it.
pub fn open_ground(world: &WorldGeometry, sites: &EncounterSites, p: V2) -> bool {
    world.traversable_at(p.x, p.y)
        && !world.forest_ground(p.x, p.y)
        && world.props_near(p, 1.0).is_empty()
        && !sites
            .settlements
            .iter()
            .any(|site| contract::ground::polygon_contains(&site.outline, [p.x, p.y]))
}

/// The open ground of a map, sampled on a square grid `step` apart.
pub fn sample_points(world: &WorldGeometry, sites: &EncounterSites, step: f64) -> Vec<V2> {
    let mut points = Vec::new();
    let mut y = step / 2.0;
    while y < world.depth() {
        let mut x = step / 2.0;
        while x < world.width() {
            let p = v2(x, y);
            if open_ground(world, sites, p) {
                points.push(p);
            }
            x += step;
        }
        y += step;
    }
    points
}

/// Sorted shares, asked for a quantile in `[0, 1]`.
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

/// Sampled analysis policy. It assesses a report; it never admits a generated map.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisPolicy {
    pub step_m: f64,
    pub min_median_open: f64,
}
impl AnalysisPolicy {
    pub fn errors(&self) -> Vec<(&'static str, &'static str)> {
        let mut errors = Vec::new();
        if !self.step_m.is_finite() || self.step_m <= 0.0 {
            errors.push(("step_m", "Sample spacing must be finite and positive"));
        }
        if !self.min_median_open.is_finite() || !(0.0..=1.0).contains(&self.min_median_open) {
            errors.push((
                "min_median_open",
                "Minimum median openness must be between zero and one",
            ));
        }
        errors
    }
}

/// Route evidence from the same placement and navigation used by a battle.
#[derive(Debug, serde::Serialize)]
pub struct SkirmishJourney {
    pub unit: String,
    pub side: contract::ids::Side,
    pub objective: String,
    pub travel_s: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct SkirmishAdmission {
    pub sites: contract::encounter::EncounterSites,
    pub journeys: Vec<SkirmishJourney>,
}

pub fn admit_skirmish(
    q: &crate::encounter::MapQueries,
    rules: &Rules,
) -> Result<SkirmishAdmission, String> {
    use crate::navigation::{self, Leg, Plan};
    use contract::catalog::Mobility;
    use contract::command::RoutePolicy;
    use contract::ids::Side;
    q.sites.validate()?;
    let sites = q
        .sites
        .skirmish
        .as_ref()
        .ok_or("skirmish sites are missing")?;
    let mut representatives = [None; 3];
    for kind in rules.catalog.indices() {
        let unit = rules.catalog.get(kind);
        let class = match unit.mobility {
            Mobility::Foot { .. } => 0,
            Mobility::Tracked { .. } => 1,
            Mobility::Wheeled { .. } => 2,
        };
        let width = crate::units::mobility(unit, rules).half_width_m;
        if representatives[class].is_none_or(|(old, _)| width > old) {
            representatives[class] = Some((width, kind));
        }
    }
    if representatives.iter().any(Option::is_none) {
        return Err("representative foot/tracked/wheeled movers are required".into());
    }
    let representatives: Vec<_> = representatives
        .into_iter()
        .flatten()
        .map(|(_, kind)| kind)
        .collect();
    for &kind in &representatives {
        let unit = rules.catalog.get(kind);
        let mobility = crate::units::mobility(unit, rules);
        for entry in &sites.entries {
            crate::encounter::legality::stands(
                q,
                rules,
                unit,
                &mobility,
                v2(entry.center[0], entry.center[1]),
                entry.yaw,
            )
            .map_err(|e| {
                format!(
                    "entry {:?} cannot admit {}: {e:?}",
                    entry.side,
                    rules.catalog.id(kind)
                )
            })?;
        }
    }
    let measure =
        |objective: &contract::encounter::ObjectiveSite| -> Result<Vec<SkirmishJourney>, String> {
            if q.map
                .forests
                .iter()
                .any(|f| f.shape.contains(objective.center, objective.radius_m))
            {
                return Err(format!("objective {} reaches forest ground", objective.id));
            }
            let mut journeys = Vec::new();
            for &kind in &representatives {
                let mobility = crate::units::mobility(rules.catalog.get(kind), rules);
                for entry in &sites.entries {
                    let from = v2(entry.center[0], entry.center[1]);
                    let goal = v2(objective.center[0], objective.center[1]);
                    let (plan, _) = navigation::plan(
                        q.grid,
                        q.roads,
                        Leg {
                            from,
                            goal,
                            m: &mobility,
                            policy: RoutePolicy::Fastest,
                            avoid: &[],
                        },
                        &rules.navigation,
                    );
                    let Plan::Route(route) = plan else {
                        return Err(format!(
                            "no route for {} to {}",
                            rules.catalog.id(kind),
                            objective.id
                        ));
                    };
                    if route
                        .last()
                        .is_some_and(|p| (*p - goal).length() > mobility.half_width_m + 2.0)
                        || !q.grid.route_fits(from, &route, &mobility)
                    {
                        return Err(format!("route to {} does not reach its site", objective.id));
                    }
                    journeys.push(SkirmishJourney {
                        unit: rules.catalog.id(kind).into(),
                        side: entry.side,
                        objective: objective.id.clone(),
                        travel_s: q.grid.route_time(from, &route, &mobility),
                    });
                }
            }
            Ok(journeys)
        };
    let fair = |a: f64, b: f64| a.is_finite() && b.is_finite() && (a - b).abs() <= 0.15 * a.max(b);
    let separated = |a: [f64; 2], b: [f64; 2]| libm::hypot(a[0] - b[0], a[1] - b[1]) >= 150.0;
    let mut selected = Vec::new();
    let mut journeys = Vec::new();
    for requested in sites.objectives.iter().filter(|o| o.counterpart.is_none()) {
        let mut admitted = None;
        for objective in std::iter::once(requested).chain(
            sites
                .candidates
                .iter()
                .filter(|o| o.counterpart.is_none() && o.kind == requested.kind),
        ) {
            if (objective.center[1] - q.world.depth() / 2.0).abs() > 0.01
                || selected
                    .iter()
                    .any(|s: &contract::encounter::ObjectiveSite| {
                        !separated(s.center, objective.center)
                    })
                || sites
                    .entries
                    .iter()
                    .any(|e| !separated(e.center, objective.center))
            {
                continue;
            }
            let Ok(measured) = measure(objective) else {
                continue;
            };
            if measured
                .as_chunks::<2>()
                .0
                .iter()
                .all(|r| fair(r[0].travel_s, r[1].travel_s))
            {
                admitted = Some((objective.clone(), measured));
                break;
            }
        }
        let Some((objective, measured)) = admitted else {
            return Err(format!(
                "central objective candidates exhausted: travel differs beyond 15% for {}",
                requested.id
            ));
        };
        selected.push(objective);
        journeys.extend(measured);
    }
    let required = (sites.objectives.len() - selected.len()) / 2;
    let mut pool = Vec::new();
    for objective in sites
        .all_reserved_objectives()
        .filter(|o| o.counterpart.is_some())
    {
        if selected
            .iter()
            .any(|s| !separated(s.center, objective.center))
            || sites
                .entries
                .iter()
                .any(|e| !separated(e.center, objective.center))
        {
            continue;
        }
        if let Ok(measured) = measure(objective) {
            pool.push((objective, measured));
        }
    }
    let own = |y: f64| {
        if y > q.world.depth() / 2.0 {
            Side::Blue
        } else {
            Side::Red
        }
    };
    let mut matches = Vec::new();
    for (i, (a, aj)) in pool.iter().enumerate() {
        for (j, (b, bj)) in pool.iter().enumerate().skip(i + 1) {
            if own(a.center[1]) == own(b.center[1])
                || a.kind != b.kind
                || a.radius_m != b.radius_m
                || !separated(a.center, b.center)
            {
                continue;
            }
            let Some(first) = q.world.ground_surface_at(a.center[0], a.center[1]) else {
                continue;
            };
            let Some(second) = q.world.ground_surface_at(b.center[0], b.center[1]) else {
                continue;
            };
            if first.kind != second.kind || first.road_factor != second.road_factor {
                continue;
            }
            let mut score: f64 = 0.0;
            let mut accepted = true;
            for (ar, br) in aj
                .as_chunks::<2>()
                .0
                .iter()
                .zip(bj.as_chunks::<2>().0.iter())
            {
                let at = ar
                    .iter()
                    .find(|r| r.side == own(a.center[1]))
                    .unwrap()
                    .travel_s;
                let bt = br
                    .iter()
                    .find(|r| r.side == own(b.center[1]))
                    .unwrap()
                    .travel_s;
                accepted &= fair(at, bt);
                score = score.max((at - bt).abs() / at.max(bt));
            }
            if accepted {
                matches.push((score, i, j));
            }
        }
    }
    matches.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut chosen = Vec::new();
    fn choose(
        matches: &[(f64, usize, usize)],
        pool: &[(&contract::encounter::ObjectiveSite, Vec<SkirmishJourney>)],
        start: usize,
        required: usize,
        chosen: &mut Vec<(usize, usize)>,
    ) -> bool {
        if chosen.len() == required {
            return true;
        }
        for (offset, (_, i, j)) in matches[start..].iter().enumerate() {
            if chosen.iter().any(|(a, b)| {
                [*a, *b].iter().any(|k| {
                    [*i, *j].iter().any(|l| {
                        libm::hypot(
                            pool[*k].0.center[0] - pool[*l].0.center[0],
                            pool[*k].0.center[1] - pool[*l].0.center[1],
                        ) < 150.0
                    })
                })
            }) {
                continue;
            }
            chosen.push((*i, *j));
            let next = start + offset + 1;
            if choose(matches, pool, next, required, chosen) {
                return true;
            }
            chosen.pop();
        }
        false
    }
    if !choose(&matches, &pool, 0, required, &mut chosen) {
        return Err("reserved objective candidates exhausted: no spaced counterpart set matches access and travel within 15%".into());
    }
    for (i, j) in chosen {
        let mut a = pool[i].0.clone();
        let mut b = pool[j].0.clone();
        a.counterpart = Some(b.id.clone());
        b.counterpart = Some(a.id.clone());
        selected.extend([a, b]);
        for index in [i, j] {
            journeys.append(&mut pool[index].1);
        }
    }
    let mut admitted = q.sites.clone();
    let skirmish = admitted.skirmish.as_mut().unwrap();
    skirmish.objectives = selected;
    skirmish.candidates.clear();
    admitted.validate()?;
    Ok(SkirmishAdmission {
        sites: admitted,
        journeys,
    })
}

#[cfg(test)]
mod skirmish_tests {
    #[test]
    fn admission_returns_route_selected_sites() {
        let rules: contract::scenario::Rules =
            serde_json::from_value(crate::fixtures::game()).unwrap();
        let map: contract::map::MapDefinition = serde_json::from_str(
            r#"{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#,
        )
        .unwrap();
        let mut sites: serde_json::Value = serde_json::json!({"settlements":[],"approaches":[],"skirmish":{
            "entries":[{"side":"blue","center":[500,990],"yaw":-1.57},{"side":"red","center":[500,10],"yaw":1.57}],
            "objectives":[
                {"id":"center","center":[500,500],"radius_m":50,"kind":"junction","counterpart":null},
                {"id":"bad-a","center":[500,790],"radius_m":50,"kind":"field","counterpart":"bad-b"},
                {"id":"bad-b","center":[500,110],"radius_m":50,"kind":"field","counterpart":"bad-a"}],
            "candidates":[
                {"id":"good-a","center":[400,750],"radius_m":50,"kind":"field","counterpart":"good-b"},
                {"id":"good-b","center":[600,250],"radius_m":50,"kind":"field","counterpart":"good-a"}]}});
        let prepared = crate::encounter::PreparedMap::new(&map, &rules);
        let base = sites["skirmish"]["candidates"].take();
        sites["skirmish"]
            .as_object_mut()
            .unwrap()
            .remove("candidates");
        let without = serde_json::from_value(sites.clone()).unwrap();
        assert!(super::admit_skirmish(&prepared.queries(&map, &without), &rules).is_err());
        sites["skirmish"]["candidates"] = base;
        let with = serde_json::from_value(sites).unwrap();
        let admitted = super::admit_skirmish(&prepared.queries(&map, &with), &rules).unwrap();
        let value = serde_json::to_value(admitted).unwrap();
        let repeated = super::admit_skirmish(&prepared.queries(&map, &with), &rules).unwrap();
        assert_eq!(value, serde_json::to_value(repeated).unwrap());
        assert_eq!(value["sites"]["skirmish"]["objectives"][1]["id"], "good-a");
        assert_eq!(
            value["sites"]["skirmish"]["objectives"][1]["center"],
            serde_json::json!([400.0, 750.0])
        );
        assert!(value["journeys"]
            .as_array()
            .unwrap()
            .iter()
            .all(|j| !j["objective"].as_str().unwrap().starts_with("bad")));
    }

    #[test]
    fn central_fields_use_fair_reserved_centerline_locations() {
        let rules: contract::scenario::Rules =
            serde_json::from_value(crate::fixtures::game()).unwrap();
        let map: contract::map::MapDefinition = serde_json::from_str(
            r#"{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#,
        )
        .unwrap();
        let objective = |id: &str, x: f64, y: f64, mate: Option<&str>, kind: &str| {
            serde_json::json!({
            "id":id,"center":[x,y],"radius_m":50,"kind":kind,"counterpart":mate})
        };
        let mut sites = serde_json::json!({"settlements":[],"approaches":[],"skirmish":{
            "entries":[{"side":"blue","center":[600,990],"yaw":-1.57},{"side":"red","center":[400,10],"yaw":1.57}],
            "objectives":[objective("hub",500.0,500.0,None,"junction"),
                objective("bad-left",50.0,500.0,None,"field"),objective("bad-right",950.0,500.0,None,"field"),
                objective("a",200.0,750.0,Some("b"),"field"),objective("b",800.0,250.0,Some("a"),"field"),
                objective("c",800.0,750.0,Some("d"),"field"),objective("d",200.0,250.0,Some("c"),"field")]}});
        let prepared = crate::encounter::PreparedMap::new(&map, &rules);
        let base = serde_json::from_value(sites.clone()).unwrap();
        assert!(super::admit_skirmish(&prepared.queries(&map, &base), &rules).is_err());
        sites["skirmish"]["candidates"] = serde_json::json!([
            objective("left", 300.0, 500.0, None, "field"),
            objective("right", 700.0, 500.0, None, "field")
        ]);
        let candidates = serde_json::from_value(sites).unwrap();
        let admitted = super::admit_skirmish(&prepared.queries(&map, &candidates), &rules).unwrap();
        let objectives = &admitted.sites.skirmish.unwrap().objectives;
        assert_eq!(objectives[1].center, [300.0, 500.0]);
        assert_eq!(objectives[2].center, [700.0, 500.0]);
    }

    #[test]
    fn unequal_objective_routes_are_refused() {
        let rules: contract::scenario::Rules =
            serde_json::from_value(crate::fixtures::game()).unwrap();
        let map: contract::map::MapDefinition = serde_json::from_str(
            r#"{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#,
        )
        .unwrap();
        let sites: contract::encounter::EncounterSites = serde_json::from_value(serde_json::json!({
            "settlements": [], "approaches": [], "skirmish": {
                "entries": [{"side":"blue","center":[500,990],"yaw":-1.57},
                    {"side":"red","center":[500,10],"yaw":1.57}],
                "objectives": [
                    {"id":"center","center":[500,500],"radius_m":50,"kind":"junction","counterpart":null},
                    {"id":"a","center":[500,790],"radius_m":50,"kind":"field","counterpart":"b"},
                    {"id":"b","center":[500,110],"radius_m":50,"kind":"field","counterpart":"a"}
                ]
            }
        })).unwrap();
        let prepared = crate::encounter::PreparedMap::new(&map, &rules);
        let result = super::admit_skirmish(&prepared.queries(&map, &sites), &rules);
        assert!(result.unwrap_err().contains("travel"));
    }
}
