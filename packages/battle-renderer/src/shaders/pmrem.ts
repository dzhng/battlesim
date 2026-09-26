/** Pinned Three 0.185.1 PMREM CubeUV/GGX algorithms, MIT (./LICENSE.three).
 * Resource allocation and commands belong to the environment owner. */
export const cubeUvFunctions = {
  cubeFace: `(d:vec3f)->f32 {
  let a=abs(d);
  if(a.x>a.z) {
    if(a.x>a.y){return select(3.0,0.0,d.x>0.0);}
    return select(4.0,1.0,d.y>0.0);
  }
  if(a.z>a.y){return select(5.0,2.0,d.z>0.0);}
  return select(4.0,1.0,d.y>0.0);
}`,
  cubeUv: `(d:vec3f,face:f32)->vec2f {
  var p=vec2f(0);
  if(face==0){p=vec2f(d.z,d.y)/abs(d.x);}
  else if(face==1){p=vec2f(-d.x,-d.z)/abs(d.y);}
  else if(face==2){p=vec2f(-d.x,d.y)/abs(d.z);}
  else if(face==3){p=vec2f(-d.z,d.y)/abs(d.x);}
  else if(face==4){p=vec2f(-d.x,d.z)/abs(d.y);}
  else{p=vec2f(d.x,d.y)/abs(d.z);}
  return 0.5*(p+vec2f(1));
}`,
  cubeDirection: `(uv:vec2f,face:f32)->vec3f {
  let p=uv*2.0-vec2f(1);
  if(face==0){return vec3f(1,p.y,p.x);}
  if(face==1){return vec3f(-p.x,1,-p.y);}
  if(face==2){return vec3f(-p.x,p.y,1);}
  if(face==3){return vec3f(-1,p.y,-p.x);}
  if(face==4){return vec3f(-p.x,-1,p.y);}
  return vec3f(p.x,p.y,-1);
}`,
  cubeSample: `(env:texture_2d<f32>,linear:sampler,d:vec3f,mip:f32,maxMip:f32)->vec3f {
  var face=cubeFace(d);
  let filterIndex=max(4.0-mip,0.0);
  let size=exp2(max(mip,4.0));
  var uv=cubeUv(d,face)*(size-2.0)+vec2f(1);
  if(face>2.0){uv.y+=size;face-=3.0;}
  uv.x+=face*size;
  uv.x+=filterIndex*48.0;
  uv.y+=4.0*(exp2(maxMip)-size);
  uv/=vec2f(textureDimensions(env));
  return textureSampleGrad(env,linear,uv,vec2f(0),vec2f(0)).rgb;
}`,
  roughnessMip: `(r:f32)->f32 {
  if(r>=0.8){return (1.0-r)/0.2-2.0;}
  if(r>=0.4){return (0.8-r)*3.0/0.4-1.0;}
  if(r>=0.305){return (0.4-r)/0.095+2.0;}
  if(r>=0.21){return (0.305-r)/0.095+3.0;}
  return -2.0*log2(1.16*r);
}`,
  samplePmrem: `(env:texture_2d<f32>,linear:sampler,d:vec3f,r:f32,maxMip:f32)->vec3f {
  let mip=clamp(roughnessMip(r),-2.0,maxMip);
  let f=fract(mip);
  let a=cubeSample(env,linear,d,floor(mip),maxMip);
  if(f==0){return a;}
  return mix(a,cubeSample(env,linear,d,floor(mip)+1.0,maxMip),f);
}`,
} as const;

export const cubeUvWGSL = Object.entries(cubeUvFunctions)
  .map(([name, body]) => `fn ${name}${body}`)
  .join("\n");

export const ggxFunctions = {
  radicalInverse: `(i:u32)->f32 {
  var b=(i<<16u)|(i>>16u);
  b=((b&0x55555555u)<<1u)|((b&0xAAAAAAAAu)>>1u);
  b=((b&0x33333333u)<<2u)|((b&0xCCCCCCCCu)>>2u);
  b=((b&0x0F0F0F0Fu)<<4u)|((b&0xF0F0F0F0u)>>4u);
  b=((b&0x00FF00FFu)<<8u)|((b&0xFF00FF00u)>>8u);
  return f32(b)*2.3283064365386963e-10;
}`,
  importanceGGX: `(xi:vec2f,r:f32)->vec3f {
  let alpha=r*r;
  let radius=sqrt(xi.x);
  let phi=(2.0*3.14159265359)*xi.y;
  let t1=radius*cos(phi);
  // PMREM views along the surface normal: tangent-space V=(0,0,1), s=1.
  let t2=radius*sin(phi);
  let nh=vec3f(t1,t2,sqrt(max(0.0,1.0-(t1*t1+t2*t2))));
  return normalize(vec3f(alpha*nh.x,alpha*nh.y,max(0.0,nh.z)));
}`,
  convolve: `(env:texture_2d<f32>,linear:sampler,n:vec3f,r:f32,mip:f32,maxMip:f32)->vec3f {
  if(r<0.001){return cubeSample(env,linear,n,mip,maxMip);}
  let up=select(vec3f(1,0,0),vec3f(0,0,1),abs(n.z)<0.999);
  let tangent=normalize(cross(up,n));
  let bitangent=cross(n,tangent);
  var color=vec3f(0);var weight=0.0;
  for(var i=0u;i<512u;i++) {
    let xi=vec2f(f32(i)/512.0,radicalInverse(i));
    let ht=importanceGGX(xi,r);
    let h=normalize(tangent*ht.x+bitangent*ht.y+n*ht.z);
    let l=normalize(h*(dot(n,h)*2.0)-n);
    let nl=max(dot(n,l),0.0);
    if(nl>0.0){color+=cubeSample(env,linear,l,mip,maxMip)*nl;weight+=nl;}
  }
  if(weight>0.0){color/=weight;}
  return color;
}`,
} as const;

export const ggxConvolutionWGSL = Object.entries(ggxFunctions)
  .map(([name, body]) => `fn ${name}${body}`)
  .join("\n");

/** The six padded quads exactly follow the pinned WebGPU PMREM face layout. */
export function pmremPlanes(size: number): Float32Array<ArrayBuffer> {
  const data = new Float32Array(36 * 6);
  const border = 1 / (size - 2);
  const uv = [
    -border,
    -border,
    1 + border,
    -border,
    1 + border,
    1 + border,
    -border,
    -border,
    1 + border,
    1 + border,
    -border,
    1 + border,
  ];
  const faces = [3, 1, 5, 0, 4, 2];
  for (let face = 0; face < 6; face++) {
    const x = ((face % 3) * 2) / 3 - 1,
      y = face > 2 ? 0 : -1;
    const xy = [x, y, x + 2 / 3, y, x + 2 / 3, y + 1, x, y, x + 2 / 3, y + 1, x, y + 1];
    for (let v = 0; v < 6; v++)
      data.set(
        [xy[v * 2], xy[v * 2 + 1], 0, uv[v * 2], uv[v * 2 + 1], faces[face]],
        (faces[face] * 6 + v) * 6,
      );
  }
  return data;
}
