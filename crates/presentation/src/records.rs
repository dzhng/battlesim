//! The records a renderer reads, defined once. Each is `#[repr(C)]`,
//! a multiple of 16 bytes, of 4-byte scalars, nested records or arrays, so the same
//! bytes are a WGSL struct, a vertex instance attribute and a typed array.
//! `records_are_current` renders the browser's chosen layouts into
//! `packages/battle-renderer/src/records.gen.ts`; nothing is kept by hand.

/// `u32::MAX`: no row, no id.
pub const NONE: u32 = u32::MAX;

/// Shared camera/render extent floor, keeping the post-processing pyramid usable.
pub const MIN_TARGET_PX: u32 = 64;

/// What a batch draws for (`Batch.pass`): the view; the sun's cascades; a
/// placement ghost, translucent (no shadow, no depth write); a high-air
/// model in view, which casts nothing and which a client with cards over
/// the world (the browser's) draws on its own plane above them, and any
/// other client with the view.
pub const VIEW_PASS: u32 = 0;
pub const CAST_PASS: u32 = 1;
pub const GHOST_PASS: u32 = 2;
pub const HIGH_PASS: u32 = 3;

/// How fog of war takes a batch (`Batch.fog`): a unit never (it is drawn
/// only while identified); the world's faces one by one; remains lying on
/// the ground whole, as the ground under them; a surface movers stand on
/// face by face, with the ground's paint.
pub const FOG_UNITS: u32 = 0;
pub const FOG_FACES: u32 = 1;
pub const FOG_GROUND: u32 = 2;
pub const FOG_PAINTED_FACES: u32 = 3;

mod layout;
#[doc(hidden)]
pub use bytemuck;
pub use layout::{Binding, Field, FieldLayout, Layout};

