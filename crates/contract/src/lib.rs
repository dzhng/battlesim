#![deny(clippy::disallowed_methods)]
//! Shared vocabulary between the simulation, its WASM boundary and fixture
//! readers: identifiers, scenario/config records, commands, acknowledgements and
//! observation records. Units are metres, seconds and radians; XY is ground and
//! +Z is up.
pub mod ballistics;
pub mod bundle;
pub mod catalog;
pub mod command;
pub mod curve;
pub mod encounter;
pub mod forest;
pub mod generation;
pub mod generation_physics;
pub mod ground;
pub mod identity;
pub mod ids;
pub mod labels;
pub mod map;
pub mod maps;
pub mod movement;
pub mod numbers;
pub mod observation;
pub mod players;
pub mod preparation;
pub mod random;
pub mod river;
pub mod scenario;
pub mod skirmish;
pub mod skirmish_sizes;
pub mod swamp;
pub mod templates;
pub mod weapons;

pub mod script;
