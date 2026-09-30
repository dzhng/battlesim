//! Only template numeric fields use correctly rounded token parsing. Turning
//! on JSON's global float reader feature would also change scenario parsing.
use serde::{de::Error, Deserialize, Deserializer};

struct Number(f64);

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let token = Box::<serde_json::value::RawValue>::deserialize(deserializer)?;
        token.get().parse().map(Self).map_err(D::Error::custom)
    }
}

pub fn scalar<'de, D: Deserializer<'de>>(de: D) -> Result<f64, D::Error> {
    Number::deserialize(de).map(|n| n.0)
}

fn array_from<E: Error, const N: usize>(numbers: Vec<Number>) -> Result<[f64; N], E> {
    numbers
        .into_iter()
        .map(|n| n.0)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| E::custom(format!("expected {N} coordinates")))
}

pub fn array<'de, D: Deserializer<'de>, const N: usize>(de: D) -> Result<[f64; N], D::Error> {
    array_from(Vec::<Number>::deserialize(de)?)
}

pub fn optional_list<'de, D: Deserializer<'de>>(de: D) -> Result<Option<Vec<f64>>, D::Error> {
    Option::<Vec<Number>>::deserialize(de)
        .map(|list| list.map(|list| list.into_iter().map(|n| n.0).collect()))
}

pub fn span<'de, D: Deserializer<'de>>(de: D) -> Result<[[f64; 2]; 2], D::Error> {
    let [a, b] = <[Vec<Number>; 2]>::deserialize(de)?;
    Ok([array_from(a)?, array_from(b)?])
}

pub fn optional_points<'de, D: Deserializer<'de>>(
    de: D,
) -> Result<Option<Vec<[f64; 2]>>, D::Error> {
    Option::<Vec<Vec<Number>>>::deserialize(de)?
        .map(|points| points.into_iter().map(array_from).collect())
        .transpose()
}
