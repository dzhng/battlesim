//! The publication reader (`sim::publication::reader`) behind one class that
//! native's GDScript runner calls, until the runner itself is Rust (slice 18).
//! A sample's observation comes back as the browser's view: dictionaries and
//! arrays of floats, the fog bits as a `PackedInt64Array` and the ground runs
//! as a `PackedFloat32Array`. A static group that didn't change is the same
//! `Array` as before, so callers can test it with `is_same`.
use std::sync::Arc;

use godot::prelude::*;
use serde::Serialize;
use serde_json::Value;
use sim::publication::reader::{Corpse, FallenBody, KnownProp, Reader, Shared};

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct PublicationReader {
    reader: Option<Reader>,
    /// Why the last call failed; empty after a success.
    #[var]
    error: GString,
    /// Each static group's last view, with the group it shows.
    corpses: Option<(Shared<Corpse>, Variant)>,
    known_props: Option<(Shared<KnownProp>, Variant)>,
    fallen_bodies: Option<(Shared<FallenBody>, Variant)>,
}

fn variant(value: &Value) -> Variant {
    match value {
        Value::Null => Variant::nil(),
        Value::Bool(b) => b.to_variant(),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN).to_variant(),
        Value::String(s) => s.to_variant(),
        Value::Array(items) => {
            let mut array = VarArray::new();
            for item in items {
                array.push(&variant(item));
            }
            array.to_variant()
        }
        Value::Object(fields) => {
            let mut dictionary = VarDictionary::new();
            for (key, item) in fields {
                dictionary.set(key.as_str(), &variant(item));
            }
            dictionary.to_variant()
        }
    }
}

fn view<T: Serialize>(value: &T) -> Variant {
    variant(&serde_json::to_value(value).unwrap())
}

/// `group`'s view: the one shown last while the group is unchanged.
fn shared<T: Serialize>(last: &mut Option<(Shared<T>, Variant)>, group: &Shared<T>) -> Variant {
    if !last
        .as_ref()
        .is_some_and(|(seen, _)| Arc::ptr_eq(seen, group))
    {
        *last = Some((group.clone(), view(&**group)));
    }
    last.as_ref().unwrap().1.clone()
}

#[godot_api]
impl PublicationReader {
    /// Opens the stream `layout` (a capture's layout JSON) names; false, with
    /// `error`, when it isn't the writer's layout.
    #[func]
    fn open(&mut self, layout: GString) -> bool {
        match Reader::new(&layout.to_string()) {
            Ok(reader) => {
                (self.reader, self.error) = (Some(reader), GString::new());
                true
            }
            Err(e) => {
                self.error = (&e).into();
                false
            }
        }
    }

    /// The next sample's observation from its raw u32 words, or null with
    /// `error` set. A capture is one stream, so a stale record is an error.
    #[func]
    fn decode(&mut self, words: VarArray) -> Variant {
        let words: Vec<u32> = words
            .iter_shared()
            .map(|w| w.try_to::<f64>().unwrap_or_else(|_| w.to::<i64>() as f64) as u32)
            .collect();
        let decoded = match self.reader.as_mut() {
            None => Err("the reader has no layout".to_string()),
            Some(reader) => reader.decode(&words),
        };
        let observation = match decoded {
            Ok(Some(observation)) => observation,
            Ok(None) => {
                self.error = "the publication is stale".into();
                return Variant::nil();
            }
            Err(e) => {
                self.error = (&e).into();
                return Variant::nil();
            }
        };
        self.error = GString::new();
        let mut dictionary: VarDictionary = view(&observation).to();
        dictionary.set("corpses", &shared(&mut self.corpses, &observation.corpses));
        dictionary.set(
            "knownProps",
            &shared(&mut self.known_props, &observation.known_props),
        );
        dictionary.set(
            "fallenBodies",
            &shared(&mut self.fallen_bodies, &observation.fallen_bodies),
        );
        let mut fog: VarDictionary = dictionary.at("fog").to();
        let bits: Vec<i64> = observation.fog.bits.iter().map(|&w| w as i64).collect();
        fog.set("bits", &PackedInt64Array::from(bits.as_slice()));
        let mut ground: VarDictionary = dictionary.at("groundPatch").to();
        ground.set(
            "runs",
            &PackedFloat32Array::from(observation.ground_patch.runs.as_slice()),
        );
        dictionary.to_variant()
    }
}
