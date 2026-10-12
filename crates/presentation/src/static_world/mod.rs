//! The static world resolved once per map for every renderer (slice 20b):
//! the buildings resolved against the template art library, the walk that
//! chooses what each view draws of them, the state a side's knowledge stands
//! each in, and the batches a frame lists them in. The simulation never reads
//! art.
pub mod batches;
pub mod buildings;
pub mod fallen;
#[cfg(feature = "files")]
pub mod files;
pub mod library;
pub mod walk;
