// WGSL owner: ground_params.

// @block ground_params_RoadLook
struct RoadLook {
  core: vec4f,
  worn: vec4f,
  shape: vec4f,
  join: vec4f,
  shoulder: vec4f,
  edge: vec4f,
  ruts: vec4f,
  track: vec4f,
  slabs: vec4f,
  curb: vec4f,
  paint: vec4f,
  marks: vec4f,
  crossing: vec4f,
  stones: vec4f,
}

// @block ground_params_ShoreTuning
struct ShoreTuning {
  bands: vec4f,
  turns: vec4f,
  ground: vec4f,
  fade: vec4f,
}

// @block ground_params_TerrainParams
struct TerrainParams {
  region: vec4f,
  field: vec4f,
  fieldGrid: vec4u,
  shape: vec4f,
  verge: vec4f,
  feathers: vec4f,
  roads: array<RoadLook, ROAD_LOOKS_MAX>,
  roadOrder: vec4u,
  forestLitter: vec4f,
  forestMoss: vec4f,
  forestHumus: vec4f,
  forestDetail: vec4f,
  forestVerge: vec4f,
  forestDapple: vec4f,
  forestLine: vec4f,
  waterBed: vec4f,
  shore: vec4f,
  shoreEarth: vec4f,
  shoreEdge: vec4f,
  distant: vec4f,
  shoreTuning: ShoreTuning,
  waterLooks: array<WaterLook, WATER_LOOKS_MAX>,
  view: vec4u,
}

// @block ground_params_params
@group(1) @binding(0) var<uniform> params: TerrainParams;
