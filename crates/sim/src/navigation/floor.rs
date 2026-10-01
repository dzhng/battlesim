//! Bounded navigation admission for generated forest-floor cover.
use super::{Mobility, Mover, NavGrid, MAX_CLEARANCE_M, NAV_CELL_M};
use crate::world::{Prop, WorldGeometry};

impl NavGrid {
    /// Preserve the shared-grid connectivity of every surviving local cell.
    /// The border is beyond both body stamping and the capped clearance
    /// transform, so unchanged exterior routes can splice through it.
    pub(crate) fn admit_floor_body(
        &mut self,
        world: &WorldGeometry,
        prop: Prop,
        movers: &[Mobility],
    ) -> bool {
        let radius = prop.half.x.hypot(prop.half.y)
            + MAX_CLEARANCE_M.max(self.base.soldier_radius)
            + 4.0 * NAV_CELL_M;
        let lo = |v: f64| ((v - radius) / NAV_CELL_M).floor().max(0.0) as usize;
        let hi = |v: f64, n: usize| (((v + radius) / NAV_CELL_M).ceil() as usize).min(n - 1);
        let (x0, y0, x1, y1) = (
            lo(prop.center.x),
            lo(prop.center.y),
            hi(prop.center.x, self.nx),
            hi(prop.center.y, self.ny),
        );
        let width = x1 - x0 + 1;
        let height = y1 - y0 + 1;
        // Extremely large imported floor extents cannot turn sparse generation
        // into an unbounded flood. Such a candidate is simply not admitted.
        if width * height > 4096 {
            return false;
        }
        let nx = self.nx;
        let cells: Vec<_> = (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| y * nx + x))
            .collect();
        let before: Vec<_> = movers
            .iter()
            .map(|m| self.floor_components(&cells, width, height, m))
            .collect();
        let id = prop.id;
        self.update(world, std::iter::once((id, Some(prop))), std::iter::empty());
        let safe = movers.iter().zip(before).all(|(m, before)| {
            let after = self.floor_components(&cells, width, height, m);
            let mut retained = vec![None; cells.len()];
            for k in 0..cells.len() {
                let boundary = k % width == 0
                    || k % width + 1 == width
                    || k / width == 0
                    || k / width + 1 == height;
                if boundary && before[k].is_some() && after[k].is_none() {
                    return false;
                }
                let (Some(b), Some(a)) = (before[k], after[k]) else {
                    continue;
                };
                match retained[b] {
                    Some(prior) if prior != a => return false,
                    _ => retained[b] = Some(a),
                }
            }
            true
        });
        if !safe {
            self.update(world, std::iter::once((id, None)), std::iter::empty());
        }
        safe
    }

    /// Same cell fit and shared-edge rules used by navigation, rather than
    /// a parallel geometry approximation or centre-only infantry probes.
    fn floor_components(
        &self,
        cells: &[usize],
        width: usize,
        height: usize,
        m: &Mobility,
    ) -> Vec<Option<usize>> {
        let mut labels = vec![None; cells.len()];
        for start in 0..cells.len() {
            if labels[start].is_some() || !self.fits(cells[start], Mover::free(m)) {
                continue;
            }
            labels[start] = Some(start);
            let mut queue = std::collections::VecDeque::from([start]);
            while let Some(k) = queue.pop_front() {
                let x = k % width;
                let y = k / width;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (x, y) = (x as isize + dx, y as isize + dy);
                    if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
                        continue;
                    }
                    let next = y as usize * width + x as usize;
                    if labels[next].is_none()
                        && self.fits(cells[next], Mover::free(m))
                        && self.crosses(cells[k], cells[next], m)
                    {
                        labels[next] = Some(start);
                        queue.push_back(next);
                    }
                }
            }
        }
        labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::v2;
    use crate::navigation::NavBase;
    use contract::{map::MapDefinition, scenario::Rules};
    use std::sync::Arc;

    fn floor_movers(rules: &Rules) -> [Mobility; 2] {
        ["rifle", "jeep"].map(|id| crate::units::mobility(rules.catalog.by_id(id), rules))
    }

    #[test]
    fn a_prop_only_catalog_has_no_required_infantry_or_jeep_kind() {
        let mut input = crate::fixtures::game();
        input["forests"]["rule"]["logs_per_ha"] = serde_json::json!(5);
        input["forests"]["rule"]["boulders_per_ha"] = serde_json::json!(3);
        for document in input["catalog"].as_array_mut().unwrap() {
            document.as_object_mut().unwrap().remove("units");
        }
        let rules: Rules = serde_json::from_value(input).unwrap();
        assert_eq!(rules.catalog.indices().count(), 0);
        let map: MapDefinition = serde_json::from_value(serde_json::json!({
            "size":[200,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "forests":[{"shape":{"kind":"polygon","ring":[[10,10],[190,10],[190,190],[10,190]]}}]
        }))
        .unwrap();
        let world = WorldGeometry::new(&map, &rules);
        assert!(
            world.props().any(|p| !p.forest_tree),
            "physical floor still works without unit types"
        );
    }

    #[test]
    fn isolated_cover_is_admitted_with_routes_around_it() {
        let rules: Rules = serde_json::from_value(crate::fixtures::game()).unwrap();
        let map: MapDefinition = serde_json::from_value(serde_json::json!({
            "size":[100,100], "fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "props":[{"kind":"boulder","center":[50,50],"yaw":0,"half_extents":[2,1,1]}]
        }))
        .unwrap();
        let world = WorldGeometry::new(&map, &rules);
        let mut grid = NavGrid::new(Arc::new(NavBase::build(
            &world,
            std::iter::empty(),
            rules.physics.soldier_radius_m,
        )));
        assert!(grid.admit_floor_body(
            &world,
            world.prop(0).unwrap().clone(),
            &floor_movers(&rules)
        ));
        for m in floor_movers(&rules) {
            assert!(
                !grid.fits_at(v2(50., 50.), &m),
                "accepted cover changes the held grid"
            );
            assert!(grid.route_fits(v2(40., 40.), &[v2(60., 40.)], &m));
        }
    }

    #[test]
    fn floor_admission_uses_the_current_jeep_hull_width() {
        let map: MapDefinition = serde_json::from_value(serde_json::json!({
            "size":[200,200], "fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "props":[
                {"kind":"wall","center":[100,88],"yaw":0,"half_extents":[100,2,2]},
                {"kind":"wall","center":[100,112],"yaw":0,"half_extents":[100,2,2]},
                {"kind":"boulder","center":[100,100],"yaw":0,"half_extents":[1,3,1]}
            ]
        }))
        .unwrap();
        for (half_width, accepted) in [(1.0, true), (4.0, false)] {
            let mut input = crate::fixtures::game();
            for document in input["catalog"].as_array_mut().unwrap() {
                if let Some(jeep) = document
                    .get_mut("units")
                    .and_then(|units| units.get_mut("jeep"))
                {
                    jeep["body"]["hull"]["half_extents_m"][1] = serde_json::json!(half_width);
                }
            }
            let rules: Rules = serde_json::from_value(input).unwrap();
            let world = WorldGeometry::new(&map, &rules);
            let movers = floor_movers(&rules);
            assert_eq!(movers[1].half_width_m, half_width);
            let mut grid = NavGrid::new(Arc::new(NavBase::build(
                &world,
                world.props().filter(|p| p.id != 2),
                rules.physics.soldier_radius_m,
            )));
            assert_eq!(
                grid.admit_floor_body(&world, world.prop(2).unwrap().clone(), &movers),
                accepted,
                "guard follows actual jeep width {half_width}"
            );
        }
    }

    #[test]
    fn floor_admission_rejects_a_new_pocket_and_retains_the_route() {
        let rules: Rules = serde_json::from_value(crate::fixtures::game()).unwrap();
        let map: MapDefinition = serde_json::from_value(serde_json::json!({
            "size":[200,200], "fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "props":[
                {"kind":"wall","center":[100,88],"yaw":0,"half_extents":[100,2,2]},
                {"kind":"wall","center":[100,112],"yaw":0,"half_extents":[100,2,2]},
                {"kind":"boulder","center":[100,100],"yaw":0,"half_extents":[1,10,1]}
            ]
        }))
        .unwrap();
        let world = WorldGeometry::new(&map, &rules);
        let candidate = world.prop(2).unwrap().clone();
        let mut grid = NavGrid::new(Arc::new(NavBase::build(
            &world,
            world.props().filter(|p| p.id != 2),
            rules.physics.soldier_radius_m,
        )));
        assert!(
            !grid.admit_floor_body(&world, candidate, &floor_movers(&rules)),
            "floor body must not split the only corridor"
        );
        let m = floor_movers(&rules)[1];
        assert!(
            grid.route_fits(v2(70., 100.), &[v2(130., 100.)], &m),
            "rejected candidate cannot alter the held navigation grid"
        );
    }
}
