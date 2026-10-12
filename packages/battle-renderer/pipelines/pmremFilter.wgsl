struct VertexOut {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
  @location(1) face: f32,
}

struct VertexIn {
  @location(0) position: vec3f,
  @location(1) uv: vec2f,
  @location(2) face: f32,
}

@vertex fn vertex(_arg_0: VertexIn) -> VertexOut {
  return VertexOut(vec4f(_arg_0.position, 1f), _arg_0.uv, _arg_0.face);
}

fn cubeFace(d: vec3f) -> f32 {
  let a=abs(d);
  if(a.x>a.z) {
    if(a.x>a.y){return select(3.0,0.0,d.x>0.0);}
    return select(4.0,1.0,d.y>0.0);
  }
  if(a.z>a.y){return select(5.0,2.0,d.z>0.0);}
  return select(4.0,1.0,d.y>0.0);
}

fn cubeUv(d: vec3f, face: f32) -> vec2f {
  var p=vec2f(0);
  if(face==0){p=vec2f(d.z,d.y)/abs(d.x);}
  else if(face==1){p=vec2f(-d.x,-d.z)/abs(d.y);}
  else if(face==2){p=vec2f(-d.x,d.y)/abs(d.z);}
  else if(face==3){p=vec2f(-d.z,d.y)/abs(d.x);}
  else if(face==4){p=vec2f(-d.x,d.z)/abs(d.y);}
  else{p=vec2f(d.x,d.y)/abs(d.z);}
  return 0.5*(p+vec2f(1));
}

fn cubeSample(env: texture_2d<f32>, linear: sampler, d: vec3f, mip: f32, maxMip: f32) -> vec3f {
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
}

fn radicalInverse(i: u32) -> f32 {
  var b=(i<<16u)|(i>>16u);
  b=((b&0x55555555u)<<1u)|((b&0xAAAAAAAAu)>>1u);
  b=((b&0x33333333u)<<2u)|((b&0xCCCCCCCCu)>>2u);
  b=((b&0x0F0F0F0Fu)<<4u)|((b&0xF0F0F0F0u)>>4u);
  b=((b&0x00FF00FFu)<<8u)|((b&0xFF00FF00u)>>8u);
  return f32(b)*2.3283064365386963e-10;
}

fn importanceGGX(xi: vec2f, r: f32) -> vec3f {
  let alpha=r*r;
  let radius=sqrt(xi.x);
  let phi=(2.0*3.14159265359)*xi.y;
  let t1=radius*cos(phi);
  // PMREM views along the surface normal: tangent-space V=(0,0,1), s=1.
  let t2=radius*sin(phi);
  let nh=vec3f(t1,t2,sqrt(max(0.0,1.0-(t1*t1+t2*t2))));
  return normalize(vec3f(alpha*nh.x,alpha*nh.y,max(0.0,nh.z)));
}

fn convolve(env: texture_2d<f32>, linear: sampler, n: vec3f, r: f32, mip: f32, maxMip: f32) -> vec3f {
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
}

fn cubeDirection(uv: vec2f, face: f32) -> vec3f {
  let p=uv*2.0-vec2f(1);
  if(face==0){return vec3f(1,p.y,p.x);}
  if(face==1){return vec3f(-p.x,1,-p.y);}
  if(face==2){return vec3f(-p.x,p.y,1);}
  if(face==3){return vec3f(-1,p.y,-p.x);}
  if(face==4){return vec3f(-p.x,-1,p.y);}
  return vec3f(p.x,p.y,-1);
}

fn filterAlgorithm(uv: vec2f, face: f32, source: texture_2d<f32>, linear: sampler, params: vec4f) -> vec4f {
      return vec4f(convolve(source,linear,normalize(cubeDirection(uv,face)),params.x,params.y,params.z),1);
    }

@group(0) @binding(0) var source: texture_2d<f32>;

@group(0) @binding(1) var linear: sampler;

@group(0) @binding(2) var<uniform> params: vec4f;

fn filterShade(uv: vec2f, face: f32) -> vec4f {
  return filterAlgorithm(uv, face, source, linear, params);
}

struct FragmentIn {
  @location(0) uv: vec2f,
  @location(1) face: f32,
}

@fragment fn fragment(_arg_0: FragmentIn) -> @location(0) vec4f {
  return filterShade(_arg_0.uv, _arg_0.face);
}