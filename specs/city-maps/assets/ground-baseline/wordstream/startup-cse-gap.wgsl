@group(0) @binding(0) var<uniform> count: vec4u;

@group(0) @binding(1) var<storage, read> points: array<vec4f>;

@group(0) @binding(3) var reference: texture_2d_array<f32>;

struct CubicTaps {
  h0: vec2f,
  h1: vec2f,
  g0: vec2f,
  g1: vec2f,
}

fn scarCubicTaps(uv: vec2f, size: vec2f) -> CubicTaps {
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
  let h0 = (i - 0.5 + w1 / g0) / size;
  let h1 = (i + 1.5 + w3 / g1) / size;
  return CubicTaps(h0,h1,g0,g1);
}

fn scarFilterPosition(uv: vec2f, size: vec2f) -> vec2f {
 let q=floor(uv*(size*256.0)+0.5);
 return clamp((q-128.0)/256.0,vec2f(0.0),size-1.0);
}

@group(0) @binding(2) var<storage, read_write> output: array<vec4u>;

fn scarFilterQuanta(a: vec4u, b: vec4u, c: vec4u, other: vec4u, f: vec2f) -> vec4u {
 let wx=u32(floor(f.x*256.0+0.5));let wy=u32(floor(f.y*256.0+0.5));
 let total=a*((256u-wx)*(256u-wy))+b*(wx*(256u-wy))+c*((256u-wx)*wy)+other*(wx*wy);
 return (total+vec4u(2048u))/vec4u(4096u);
}

@group(0) @binding(4) var sampler_1: sampler;

@compute @workgroup_size(64) fn kernel(@builtin(global_invocation_id) gid: vec3u) {
 if(gid.x>=count.x){return;}let p=points[gid.x];let layer=i32(p.z);
 let size=vec2f(textureDimensions(reference));let hi=vec2i(size)-1;
 var uv=select((p.xy+0.5)/2.0,p.xy,count.y!=0u);
 if(count.y==2u){let taps=scarCubicTaps(p.xy,size);let tap=u32(p.w);uv=vec2f(select(taps.h0.x,taps.h1.x,(tap&1u)!=0u),select(taps.h0.y,taps.h1.y,(tap&2u)!=0u));}
 let position=scarFilterPosition(uv,size);let cell=vec2i(floor(position));
 let a=vec4u(round(textureLoad(reference,cell,layer,0)*255.0));
 let b=vec4u(round(textureLoad(reference,min(cell+vec2i(1,0),hi),layer,0)*255.0));
 let c=vec4u(round(textureLoad(reference,min(cell+vec2i(0,1),hi),layer,0)*255.0));
 let other=vec4u(round(textureLoad(reference,min(cell+vec2i(1,1),hi),layer,0)*255.0));
 output[gid.x*2u]=scarFilterQuanta(a,b,c,other,fract(position));
 output[gid.x*2u+1u]=bitcast<vec4u>(textureSampleLevel(reference,sampler_1,uv,layer,0.0));
}