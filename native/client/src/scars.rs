//! The side's learned scars on the GPU (`frame/scarTexture.ts`): the pool
//! texture, the directory over it and `ScarParams`, written with what the
//! presentation crate's scar cache hands over (`Presentation::prepare_scars`).
//! The uploads are taken once per drawn frame, in the order the browser
//! makes them: for the grass's window when it regrows, then for the whole
//! view, or for each region in turn when the marks don't fit at once. The
//! world pass applies each at the submission it belongs to.
use presentation::presentation::Presentation;
use presentation::scars::{self, ScarUpload, PARAMS_WORDS, POOL_HEIGHT, POOL_WIDTH};
use wgpu::*;

use crate::gpu::buffer;

/// `ScarParams`: the cache's words, then the biome's look (`full` on).
const WORDS: usize = 52;

fn texture(device: &Device, label: &str, size: [u32; 2], format: TextureFormat) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

pub struct Scars {
    pool: Texture,
    directory: Texture,
    pub params: Buffer,
    /// `ScarParams` as last written: the look stays, the cache's words change.
    words: [u32; WORDS],
    /// This frame's uploads: for the grass's window, then the view's.
    grass: Option<ScarUpload>,
    view: Option<ScarUpload>,
    /// Or, when the marks don't fit at once, each region on screen: its
    /// scissor (`scars::scissor`) and its upload.
    regions: Vec<([u32; 4], ScarUpload)>,
    /// Whether the marks don't fit at once: the terrain draws by region
    /// (none of them, when none is on screen).
    regional: bool,
}

impl Scars {
    /// The scars' textures, empty, under the biome's `look` (`ScarParams`
    /// from `full` on).
    pub fn new(device: &Device, queue: &Queue, look: &[u32]) -> Self {
        let pool = texture(
            device,
            "ground-scar-words",
            [POOL_WIDTH, POOL_HEIGHT],
            TextureFormat::Rgba8Unorm,
        );
        let table = scars::normalization();
        queue.write_texture(
            pool.as_image_copy(),
            &table,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(POOL_WIDTH * 4),
                rows_per_image: None,
            },
            Extent3d {
                width: POOL_WIDTH,
                height: (table.len() / 4) as u32 / POOL_WIDTH,
                depth_or_array_layers: 1,
            },
        );
        let mut words = [0u32; WORDS];
        words[PARAMS_WORDS..PARAMS_WORDS + look.len()].copy_from_slice(look);
        let usage = BufferUsages::UNIFORM | BufferUsages::COPY_DST;
        Self {
            pool,
            directory: texture(
                device,
                "ground-scar-directory",
                [1, 1],
                TextureFormat::Rg32Uint,
            ),
            params: buffer(device, "scar-params", usage, bytemuck::cast_slice(&words)),
            words,
            grass: None,
            view: None,
            regions: Vec::new(),
            regional: false,
        }
    }

    pub fn directory_view(&self) -> TextureView {
        self.directory.create_view(&Default::default())
    }

    pub fn pool_view(&self) -> TextureView {
        self.pool.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        })
    }

    /// Takes this frame's uploads from `presentation`: for the grass's
    /// window `grass` when it regrows, then the view's or, for the regions
    /// on screen through the map's box `map_box`, each region's. True when
    /// the directory outgrew its texture (the group must be made again).
    pub fn prepare(
        &mut self,
        device: &Device,
        presentation: &mut Presentation,
        grass: Option<[f64; 4]>,
        map_box: Option<&[f64; 6]>,
    ) -> bool {
        self.grass = grass.map(|window| presentation.prepare_scars(Some(window), false));
        let regions = presentation.scar_regions();
        self.regional = !regions.is_empty();
        let camera = presentation.last_frame().camera;
        let size = [camera.width as u32, camera.height as u32];
        let view_proj = camera.view_proj.first_chunk().expect("a 4×4 matrix");
        let on_screen = |r| match map_box {
            Some(b) => scars::scissor(r, b, view_proj, size),
            None => Some([0, 0, size[0], size[1]]),
        };
        (self.view, self.regions) = if regions.is_empty() {
            (Some(presentation.prepare_scars(None, false)), Vec::new())
        } else {
            let shown: Vec<_> = regions
                .into_iter()
                .filter_map(|r| on_screen(r).map(|rect| (r, rect)))
                .collect();
            (
                None,
                shown
                    .into_iter()
                    .map(|(r, rect)| (rect, presentation.prepare_scars(Some(r), true)))
                    .collect(),
            )
        };
        let size = self.directory.size();
        let [mut w, mut h] = [size.width, size.height];
        let regions = self.regions.iter().map(|(_, u)| u);
        let uploads = self.grass.iter().chain(&self.view).chain(regions);
        for [uw, uh] in uploads.filter_map(|u| u.directory.as_ref().map(|d| d.1)) {
            (w, h) = (w.max(uw), h.max(uh));
        }
        let grew = [w, h] != [size.width, size.height];
        if grew {
            self.directory = texture(
                device,
                "ground-scar-directory",
                [w, h],
                TextureFormat::Rg32Uint,
            );
        }
        grew
    }

    /// Whether the scars draw region by region this frame.
    pub fn regional(&self) -> bool {
        self.regional
    }

    pub fn grass_upload(&self) -> Option<&ScarUpload> {
        self.grass.as_ref()
    }

    pub fn view_upload(&self) -> Option<&ScarUpload> {
        self.view.as_ref()
    }

    /// Each region on screen: its scissor and its upload.
    pub fn region_uploads(&self) -> &[([u32; 4], ScarUpload)] {
        &self.regions
    }

    /// Writes `upload`: the directory, the pool's runs and the cache's words.
    pub fn apply(&self, queue: &Queue, upload: &ScarUpload) {
        if let Some((words, [w, h])) = &upload.directory {
            queue.write_texture(
                self.directory.as_image_copy(),
                bytemuck::cast_slice(words),
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 8),
                    rows_per_image: Some(*h),
                },
                Extent3d {
                    width: *w,
                    height: *h,
                    depth_or_array_layers: 1,
                },
            );
        }
        let mut at = 0usize;
        for &[offset, count] in &upload.writes {
            let bytes = &upload.texels[at..at + count as usize * 4];
            at += bytes.len();
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &self.pool,
                    mip_level: 0,
                    origin: Origin3d {
                        x: offset % POOL_WIDTH,
                        y: offset / POOL_WIDTH,
                        z: 0,
                    },
                    aspect: TextureAspect::All,
                },
                bytes,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(count * 4),
                    rows_per_image: Some(1),
                },
                Extent3d {
                    width: count,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
        }
        let mut words = self.words;
        words[..PARAMS_WORDS].copy_from_slice(&upload.params);
        queue.write_buffer(&self.params, 0, bytemuck::cast_slice(&words));
    }
}
