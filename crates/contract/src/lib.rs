//! Shared vocabulary between the simulation, its WASM boundary and fixture
//! readers: identifiers, scenario/config records, commands, acknowledgements and
//! observation records. Units are metres, seconds and radians; XY is ground and
//! +Z is up.
pub mod ballistics;
pub mod catalog;
pub mod command;
pub mod curve;
pub mod encounter;
pub mod generation;
pub mod ground;
pub mod identity;
pub mod ids;
pub mod map;
pub mod maps;
pub mod numbers;
pub mod observation;
pub mod preparation;
pub mod river;
pub mod scenario;
pub mod templates;
pub mod weapons;
