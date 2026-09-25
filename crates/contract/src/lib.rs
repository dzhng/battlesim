//! Shared vocabulary between the simulation, its WASM boundary and fixture
//! readers: identifiers, scenario/config records, commands, acknowledgements and
//! observation records. Units are metres, seconds and radians; XY is ground and
//! +Z is up.
pub mod ballistics;
pub mod command;
pub mod ids;
pub mod map;
pub mod observation;
pub mod scenario;
pub mod weapons;