crate::records! {
    /// One paved kind's material look.
    Struct RoadLook {
        core: [f32; 4],
        worn: [f32; 4],
        shape: [f32; 4],
        join: [f32; 4],
        shoulder: [f32; 4],
        edge: [f32; 4],
        ruts: [f32; 4],
        track: [f32; 4],
        slabs: [f32; 4],
        curb: [f32; 4],
        paint: [f32; 4],
        marks: [f32; 4],
        crossing: [f32; 4],
        stones: [f32; 4],
    }

    /// Shared shore bands and ground shading.
    Struct ShoreTuning {
        bands: [f32; 4],
        turns: [f32; 4],
        ground: [f32; 4],
        fade: [f32; 4],
    }

    /// One water look, ten vec4s.
    Struct WaterLook {
        channel: [f32; 4],
        edge: [f32; 4],
        surface: [f32; 4],
        normals: [f32; 4],
        drifts: [f32; 4],
        fades: [f32; 4],
        roughness: [f32; 4],
        lobes: [f32; 4],
        refraction: [f32; 4],
        caustics: [f32; 4],
    }

    /// Terrain material tables, followed by the frame's view settings.
    Struct TerrainParams {
        region: [f32; 4],
        field: [f32; 4],
        field_grid: [u32; 4],
        shape: [f32; 4],
        verge: [f32; 4],
        feathers: [f32; 4],
        roads: [RoadLook; crate::static_world::ground::tables::ROAD_LOOKS_MAX],
        road_order: [u32; 4],
        forest_litter: [f32; 4],
        forest_moss: [f32; 4],
        forest_humus: [f32; 4],
        forest_detail: [f32; 4],
        forest_verge: [f32; 4],
        forest_dapple: [f32; 4],
        forest_line: [f32; 4],
        water_bed: [f32; 4],
        shore: [f32; 4],
        shore_earth: [f32; 4],
        shore_edge: [f32; 4],
        distant: [f32; 4],
        shore_tuning: ShoreTuning,
        water_looks: [WaterLook; crate::static_world::ground::biome::WATER_LOOKS_MAX],
        view: [u32; 4],
    }

    /// Grass growth, appearances and the frame's window.
    Struct GrassParams {
        planes: [[f32; 4]; 4],
        eye: [f32; 4],
        window: [f32; 4],
        grid: [u32; 4],
        density: [f32; 4],
        tiers: [f32; 4],
        shading: [f32; 4],
        clear: [f32; 4],
        counts: [u32; 4],
        ground: [f32; 4],
        sway: [f32; 4],
        growth: [[f32; 4]; crate::static_world::ground::grass::GROWTH_ROWS],
        patches: [[f32; 4]; crate::static_world::ground::grass::GROWTH_ROWS],
        mixes: [[f32; 4]; crate::static_world::ground::grass::GROWTH_ROWS * crate::static_world::ground::biome::GRASS_MIX_MAX],
        patch_scales: [f32; 4],
        grain: [f32; 4],
        kinds: [[f32; 4]; crate::static_world::ground::grass::MAX_KINDS],
        bases: [[u32; 4]; crate::static_world::ground::grass::MAX_KINDS],
    }

    /// The scar cache prefix followed by the biome's look.
    Struct ScarParams {
        grid: [f32; 4],
        pages: [u32; 4],
        clip: [f32; 4],
        cache_bounds: [u32; 4],
        default_word: [u32; 4],
        full: [f32; 4],
        crater: [f32; 4],
        ejecta: [f32; 4],
        scorch: [f32; 4],
        tracks: [f32; 4],
        trampled: [f32; 4],
        relief: [f32; 4],
        grass: [f32; 4],
    }

    /// Where a frame's sets lie, as pointers into the module's memory and
    /// record counts (the WASM binding fills them; native reads the
    /// `Frame` directly).
    Plain FrameHeader {
        /// Presentation time, seconds on the sim clock.
        clock: f32,
        /// The publication drawn.
        tick: u32,
        /// Frames framed so far.
        index: u32,
        /// How far round a surface every cast light reaches past facing it, 0..1.
        light_wrap: f32,
        batches: [u32; 2],
        instances: [u32; 2],
        palettes: [u32; 2],
        effects: [u32; 2],
        lights: [u32; 2],
        picks: [u32; 2],
        emitters: [u32; 2],
        counts: [u32; 2],
        /// The frame's view: one `View`.
        view: [u32; 2],
        /// The camera every world shader binds: one `Camera`.
        camera: [u32; 2],
        /// The sun's cascades as every shadow receiver binds them: one
        /// `SunShadow`, or none before the presentation has a light.
        sun_shadow: [u32; 2],
        /// Each fitted cascade's caster `Camera`.
        casters: [u32; 2],
    }

    /// Where the world's sets lie (pointer, count), as `FrameHeader`.
    Plain WorldHeader {
        appearances: [u32; 2],
        /// The UTF-8 arena names point into (pointer, bytes).
        strings: [u32; 2],
    }

    /// One drawable an instance may take: an appearance's posed body or
    /// first state, a way it lies dead, or one of its states.
    Plain Appearance {
        /// Byte offset into `strings` of the catalog name the renderer loads.
        name: u32,
        name_len: u32,
        /// The static state drawn (offset, bytes); 0 bytes: the posed body,
        /// or a static bundle's first state.
        state: u32,
        state_len: u32,
        /// The way of lying (`corpse_poses` index) for a corpse; `NONE` otherwise.
        lies: u32,
        /// Joints a palette for it holds (0: rigid).
        skin: u32,
        /// 1 for a drawable of the static world (a building's kit module, a
        /// tree, a prop), whose batches a frame writes only when they change
        /// and names unchanged otherwise; 0 for one the frame poses every time.
        placed: u32,
        /// What draws it: 0 a model or a building's kit module; scenery 1
        /// the forest, 2 a tree line's shrubs, 3 the backdrop, 4 forest-floor
        /// dressing (its seed is its height over the size it fades at), 5 a
        /// felled tree (two records an instance: the tree with its fall's
        /// way, angle and hinge; then its press up-and-down and across,
        /// trunk, landing and crown base), 6 a felled tree's stump
        /// (`Presentation::stump_mesh`); a prop drawn as a model with the
        /// units, 7 fogged face by face, 8 a surface movers stand on, which
        /// takes the ground paint.
        draws: u32,
    }

    /// What one frame draws of one drawable at one tier.
    Plain Batch {
        /// Stable across frames; the renderer keys its buffers by it.
        key: u32,
        /// `World.appearances` row.
        appearance: u32,
        /// Detail tier, 0 finest; 4 is the impostor card.
        tier: u32,
        /// `VIEW_PASS`, `CAST_PASS`, `GHOST_PASS` or `HIGH_PASS`.
        pass: u32,
        /// Caster tier, or `NONE`. Static view batches cast through separate
        /// caster batches; dynamic models use `view::batch_caster`.
        caster: u32,
        /// How fog of war takes it (`FOG_UNITS` and the rest).
        fog: u32,
        /// Changes when the contents change.
        version: u32,
        /// First `Instance`, or `NONE` when unchanged since `version`.
        first: u32,
        count: u32,
        /// Bit i admits this batch to cascade i; zero never casts.
        cascade_mask: u32,
        pad: [u32; 2],
    }

    /// One drawn model. Palette indices and the card layer are integral floats.
    Vertex Instance {
        /// x, y, z (m, sim) and yaw (rad, counter-clockwise from +x).
        placement: [f32; 4],
        /// First `Joint` of its palette (0: the identity), the left and right
        /// track scrolls (fractions of a link), and the x-ray or ghost colour
        /// packed as `r·65536 + g·256 + b` bytes.
        data: [f32; 4],
        /// The side's tint (linear rgb) on tint-masked surfaces, and the
        /// impostor atlas layer a card draws (the renderer's; >=0), or
        /// a mesh cue: −1 ordinary, −2 highlighted. Highlight keeps mesh geometry.
        tint: [f32; 4],
        /// Per-axis scale in its own frame, and the x-ray or ghost alpha.
        scale: [f32; 4],
    }

    /// One joint's or node's motion, bind to model space: a 3×4 row-major
    /// affine (rows of basis x, basis y, basis z and origin).
    Struct Joint {
        x: [f32; 4],
        y: [f32; 4],
        z: [f32; 4],
    }

    /// One effect instance: four vec4 whose meaning depends on `misc.x`
    /// (0 streak, 1 glow, 2 flipbook, 3 rotor, 4 shadow), in draw order.
    Vertex Effect {
        a: [f32; 4],
        b: [f32; 4],
        colour: [f32; 4],
        misc: [f32; 4],
    }

    /// One chosen cast light (at most 24 a frame).
    Struct CastLight {
        /// xyz (m, sim), and 1/radius² (m⁻²).
        position: [f32; 4],
        /// rgb irradiance at its centre; w unused.
        color: [f32; 4],
    }

    /// A box of what this frame draws, for the UI's hit tests.
    Plain Pick {
        unit: u32,
        /// A soldier's id, or `NONE` for the whole unit.
        member: u32,
        /// 1 own, 2 identified enemy, 4 aloft.
        flags: u32,
        pad: u32,
        /// Centre (m, sim), and yaw (rad).
        centre: [f32; 4],
        /// Half extents (m); w unused.
        half: [f32; 4],
    }

    /// A drawn unit as audio hears it.
    Plain Emitter {
        unit: u32,
        member: u32,
        /// The unit type's index in the world's `units.types` (by name), or
        /// `NONE` for a soldier.
        class: u32,
        /// 1 enemy, 2 reversing.
        flags: u32,
        /// Position (m, sim); w unused.
        position: [f32; 4],
        /// Left and right track travel (m), turret yaw (rad), unused.
        motion: [f32; 4],
    }

    /// What the frame's camera sees (sim space).
    Plain View {
        /// Sim → view, column-major.
        view: [f32; 16],
        /// Reverse-Z with an infinite far plane, column-major.
        projection: [f32; 16],
        /// The eye (m); w unused.
        eye: [f32; 4],
        /// What it looks at (m), where the haze is measured from; w unused.
        target: [f32; 4],
        /// Width and height (px), 1/height, pixels per metre at 1 m.
        size: [f32; 4],
        /// The unit ray through the top-left corner, and its change across
        /// the width and down the height (the sky's `Rays`: w unused).
        sky_origin: [f32; 4],
        sky_dx: [f32; 4],
        sky_dy: [f32; 4],
    }

    /// The sky's light for the battle: what the sky lookup bakes and the
    /// shown sky and its clouds read.
    Struct SkyLight {
        /// Unit vector toward the sun.
        sun_direction: [f32; 3],
        /// 0 clear to 1 overcast, from turbidity.
        overcast: f32,
        /// Max-channel-normalised transmittance toward the sun.
        sun_transmittance: [f32; 3],
        /// `sky.radiance`.
        radiance: f32,
        /// The lower hemisphere's ground bounce, greyed by overcast.
        ground_tint: [f32; 3],
        mie_scatter: f32,
        mie_extinction: f32,
        /// The multiple-scattering floor's share for this sun.
        multiple_scatter: f32,
        /// The low sun's aureole strength.
        aureole: f32,
        /// The overcast dome's brightness for this sun.
        overcast_scale: f32,
        /// The sun disc's radiance through the overcast.
        disc_radiance: f32,
        /// The cloud layer's height over its mass size.
        cloud_reach: f32,
        /// Cloud density's smoothstep edges about the coverage threshold.
        cloud_edge: [f32; 2],
        /// A step across the layer toward the sun.
        cloud_to_sun: [f32; 2],
        /// The coverage threshold, and where the sunward shadow and the
        /// thick core saturate.
        cloud_low: f32,
        cloud_shadow_top: f32,
        cloud_core_top: f32,
        cloud_opacity: f32,
        pad0: [f32; 2],
        /// A sunlit top's radiance.
        cloud_lit: [f32; 3],
        /// The silver lining's radiance.
        cloud_silver: f32,
        /// A shaded base's radiance, before the sky shows through it.
        cloud_base: [f32; 3],
        /// The clear sky's saturation (1 as modelled, 0 its luminance).
        sky_saturation: f32,
        /// Multiplies the clear sky, linear rgb.
        sky_tint: [f32; 3],
        pad1: f32,
    }

    /// The haze, in every lit shader's environment uniform: extinction over
    /// distance from the camera's ground focus, a range term, a sunward Mie
    /// tint and a low valley mist.
    Struct Aerial {
        /// Per-channel extinction σ (km⁻¹).
        extinction: [f32; 3],
        /// No haze within this distance of the focus (km).
        clear_radius_km: f32,
        /// The sunward Mie tint (linear rgb) and its strength.
        sun_tint: [f32; 3],
        sun_strength: f32,
        /// The valley mist's colour (linear rgb) and how much of it the far air takes.
        mist_color: [f32; 3],
        mist_color_strength: f32,
        overview_from_m: f32,
        range_near_m: f32,
        range_far_m: f32,
        range_power: f32,
        range_strength: f32,
        mist_opacity: f32,
        sun_power: f32,
        pad0: f32,
        /// The mist's height band and distance ramp (m).
        mist_height_m: [f32; 2],
        mist_distance_m: [f32; 2],
        /// Where the horizon's sky takes over from the view's, below and above it.
        horizon_fade: [f32; 2],
        pad1: [f32; 2],
    }

    /// The one wind, in the camera every shader binds (`Camera.wind`,
    /// `wind.rs`): grass, water and trees move in it on the camera's clock,
    /// and smoke and dust drift on it.
    Struct Wind {
        /// Unit vector it blows toward, level (z 0).
        toward: [f32; 3],
        /// How fast it carries smoke and dust (m/s).
        drift_mps: f32,
        /// How fast a gust front rolls downwind (m/s).
        gust_mps: f32,
        /// 1 / the gust fronts' spacing (per metre).
        gust_per_m: f32,
        pad: [f32; 2],
    }

    /// The sun and the shadow fill, as the environment uniform carries them
    /// after its view (`sunDirection` to `fill`).
    Plain Sun {
        /// Unit vector toward the sun; w 0.
        sun_direction: [f32; 4],
        /// The sun's colour (its transmittance, greyed by overcast) times its
        /// intensity, linear rgb; w 0.
        sun_radiance: [f32; 4],
        /// x the renderer's environment map's top mip (0 here); y the share
        /// of the sun a sun-shadowed surface keeps.
        settings: [f32; 4],
        /// Linear rgb multiplier on the sky's environment light; w 0.
        fill: [f32; 4],
    }

    /// The bloom, as post's highpass and composite read it.
    Struct Bloom {
        /// The first four blurred levels' shares of the composite.
        weights: [f32; 4],
        /// The fifth level's share.
        weight4: f32,
        strength: f32,
        /// Where the highpass starts and where it is full (linear HDR luminance).
        threshold: f32,
        knee: f32,
    }

    /// The grade and exposure, as post's final pass reads it.
    Struct Grade {
        saturation: f32,
        contrast: f32,
        split_tone: f32,
        shadow_lift: f32,
        exposure: f32,
        /// How much of the grade the sky takes (1 all, 0 none).
        sky_grade_mix: f32,
        pad: [f32; 2],
        /// rgb; w 1.
        shadow_tint: [f32; 4],
        /// rgb; w 1.
        highlight_tint: [f32; 4],
        /// rgb; w 0.
        lift: [f32; 4],
    }

    /// What unseen looks like to the fog mask passes: a `presentation.fog`
    /// style resolved (`fog::FogStyle::look`).
    Struct FogLook {
        /// The night tint at unit luminance.
        tint: [f32; 3],
        dim: f32,
        cool: f32,
        saturation: f32,
        veil: f32,
        line_strength: f32,
        line_spacing_px: f32,
        line_width_px: f32,
        /// Radians.
        line_angle: f32,
        line_floor: f32,
        edge_softness_px: f32,
        pad0: [f32; 3],
        /// Display sRGB.
        rim_color: [f32; 3],
        rim_alpha: f32,
        rim_width_px: f32,
        /// How far the distance passes search for the other side, pixels.
        reach_px: f32,
        pad1: [f32; 2],
    }

    /// A camera as every world, shadow and overlay shader binds it: the
    /// frame's eye, or a sun cascade posed as its light, with what animates
    /// the world it draws: the clock and the wind.
    Struct Camera {
        view_proj: [f32; 16],
        inv_view_proj: [f32; 16],
        eye: [f32; 3],
        znear: f32,
        /// The viewport (px).
        width: f32,
        height: f32,
        /// The animation clock (s).
        time: f32,
        pad: f32,
        /// The ground's wind (calm before a ground), its one record: grass,
        /// water and trees move in it, in every pass that draws them,
        /// shadows included.
        wind: [Wind; 1],
        /// The wave field (`wave::area`): its south-west corner and side
        /// (m), and how much of its waves show, 0 where none are drawn.
        waves: [f32; 4],
    }

    /// A wake impulse as the wave field splats it (`EffectFrame::waves`).
    Struct WaveSplat {
        /// Where in the field (texels from its south-west corner), how wide
        /// (texels) and how hard.
        splat: [f32; 4],
        /// Which of the frame's steps splats it, and its kind (0 rings, 1 a
        /// V-wake).
        step: f32,
        kind: f32,
        pad: [f32; 2],
    }

    /// One sun cascade as a shadow receiver reads it. An unfitted cascade is
    /// the identity over [1, 1].
    Struct SunCascade {
        /// Its light's view-projection, column-major.
        matrix: [f32; 16],
        /// Depth bias, normal bias (m), PCF radius (texels), 0.
        bias: [f32; 4],
        /// The receiver-depth interval it shades, 0, 0.
        interval: [f32; 4],
    }

    /// Every shadow receiver's sun cascades, from the frame's fit
    /// (`shadow::sun_shadow`).
    Struct SunShadow {
        /// Per cascade slot (`shadow::CASCADES`).
        cascades: [SunCascade; crate::shadow::CASCADES],
        /// The receiver range's capped far, 0, its near, 0.
        control: [f32; 4],
    }

    /// Fog of war's sight for one frame (bound by the fog compute and every
    /// fogged material): the frame's camera and size, the
    /// sight lights' geometry, the static world sight is cut by, and the
    /// structures that take fog whole (`fog::sight`).
    Struct FogParams {
        inv_view_proj: [f32; 16],
        camera_eye: [f32; 3],
        /// The tallest canopy over the ground (m).
        translucent_lift_m: f32,
        azimuth_bins: u32,
        terrain_azimuth_bins: u32,
        tiles_x: u32,
        tile_eyes_max: u32,
        eye_count: u32,
        /// The foliage grid's cells across and down (0: no foliage).
        foliage_nx: u32,
        foliage_ny: u32,
        height_nx: u32,
        height_ny: u32,
        /// Eyes whose maps this frame rebuilds.
        rebuild_count: u32,
        /// 0: no fog, everything seen.
        enabled: u32,
        probe_count: u32,
        first_bin_m: f32,
        target_height_m: f32,
        foliage_cell_m: f32,
        foliage_full_block: f32,
        min_sight_gap_m: f32,
        /// WGSL aligns the following vec4 to 16 bytes.
        pad0: [f32; 3],
        /// The playable rectangle as the map's extents give it; outside it
        /// is fogged.
        playable: [f32; 4],
        height_spacing: f32,
        face_probe_m: f32,
        width: f32,
        height: f32,
        step_min_m: f32,
        step_max_m: f32,
        step_fraction: f32,
        roof_reach_m: f32,
        /// The structures that take fog whole: how many, their lookup grid
        /// in the `wholes` texture (cells across and down, cell size,
        /// origin) and where its item lists, boxes and seen flags start.
        whole_count: u32,
        whole_nx: u32,
        whole_ny: u32,
        whole_cell_m: f32,
        whole_origin: [f32; 2],
        whole_items_base: u32,
        whole_boxes_base: u32,
        whole_flags_base: u32,
        pad1: [f32; 3],
    }

    /// One eye as its map was built: `position` the map's
    /// origin and `reach` its radius; forward, shape and range the latest
    /// publication's.
    Struct FogEye {
        position: [f32; 3],
        reach: f32,
        forward: f32,
        front: f32,
        side: f32,
        rear: f32,
        range: f32,
        /// The map slot holding this eye's maps.
        slot: u32,
        pad0: u32,
        pad1: u32,
    }

    /// An oriented box that hides what lies behind it.
    Struct FogBox {
        center: [f32; 2],
        /// Half extents along and across its heading.
        half: [f32; 2],
        yaw: f32,
        top: f32,
        base: f32,
        pad1: f32,
    }

    /// What this frame poses and runs, for the gates.
    Plain Counts {
        soldiers: u32,
        vehicles: u32,
        aircraft: u32,
        corpses: u32,
        fading: u32,
        /// Effects running by kind: tracer, flash, impact, sparks, blast,
        /// puff, flame, trail.
        running: [u32; 8],
        lights: u32,
        /// Effect instances past the capacity.
        dropped: u32,
        /// Soldiers off screen this frame.
        culled: u32,
    }
}

mod typescript;
pub use typescript::{camel, typescript};
