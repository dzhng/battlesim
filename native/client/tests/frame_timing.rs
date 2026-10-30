//! Shared timing must preserve frame ownership while GPU completions are pending.
use native_client::gpu;
use renderer_core::timing::{FrameTimer, Resolve};

#[test]
fn frame_results_survive_delayed_completion_and_reset_in_both_resolve_modes() {
    for mode in [Resolve::InFrame, Resolve::AfterCompletion] {
        // Native InFrame counters are known to be stale on Metal. Keep its
        // device state independent of the AfterCompletion measurement control.
        let gpu = gpu::open(&gpu::headless_instance(), None).unwrap();
        let (slow, fast, target) = fragment_workload(&gpu.device);
        let mut timer = FrameTimer::new(&gpu.device, &gpu.queue, mode).expect("timestamp support");
        let mut pending = Vec::new();
        for frame in [17, 18, 19] {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            timer.begin(&mut encoder, frame);
            draw(
                &mut encoder,
                if frame == 17 { &slow } else { &fast },
                &target,
            );
            timer.end(&mut encoder);
            if frame == 19 {
                // Even a reset between encoding and submission excludes this frame.
                timer.reset();
            }
            let submission = gpu.queue.submit([encoder.finish()]);
            pending.push((frame, timer.submitted(submission.clone()).unwrap()));
            // Drain rendering while deferring every resolve/map. Overlapping
            // GPU stages otherwise make workload cost an ambiguous control;
            // all three query sets must still retain their own frame's values.
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
        }
        // Read results later, in reverse order, rather than consuming a latest value.
        let (mut long, mut short) = (None, None);
        for (frame, pending) in pending.into_iter().rev() {
            let sample = pending.wait().unwrap();
            assert_eq!(sample.frame, frame, "{mode:?}");
            match frame {
                17 => long = sample.ms,
                19 => short = sample.ms,
                _ => {}
            }
        }
        if mode == Resolve::AfterCompletion {
            let (long, short) = (long.unwrap(), short.unwrap());
            assert!(
                long > short * 10.0,
                "distinct GPU workloads were misattributed: {long} vs {short} ms"
            );
        }
        assert_eq!(
            timer.stats().frames,
            0,
            "{mode:?}: pre-reset samples leaked"
        );

        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        timer.begin(&mut encoder, 20);
        draw(&mut encoder, &fast, &target);
        timer.end(&mut encoder);
        let submission = gpu.queue.submit([encoder.finish()]);
        let pending = timer.submitted(submission.clone()).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        // Exercise the browser's non-blocking resolve pump before reading the
        // native ticket; wait alone must not hide a broken browser path.
        timer.resolve_completed();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
        if mode == Resolve::AfterCompletion {
            assert_eq!(timer.stats().frames, 1);
        }
        let sample = pending.wait().unwrap();
        assert_eq!(sample.frame, 20, "{mode:?}");
        if mode == Resolve::AfterCompletion {
            assert!(
                sample.ms.unwrap() < long.unwrap() / 10.0,
                "a reused slot retained its old frame's duration"
            );
        }
        assert_eq!(timer.stats().frames, usize::from(sample.ms.is_some()));
        assert_eq!(timer.stats().mean_ms, sample.ms.unwrap_or(0.0));
    }
}

fn draw(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    target: &wgpu::TextureView,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    pass.set_pipeline(pipeline);
    pass.draw(0..3, 0..1);
}

// A large work gap absorbs scheduling/counter noise without pinning GPU speed.
fn fragment_workload(
    device: &wgpu::Device,
) -> (
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
    wgpu::TextureView,
) {
    use wgpu::*;
    let pipeline = |iterations| {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("timing-attribution-work"),
            source: ShaderSource::Wgsl(
                r#"
            @vertex fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
                let positions = array(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
                return vec4f(positions[index], 0, 1);
            }
            @fragment fn fs(@builtin(position) p: vec4f) -> @location(0) f32 {
                var value = fract(p.x * 0.003 + p.y * 0.001);
                for (var i = 0u; i < 16384u; i++) {
                    value = fract(value * 1.731 + 0.117);
                }
                return value;
            }
        "#
                .replace("16384u", &format!("{iterations}u"))
                .into(),
            ),
        });
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("timing-attribution-work"),
            layout: None,
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(TextureFormat::R8Unorm.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
    };
    let target = device
        .create_texture(&TextureDescriptor {
            label: Some("timing-attribution-work"),
            size: Extent3d {
                width: 256,
                height: 256,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default());
    (pipeline(16384), pipeline(0), target)
}
