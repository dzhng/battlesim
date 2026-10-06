//! Capture and score use physical combat presence, never the observer's contacts.
use contract::catalog::Category;
use contract::encounter::ObjectiveSite;
use contract::ids::Side;
use contract::scenario::Rules;
use contract::skirmish::{MatchResult, ObjectiveView};

use crate::digest::Digest;
use crate::math::v2;
use crate::units::Unit;

const CAPTURE_SECONDS: u64 = 20;
const VICTORY_SCORE: f64 = 1000.0;
const MAJORITY_SECONDS: u64 = 1800;

#[derive(Default)]
struct Objective {
    owner: Option<Side>,
    capturing: Option<Side>,
    capture_ticks: u64,
    contested: bool,
}

/// A score crossing may finish partway through the final fixed tick.
struct FinalFraction {
    numerator: u64,
    denominator: u64,
    rates: [u64; 2],
}

pub struct Objectives {
    sites: Vec<Objective>,
    flag_ticks: [u64; 2],
    final_fraction: Option<FinalFraction>,
    pub result: Option<MatchResult>,
}

impl Objectives {
    pub fn new(count: usize) -> Self {
        Self {
            sites: (0..count).map(|_| Objective::default()).collect(),
            flag_ticks: [0; 2],
            final_fraction: None,
            result: None,
        }
    }

    fn threshold(&self, hz: u32) -> u64 {
        MAJORITY_SECONDS * (self.sites.len() as u64 / 2 + 1) * u64::from(hz)
    }

    pub fn advance(&mut self, geography: &[ObjectiveSite], units: &[Unit], rules: &Rules) {
        let presence: Vec<[bool; 2]> = geography
            .iter()
            .map(|site| {
                let center = v2(site.center[0], site.center[1]);
                let inside = |p: crate::math::V2| (p - center).within_radius(site.radius_m);
                let mut present = [false; 2];
                for unit in units.iter().filter(|u| u.alive()) {
                    let kind = rules.catalog.get(unit.kind);
                    let eligible = kind.roster.as_ref().is_some_and(|r| {
                        matches!(r.category, Category::Rec | Category::Inf | Category::Veh)
                    });
                    if !eligible {
                        continue;
                    }
                    let holds = if unit.hull.is_some() {
                        inside(unit.position.xy())
                    } else {
                        unit.member_positions().any(|p| inside(p.xy()))
                    };
                    present[unit.side.index()] |= holds;
                }
                present
            })
            .collect();
        self.advance_presence(&presence, rules.tick_hz);
    }

    fn advance_presence(&mut self, presence: &[[bool; 2]], hz: u32) {
        if self.result.is_some() {
            return;
        }
        let mut rates = [0; 2];
        for (site, present) in self.sites.iter_mut().zip(presence) {
            site.contested =
                present[0] && present[1] || site.owner.is_some_and(|s| present[1 - s.index()]);
            if let Some(owner) = site.owner {
                if !present[1 - owner.index()] {
                    rates[owner.index()] += 1;
                }
            }
        }
        let threshold = self.threshold(hz);
        let crossing: [Option<u64>; 2] = std::array::from_fn(|i| {
            let needed = threshold.saturating_sub(self.flag_ticks[i]);
            (rates[i] > 0 && needed <= rates[i]).then_some(needed)
        });
        let finish = match crossing {
            [Some(a), Some(b)] => {
                match (u128::from(a) * u128::from(rates[1]))
                    .cmp(&(u128::from(b) * u128::from(rates[0])))
                {
                    std::cmp::Ordering::Less => {
                        Some((MatchResult::Winner { side: Side::Blue }, a, rates[0]))
                    }
                    std::cmp::Ordering::Greater => {
                        Some((MatchResult::Winner { side: Side::Red }, b, rates[1]))
                    }
                    std::cmp::Ordering::Equal => Some((MatchResult::Draw, a, rates[0])),
                }
            }
            [Some(a), None] => Some((MatchResult::Winner { side: Side::Blue }, a, rates[0])),
            [None, Some(b)] => Some((MatchResult::Winner { side: Side::Red }, b, rates[1])),
            [None, None] => None,
        };
        if let Some((result, numerator, denominator)) = finish {
            self.result = Some(result);
            self.final_fraction = Some(FinalFraction {
                numerator,
                denominator,
                rates,
            });
            return;
        }
        for i in 0..2 {
            self.flag_ticks[i] += rates[i];
        }
        for (site, present) in self.sites.iter_mut().zip(presence) {
            if present[0] && present[1] {
                continue;
            }
            let alone = if present[0] {
                Some(Side::Blue)
            } else if present[1] {
                Some(Side::Red)
            } else {
                None
            };
            match alone {
                Some(side) if site.owner != Some(side) => {
                    if site.capturing != Some(side) {
                        site.capturing = Some(side);
                        site.capture_ticks = 0;
                    }
                    site.capture_ticks += 1;
                    if site.capture_ticks >= CAPTURE_SECONDS * u64::from(hz) {
                        site.owner = Some(side);
                        site.capturing = None;
                        site.capture_ticks = 0;
                        site.contested = false;
                    }
                }
                _ => {
                    site.capturing = None;
                    site.capture_ticks = 0;
                }
            }
        }
        for side in Side::ALL {
            if self
                .sites
                .iter()
                .all(|s| s.owner == Some(side) && !s.contested)
            {
                self.result = Some(MatchResult::Winner { side });
            }
        }
    }

    pub fn scores(&self, hz: u32) -> [f64; 2] {
        std::array::from_fn(|i| {
            let ticks = self.flag_ticks[i] as f64
                + self.final_fraction.as_ref().map_or(0.0, |f| {
                    f.rates[i] as f64 * f.numerator as f64 / f.denominator as f64
                });
            ticks * VICTORY_SCORE / self.threshold(hz) as f64
        })
    }

