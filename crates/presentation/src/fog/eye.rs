//! One own eye at the published tick, as fog of war's sight sees from it:
//! where it stands, the way it faces, its sight shape and range.
use serde::{Deserialize, Serialize};
use sim::publication::reader::Sight;

/// A sight shape's reach multipliers ahead, abeam and behind.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Shape {
    pub front: f64,
    pub side: f64,
    pub rear: f64,
}

/// One own eye at the published tick.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Eye {
    /// Stable per unit and eye index (`<unit>:<eye>`), so an eye that did
    /// not move keeps its map.
    pub key: String,
    pub position: [f64; 3],
    pub forward: f64,
    pub shape: Shape,
    pub range: f64,
}

impl Eye {
    /// How far it reaches in any direction: its map's radius.
    pub fn reach(&self) -> f64 {
        let s = self.shape;
        self.range * s.front.max(s.side).max(s.rear)
    }

    /// Every eye of the side's own units, each `(id, sight)` at the
    /// published tick: a garrison gives one per facade it holds.
    pub fn of_units<'a>(units: impl IntoIterator<Item = (f64, &'a Sight)>) -> Vec<Eye> {
        units
            .into_iter()
            .flat_map(|(id, sight)| {
                sight
                    .eyes
                    .iter()
                    .enumerate()
                    .map(move |(k, &position)| Eye {
                        key: format!("{id}:{k}"),
                        position,
                        forward: sight.forward,
                        shape: Shape {
                            front: sight.shape.front,
                            side: sight.shape.side,
                            rear: sight.shape.rear,
                        },
                        range: sight.range,
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::publication::reader::SightShape;

    #[test]
    fn a_garrison_gives_an_eye_per_facade_keyed_by_unit_and_eye() {
        let sight = Sight {
            eyes: vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
            forward: 0.5,
            shape: SightShape {
                front: 1.0,
                side: 1.0,
                rear: 1.0,
            },
            range: 600.0,
        };
        let eyes = Eye::of_units([(7.0, &sight)]);
        let got: Vec<_> = eyes.iter().map(|e| (e.key.as_str(), e.position)).collect();
        assert_eq!(got, [("7:0", [1.0, 2.0, 3.0]), ("7:1", [4.0, 5.0, 6.0])]);
    }
}
