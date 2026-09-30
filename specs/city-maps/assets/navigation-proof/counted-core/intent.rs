use super::topology::Point;
// Accepted rule data. Battle integration must carry this in its rules identity.
pub const AUTOMATIC_ROAD_TRIGGER_M: f64 = 2_000.0;
#[derive(Clone, Copy, Debug)]
pub struct QueuedMove {
    pub goal: Point,
}
#[derive(Clone, Copy, Debug)]
pub struct ActivatedMove {
    goal: Point,
    origin: Point,
    automatic_roads: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct ReplanIntent {
    pub start: Point,
    pub goal: Point,
    pub automatic_roads: bool,
}
impl QueuedMove {
    pub fn activate(self, start: Point) -> ActivatedMove {
        ActivatedMove {
            goal: self.goal,
            origin: start,
            automatic_roads: start.distance(self.goal) > AUTOMATIC_ROAD_TRIGGER_M,
        }
    }
}
impl ActivatedMove {
    pub fn replan_from(self, start: Point) -> ReplanIntent {
        ReplanIntent {
            start,
            goal: self.goal,
            automatic_roads: self.automatic_roads,
        }
    }
    pub fn automatic_roads(self) -> bool {
        self.automatic_roads
    }
    pub fn origin(self) -> Point {
        self.origin
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepted_threshold_is_strict_and_frozen_at_activation() {
        let mut results = Vec::new();
        for distance in [1999.999, 2000.0, 2000.001] {
            let active = QueuedMove {
                goal: Point(distance, 0.0),
            }
            .activate(Point(0.0, 0.0));
            results.push(active.automatic_roads());
            assert_eq!(
                active.replan_from(Point(-3000.0, 0.0)).automatic_roads,
                active.automatic_roads()
            );
            assert_eq!(
                active
                    .replan_from(Point(distance - 1.0, 0.0))
                    .automatic_roads,
                active.automatic_roads()
            );
        }
        assert_eq!(results, vec![false, false, true]);
    }
    #[test]
    fn queued_leg_decides_from_its_actual_activation_pose() {
        let queued = QueuedMove {
            goal: Point(3000.0, 0.0),
        };
        assert!(queued.activate(Point(0.0, 0.0)).automatic_roads());
        let later = queued.activate(Point(2500.0, 0.0));
        assert!(!later.automatic_roads());
        assert_eq!(later.origin(), Point(2500.0, 0.0));
        assert_eq!(later.replan_from(Point(0.0, 0.0)).goal, queued.goal);
        assert!(!later.replan_from(Point(0.0, 0.0)).automatic_roads);
    }
}
