import { tgpu, d } from "typegpu";

/** Observe normalized-f32 bits before the product: otherwise an inlined
 * cubic tap's /size then *size can cancel its hardware UV rounding boundary.
 * The decoded mantissa product still rounds to f32, as hardware does. */
const scarFilterAxis = tgpu.fn(
  [d.f32, d.f32],
  d.u32,
)(/* wgsl */ `(uv:f32,size:f32)->u32 {
 let bits=bitcast<u32>(clamp(uv,0.0,1.0))&0x7fffffffu;let exponent=bits>>23u;
 let mantissa=(bits&0x7fffffu)|select(0u,0x800000u,exponent!=0u);
 let product=bitcast<u32>(f32(mantissa)*size);
 let value=(product&0x7fffffu)|select(0u,0x800000u,product>>23u!=0u);
 // Each decoded f32 mantissa has exponent bias 150; Q8 adds eight bits.
 let shift=292-i32(exponent)-i32(product>>23u);
 if(shift>=32){return 0u;}if(shift<=0){return value<<u32(-shift);}
 return (value+(1u<<u32(shift-1)))>>u32(shift);
}`);
export const scarFilterPosition = tgpu
  .fn(
    [d.vec2f, d.vec2f],
    d.vec2f,
  )(/* wgsl */ `(uv:vec2f,size:vec2f)->vec2f {
 let q=vec2f(f32(scarFilterAxis(uv.x,size.x)),f32(scarFilterAxis(uv.y,size.y)));
 return clamp((q-128.0)/256.0,vec2f(0.0),size-1.0);
}`)
  .$uses({ scarFilterAxis });

/** The RGBA8 bilinear filter's arithmetic, as hardware was measured to do it,
 * done in the shader from loaded texels: the scars never use a sampler. */
export const scarFilterQuanta = tgpu.fn(
  [d.vec4u, d.vec4u, d.vec4u, d.vec4u, d.vec2f],
  d.vec4u,
)(/* wgsl */ `(a:vec4u,b:vec4u,c:vec4u,other:vec4u,f:vec2f)->vec4u {
 let wx=u32(floor(f.x*256.0+0.5));let wy=u32(floor(f.y*256.0+0.5));
 let total=a*((256u-wx)*(256u-wy))+b*(wx*(256u-wy))+c*((256u-wx)*wy)+other*(wx*wy);
 return (total+vec4u(2048u))/vec4u(4096u);
}`);
