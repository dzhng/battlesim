@group(0) @binding(0) var<uniform> count: vec4u;

@group(0) @binding(1) var<storage, read> points: array<vec4f>;

@group(0) @binding(3) var reference: texture_2d_array<f32>;

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

fn scarFilterPosition(uv: vec2f, size: vec2f) -> vec2f {
 let q=vec2f(f32(scarFilterAxis(uv.x,size.x)),f32(scarFilterAxis(uv.y,size.y)));
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
 let uv=select((p.xy+0.5)/2.0,p.xy,count.y!=0u);
 let position=scarFilterPosition(uv,size);let cell=vec2i(floor(position));
 let a=vec4u(round(textureLoad(reference,cell,layer,0)*255.0));
 let b=vec4u(round(textureLoad(reference,min(cell+vec2i(1,0),hi),layer,0)*255.0));
 let c=vec4u(round(textureLoad(reference,min(cell+vec2i(0,1),hi),layer,0)*255.0));
 let other=vec4u(round(textureLoad(reference,min(cell+vec2i(1,1),hi),layer,0)*255.0));
 output[gid.x*2u]=scarFilterQuanta(a,b,c,other,fract(position));
 output[gid.x*2u+1u]=bitcast<vec4u>(textureSampleLevel(reference,sampler_1,uv,layer,0.0));
}