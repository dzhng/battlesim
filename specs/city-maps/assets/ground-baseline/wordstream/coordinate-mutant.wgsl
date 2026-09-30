@group(0) @binding(0) var<storage, read> points: array<vec2f>;

@group(0) @binding(1) var<storage, read_write> output: array<vec4f>;

struct ScarParams {
  grid: vec4f,
  pages: vec4u,
  clip: vec4f,
  cacheBounds: vec4u,
  defaultWord: vec4u,
  full: vec4f,
  crater: vec4f,
  ejecta: vec4f,
  scorch: vec4f,
  tracks: vec4f,
  trampled: vec4f,
  relief: vec4f,
  grass: vec4f,
}

@group(1) @binding(5) var<uniform> scarParams: ScarParams;

fn scarFilterPosition(uv: vec2f, size: vec2f) -> vec2f {
 let q=floor(uv*(size*256.0)+0.5);
 return clamp((q-128.0)/256.0,vec2f(0.0),size-1.0);
}

@group(1) @binding(6) var scarPages: texture_2d<u32>;

fn scarEntry(tile: vec2u) -> vec2u {
 let P=scarParams;
 if(any(tile<P.cacheBounds.xy)||any(tile>P.cacheBounds.zw)){return vec2u(0u);}
 let tilesX=(P.pages.x+15u)/16u;let key=tile.y*tilesX+tile.x;
 var at=((key*0x9e3779b1u)^(key>>16u))&P.pages.z;
 loop {let dim=textureDimensions(scarPages);let e=textureLoad(scarPages,vec2i(i32(at%dim.x),i32(at/dim.x)),0).xy;
  if(e.x==0u){return vec2u(2147483648u,P.defaultWord.x);}
  if((e.x&536870911u)==key+1u){return e;}at=(at+1u)&P.pages.z;}
}

@group(1) @binding(7) var scars: texture_2d_array<f32>;

fn scarPoolWord(offset: u32) -> vec4f {
 let width=scarParams.pages.w;
 return textureLoad(scars,vec2i(i32(offset%width),i32(offset/width)),0,0);
}

fn scarNormalized(q: u32) -> f32 {
 let bytes=vec4u(round(scarPoolWord(q)*255.0));return bitcast<f32>(bytes.x|(bytes.y<<8u)|(bytes.z<<16u)|(bytes.w<<24u));
}

fn scarWord(word: u32) -> vec4f {
 return vec4f(scarNormalized((word&255u)*16u),scarNormalized(((word>>8u)&255u)*16u),scarNormalized(((word>>16u)&255u)*16u),scarNormalized((word>>24u)*16u));
}

fn scarCell(cell: vec2i) -> vec4f {
 let P=scarParams;let c=vec2u(clamp(cell,vec2i(0),vec2i(P.pages.xy)-1));let e=scarEntry(c/16u);
 if(e.x==0u){return vec4f(0.0);}if((e.x&2147483648u)!=0u){return scarWord(e.y);}
 let local=(c.y%16u)*16u+c.x%16u;
 if((e.x&536870912u)!=0u){return scarPoolWord(e.y+local);}
 let offset=e.y>>8u;var lo=0u;var hi=e.y&255u;
 while(lo<hi){let mid=(lo+hi)/2u;let bytes=vec4u(round(scarPoolWord(offset+mid*2u)*255.0));let end=bytes.x+bytes.y*256u;if(end<=local){lo=mid+1u;}else{hi=mid;}}
 return scarPoolWord(offset+lo*2u+1u);
}

fn scarFilterQuanta(a: vec4u, b: vec4u, c: vec4u, other: vec4u, f: vec2f) -> vec4u {
 let wx=u32(floor(f.x*256.0+0.5));let wy=u32(floor(f.y*256.0+0.5));
 let total=a*((256u-wx)*(256u-wy))+b*(wx*(256u-wy))+c*((256u-wx)*wy)+other*(wx*wy);
 return (total+vec4u(2048u))/vec4u(4096u);
}

fn sampleGroundLinear(uv: vec2f) -> vec4f {
 let P=scarParams;let size=vec2f(P.pages.xy);let p=scarFilterPosition(uv,size);
 let cell=vec2i(floor(p));let f=fract(p);let e=scarEntry(vec2u(cell)/16u);
 if((e.x==0u||(e.x&2147483648u)!=0u)&&all(vec2u(cell)%16u<vec2u(15u))){return scarWord(e.y);}
 let a=vec4u(round(scarCell(cell)*255.0));let b=vec4u(round(scarCell(cell+vec2i(1,0))*255.0));let c=vec4u(round(scarCell(cell+vec2i(0,1))*255.0));let other=vec4u(round(scarCell(cell+vec2i(1,1))*255.0));
 let q=scarFilterQuanta(a,b,c,other,f);
 return vec4f(scarNormalized(q.x),scarNormalized(q.y),scarNormalized(q.z),scarNormalized(q.w));
}

fn sampleGroundCubic(uv: vec2f) -> vec4f {
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
  let h0 = (i - 0.5 + w1 / g0) / size;
  let h1 = (i + 1.5 + w3 / g1) / size;
  let a = sampleGroundLinear(vec2f(h0.x, h0.y));
  let b = sampleGroundLinear(vec2f(h1.x, h0.y));
  let c = sampleGroundLinear(vec2f(h0.x, h1.y));
  let e = sampleGroundLinear(vec2f(h1.x, h1.y));
  return fma(vec4f(g0.y), g0.x * a + g1.x * b, g1.y * (g0.x * c + g1.x * e));
}

@group(0) @binding(2) var reference: texture_2d<f32>;

@group(0) @binding(3) var sampler_1: sampler;

fn original(uv: vec2f) -> vec4f {
          let size=vec2f(textureDimensions(reference));let p=uv*size-0.5;let i=floor(p);let f=p-i;
          let f2=f*f;let f3=f2*f;let w0=(1.0-3.0*f+3.0*f2-f3)/6.0;let w1=(4.0-6.0*f2+3.0*f3)/6.0;let w2=(1.0+3.0*f+3.0*f2-3.0*f3)/6.0;let w3=f3/6.0;
          let g0=w0+w1;let g1=w2+w3;let h0=(i-0.5+w1/g0)/size;let h1=(i+1.5+w3/g1)/size;
          let a=textureSampleLevel(reference,sampler_1,vec2f(h0.x,h0.y),0.0);let b=textureSampleLevel(reference,sampler_1,vec2f(h1.x,h0.y),0.0);let c=textureSampleLevel(reference,sampler_1,vec2f(h0.x,h1.y),0.0);let other=textureSampleLevel(reference,sampler_1,vec2f(h1.x,h1.y),0.0);
          return g0.y*(g0.x*a+g1.x*b)+g1.y*(g0.x*c+g1.x*other);
        }

@compute @workgroup_size(1) fn item(@builtin(global_invocation_id) gid: vec3u) {
          let uv=points[gid.x];output[gid.x*4u]=sampleGroundCubic(uv);output[gid.x*4u+1u]=original(uv);output[gid.x*4u+2u]=sampleGroundLinear(uv);output[gid.x*4u+3u]=textureSampleLevel(reference,sampler_1,uv,0.0);
        }