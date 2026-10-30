//! Suppressing material marks preserves the latest canonical GPU upload.
mod support;
use presentation::scars::ScarUpload;
use renderer_core::scars::Scars;
use wgpu::*;

fn sample(gpu: &support::Gpu, scars: &Scars) -> [f32; 4] {
    let source = format!(
        "{}\n{}",
        renderer_core::pipelines::builtin_source("worldTerrain.wgsl").unwrap(),
        r#"
@group(0) @binding(0) var<storage, read_write> capturedScars: array<vec4f>;
@compute @workgroup_size(1) fn inspectScars() {
    capturedScars[0] = groundScars(vec2f(0.5), 0.0).weights;
}
"#
    );
    let shader = gpu.device.create_shader_module(ShaderModuleDescriptor {
        label: Some("test-scar-material"),
        source: ShaderSource::Wgsl(source.into()),
    });
    let pipeline = gpu
        .device
        .create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("test-scar-material"),
            layout: None,
            module: &shader,
            entry_point: Some("inspectScars"),
            compilation_options: Default::default(),
            cache: None,
        });
    let output = gpu.device.create_buffer(&BufferDescriptor {
        label: Some("test-scar-material-output"),
        size: 16,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let result = renderer_core::bind(
        &gpu.device,
        &pipeline.get_bind_group_layout(0),
        "test-scar-material-output",
        &[output.as_entire_binding()],
    );
    let directory = scars.directory_view();
    let pool = scars.pool_view();
    let material = gpu.device.create_bind_group(&BindGroupDescriptor {
        label: Some("test-scar-material"),
        layout: &pipeline.get_bind_group_layout(1),
        entries: &[
            BindGroupEntry {
                binding: 5,
                resource: scars.params.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 6,
                resource: BindingResource::TextureView(&directory),
            },
            BindGroupEntry {
                binding: 7,
                resource: BindingResource::TextureView(&pool),
            },
        ],
    });
    let readback = gpu.device.create_buffer(&BufferDescriptor {
        label: Some("test-scar-material-readback"),
        size: 16,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &result, &[]);
        pass.set_bind_group(1, &material, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 16);
    gpu.queue.submit([encoder.finish()]);
    readback.map_async(MapMode::Read, .., |result| {
        result.expect("scar sample mapped")
    });
    gpu.device.poll(PollType::wait_indefinitely()).unwrap();
    let mapped = readback.get_mapped_range(..).unwrap();
    let words = bytemuck::pod_read_unaligned(&mapped);
    drop(mapped);
    readback.unmap();
    words
}

fn upload(word: u32) -> ScarUpload {
    let f = 1f32.to_bits();
    ScarUpload {
        params: [
            f,
            f,
            f,
            f,
            1,
            1,
            0,
            presentation::scars::POOL_WIDTH,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            word,
            0,
            0,
            0,
        ],
        ..Default::default()
    }
}

#[test]
fn hiding_marks_keeps_new_knowledge_and_restores_the_material() {
    let gpu = support::gpu().expect("real GPU");
    let mut scars = Scars::new(&gpu.device, &gpu.queue, &[1f32.to_bits(); 32]);
    scars.apply(&gpu.queue, &upload(u32::MAX));
    let before = sample(&gpu, &scars);
    assert!(
        before.iter().any(|&value| value > 0.),
        "control has no material marks: {before:?}"
    );
    scars.set_shown(&gpu.queue, false);
    assert_eq!(sample(&gpu, &scars), [0.; 4]);
    scars.apply(&gpu.queue, &upload(0xff00_0000));
    assert_eq!(
        sample(&gpu, &scars),
        [0.; 4],
        "new upload re-enabled hidden marks"
    );
    scars.set_shown(&gpu.queue, true);
    let after = sample(&gpu, &scars);
    assert!(
        after.iter().any(|&value| value > 0.),
        "restored material is empty"
    );
    assert_ne!(after, before, "restored material replayed old knowledge");
    scars.set_shown(&gpu.queue, false);
    scars.set_shown(&gpu.queue, true);
    assert_eq!(sample(&gpu, &scars), after);
}
