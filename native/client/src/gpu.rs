//! The device: Metal on macOS (whatever `wgpu` prefers elsewhere), with the
//! adapter's own limits, since the browser's tables need more than
//! `Limits::default()`'s 128 MiB storage bindings, and the two features the
//! browser asks for where the adapter has them.
use wgpu::*;

pub struct Gpu {
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

/// An instance that can present to a window on this display.
pub fn instance(display: Box<dyn wgpu::wgt::WgpuHasDisplayHandle>) -> Instance {
    Instance::new(InstanceDescriptor::new_with_display_handle(display))
}

pub fn headless_instance() -> Instance {
    Instance::new(InstanceDescriptor::new_without_display_handle())
}

pub fn open(instance: &Instance, surface: Option<&Surface>) -> Result<Gpu, String> {
    let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference: PowerPreference::HighPerformance,
        compatible_surface: surface,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .map_err(|e| format!("no adapter: {e}"))?;
    let wanted = Features::TIMESTAMP_QUERY | Features::INDIRECT_FIRST_INSTANCE;
    let (device, queue) = pollster::block_on(adapter.request_device(&DeviceDescriptor {
        label: Some("battle"),
        required_features: adapter.features() & wanted,
        required_limits: adapter.limits(),
        experimental_features: ExperimentalFeatures::disabled(),
        memory_hints: MemoryHints::Performance,
        trace: Trace::Off,
    }))
    .map_err(|e| format!("no device: {e}"))?;
    Ok(Gpu {
        adapter,
        device,
        queue,
    })
}
