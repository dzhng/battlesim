// What each paid step calls, with which settings, and its listed price (fal's
// model pages; GPT Image from Bento's model catalog). `front` is where the
// generator's model faces, for `generated_body.py`. `generator` and `terms`
// are the pin's provenance text.
export const IMAGE = {
  model: "openai/gpt-image-2.6-flare",
  name: "GPT Image 2.6 Flare",
  size: "1024x1024",
  price_usd: 0.04,
};

export const GENERATORS = {
  tripo: {
    endpoint: "tripo3d/h3.1/image-to-3d",
    options: {
      face_limit: 40000,
      geometry_quality: "detailed",
      texture_quality: "detailed",
      pbr: false,
      texture: true,
      orientation: "align_image",
    },
    front: "+x",
    price_usd: 0.6,
    generator:
      "Tripo H3.1 image to 3D, through Bento's fal service proxy (tripo3d/h3.1/image-to-3d): textured (detailed texture, PBR off), detailed geometry, 40,000-face limit, orientation aligned to the image, from one front T-pose view",
    terms:
      "A paid generation: fal lists the model for commercial use. Listed price $0.60 (HD texture $0.40 plus detailed geometry $0.20).",
  },
  meshy: {
    endpoint: "meshy/v7.1/image-to-3d",
    options: {
      pose_mode: "t-pose",
      target_polycount: 40000,
      symmetry_mode: "auto",
      should_texture: true,
      enable_pbr: false,
      topology: "triangle",
    },
    front: "-y",
    price_usd: 1.2,
    generator:
      "Meshy 7.1 image to 3D, through Bento's fal service proxy (meshy/v7.1/image-to-3d): T-pose, textured (PBR off), 40,000-triangle target, automatic symmetry, from one front T-pose view",
    terms:
      "A paid generation: fal lists the model for commercial use. Listed price $1.20 (textured).",
  },
};

// The airframe skin: Hunyuan3D 3.1 Pro, geometry only, from four guide views.
export const AIRFRAME = {
  endpoint: "fal-ai/hunyuan-3d/v3.1/pro/image-to-3d",
  options: { generate_type: "Geometry", face_count: 300000 },
  // The right side is its front, the left its back.
  views: {
    right: "input_image_url",
    left: "back_image_url",
    top: "top_image_url",
    bottom: "bottom_image_url",
  },
  // $0.375 a generation, plus $0.15 for multi-view input and $0.15 for a custom face count.
  price_usd: 0.675,
  generator:
    "Hunyuan3D 3.1 Pro image to 3D, through Bento's fal service proxy (fal-ai/hunyuan-3d/v3.1/pro/image-to-3d), geometry only, 300,000 faces, from four views: the right side as its front, the left side as its back, the top and the bottom",
  terms:
    "A paid generation: fal lists the model for commercial use, and its terms leave output content to the customer.",
};

/** The generator settings behind a ledger line's endpoint. */
export const byEndpoint = (endpoint) =>
  Object.values(GENERATORS).find((g) => g.endpoint === endpoint);
