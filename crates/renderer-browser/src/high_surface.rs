//! Optional transparent canvas above the DOM's world annotations.
use super::*;

pub(super) struct HighSurface {
    pub(super) canvas: HtmlCanvasElement,
    pub(super) surface: wgpu::Surface<'static>,
    pub(super) group: Option<BindGroup>,
}

impl HighSurface {
    pub(super) fn new(
        instance: &Instance,
        adapter: &Adapter,
        device: &Device,
        canvas: HtmlCanvasElement,
        size: [u32; 2],
    ) -> Result<Self, JsValue> {
        let surface = instance
            .create_surface(SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?;
        let caps = surface.get_capabilities(adapter);
        if !caps.formats.contains(&TextureFormat::Bgra8Unorm) {
            return Err(js_error("high-air canvas needs BGRA8"));
        }
        // wgpu's browser backend advertises only Opaque but explicitly supports
        // PreMultiplied in configure; the actual canvas configuration is the proof.
        let output = Self {
            canvas,
            surface,
            group: None,
        };
        output.configure(device, size);
        Ok(output)
    }

    pub(super) fn configure(&self, device: &Device, size: [u32; 2]) {
        self.canvas.set_width(size[0]);
        self.canvas.set_height(size[1]);
        self.surface.configure(
            device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: TextureFormat::Bgra8Unorm,
                color_space: Default::default(),
                width: size[0],
                height: size[1],
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: CompositeAlphaMode::PreMultiplied,
                view_formats: vec![],
            },
        );
    }
}
