//! Appearance routing and placement share the authored GPU mesh allocation.
#[path = "support/runtime_library.rs"]
mod runtime_library;
mod support;

use presentation::records::{Appearance, Batch, Instance, NONE, VIEW_PASS};
use presentation::static_world::props::{DRAWS_PROP, DRAWS_PROP_GROUND};
use renderer_core::bundle::Shape;
use renderer_core::memory::ResourceMemory;
use renderer_core::models::Models;
use renderer_core::pipelines;

#[test]
fn static_routing_placement_and_default_alias_share_geometry() {
    let gpu = support::gpu().expect("real GPU");
    let pipelines = pipelines::build_builtin(&gpu.device).unwrap();
    let name = "crate_stack";
    let library = runtime_library::runtime_library(&[name]).unwrap();
    let bundle = library.bundle(name).unwrap();
    let Shape::Static(states) = &bundle.shape else {
        panic!("the fixture is a static model");
    };
    let state = &states[0];
    let strings = format!("{name}{}", state.name);
    let body = Appearance {
        name_len: name.len() as u32,
        state: name.len() as u32,
        state_len: state.name.len() as u32,
        lies: NONE,
        draws: DRAWS_PROP,
        ..Default::default()
    };
    let rows = [
        body,
        Appearance {
            draws: DRAWS_PROP_GROUND,
            ..body
        },
        Appearance {
            draws: 0,
            placed: 1,
            ..body
        },
        Appearance {
            draws: 0,
            state_len: 0,
            ..body
        },
    ];
    let admit = |rows: &[Appearance]| {
        Models::from_world(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            &library,
            rows,
            &strings,
            |_| None,
            |_, _| None,
        )
        .unwrap()
    };
    let one = admit(&rows[..1]);
    let mut routed = admit(&rows);
    let allocations = |models: &Models| {
        let mut memory = ResourceMemory::default();
        models.record_memory(&mut memory);
        let counts = memory.counts();
        (counts.buffer_bytes, counts.buffers)
    };
    let expected_vertices = state
        .tiers
        .iter()
        .map(|mesh| mesh.vertices() as u32)
        .sum::<u32>();
    assert!(
        expected_vertices > 0,
        "the fixture must contain real geometry"
    );
    assert_eq!(
        allocations(&routed),
        allocations(&one),
        "routing, placement and an empty-state alias must not allocate another mesh"
    );
    let instances = [Instance::default(); 4];
    let batches = (0..4)
        .map(|row| Batch {
            appearance: row,
            key: row,
            first: row,
            count: 1,
            pass: VIEW_PASS,
            caster: NONE,
            ..Default::default()
        })
        .collect::<Vec<_>>();
    routed
        .prepare_records(&gpu.device, &gpu.queue, &[], &instances, &batches)
        .unwrap();
    assert_eq!(
        routed.drawn().listed,
        4,
        "every routing row remains drawable"
    );
    assert_eq!(
        routed.drawn().triangles,
        (state.tiers[0].indices.len() / 3 * 4) as u64,
        "sharing preserves the selected geometry for every row"
    );
    let building = routed.building_diagnostics();
    assert_eq!(building["tiers"][0], 1, "placement metadata stays per row");
}
