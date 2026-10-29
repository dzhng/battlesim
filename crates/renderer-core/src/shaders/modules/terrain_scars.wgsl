// WGSL owner: terrain_scars.

// @block terrain_scars_ScarSample
struct ScarSample {
  weights: vec4f,
  relief: vec4f,
  at: vec4f,
}

// @block terrain_scars_ScarTiles
struct ScarTiles {
  lo: vec2u,
  e00: vec2u,
  e10: vec2u,
  e01: vec2u,
  e11: vec2u,
}

// @block terrain_scars_groundCubic
fn groundCubic(uv: vec2f) -> GroundCubic {
  let size = vec2f(scarParams.pages.xy);
  let p = uv * size - 0.5;
  let i = floor(p);
  let f = p - i;
  let f2 = f * f;
  let f3 = f2 * f;
  let w0 = (1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0;
  let w1 = (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0;
  let w2 = (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0;
  let w3 = f3 / 6.0;
  let g0 = w0 + w1;
  let g1 = w2 + w3;
  return GroundCubic(vec4f((i - 0.5 + w1 / g0) / size, (i + 1.5 + w3 / g1) / size), vec4f(g0, g1));
}

// @block terrain_scars_groundScars
fn groundScars(xy: vec2f, footprint: f32) -> ScarSample {
  var out: ScarSample;
  out.at = vec4f(xy, 0.0, 0.0);
  let P = scarParams;
  if (P.grid.w <= 0.0) { return out; }
  let cell = P.grid.z;
  let uv = xy * P.grid.xy;
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return out; }
  let texel = P.grid.xy * cell;
  // Every cell the taps below read is in these tiles: where none is marked,
  // every tap is zero, and the cheap reject would return the same.
  let centre = uv * vec2f(P.pages.xy) - 0.5;
  let last = vec2i(P.pages.xy) - 1;
  let tiles = scarTiles(
    vec2u(clamp(vec2i(floor(centre - 3.0)), vec2i(0), last)),
    vec2u(clamp(vec2i(floor(centre + 3.0)) + 1, vec2i(0), last)));
  if (scarTilesBlank(tiles)) { return out; }
  // Twelve bilinear taps through one call, as a shader compiler inlines each
  // call: the rim's four diagonals (a, b, c, e), the cubic's four, then
  // either side of `uv` along x and y for the slopes (xp, xm, yp, ym).
  let r = 1.5;
  let h = 0.75;
  let cubic = groundCubic(uv);
  var taps = array<vec4f, 12>();
  for (var t = 0u; t < 12u; t++) {
    if (t == 8u) {
      // Cheap reject: nothing marked within reach.
      let near = max(max(taps[0], taps[1]), max(taps[2], taps[3]));
      let s = groundCubicBlend(cubic, taps[4], taps[5], taps[6], taps[7]);
      if (max(max(s.x, s.y), max(s.z, s.w)) + max(max(near.x, near.y), max(near.z, near.w)) <= 0.0) {
        return out;
      }
    }
    var at: vec2f;
    if (t < 4u) {
      at = uv + texel * vec2f(select(-r, r, (t & 1u) == 1u), select(-r, r, t >= 2u));
    } else if (t < 8u) {
      at = groundCubicTap(cubic, t - 4u);
    } else {
      let side = texel * select(vec2f(0.0, h), vec2f(h, 0.0), t < 10u);
      at = select(uv - side, uv + side, (t & 1u) == 0u);
    }
    taps[t] = scarLinear(at, tiles);
  }
  let a = taps[0];
  let b = taps[1];
  let c = taps[2];
  let e = taps[3];
  let near = max(max(a, b), max(c, e));
  let s = groundCubicBlend(cubic, taps[4], taps[5], taps[6], taps[7]);
  let xp = taps[8];
  let xm = taps[9];
  let yp = taps[10];
  let ym = taps[11];
  let detail = 1.0 - smoothstep(0.5, 2.0, footprint / cell);
  let k = P.full;
  let depth = min(s.x * k.x, 1.6);
  let wide = min((a.x + b.x + c.x + e.x) * 0.25 * k.x, 1.6);
  // Slopes per metre of the crater field, from the taps either side.
  let bowlSlope = vec2f(xp.x - xm.x, yp.x - ym.x) * k.x / (2.0 * h * cell);
  let wideSlope = vec2f((b.x + e.x) - (a.x + c.x), (c.x + e.x) - (a.x + b.x)) * 0.5 * k.x / (2.0 * r * cell);
  // The height: down into the bowl, up onto the rim just outside it.
  let onRim = select(0.0, 1.0, wide > depth);
  let slope = (P.relief.y * onRim * (wideSlope - bowlSlope) - bowlSlope) * P.relief.x * detail;
  // Iso-lines, each crossing within a pixel.
  let grain = valueNoise(xy * 2.2222222222222223 + vec2f(5.7, 2.1)) - 0.5;
  let pixelDepth = length(bowlSlope) * footprint;
  let bowl = crossing(depth, 0.3 + 0.14 * grain, pixelDepth);
  let skirt = crossing(depth, 0.035 + 0.03 * grain, pixelDepth) * (1.0 - bowl);
  // The deepest crater within the taps: a lone pock's skirt is scorched
  // earth, a real crater's is its thrown soil.
  let peak = max(max(max(s.x, xp.x), max(xm.x, yp.x)), max(ym.x, near.x)) * k.x;
  let soil = smoothstep(0.05, 0.09, peak);
  let ring = skirt * soil;
  let scorched = crossing(depth, 0.013 + 0.006 * grain, pixelDepth) * (1.0 - bowl);
  // Darkest at its heart, browner toward its edge, as a burn is.
  let heart = 0.45 + 0.55 * smoothstep(0.013, 0.035, depth);
  let pock = scorched * (1.0 - soil) * 0.85 * heart;
  // Soot: ash in flecks over the scorch, thickest near the burst, never a
  // solid sheet: a flecked dark is one thing no shadow here is.
  let burnt = saturate(s.y * k.y);
  let fleckAt = valueNoise(xy * 4.5 + vec2f(8.1, 3.3)) * 0.75 + valueNoise(xy * 13.950000000000001) * 0.25;
  let ash = crossing(fleckAt, mix(0.97, 0.72, burnt), footprint * 4.5) * step(0.45, burnt);
  let soot = max(ash * (1.0 - bowl) * (1.0 - ring), pock);
  let wear = 1.0 - (1.0 - saturate(s.zw * k.zw)) * (1.0 - saturate(s.zw * k.zw));
  // Ruts have edges: tracks cross their iso-line within a pixel, ragged.
  let trackSlope = vec2f(xp.z - xm.z, yp.z - ym.z) * k.z / (2.0 * h * cell);
  let rut = crossing(wear.x, 0.35 + 0.2 * grain, length(trackSlope) * footprint);
  out.weights = vec4f(bowl, soot, rut * wear.x, wear.y);
  out.relief = vec4f(slope, ring, depth);
  return out;
}

// @block terrain_scars_scarCell
fn scarCell(c: vec2u, e: vec2u) -> vec4f {
 if(e.x==0u){return vec4f(0.0);}if((e.x&2147483648u)!=0u){return scarWord(e.y);}
 let local=(c.y%16u)*16u+c.x%16u;
 if((e.x&536870912u)!=0u){return scarPoolWord(e.y+local);}
 let offset=e.y>>8u;var lo=0u;var hi=e.y&255u;
 while(lo<hi){let mid=(lo+hi)/2u;let bytes=vec4u(round(scarPoolWord(offset+mid*2u)*255.0));let end=bytes.x+bytes.y*256u;if(end<=local){lo=mid+1u;}else{hi=mid;}}
 return scarPoolWord(offset+lo*2u+1u);
}

// @block terrain_scars_scarEntry
fn scarEntry(tile: vec2u) -> vec2u {
 let P=scarParams;
 if(any(tile<P.cacheBounds.xy)||any(tile>P.cacheBounds.zw)){return vec2u(0u);}
 let tilesX=(P.pages.x+15u)/16u;let key=tile.y*tilesX+tile.x;
 var at=((key*0x9e3779b1u)^(key>>16u))&P.pages.z;
 loop {let dim=textureDimensions(scarPages);let e=textureLoad(scarPages,vec2i(i32(at%dim.x),i32(at/dim.x)),0).xy;
  if(e.x==0u){break;}
  if((e.x&536870911u)==key+1u){return e;}at=(at+1u)&P.pages.z;}
 return vec2u(2147483648u,P.defaultWord.x);
}

// @block terrain_scars_scarFilterAxis
fn scarFilterAxis(uv: f32, size: f32) -> u32 {
 let bits=bitcast<u32>(clamp(uv,0.0,1.0))&0x7fffffffu;let exponent=bits>>23u;
 let mantissa=(bits&0x7fffffu)|select(0u,0x800000u,exponent!=0u);
 let product=bitcast<u32>(f32(mantissa)*size);
 let value=(product&0x7fffffu)|select(0u,0x800000u,product>>23u!=0u);
 // Each decoded f32 mantissa has exponent bias 150; Q8 adds eight bits.
 let shift=292-i32(exponent)-i32(product>>23u);
 if(shift>=32){return 0u;}if(shift<=0){return value<<u32(-shift);}
 return (value+(1u<<u32(shift-1)))>>u32(shift);
}

// @block terrain_scars_scarFilterPosition
fn scarFilterPosition(uv: vec2f, size: vec2f) -> vec2f {
 let q=vec2f(f32(scarFilterAxis(uv.x,size.x)),f32(scarFilterAxis(uv.y,size.y)));
 return clamp((q-128.0)/256.0,vec2f(0.0),size-1.0);
}

// @block terrain_scars_scarFilterQuanta
fn scarFilterQuanta(a: vec4u, b: vec4u, c: vec4u, other: vec4u, f: vec2f) -> vec4u {
 let wx=u32(floor(f.x*256.0+0.5));let wy=u32(floor(f.y*256.0+0.5));
 let total=a*((256u-wx)*(256u-wy))+b*(wx*(256u-wy))+c*((256u-wx)*wy)+other*(wx*wy);
 return (total+vec4u(2048u))/vec4u(4096u);
}

// @block terrain_scars_scarLinear
fn scarLinear(uv: vec2f, tiles: ScarTiles) -> vec4f {
 let P=scarParams;let size=vec2f(P.pages.xy);let p=scarFilterPosition(uv,size);
 let cell=vec2i(floor(p));let f=fract(p);var q=array<vec4u,4>();
 for(var k=0u;k<4u;k++){
  let c=vec2u(clamp(cell+vec2i(i32(k&1u),i32(k>>1u)),vec2i(0),vec2i(P.pages.xy)-1));let e=scarTileEntry(tiles,c/16u);
  // The cell's own tile, one word throughout, with all four corners in it.
  if(k==0u&&(e.x==0u||(e.x&2147483648u)!=0u)&&all(c%16u<vec2u(15u))){return scarWord(e.y);}
  q[k]=vec4u(round(scarCell(c,e)*255.0));
 }
 let w=scarFilterQuanta(q[0],q[1],q[2],q[3],f);
 return vec4f(scarNormalized(w.x),scarNormalized(w.y),scarNormalized(w.z),scarNormalized(w.w));
}

// @block terrain_scars_scarNormalized
fn scarNormalized(q: u32) -> f32 {
 let bytes=vec4u(round(scarPoolWord(q)*255.0));return bitcast<f32>(bytes.x|(bytes.y<<8u)|(bytes.z<<16u)|(bytes.w<<24u));
}

// @block terrain_scars_scarPages
@group(1) @binding(6) var scarPages: texture_2d<u32>;

// @block terrain_scars_scarParams
@group(1) @binding(5) var<uniform> scarParams: ScarParams;

// @block terrain_scars_scarPoolWord
fn scarPoolWord(offset: u32) -> vec4f {
 let width=scarParams.pages.w;
 return textureLoad(scars,vec2i(i32(offset%width),i32(offset/width)),0,0);
}

// @block terrain_scars_scarRegionContains
fn scarRegionContains(world: vec2f) -> bool {
  let r = (&scarParams.clip);
  return (((*r).z <= (*r).x) || ((((world.x >= (*r).x) && (world.y >= (*r).y)) && (world.x < (*r).z)) && (world.y < (*r).w)));
}

// @block terrain_scars_scarTileEntry
fn scarTileEntry(t: ScarTiles, tile: vec2u) -> vec2u {
 let o=tile-t.lo;
 return select(select(t.e00,t.e10,o.x==1u),select(t.e01,t.e11,o.x==1u),o.y==1u);
}

// @block terrain_scars_scarTiles
fn scarTiles(lo: vec2u, hi: vec2u) -> ScarTiles {
 let a=lo/16u;let span=hi/16u-a;var e=array<vec2u,4>();
 // One probe site in a loop, as for the corners in `scarLinear`; a tile
 // that repeats where the cells span one is copied, not probed again.
 for(var k=0u;k<4u;k++){
  let o=min(vec2u(k&1u,k>>1u),span);let j=o.x+o.y*2u;
  if(j<k){e[k]=e[j];}else{e[k]=scarEntry(a+o);}
 }
 return ScarTiles(a,e[0],e[1],e[2],e[3]);
}

// @block terrain_scars_scarTilesBlank
fn scarTilesBlank(t: ScarTiles) -> bool {
 let x=vec4u(t.e00.x,t.e10.x,t.e01.x,t.e11.x);let y=vec4u(t.e00.y,t.e10.y,t.e01.y,t.e11.y);
 let single=(x&vec4u(2147483648u))!=vec4u(0u);
 return all((x==vec4u(0u))|(single&(y==vec4u(0u))));
}

// @block terrain_scars_scarWord
fn scarWord(word: u32) -> vec4f {
 return vec4f(scarNormalized((word&255u)*16u),scarNormalized(((word>>8u)&255u)*16u),scarNormalized(((word>>16u)&255u)*16u),scarNormalized((word>>24u)*16u));
}

// @block terrain_scars_scarredNormal
fn scarredNormal(n: vec3f, scar: ScarSample) -> vec3f {
  // At most 40° of tilt: steeper walls turn to the blue sky and read as haze.
  let slope = scar.relief.xy;
  let steep = length(slope);
  let capped = slope * min(1.0, 0.84 / max(steep, 1e-6));
  return normalize(n - vec3f(capped, 0.0) * n.z);
}

// @block terrain_scars_scars
@group(1) @binding(7) var scars: texture_2d_array<f32>;
