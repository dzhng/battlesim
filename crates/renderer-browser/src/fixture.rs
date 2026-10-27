use super::*;

pub(super) struct Fixture<'a> {
    pub(super) depth: &'a TextureView,
    pub(super) camera: &'a BindGroup,
    pub(super) solid: &'a Bound,
    pub(super) world: &'a Bound,
    pub(super) model: &'a Model,
}

impl Fixture<'_> {
    fn draw(&self, pass: &mut RenderPass<'_>, pipeline: &Bound) {
        pipeline.set(pass, &[("camera", self.camera)]);
        pass.set_vertex_buffer(0, self.model.vertex().slice(..));
        pass.set_index_buffer(self.model.index().slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..self.model.index_count(), 0, 0..1);
    }
}

impl renderer_core::depth::DepthInputs for Fixture<'_> {
    fn depth_view(&self) -> &TextureView {
        self.depth
    }
    fn camera_group(&self) -> (&'static str, &BindGroup) {
        ("camera", self.camera)
    }
    fn draw_ground_depth(&self, pass: &mut RenderPass<'_>) {
        self.draw(pass, self.solid);
    }
    fn draw_scenery_depth(&self, _pass: &mut RenderPass<'_>) {}
    fn draw_models_depth(&self, pass: &mut RenderPass<'_>, pipeline: &Bound, _cutout: bool) {
        self.draw(pass, pipeline);
    }
}

impl renderer_core::world::WorldInputs for Fixture<'_> {
    fn draw_sky(&self, _pass: &mut RenderPass<'_>) {}
    fn draw_ground(&self, pass: &mut RenderPass<'_>, _layer: renderer_core::world::Layer) {
        self.draw(pass, self.world);
    }
    fn draw_skirt(&self, _pass: &mut RenderPass<'_>, _layer: renderer_core::world::Layer) {}
    fn bind_models(
        &self,
        pass: &mut RenderPass<'_>,
        _layer: renderer_core::world::Layer,
        _surface: renderer_core::world::Surface,
    ) {
        self.world.set(pass, &[("camera", self.camera)]);
        pass.set_vertex_buffer(0, self.model.vertex().slice(..));
    }
    fn draw_models(
        &self,
        pass: &mut RenderPass<'_>,
        _layer: renderer_core::world::Layer,
        _surface: renderer_core::world::Surface,
    ) {
        pass.set_index_buffer(self.model.index().slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..self.model.index_count(), 0, 0..1);
    }
    fn draw_scenery(&self, _pass: &mut RenderPass<'_>, _layer: renderer_core::world::Layer) {}
    fn draw_grass(&self, _pass: &mut RenderPass<'_>, _layer: renderer_core::world::Layer) {}
    fn draw_backdrop(&self, _pass: &mut RenderPass<'_>, _layer: renderer_core::world::Layer) {}
    fn draw_water(&self, _pass: &mut RenderPass<'_>, _layer: renderer_core::world::Layer) {}
    fn bind_glass(&self, _pass: &mut RenderPass<'_>) {}
    fn draw_glass(&self, _pass: &mut RenderPass<'_>) {}
}
