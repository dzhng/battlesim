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

pub fn admit_skirmish(q: &crate::encounter::MapQueries, rules: &Rules)
    -> Result<Vec<SkirmishJourney>, String> {
    use contract::catalog::Mobility;
    use contract::command::RoutePolicy;
    use crate::navigation::{self, Leg, Plan};
    q.sites.validate()?;
    let sites = q.sites.skirmish.as_ref().ok_or("skirmish sites are missing")?;
    let mut representatives = [None; 3];
    for kind in rules.catalog.indices() {
        let unit = rules.catalog.get(kind);
        let class = match unit.mobility { Mobility::Foot {..} => 0,
            Mobility::Tracked {..} => 1, Mobility::Wheeled {..} => 2 };
        let width = crate::units::mobility(unit,rules).half_width_m;
        if representatives[class].is_none_or(|(old,_)| width > old) {
            representatives[class] = Some((width,kind));
        }
    }
    if representatives.iter().any(Option::is_none) { return Err("representative foot/tracked/wheeled movers are required".into()); }
    let mut results = Vec::new();
    for (_,kind) in representatives.into_iter().flatten() {
        let unit = rules.catalog.get(kind);
        let mobility = crate::units::mobility(unit,rules);
        for entry in &sites.entries {
            let from = v2(entry.center[0],entry.center[1]);
            crate::encounter::legality::stands(q,rules,unit,&mobility,from,entry.yaw)
                .map_err(|e| format!("entry {:?} cannot admit {}: {e:?}",entry.side,rules.catalog.id(kind)))?;
            for objective in &sites.objectives {
                let goal = v2(objective.center[0],objective.center[1]);
                if q.map.forests.iter().any(|f| f.shape.contains(objective.center,objective.radius_m)) { return Err(format!("objective {} reaches forest ground",objective.id)); }
                let (plan,_) = navigation::plan(q.grid,q.roads,Leg {from,goal,m:&mobility,
                    policy:RoutePolicy::Fastest,avoid:&[]},&rules.navigation);
                let Plan::Route(route) = plan else { return Err(format!("no route for {} to {}",rules.catalog.id(kind),objective.id)); };
                if route.last().is_some_and(|p| (*p-goal).length() > mobility.half_width_m+2.0)
                    || !q.grid.route_fits(from,&route,&mobility) {
                    return Err(format!("route to {} does not reach its site",objective.id));
                }
                results.push(SkirmishJourney {unit:rules.catalog.id(kind).into(),side:entry.side,
                    objective:objective.id.clone(),travel_s:q.grid.route_time(from,&route,&mobility)});
            }
        }
        let travel = |id:&str,side| results.iter().find(|r| r.unit==rules.catalog.id(kind)
            && r.objective==id && r.side==side).unwrap().travel_s;
        for objective in &sites.objectives {
            let (a,b) = if let Some(other) = &objective.counterpart {
                let mate = sites.objectives.iter().find(|o| &o.id==other).ok_or("unknown counterpart")?;
                let own = |y:f64| if y > q.world.depth()/2.0 {contract::ids::Side::Blue} else {contract::ids::Side::Red};
                let first = q.world.ground_surface_at(objective.center[0],objective.center[1]).unwrap();
                let second = q.world.ground_surface_at(mate.center[0],mate.center[1]).unwrap();
                if first.kind != second.kind || first.road_factor != second.road_factor || objective.kind!=mate.kind {
                    return Err(format!("counterpart makeup/access differs for {}",objective.id));
                }
                (travel(&objective.id,own(objective.center[1])),travel(other,own(mate.center[1])))
            } else { (travel(&objective.id,contract::ids::Side::Blue),travel(&objective.id,contract::ids::Side::Red)) };
            if !a.is_finite() || !b.is_finite() || (a-b).abs() > 0.15*a.max(b) {
                return Err(format!("objective travel differs beyond 15% for {}: {a:.2}/{b:.2} s",objective.id));
            }
        }
    }
    Ok(results)
}

#[cfg(test)]
mod skirmish_tests {
    #[test]
    fn unequal_objective_routes_are_refused() {
        let rules: contract::scenario::Rules = serde_json::from_value(crate::fixtures::game()).unwrap();
        let map: contract::map::MapDefinition = serde_json::from_str(
            r#"{"size":[1000,1000],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#
        ).unwrap();
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
        let prepared = crate::encounter::PreparedMap::new(&map,&rules);
        let result = super::admit_skirmish(&prepared.queries(&map,&sites), &rules);
        assert!(result.unwrap_err().contains("travel"));
    }
}