    pub fn views(&self, geography: &[ObjectiveSite], hz: u32) -> Vec<ObjectiveView> {
        self.sites
            .iter()
            .zip(geography)
            .map(|(state, site)| ObjectiveView {
                id: site.id.clone(),
                center: site.center,
                radius_m: site.radius_m,
                owner: state.owner,
                capturing: state.capturing,
                capture_progress: state.capture_ticks as f64
                    / (CAPTURE_SECONDS * u64::from(hz)) as f64,
                contested: state.contested,
            })
            .collect()
    }

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.flag_ticks[0]).u64(self.flag_ticks[1]);
        d.u64(self.sites.len() as u64);
        for s in &self.sites {
            d.u64(s.owner.map_or(2, |s| s.index() as u64))
                .u64(s.capturing.map_or(2, |s| s.index() as u64))
                .u64(s.capture_ticks)
                .u64(s.contested as u64);
        }
        d.u64(match self.result {
            None => 0,
            Some(MatchResult::Winner { side }) => side.index() as u64 + 1,
            Some(MatchResult::Draw) => 3,
        });
        d.u64(self.final_fraction.is_some() as u64);
        if let Some(f) = &self.final_fraction {
            d.u64(f.numerator)
                .u64(f.denominator)
                .u64(f.rates[0])
                .u64(f.rates[1]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interruption_preserves_progress_while_both_sides_remain_then_resumes() {
        let mut referee = Objectives::new(3);
        for _ in 0..10 {
            referee.advance_presence(&[[true, false], [false; 2], [false; 2]], 1);
        }
        for _ in 0..30 {
            referee.advance_presence(&[[true, true], [false; 2], [false; 2]], 1);
        }
        assert_eq!(referee.sites[0].capture_ticks, 10);
        assert_eq!(referee.sites[0].owner, None);
        assert!(referee.sites[0].contested);
        for _ in 0..10 {
            referee.advance_presence(&[[true, false], [false; 2], [false; 2]], 1);
        }
        assert_eq!(referee.sites[0].owner, Some(Side::Blue));
        assert_eq!(referee.flag_ticks, [0, 0]);
        referee.advance_presence(&[[false; 2]; 3], 1);
        assert_eq!(
            referee.flag_ticks,
            [1, 0],
            "an empty owned flag still scores"
        );
    }
    #[test]
    fn leaving_resets_an_incomplete_capture_and_enemy_presence_stops_owned_scoring() {
        let mut referee = Objectives::new(3);
        for _ in 0..10 {
            referee.advance_presence(&[[true, false], [false; 2], [false; 2]], 1);
        }
        referee.advance_presence(&[[false; 2]; 3], 1);
        assert_eq!(referee.sites[0].capture_ticks, 0);
        assert_eq!(referee.sites[0].capturing, None);
        for _ in 0..20 {
            referee.advance_presence(&[[true, false], [false; 2], [false; 2]], 1);
        }
        referee.advance_presence(&[[false; 2]; 3], 1);
        assert_eq!(referee.flag_ticks[0], 1);
        for _ in 0..10 {
            referee.advance_presence(&[[false, true], [false; 2], [false; 2]], 1);
        }
        assert_eq!(
            referee.flag_ticks[0], 1,
            "the absent defender's flag pauses under takeover"
        );
        assert_eq!(referee.sites[0].owner, Some(Side::Blue));
        for _ in 0..10 {
            referee.advance_presence(&[[false, true], [false; 2], [false; 2]], 1);
        }
        assert_eq!(referee.sites[0].owner, Some(Side::Red));
    }

    #[test]
    fn smallest_majority_wins_in_thirty_minutes_and_minority_keeps_scoring() {
        let mut referee = Objectives::new(3);
        referee.sites[0].owner = Some(Side::Blue);
        referee.sites[1].owner = Some(Side::Blue);
        referee.sites[2].owner = Some(Side::Red);
        for _ in 0..1800 * 30 - 1 {
            referee.advance_presence(&[[false; 2]; 3], 30);
        }
        assert_eq!(referee.result, None);
        referee.advance_presence(&[[false; 2]; 3], 30);
        assert_eq!(
            referee.result,
            Some(MatchResult::Winner { side: Side::Blue })
        );
        assert_eq!(referee.scores(30), [1000.0, 500.0]);
    }

    #[test]
    fn exact_score_crossings_draw_but_an_earlier_fraction_wins() {
        let mut tied = Objectives::new(3);
        tied.sites[0].owner = Some(Side::Blue);
        tied.sites[1].owner = Some(Side::Blue);
        tied.sites[2].owner = Some(Side::Red);
        let threshold = tied.threshold(30);
        tied.flag_ticks = [threshold - 2, threshold - 1];
        tied.advance_presence(&[[false; 2]; 3], 30);
        assert_eq!(tied.result, Some(MatchResult::Draw));
        assert_eq!(tied.scores(30), [1000.0, 1000.0]);

        let mut earlier = Objectives::new(3);
        earlier.sites[0].owner = Some(Side::Blue);
        earlier.sites[1].owner = Some(Side::Blue);
        earlier.sites[2].owner = Some(Side::Red);
        earlier.flag_ticks = [threshold - 1, threshold - 1];
        earlier.advance_presence(&[[false; 2]; 3], 30);
        assert_eq!(
            earlier.result,
            Some(MatchResult::Winner { side: Side::Blue })
        );
        let scores = earlier.scores(30);
        assert_eq!(scores[0], 1000.0);
        assert!(
            scores[1] < 1000.0,
            "later crossing does not grant a false tie"
        );
    }
}
