// Diagnostic entry points call the production terrain material's functions.
@group(0) @binding(1) var<storage, read> terrainQueries: array<vec4f>;
@group(0) @binding(2) var<storage, read_write> terrainResults: array<vec4f>;
@group(0) @binding(3) var terrainReference: texture_2d<f32>;
@group(0) @binding(4) var terrainReferenceSampler: sampler;

@compute @workgroup_size(64) fn terrainDistances(@builtin(global_invocation_id) gid: vec3u) {
  if (gid.x >= arrayLength(&terrainQueries)) { return; }
  let query = terrainQueries[gid.x];
  let cell = groundCell(query.xy, query.z);
  let site = groundSite(query.xy, cell, groundPaved(query.xy, cell));
  terrainResults[gid.x] = vec4f(site.z, site.w, groundWater(query.xy, cell), 0.0);
}

@compute @workgroup_size(64) fn terrainPaved(@builtin(global_invocation_id) gid: vec3u) {
  if (gid.x >= arrayLength(&terrainQueries)) { return; }
  let query = terrainQueries[gid.x];
  let cell = groundCell(query.xy, query.z);
  let paved = groundPaved(query.xy, cell);
  let site = groundSite(query.xy, cell, paved);
  let actual = groundColour(query.xy, 0.1, site, paved, -1e9);
  var interiorDrawn = vec4f(-1e9);
  interiorDrawn[u32(query.w)] = 5.0;
  let interior = groundColour(query.xy, 0.1, site,
    GroundPaved(interiorDrawn, vec4f(0.0), vec4f(0.0,0.0,0.0,-1.0),5.0,vec2u(0u),-1.0), -1e9);
  terrainResults[gid.x] = vec4f(site.z, actual.w, interior.w, site.w);
}

// Independent cubic-filter reference; keep separate from the production filter.
fn originalTerrainCubic(uv: vec2f) -> vec4f {
  let size=vec2f(textureDimensions(terrainReference));let p=uv*size-0.5;let i=floor(p);let f=p-i;
  let f2=f*f;let f3=f2*f;let w0=(1.0-3.0*f+3.0*f2-f3)/6.0;let w1=(4.0-6.0*f2+3.0*f3)/6.0;let w2=(1.0+3.0*f+3.0*f2-3.0*f3)/6.0;let w3=f3/6.0;
  let g0=w0+w1;let g1=w2+w3;let h0=(i-0.5+w1/g0)/size;let h1=(i+1.5+w3/g1)/size;
  let a=textureSampleLevel(terrainReference,terrainReferenceSampler,vec2f(h0.x,h0.y),0.0);let b=textureSampleLevel(terrainReference,terrainReferenceSampler,vec2f(h1.x,h0.y),0.0);let c=textureSampleLevel(terrainReference,terrainReferenceSampler,vec2f(h0.x,h1.y),0.0);let other=textureSampleLevel(terrainReference,terrainReferenceSampler,vec2f(h1.x,h1.y),0.0);
  return g0.y*(g0.x*a+g1.x*b)+g1.y*(g0.x*c+g1.x*other);
}

// Select the same production scar tiles for an explicit normalized sample.
fn terrainLinear(uv: vec2f) -> vec4f {
  let last=vec2i(scarParams.pages.xy)-1;
  let cell=vec2i(floor(scarFilterPosition(uv,vec2f(scarParams.pages.xy))));
  return scarLinear(uv,scarTiles(vec2u(clamp(cell,vec2i(0),last)),vec2u(clamp(cell+1,vec2i(0),last))));
}

@compute @workgroup_size(64) fn terrainFilters(@builtin(global_invocation_id) gid: vec3u) {
  if (gid.x >= arrayLength(&terrainQueries)) { return; }
  let uv = terrainQueries[gid.x].xy;
  let cubic = groundCubic(uv);
  var taps = array<vec4f,4>();
  for(var k=0u;k<4u;k++) { taps[k]=terrainLinear(groundCubicTap(cubic,k)); }
  terrainResults[gid.x*4u] = groundCubicBlend(cubic,taps[0],taps[1],taps[2],taps[3]);
  terrainResults[gid.x*4u+1u] = originalTerrainCubic(uv);
  terrainResults[gid.x*4u+2u] = terrainLinear(uv);
  terrainResults[gid.x*4u+3u] = textureSampleLevel(terrainReference,terrainReferenceSampler,uv,0.0);
}
