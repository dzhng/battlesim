//! Concrete browser resources adapted to the shared depth and world order.
use super::*;
use renderer_core::fog::Layer;
use renderer_core::models::Surface as ModelSurface;
use renderer_core::world::{Surface, WorldInputs};

pub(super) struct BrowserWorld<'a> {
    pub(super) renderer: &'a Renderer,
}

fn surface(surface: Surface) -> ModelSurface {
    match surface {
        Surface::Opaque => ModelSurface::Opaque,
        Surface::Cutout => ModelSurface::Cutout,
        Surface::Room => ModelSurface::Room,
        Surface::Blended => ModelSurface::Blended,
    }
}

impl BrowserWorld<'_> {
    fn fog(&self, layer: Layer) -> &BindGroup {
        let index = match layer {
            Layer::PaintedGround => 0,
            Layer::PaintedFaces => 1,
            Layer::Ground => 2,
            Layer::Faces => 3,
            Layer::Units => 4,
        };
        self.renderer.shared_fog_sight.group(index)
    }

    fn groups(&self, layer: Layer) -> [(&'static str, &BindGroup); 3] {
        let renderer = self.renderer;
        [
            ("typegpuCameraLayout", &renderer.production_camera_group),
            ("environment", renderer.environment.bindings.group()),
            ("fogLayout", self.fog(layer)),
        ]
    }
}

impl renderer_core::depth::DepthInputs for BrowserWorld<'_> {
    fn depth_view(&self) -> &TextureView {
        &self.renderer.targets.depth
    }
    fn camera_group(&self) -> (&'static str, &BindGroup) {
        (
            "typegpuCameraLayout",
            &self.renderer.production_camera_group,
        )
    }
    fn draw_ground_depth(&self, pass: &mut RenderPass<'_>) {
        let r = self.renderer;
        r.production_pipelines
            .bound("worldPrepass")
            .set(pass, &[self.camera_group()]);
        pass.set_vertex_buffer(1, r.overlay_instances.slice(..));
        for (vertices, count) in [
            (
                &r.production_terrain_vertices,
                r.production_terrain_vertex_count,
            ),
            (
                &r.production_static_vertices,
                r.production_static_vertex_count,
            ),
        ] {
            if count > 0 {
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.draw(0..count, 0..1);
            }
        }
    }
    fn draw_scenery_depth(&self, pass: &mut RenderPass<'_>) {
        if let (Some(scenery), Some(models)) = (&self.renderer.scenery, &self.renderer.models) {
            scenery.draw_depth(pass, self.camera_group(), models.statics());
        }
    }
    fn draw_models_depth(&self, pass: &mut RenderPass<'_>, pipeline: &Bound, cutout: bool) {
        if let Some(models) = &self.renderer.models {
            pipeline.set(pass, &[self.camera_group(), models.group()]);
            models.draw(
                pass,
                if cutout {
                    ModelSurface::Cutout
                } else {
                    ModelSurface::Solid
                },
            );
        }
    }
}

impl WorldInputs for BrowserWorld<'_> {
    fn draw_sky(&self, _pass: &mut RenderPass<'_>) {}
    fn draw_ground(&self, pass: &mut RenderPass<'_>, layer: Layer) {
        let r = self.renderer;
        if r.production_terrain_vertex_count == 0 {
            return;
        }
        let mut groups = self.groups(layer).to_vec();
        groups.push(("terrainLayout", r.terrain.bind_group()));
        r.production_pipelines
            .bound("worldTerrain")
            .set(pass, &groups);
        pass.set_vertex_buffer(0, r.production_terrain_vertices.slice(..));
        pass.set_vertex_buffer(1, r.overlay_instances.slice(..));
        pass.draw(0..r.production_terrain_vertex_count, 0..1);
    }
    fn draw_skirt(&self, pass: &mut RenderPass<'_>, layer: Layer) {
        let r = self.renderer;
        if r.production_static_vertex_count == 0 {
            return;
        }
        r.production_pipelines
            .bound("worldOpaque")
            .set(pass, &self.groups(layer));
        pass.set_vertex_buffer(0, r.production_static_vertices.slice(..));
        pass.set_vertex_buffer(1, r.overlay_instances.slice(..));
        pass.draw(0..r.production_static_vertex_count, 0..1);
    }
    fn bind_models(&self, pass: &mut RenderPass<'_>, layer: Layer, surface: Surface) {
        if let Some(models) = &self.renderer.models {
            let mut groups = self.groups(layer).to_vec();
            groups.push(models.group());
            let name = match surface {
                Surface::Opaque => "modelOpaque",
                Surface::Cutout => "modelCutout",
                Surface::Room => "modelRooms",
                Surface::Blended => "modelBlended",
            };
            self.renderer
                .production_pipelines
                .bound(name)
                .set(pass, &groups);
        }
    }
    fn draw_models(&self, pass: &mut RenderPass<'_>, layer: Layer, s: Surface) {
        if let Some(models) = &self.renderer.models {
            models.draw_fogged(pass, surface(s), layer);
        }
    }
    fn draw_scenery(&self, pass: &mut RenderPass<'_>, layer: Layer) {
        if let (Some(scenery), Some(models)) = (&self.renderer.scenery, &self.renderer.models) {
            scenery.draw(pass, &self.groups(layer), models.statics());
        }
    }
    fn draw_grass(&self, _pass: &mut RenderPass<'_>, _layer: Layer) {}
    fn draw_backdrop(&self, _pass: &mut RenderPass<'_>, _layer: Layer) {}
    fn draw_water(&self, _pass: &mut RenderPass<'_>, _layer: Layer) {}
    fn bind_glass(&self, pass: &mut RenderPass<'_>) {
        self.bind_models(pass, Layer::Units, Surface::Blended);
    }
    fn draw_glass(&self, pass: &mut RenderPass<'_>) {
        if let Some(models) = &self.renderer.models {
            models.draw(pass, ModelSurface::Blended);
        }
    }
}
